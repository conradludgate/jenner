use syn::spanned::Spanned;
use syn::{
    parse2, parse_quote, Attribute, Error, Expr, ExprLit, ItemFn, Lit, Result, Signature, Stmt,
    Type,
};

use crate::{
    gen_visit::GenVisitor,
    parse::{AttrGenerator, ExprGenerator},
};

impl ExprGenerator {
    pub fn process(mut self) -> Expr {
        GenVisitor::new(false).into_generator(&mut self.stmts)
    }
}

impl AttrGenerator {
    pub fn process(mut self) -> Result<ItemFn> {
        let ItemFn {
            attrs, sig, block, ..
        } = &mut self.func;

        let return_ty = Self::parse_return_ty(sig);
        let yield_ty = Self::parse_yield_ty(attrs)?;

        // unwrap yield type.
        let yield_ty = match yield_ty {
            Type::Paren(t) => *t.elem,
            y => y,
        };

        let sync = sig.asyncness.take().is_none();
        sig.output = if sync {
            parse_quote! { -> impl ::jenner::SyncGenerator<#yield_ty, #return_ty> }
        } else {
            parse_quote! { -> impl ::jenner::AsyncGenerator<#yield_ty, #return_ty> }
        };

        let mut visitor = GenVisitor::new(sync);
        visitor.yields = 1; // force yield inference
        block.stmts = vec![Stmt::Expr(visitor.into_generator(&mut block.stmts), None)];
        Ok(self.func)
    }

    fn parse_return_ty(sig: &Signature) -> Type {
        match &sig.output {
            syn::ReturnType::Default => parse_quote! { () },
            syn::ReturnType::Type(_, t) => (**t).clone(),
        }
    }

    fn parse_yield_ty(attrs: &mut Vec<Attribute>) -> Result<Type> {
        let yields = attrs
            .drain_filter(|attr| attr.path().get_ident().map_or(false, |i| i == "yields"))
            .next();
        match yields {
            Some(t) => match t.meta {
                syn::Meta::Path(path) => Err(Error::new(path.span(), "needs a value")),
                syn::Meta::List(list) => parse2(list.tokens),
                syn::Meta::NameValue(nv) => match nv.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(ty), ..
                    }) => ty.parse(),
                    _ => Err(Error::new(
                        nv.span(),
                        "needs a string value representing a type",
                    )),
                },
            },
            None => Ok(parse_quote! { ! }),
        }
    }
}

use syn::{parse2, parse_quote, Attribute, Expr, ItemFn, Result, Signature, Stmt, Type};

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
        block.stmts = vec![Stmt::Expr(visitor.into_generator(&mut block.stmts))];
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
            .drain_filter(|attr| attr.path.get_ident().map_or(false, |i| i == "yields"))
            .next();
        match yields {
            Some(t) => parse2(t.tokens),
            None => Ok(parse_quote! { ! }),
        }
    }
}

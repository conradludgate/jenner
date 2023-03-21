use quote::format_ident;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    AssocType, Attribute, Error, GenericArgument, ItemFn, PathArguments, Result, Signature, Stmt,
    TraitBound, Type, TypeImplTrait, TypeParamBound, TypePath, TypeTuple,
};

use crate::{gen_visit::GenVisitor, parse::AttrGenerator};

impl AttrGenerator {
    pub fn process(mut self) -> Result<ItemFn> {
        let ItemFn {
            attrs, sig, block, ..
        } = &mut self.func;

        let return_ty = Self::parse_return_ty(sig);
        let yields = Self::parse_yields(attrs)?;

        let sync = sig.asyncness.take().is_none();

        let yield_ty = if yields {
            new_path! { ::jenner::__private::effective::Multiple }
        } else {
            new_path! { ::jenner::__private::effective::Single }
        };
        let async_ty = if sync {
            new_path! { ::jenner::__private::effective::Blocking }
        } else {
            new_path! { ::jenner::__private::effective::Async }
        };

        let mut effective = new_path!(::jenner::__private::effective::Effective);
        effective.segments.last_mut().unwrap().arguments =
            PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: [
                    GenericArgument::AssocType(AssocType {
                        ident: format_ident!("Item"),
                        generics: None,
                        eq_token: Default::default(),
                        ty: return_ty,
                    }),
                    GenericArgument::AssocType(AssocType {
                        ident: format_ident!("Failure"),
                        generics: None,
                        eq_token: Default::default(),
                        ty: Type::Path(TypePath {
                            qself: None,
                            path: new_path!(::std::convert::Infallible),
                        }),
                    }),
                    GenericArgument::AssocType(AssocType {
                        ident: format_ident!("Produces"),
                        generics: None,
                        eq_token: Default::default(),
                        ty: Type::Path(TypePath {
                            qself: None,
                            path: yield_ty,
                        }),
                    }),
                    GenericArgument::AssocType(AssocType {
                        ident: format_ident!("Async"),
                        generics: None,
                        eq_token: Default::default(),
                        ty: Type::Path(TypePath {
                            qself: None,
                            path: async_ty,
                        }),
                    }),
                ]
                .into_iter()
                .collect(),
                gt_token: Default::default(),
            });

        sig.output = syn::ReturnType::Type(
            Default::default(),
            Box::new(Type::ImplTrait(TypeImplTrait {
                impl_token: Default::default(),
                bounds: [TypeParamBound::Trait(TraitBound {
                    paren_token: None,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: None,
                    path: effective,
                })]
                .into_iter()
                .collect(),
            })),
        );

        // sig.output = parse_quote! { -> impl ::jenner::__private::effective::Effective<
        //     Item = #return_ty,
        //     Fallible = ::std::convert::Infallible,
        //     Produces = #yield_ty,
        //     Async = #async_ty,
        // > };

        let visitor = GenVisitor::new(sync, yields);
        block.stmts = vec![Stmt::Expr(visitor.into_generator(&mut block.stmts), None)];
        Ok(self.func)
    }

    fn parse_return_ty(sig: &Signature) -> Type {
        match &sig.output {
            syn::ReturnType::Default => Type::Tuple(TypeTuple {
                paren_token: Default::default(),
                elems: Punctuated::new(),
            }),
            syn::ReturnType::Type(_, t) => (**t).clone(),
        }
    }

    fn parse_yields(attrs: &mut Vec<Attribute>) -> Result<bool> {
        let yields = attrs
            .drain_filter(|attr| attr.path().get_ident().map_or(false, |i| i == "yields"))
            .next();
        match yields {
            None => Ok(false),
            Some(t) => match t.meta {
                syn::Meta::Path(_) => Ok(true),
                syn::Meta::List(ml) => Err(Error::new(ml.span(), "Value not expected")),
                syn::Meta::NameValue(nv) => Err(Error::new(nv.span(), "Value not expected")),
            },
        }
    }
}

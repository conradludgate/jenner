use std::mem;

use quote::format_ident;
use syn::punctuated::Punctuated;
use syn::{
    AssocType, GenericArgument, ItemFn, Path, PathArguments, Result, Signature, Stmt, TraitBound,
    Type, TypeImplTrait, TypeParamBound, TypePath, TypeTuple,
};

use crate::{gen_visit::GenVisitor, parse::AttrGenerator};

impl AttrGenerator {
    pub fn process(mut self) -> Result<ItemFn> {
        let ItemFn { sig, block, .. } = &mut self.func;
        let return_ty = Self::take_return_ty(sig);
        let sync = sig.asyncness.take().is_none();

        let yield_ty = if self.yields {
            new_path! { ::jenner::effective::Multiple }
        } else {
            new_path! { ::jenner::effective::Single }
        };
        let async_ty = if sync {
            new_path! { ::jenner::effective::Blocking }
        } else {
            new_path! { ::jenner::effective::Async }
        };
        let fallible_ty = if self.fallible {
            create_fallible_path(&return_ty)
        } else {
            new_path! { ::core::convert::Infallible }
        };
        let return_ty = if self.fallible {
            create_fallible_return_type(return_ty)
        } else {
            return_ty
        };

        sig.output = syn::ReturnType::Type(
            Default::default(),
            Box::new(Type::ImplTrait(TypeImplTrait {
                impl_token: Default::default(),
                bounds: [TypeParamBound::Trait(TraitBound {
                    paren_token: None,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: None,
                    path: create_impl_effective(return_ty, fallible_ty, yield_ty, async_ty),
                })]
                .into_iter()
                .collect(),
            })),
        );

        let visitor = GenVisitor::new(sync, self.yields, self.fallible);
        block.stmts = vec![Stmt::Expr(visitor.into_generator(&mut block.stmts), None)];
        Ok(self.func)
    }

    fn take_return_ty(sig: &mut Signature) -> Type {
        match mem::replace(&mut sig.output, syn::ReturnType::Default) {
            syn::ReturnType::Default => Type::Tuple(TypeTuple {
                paren_token: Default::default(),
                elems: Punctuated::new(),
            }),
            syn::ReturnType::Type(_, t) => *t,
        }
    }
}

fn create_fallible_path(return_ty: &Type) -> Path {
    let simple_try = new_path!(::jenner::effective::SimpleTry::Break);
    let failure = TypePath {
        qself: Some(syn::QSelf {
            lt_token: Default::default(),
            ty: Box::new(return_ty.clone()),
            position: 3,
            as_token: Default::default(),
            gt_token: Default::default(),
        }),
        path: simple_try,
    };
    let mut path = new_path! { ::jenner::effective::Failure };
    path.segments.last_mut().unwrap().arguments =
        PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args: [GenericArgument::Type(failure.into())]
                .into_iter()
                .collect(),
            gt_token: Default::default(),
        });
    path
}

fn create_fallible_return_type(return_ty: Type) -> Type {
    let simple_try = new_path!(::jenner::effective::SimpleTry::Continue);
    let failure = TypePath {
        qself: Some(syn::QSelf {
            lt_token: Default::default(),
            ty: Box::new(return_ty),
            position: 3,
            as_token: Default::default(),
            gt_token: Default::default(),
        }),
        path: simple_try,
    };

    Type::Path(failure)
}

fn create_impl_effective(
    return_ty: Type,
    fallible_ty: Path,
    yield_ty: Path,
    async_ty: Path,
) -> Path {
    let mut effective = new_path!(::jenner::effective::Effective);
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
                        path: fallible_ty,
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
    effective
}

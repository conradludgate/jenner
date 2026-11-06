use crate::{gen_visit::GenVisitor, parse::AttrGenerator};
use std::mem;
use syn::{parse_quote, ItemFn, Result, ReturnType, Stmt, Type};

impl AttrGenerator {
    pub fn process(mut self) -> Result<ItemFn> {
        let ItemFn { sig, block, .. } = &mut self.func;
        let declared_return = match mem::replace(&mut sig.output, ReturnType::Default) {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, ty) => *ty,
        };
        let sync = sig.asyncness.take().is_none();
        let general = self.yield_ty.is_some();
        let (yield_ty, return_ty): (Type, Type) = if let Some(yield_ty) = self.yield_ty {
            (yield_ty, declared_return.clone())
        } else if self.yields && self.fallible {
            (
                parse_quote!(<#declared_return as ::jenner::__private::ResultTypes>::Ok),
                parse_quote!(::core::result::Result<(), <#declared_return as ::jenner::__private::ResultTypes>::Error>),
            )
        } else if self.yields {
            (declared_return.clone(), parse_quote!(()))
        } else {
            (
                parse_quote!(::core::convert::Infallible),
                declared_return.clone(),
            )
        };

        let output: Type = match (sync, self.yields, self.fallible, general) {
            (true, _, _, true) => {
                parse_quote!(impl ::jenner::Coroutine<(), Yield = #yield_ty, Return = #return_ty>)
            }
            (false, _, _, true) => {
                parse_quote!(impl ::jenner::AsyncGenerator<Yield = #yield_ty, Return = #return_ty>)
            }
            (true, false, _, false) => parse_quote!(impl ::jenner::SyncValue<#return_ty>),
            (false, false, _, false) => parse_quote!(impl ::jenner::AsyncValue<#return_ty>),
            (true, true, false, false) => parse_quote!(impl ::jenner::SyncStream<#yield_ty>),
            (false, true, false, false) => parse_quote!(impl ::jenner::AsyncStream<#yield_ty>),
            (true, true, true, false) => {
                parse_quote!(impl ::jenner::TrySyncStream<#yield_ty, <#declared_return as ::jenner::__private::ResultTypes>::Error>)
            }
            (false, true, true, false) => {
                parse_quote!(impl ::jenner::TryAsyncStream<#yield_ty, <#declared_return as ::jenner::__private::ResultTypes>::Error>)
            }
        };
        sig.output = parse_quote!(-> #output);
        let visitor = GenVisitor::new(sync, self.yields, self.fallible);
        let generator = visitor.into_generator(&mut block.stmts)?;
        let generator = if !sync && self.yields && self.fallible && !general {
            parse_quote!(::jenner::__private::TryAsyncStreamImpl::new(#generator))
        } else {
            generator
        };
        block.stmts = vec![Stmt::Expr(generator, None)];
        Ok(self.func)
    }
}

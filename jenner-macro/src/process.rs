use proc_macro2::Ident;
use quote::format_ident;
use rand::{distributions::Alphanumeric, Rng};
use syn::{
    parse2, parse_quote,
    visit_mut::{
        visit_block_mut, visit_expr_await_mut, visit_expr_mut, visit_expr_yield_mut, VisitMut,
    },
    Attribute, Expr, ExprAwait, ExprForLoop, ExprYield, ItemFn, Result, Signature, Stmt, Type,
};

use crate::parse::{AttrGenerator, ExprGenerator};

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
            None => Ok(parse_quote! { () }),
        }
    }
}

struct GenVisitor {
    pub cx: Ident,
    pub yields: usize,
    pub sync: bool,
}

impl GenVisitor {
    pub fn new(sync: bool) -> Self {
        let random: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        GenVisitor {
            cx: format_ident!("__cx_{}", random),
            yields: 0,
            sync,
        }
    }

    fn into_generator(mut self, stmts: &mut [Stmt]) -> Expr {
        if !self.sync {
            stmts.iter_mut().for_each(|stmt| self.visit_stmt_mut(stmt));
        }

        let Self { cx, yields, sync } = self;
        let y: Type = (yields == 0)
            .then(|| parse_quote! { () })
            .unwrap_or_else(|| parse_quote! { _ });

        if sync {
            parse_quote! {
                unsafe { ::jenner::new_sync_generator::<#y, _, _>(|| { #(#stmts)* }) }
            }
        } else {
            parse_quote! {
                unsafe { ::jenner::new_async_generator::<#y, _, _>(|mut #cx: ::jenner::UnsafeContextRef| { #(#stmts)* }) }
            }
        }
    }
}

impl VisitMut for GenVisitor {
    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        match i {
            Expr::Await(await_) => {
                visit_expr_await_mut(self, await_);
                let ExprAwait { attrs, base, .. } = await_;
                let cx = &self.cx;
                *i = parse_quote! {{
                    let mut fut = #(#attrs)* { #base };

                    loop {
                        let polled = unsafe {
                            ::jenner::__private::Future::poll(
                                ::jenner::__private::pin::Pin::new_unchecked(&mut fut),
                                #cx.get_context()
                            )
                        };
                        match polled {
                            ::jenner::__private::task::Poll::Ready(r) => break r,
                            ::jenner::__private::task::Poll::Pending => {
                                yield ::jenner::__private::task::Poll::Pending;
                            }
                        }
                    }
                }}
            }
            Expr::ForLoop(for_loop) => {
                let async_attrs = for_loop
                    .attrs
                    .drain_filter(|attr| attr.path.get_ident().map_or(false, |i| i == "async_for"))
                    .count();

                if async_attrs > 0 {
                    let ExprForLoop {
                        attrs,
                        label,
                        pat,
                        expr,
                        body,
                        ..
                    } = for_loop;

                    visit_expr_mut(self, expr);
                    visit_block_mut(self, body);

                    let cx = &self.cx;
                    *i = parse_quote! {
                        #(#attrs)*
                        {
                            let mut stream = #expr;
                            #label loop {
                                let next = loop {
                                    let polled = unsafe {
                                        ::jenner::__private::Stream::poll_next(
                                            ::jenner::__private::pin::Pin::new_unchecked(&mut stream),
                                            #cx.get_context()
                                        )
                                    };
                                    match polled {
                                        ::jenner::__private::task::Poll::Ready(r) => break r,
                                        ::jenner::__private::task::Poll::Pending => {
                                            yield ::jenner::__private::task::Poll::Pending;
                                        }
                                    }
                                };

                                match next {
                                    Some(#pat) => #body,
                                    _ => break,
                                };
                            }
                        }
                    }
                } else {
                    visit_expr_mut(self, i);
                }
            }
            i => visit_expr_mut(self, i),
        }
    }

    fn visit_expr_yield_mut(&mut self, i: &mut ExprYield) {
        self.yields += 1;
        visit_expr_yield_mut(self, i);
        let ExprYield { attrs, expr, .. } = i;

        let expr = expr.get_or_insert_with(|| Box::new(parse_quote! { () }));
        *expr = parse_quote! {
            ::jenner::__private::task::Poll::Ready( #(#attrs)* { #expr } )
        };
    }
}

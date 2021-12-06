use proc_macro2::Ident;
use quote::format_ident;
use rand::{distributions::Alphanumeric, Rng};
use syn::{
    parse_quote,
    visit_mut::{visit_expr_mut, visit_expr_yield_mut, VisitMut},
    Expr, ExprAwait, ExprForLoop, ExprYield, Stmt, Type,
};

use crate::break_visit::BreakVisitor;

pub struct GenVisitor {
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

    pub fn into_generator(mut self, stmts: &mut [Stmt]) -> Expr {
        if !self.sync {
            stmts.iter_mut().for_each(|stmt| self.visit_stmt_mut(stmt));
        }

        let Self { cx, yields, sync } = self;
        let y: Type = (yields == 0)
            .then(|| parse_quote! { ! })
            .unwrap_or_else(|| parse_quote! { _ });

        if sync {
            parse_quote! {
                unsafe { ::jenner::GeneratorImpl::new_sync::<#y, _>(|| { #(#stmts)* }) }
            }
        } else {
            parse_quote! {
                unsafe { ::jenner::GeneratorImpl::new_async::<#y, _>(|mut #cx: ::jenner::__private::UnsafeContextRef| { #(#stmts)* }) }
            }
        }
    }
}

impl VisitMut for GenVisitor {
    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        match i {
            Expr::Await(await_) => {
                self.visit_expr_await_mut(await_);
                let ExprAwait { attrs, base, .. } = await_;

                if self.handle_for_await(&mut *base) {
                    *i = parse_quote! { #(#attrs)* { #base } };
                    return;
                }

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

impl GenVisitor {
    fn handle_for_await(&mut self, i: &mut Expr) -> bool {
        if let Expr::ForLoop(for_loop) = i {
            let ExprForLoop {
                attrs,
                label,
                pat,
                expr,
                body,
                ..
            } = for_loop;

            let mut vis = BreakVisitor {
                label: &*label,
                outside: false,
                breaks: 0,
            };
            vis.visit_block_mut(body);
            let BreakVisitor { breaks, .. } = vis;

            let break_ty: Type = if breaks == 0 {
                parse_quote! { ! }
            } else {
                parse_quote! { _ }
            };

            let cx = &self.cx;
            *i = parse_quote! {
                #(#attrs)*
                {
                    let gen = #expr;
                    let mut gen = {
                        // weak form of specialisation.
                        use ::jenner::{__private::IntoAsyncGenerator, AsyncGenerator};
                        gen.into_async_generator()
                    };
                    let res: ::jenner::ForResult<#break_ty, _> = #label loop {
                        let next = loop {
                            let polled = unsafe {
                                ::jenner::AsyncGenerator::poll_resume(
                                    ::jenner::__private::pin::Pin::new_unchecked(&mut gen),
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
                            ::jenner::__private::GeneratorState::Yielded(#pat) => #body,
                            ::jenner::__private::GeneratorState::Complete(c) => break ::jenner::ForResult::Finally(c),
                        };
                    };
                    res
                }
            };
            true
        } else {
            false
        }
    }
}

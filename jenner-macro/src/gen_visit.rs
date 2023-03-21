use proc_macro2::Ident;
use quote::format_ident;
use rand::{distributions::Alphanumeric, Rng};
use syn::{
    parse_quote,
    punctuated::Punctuated,
    token,
    visit_mut::{visit_expr_method_call_mut, visit_expr_mut, visit_expr_yield_mut, VisitMut},
    Expr, ExprAssign, ExprAwait, ExprCall, ExprForLoop, ExprMethodCall, ExprPath, ExprTuple,
    ExprYield, Stmt, Type,
};

use crate::break_visit::BreakVisitor;

pub struct GenVisitor {
    pub cx: Ident,
    pub sync: bool,
    pub yields: bool,
}

impl GenVisitor {
    pub fn new(sync: bool, yields: bool) -> Self {
        let random: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        GenVisitor {
            cx: format_ident!("__cx_{}", random),
            sync,
            yields,
        }
    }

    pub fn into_generator(mut self, stmts: &mut [Stmt]) -> Expr {
        if !self.sync {
            stmts.iter_mut().for_each(|stmt| self.visit_stmt_mut(stmt));
        }

        let Self { cx, yields, sync } = self;

        match (sync, yields) {
            (true, true) => parse_quote! {
                unsafe { ::jenner::__private::SyncGeneratorImpl::create(|| { #(#stmts)* }) }
            },
            (true, false) => parse_quote! {
                unsafe { ::jenner::__private::effective::wrappers::from_fn_once(|| { #(#stmts)* }) }
            },
            (false, true) => parse_quote! {
                unsafe { ::jenner::__private::AsyncGeneratorImpl::create(
                    |mut #cx: ::jenner::__private::UnsafeContextRef| { #(#stmts)* }
                ) }
            },
            (false, false) => parse_quote! {
                unsafe { ::jenner::__private::AsyncImpl::create(
                    |mut #cx: ::jenner::__private::UnsafeContextRef| { #(#stmts)* }
                ) }
            },
        }

        // if sync {
        //     parse_quote! {
        //         unsafe { ::jenner::GeneratorImpl::new_sync::<#y, _>(|| { #(#stmts)* }) }
        //     }
        // } else {
        //     parse_quote! {
        //         unsafe { ::jenner::GeneratorImpl::new_async::<#y, _>(|mut #cx: ::jenner::__private::UnsafeContextRef| { #(#stmts)* }) }
        //     }
        // }
    }
}

impl VisitMut for GenVisitor {
    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        match i {
            Expr::Await(await_) if !self.sync => {
                let ExprAwait { attrs, base, .. } = await_;

                self.visit_expr_mut(&mut *base);
                if self.handle_for_await(&mut *base) {
                    *i = parse_quote! { #(#attrs)* { #base } };
                    return;
                }

                let cx = &self.cx;
                *i = parse_quote! {{
                    let fut = #(#attrs)* { #base };
                    let mut fut = ::jenner::__private::IntoFuture::into_future(fut);

                    loop {
                        let fut = unsafe { ::jenner::__private::pin::Pin::new_unchecked(&mut fut) };
                        let cx = unsafe { #cx.get_context() };
                        let polled = ::jenner::__private::Future::poll(fut, cx);
                        match polled {
                            ::jenner::__private::task::Poll::Ready(r) => break r,
                            ::jenner::__private::task::Poll::Pending => {
                                #cx = yield ::jenner::__private::task::Poll::Pending;
                            }
                        }
                    }
                }}
            }
            Expr::Yield(yield_) if !self.sync => {
                self.visit_expr_yield_mut(yield_);
                *i = ExprAssign {
                    attrs: vec![],
                    left: Box::new(
                        ExprPath {
                            attrs: vec![],
                            qself: None,
                            path: self.cx.clone().into(),
                        }
                        .into(),
                    ),
                    eq_token: Default::default(),
                    right: Box::new(yield_.clone().into()),
                }
                .into();
            }
            Expr::MethodCall(m) if m.method == "finally" => {
                visit_expr_method_call_mut(self, m);
                let ExprMethodCall {
                    attrs, receiver, ..
                } = m;
                if self.handle_for_finally(&mut *receiver) {
                    *i = parse_quote! { #(#attrs)* { #receiver } };
                }
            }
            i => visit_expr_mut(self, i),
        }
    }

    fn visit_expr_yield_mut(&mut self, i: &mut ExprYield) {
        visit_expr_yield_mut(self, i);
        let ExprYield { expr, .. } = i;
        let expr = expr.get_or_insert_with(|| {
            Box::new(
                ExprTuple {
                    attrs: vec![],
                    paren_token: token::Paren::default(),
                    elems: Punctuated::new(),
                }
                .into(),
            )
        });

        **expr = Expr::Call(ExprCall {
            attrs: vec![],
            func: Box::new(Expr::Path(ExprPath {
                attrs: vec![],
                qself: None,
                path: new_path!(::jenner::__private::task::Poll::Ready),
            })),
            paren_token: Default::default(),
            args: [*expr.clone()].into_iter().collect(),
        });
    }
}

impl GenVisitor {
    fn handle_for_await(&self, i: &mut Expr) -> bool {
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
                                    #cx = yield ::jenner::__private::task::Poll::Pending;
                                }
                            }
                        };

                        match next {
                            ::jenner::__private::GeneratorState::Yielded(#pat) => #body,
                            ::jenner::__private::GeneratorState::Complete(c) => break ::jenner::ForResult::Complete(c),
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
    fn handle_for_finally(&self, i: &mut Expr) -> bool {
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

            *i = parse_quote! {
                #(#attrs)*
                {
                    let __gen = #expr;
                    let mut __gen = {
                        // weak form of specialisation.
                        use ::jenner::{__private::IntoSyncGenerator, SyncGenerator};
                        __gen.into_sync_generator()
                    };
                    let mut __pinned = unsafe { ::jenner::__private::pin::Pin::new_unchecked(&mut __gen) };
                    let res: ::jenner::ForResult<#break_ty, _> = #label loop {
                        let __state = ::jenner::SyncGenerator::resume(::jenner::__private::pin::Pin::as_mut(&mut __pinned));
                        match __state {
                            ::jenner::__private::GeneratorState::Yielded(#pat) => #body,
                            ::jenner::__private::GeneratorState::Complete(c) => break ::jenner::ForResult::Complete(c),
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

use proc_macro2::Ident;
use quote::format_ident;
use rand::{distributions::Alphanumeric, Rng};
use syn::{
    parse_quote,
    visit_mut::{visit_expr_mut, VisitMut},
    Expr, ExprForLoop, Stmt,
};

pub struct GenVisitor {
    cx: Ident,
    sync: bool,
    yields: bool,
    fallible: bool,
    error: Option<syn::Error>,
}

impl GenVisitor {
    pub fn new(sync: bool, yields: bool, fallible: bool) -> Self {
        let random: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        Self {
            cx: format_ident!("__cx_{}", random),
            sync,
            yields,
            fallible,
            error: None,
        }
    }

    pub fn into_generator(mut self, stmts: &mut [Stmt]) -> syn::Result<Expr> {
        if !self.sync {
            stmts.iter_mut().for_each(|stmt| self.visit_stmt_mut(stmt));
        }
        if let Some(error) = self.error {
            return Err(error);
        }
        let body: Expr = if self.fallible {
            parse_quote!(::core::result::Result::Ok({ #(#stmts)* }))
        } else {
            parse_quote!({ #(#stmts)* })
        };
        let cx = self.cx;
        Ok(if self.sync && !self.yields && self.fallible {
            parse_quote!({
                let output = (move || #body)();
                ::jenner::__private::Generator::new(#[coroutine] static move || output)
            })
        } else if self.sync {
            parse_quote!(::jenner::__private::Generator::new(#[coroutine] static move || #body))
        } else {
            parse_quote!(::jenner::__private::Generator::new(
                #[coroutine] static move |mut #cx: ::jenner::__private::UnsafeContextRef| #body
            ))
        })
    }

    fn async_for_impl(&self, for_loop: &ExprForLoop) -> Expr {
        let ExprForLoop {
            attrs,
            label,
            pat,
            expr,
            body,
            ..
        } = for_loop;
        let cx = &self.cx;
        parse_quote! {
            #(#attrs)* {
                let mut __gen__ = ::core::pin::pin!(#expr);
                #label loop {
                    let __next__ = loop {
                        let polled = {
                            // SAFETY: resume supplied this poll's live Context.
                            // This exclusive reborrow ends before any yield;
                            // every yield replaces the token before the next use.
                            let cx = unsafe { #cx.get_context() };
                            ::jenner::Stream::poll_next(__gen__.as_mut(), cx)
                        };
                        match polled {
                            ::core::task::Poll::Ready(next) => break next,
                            ::core::task::Poll::Pending => {
                                #cx = yield ::core::task::Poll::Pending;
                            }
                        }
                    };
                    if let Some(#pat) = __next__ { #body } else { break; }
                }
            }
        }
    }
}

impl VisitMut for GenVisitor {
    fn visit_item_mut(&mut self, _: &mut syn::Item) {}

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        match expr {
            // These establish their own suspension/return boundary.
            Expr::Async(_) | Expr::Closure(_) | Expr::Const(_) => {}
            Expr::Await(await_) => {
                self.visit_expr_mut(&mut await_.base);
                let base = &await_.base;
                let cx = &self.cx;
                *expr = parse_quote! {{
                    let mut __fut__ = ::core::pin::pin!(::core::future::IntoFuture::into_future(#base));
                    loop {
                        let polled = {
                            // SAFETY: resume supplied this poll's live Context.
                            // This exclusive reborrow ends before any yield;
                            // every yield replaces the token before the next use.
                            let cx = unsafe { #cx.get_context() };
                            ::core::future::Future::poll(__fut__.as_mut(), cx)
                        };
                        match polled {
                            ::core::task::Poll::Ready(output) => break output,
                            ::core::task::Poll::Pending => {
                                #cx = yield ::core::task::Poll::Pending;
                            }
                        }
                    }
                }};
            }
            Expr::Yield(yield_) => {
                let value = yield_
                    .expr
                    .get_or_insert_with(|| Box::new(parse_quote!(())));
                self.visit_expr_mut(value);
                let cx = &self.cx;
                *expr = parse_quote!(#cx = yield ::core::task::Poll::Ready(#value));
            }
            Expr::ForLoop(for_loop) => {
                let mut asynchronous = false;
                for_loop.attrs.retain(|attr| {
                    if !attr.path().is_ident("effect") {
                        return true;
                    }
                    let result = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("async") {
                            asynchronous = true;
                            Ok(())
                        } else {
                            Err(meta.error("expected `async`"))
                        }
                    });
                    if let Err(error) = result {
                        self.error = Some(error);
                    }
                    false
                });
                syn::visit_mut::visit_expr_for_loop_mut(self, for_loop);
                if asynchronous {
                    *expr = self.async_for_impl(for_loop);
                }
            }
            expr => visit_expr_mut(self, expr),
        }
    }
}

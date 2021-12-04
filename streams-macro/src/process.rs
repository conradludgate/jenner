use proc_macro2::Ident;
use quote::format_ident;
use rand::{distributions::Alphanumeric, Rng};
use syn::{
    parse_quote,
    visit_mut::{
        visit_block_mut, visit_expr_await_mut, visit_expr_mut, visit_expr_yield_mut, VisitMut,
    },
    Expr, ExprAwait, ExprForLoop, ExprYield, Result, Type,
};

use crate::{parse::StreamGeneratorInput, tokens::StreamGenerator};

impl StreamGeneratorInput {
    pub fn process(self) -> Result<StreamGenerator> {
        let Self { mut stmts } = self;

        let random: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        let mut visitor = StreamGenVisitor {
            cx: format_ident!("__cx_{}", random),
            yields: 0,
        };

        stmts
            .iter_mut()
            .for_each(|stmt| visitor.visit_stmt_mut(stmt));

        let StreamGenVisitor { cx, yields } = visitor;

        let y: Type = if yields == 0 {
            parse_quote!{ () }
        } else {
            parse_quote!{ _ }
        };

        Ok(StreamGenerator {
            stream: parse_quote! {
                unsafe {
                    ::streams_generator::new_stream_generator::<#y, _, _>(|mut #cx: ::streams_generator::UnsafeContextRef| {
                        #(#stmts)*
                    })
                }
            },
        })
    }
}
struct StreamGenVisitor {
    pub cx: Ident,
    pub yields: usize,
}

impl VisitMut for StreamGenVisitor {
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
                            ::std::future::Future::poll(
                                ::std::pin::Pin::new_unchecked(&mut fut),
                                #cx.get_context()
                            )
                        };
                        match polled {
                            ::std::task::Poll::Ready(r) => break r,
                            ::std::task::Poll::Pending => {
                                yield ::std::task::Poll::Pending;
                            }
                        }
                    }
                }}
            }
            Expr::ForLoop(for_loop) => {
                let async_attrs = for_loop
                    .attrs
                    .drain_filter(|attr| attr.path.get_ident().map_or(false, |i| i == "async"))
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
                                        ::futures_core::stream::Stream::poll_next(
                                            ::std::pin::Pin::new_unchecked(&mut stream),
                                            #cx.get_context()
                                        )
                                    };
                                    match polled {
                                        ::std::task::Poll::Ready(r) => break r,
                                        ::std::task::Poll::Pending => {
                                            yield ::std::task::Poll::Pending;
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
                    visit_expr_mut(self, i)
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
            ::std::task::Poll::Ready( #(#attrs)* { #expr } )
        };
    }
}

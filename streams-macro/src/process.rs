use syn::{
    parse_quote,
    visit_mut::{
        visit_block_mut, visit_expr_await_mut, visit_expr_mut, visit_expr_yield_mut, VisitMut,
    },
    Expr, ExprAwait, ExprForLoop, ExprYield, Result,
};

use crate::{parse::StreamGeneratorInput, tokens::StreamGenerator};

impl StreamGeneratorInput {
    pub fn process(self) -> Result<StreamGenerator> {
        let Self { mut stmts } = self;

        stmts
            .iter_mut()
            .for_each(|stmt| StreamGenVisitor.visit_stmt_mut(stmt));

        Ok(StreamGenerator {
            stream: parse_quote! {
                unsafe {
                    ::streams_generator::new_stream(|mut __cx: ::streams_generator::UnsafeContextRef| {
                        #(#stmts)*
                    })
                }
            },
        })
    }
}
struct StreamGenVisitor;

impl VisitMut for StreamGenVisitor {
    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        match i {
            Expr::Await(await_) => {
                visit_expr_await_mut(self, await_);
                let ExprAwait { attrs, base, .. } = await_;
                *i = parse_quote! {{
                    let mut fut = #(#attrs)* { #base };

                    loop {
                        let polled = unsafe {
                            ::std::future::Future::poll(
                                ::std::pin::Pin::new_unchecked(&mut fut),
                                __cx.get_context()
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

                    *i = parse_quote! {{
                        #(#attrs)*
                        {
                            let mut stream = #expr;
                            #label loop {
                                let next = loop {
                                    let polled = unsafe {
                                        ::futures_core::stream::Stream::poll_next(
                                            ::std::pin::Pin::new_unchecked(&mut stream),
                                            __cx.get_context()
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
                                }
                            }
                        }
                    }}
                } else {
                    visit_expr_mut(self, i)
                }
            }
            i => visit_expr_mut(self, i),
        }
    }

    fn visit_expr_yield_mut(&mut self, i: &mut ExprYield) {
        visit_expr_yield_mut(self, i);
        let ExprYield { attrs, expr, .. } = i;

        let expr = expr.get_or_insert_with(|| Box::new(parse_quote! { () }));
        *expr = parse_quote! {
            ::std::task::Poll::Ready( #(#attrs)* { #expr } )
        };
    }
}

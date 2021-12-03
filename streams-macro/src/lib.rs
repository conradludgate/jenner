use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::Parse,
    parse_macro_input, parse_quote,
    visit_mut::{visit_expr_mut, visit_expr_yield_mut, VisitMut},
    Block, Expr, ExprAwait, ExprYield, Result, Stmt,
};

#[proc_macro]
pub fn stream(input: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(input as StreamGeneratorInput);

    input
        .process()
        .map_or_else(|e| e.to_compile_error(), ToTokens::into_token_stream)
        .into()
}

struct StreamGeneratorInput {
    stmts: Vec<Stmt>,
}

impl Parse for StreamGeneratorInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(StreamGeneratorInput {
            stmts: Block::parse_within(input)?,
        })
    }
}

impl StreamGeneratorInput {
    fn process(self) -> Result<StreamGenerator> {
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

struct StreamGenerator {
    stream: Expr,
}

impl ToTokens for StreamGenerator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.stream.to_tokens(tokens)
    }
}

struct StreamGenVisitor;

impl VisitMut for StreamGenVisitor {
    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        visit_expr_mut(self, i);

        if let Expr::Await(ExprAwait { attrs, base, .. }) = i {
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
            }};
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

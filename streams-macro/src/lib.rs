use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::Parse,
    parse_macro_input, parse_quote,
    visit_mut::{visit_expr_mut, VisitMut, visit_expr_yield_mut},
    AttributeArgs, Expr, ExprAwait, ExprYield, ItemFn, Result,
};

#[proc_macro_attribute]
pub fn stream_generator(args: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as StreamGeneratorInput);

    input
        .process(args)
        .map_or_else(|e| e.to_compile_error(), ToTokens::into_token_stream)
        .into()
}

struct StreamGeneratorInput {
    func: ItemFn,
}

impl Parse for StreamGeneratorInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(StreamGeneratorInput {
            func: input.parse()?,
        })
    }
}

impl StreamGeneratorInput {
    fn process(self, _args: AttributeArgs) -> Result<StreamGenerator> {
        let Self { mut func } = self;

        let block = &mut func.block;

        StreamGenVisitor.visit_block_mut(block);

        eprintln!("{}", block.into_token_stream());

        *block = parse_quote! {
            {
                unsafe {
                    ::streams_generator::new_stream(|mut __cx: ::streams_generator::UnsafeContextRef| #block )
                }
            }
        };

        Ok(StreamGenerator { func })
    }
}

struct StreamGenerator {
    func: ItemFn,
}

impl ToTokens for StreamGenerator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.func.to_tokens(tokens)
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

        let expr = expr.get_or_insert_with(|| Box::new(parse_quote!{ () }));
        *expr = parse_quote!{
            ::std::task::Poll::Ready( #(#attrs)* { #expr } )
        };
    }
}

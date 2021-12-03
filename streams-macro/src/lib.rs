use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, Block, Error, Result, Stmt, Expr, Item};

#[proc_macro]
pub fn stream_generator(input: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(input as StreamGeneratorInput);
    let gen: Result<StreamGenerator> = input.try_into();
    gen.map_or_else(|e| e.to_compile_error(), ToTokens::into_token_stream)
        .into()
}

struct StreamGeneratorInput {
    stmts: Vec<Stmt>,
}

impl Parse for StreamGeneratorInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Block::parse_within(input).map(|stmts| StreamGeneratorInput { stmts })
    }
}

impl TryFrom<StreamGeneratorInput> for StreamGenerator {
    type Error = Error;

    fn try_from(value: StreamGeneratorInput) -> Result<Self> {
        todo!()
    }
}

struct StreamGenerator {
    yields: Vec<BoundaryPoint>,
    items: Vec<Item>,
}

enum BoundaryPoint {
    Yield(Expr),
    Await(Expr),
    Break(Expr),
    Return(Expr),
    Loop(LoopBoundary),
}

enum LoopBoundary {
    For(Vec<BoundaryPoint>),
    While(Vec<BoundaryPoint>),
    Loop(Vec<BoundaryPoint>),
}

impl ToTokens for StreamGenerator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! {
            { todo!(); }
        })
    }
}

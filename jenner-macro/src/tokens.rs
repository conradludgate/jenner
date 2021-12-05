use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Expr, ItemFn};

pub struct StreamExprGenerator {
    pub stream: Expr,
}

impl ToTokens for StreamExprGenerator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.stream.to_tokens(tokens)
    }
}

pub struct StreamItemGenerator {
    pub stream: ItemFn,
}

impl ToTokens for StreamItemGenerator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.stream.to_tokens(tokens)
    }
}

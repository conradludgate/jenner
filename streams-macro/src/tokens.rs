use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Expr;

pub struct StreamGenerator {
    pub stream: Expr,
}

impl ToTokens for StreamGenerator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.stream.to_tokens(tokens)
    }
}

#![feature(drain_filter)]

use parse::{AttrGenerator, ExprGenerator};
use proc_macro::TokenStream as TokenStream1;
use quote::ToTokens;
use syn::parse_macro_input;

mod break_visit;
mod gen_visit;
mod parse;
mod process;

#[proc_macro]
pub fn async_generator(input: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(input as ExprGenerator);
    input.process().into_token_stream().into()
}

#[proc_macro_attribute]
pub fn generator(_args: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(input as AttrGenerator);

    input
        .process()
        .map_or_else(|e| e.to_compile_error(), ToTokens::into_token_stream)
        .into()
}

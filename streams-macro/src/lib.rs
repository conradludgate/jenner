#![feature(drain_filter)]

use parse::{replace_async_for, StreamGeneratorInput};
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse2;

mod parse;
mod process;
mod tokens;

#[proc_macro]
pub fn stream(input: TokenStream1) -> TokenStream1 {
    let input: TokenStream = input.into();
    let input: StreamGeneratorInput = match parse2(replace_async_for(input.into_iter())) {
        Ok(input) => input,
        Err(err) => {
            return err.to_compile_error().into();
        }
    };

    input
        .process()
        .map_or_else(|e| e.to_compile_error(), ToTokens::into_token_stream)
        .into()
}

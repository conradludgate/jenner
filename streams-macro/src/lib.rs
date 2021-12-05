#![feature(drain_filter)]

use parse::{replace_async_for, AsyncGeneratorExprInput, AsyncGeneratorItemInput};
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse2;

mod parse;
mod process;
mod tokens;

#[proc_macro]
pub fn async_generator(input: TokenStream1) -> TokenStream1 {
    let input: TokenStream = input.into();
    let input: AsyncGeneratorExprInput = match parse2(replace_async_for(input)) {
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

#[proc_macro_attribute]
pub fn generator(_args: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let input: TokenStream = input.into();
    let input = replace_async_for(input);
    let input: AsyncGeneratorItemInput = match parse2(input) {
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

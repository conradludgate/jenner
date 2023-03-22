#![feature(drain_filter)]

use parse::AttrGenerator;
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::Ident;
use quote::ToTokens;
use syn::{
    ext::IdentExt, parse::Parser, parse_macro_input, punctuated::Punctuated, token::Comma, Error,
};

macro_rules! new_path {
    (::$($ident:ident)::*) => {
        new_path!(::proc_macro2::Span::call_site() => ::$($ident)::*)
    };
    ($span:expr => ::$($ident:ident)::*) => {
        ::syn::Path {
            leading_colon: Some(::syn::token::PathSep::default()),
            segments: segments!($span => $($ident)::*)
        }
    };
    ($($ident:ident)::*) => {
        new_path!(::proc_macro2::Span::call_site() => $($ident)::*)
    };
    ($span:expr => $($ident:ident)::*) => {
        ::syn::Path {
            leading_colon: None,
            segments: segments!($span => $($ident)::*)
        }
    };
}

macro_rules! segments {
    ($span:expr => $($ident:ident)::*) => {{
        [$(
            ::syn::PathSegment{
                ident: ::syn::Ident::new(stringify!($ident), $span),
                arguments: ::syn::PathArguments::None,
            }
        ),*].into_iter().collect()
    }};
}

mod break_visit;
mod gen_visit;
mod parse;
mod process;

#[proc_macro_attribute]
pub fn effect(args: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let mut input = parse_macro_input!(input as AttrGenerator);

    fn parser(input: syn::parse::ParseStream) -> syn::Result<Punctuated<Ident, Comma>> {
        Punctuated::<Ident, Comma>::parse_terminated_with(input, Ident::parse_any)
    }
    let effects = match parser.parse(args) {
        Ok(x) => x,
        Err(e) => return e.to_compile_error().into(),
    };
    for effect in effects {
        match effect.to_string().as_str() {
            "fallible" => input.fallible = true,
            "yields" => input.yields = true,
            _other => {
                return Error::new(effect.span(), "unknown effect")
                    .into_compile_error()
                    .into()
            }
        }
    }

    input
        .process()
        .map_or_else(|e| e.to_compile_error(), ToTokens::into_token_stream)
        .into()
}

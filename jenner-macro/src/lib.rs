#![feature(drain_filter)]

use parse::AttrGenerator;
use proc_macro::TokenStream as TokenStream1;
use quote::ToTokens;
use syn::parse_macro_input;

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
pub fn generator(_args: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(input as AttrGenerator);

    input
        .process()
        .map_or_else(|e| e.to_compile_error(), ToTokens::into_token_stream)
        .into()
}

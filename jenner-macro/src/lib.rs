use parse::AttrGenerator;
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, punctuated::Punctuated, Meta, Token};

mod gen_visit;
mod parse;
mod process;

#[proc_macro_attribute]
pub fn effect(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as AttrGenerator);
    let effects = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    for effect in effects {
        match effect {
            Meta::Path(path) if path.is_ident("fallible") => input.fallible = true,
            Meta::Path(path) if path.is_ident("yields") => input.yields = true,
            Meta::List(list) if list.path.is_ident("yields") => {
                input.yields = true;
                match syn::parse2(list.tokens) {
                    Ok(ty) => input.yield_ty = Some(ty),
                    Err(error) => return error.into_compile_error().into(),
                }
            }
            other => {
                return syn::Error::new_spanned(other, "unknown effect")
                    .into_compile_error()
                    .into()
            }
        }
    }

    input
        .process()
        .map_or_else(
            |error| error.to_compile_error(),
            ToTokens::into_token_stream,
        )
        .into()
}

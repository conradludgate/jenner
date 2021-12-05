use proc_macro2::{Group, TokenStream, TokenTree};
use quote::{format_ident, quote_spanned};
use syn::{parse::Parse, Block, ItemFn, Result, Stmt};

pub struct AsyncGeneratorExprInput {
    pub stmts: Vec<Stmt>,
}

impl Parse for AsyncGeneratorExprInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(AsyncGeneratorExprInput {
            stmts: Block::parse_within(input)?,
        })
    }
}

pub fn replace_async_for(input: impl IntoIterator<Item = TokenTree>) -> TokenStream {
    let mut input = input.into_iter().peekable();
    let mut tokens = Vec::new();

    while let Some(token) = input.next() {
        match token {
            TokenTree::Ident(ident) => match input.peek() {
                Some(TokenTree::Ident(next)) if ident == "async" && next == "for" => {
                    let async_for = format_ident!("{}_for", ident);
                    tokens.extend(quote_spanned! { ident.span() => #[#async_for] });
                }
                _ => tokens.push(ident.into()),
            },
            TokenTree::Group(group) => {
                let stream = replace_async_for(group.stream());
                tokens.push(Group::new(group.delimiter(), stream).into());
            }
            _ => tokens.push(token),
        }
    }

    tokens.into_iter().collect()
}

pub struct AsyncGeneratorItemInput {
    pub func: ItemFn,
}

impl Parse for AsyncGeneratorItemInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(AsyncGeneratorItemInput {
            func: input.parse()?,
        })
    }
}

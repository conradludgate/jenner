use proc_macro2::{TokenStream, TokenTree, Group};
use quote::{quote_spanned};
use syn::{parse::Parse, Block, Result, Stmt};

pub struct StreamGeneratorInput {
    pub stmts: Vec<Stmt>,
}

impl Parse for StreamGeneratorInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(StreamGeneratorInput {
            stmts: Block::parse_within(input)?,
        })
    }
}

pub fn replace_async_for(input: impl IntoIterator<Item = TokenTree>) -> TokenStream {
    let mut input = input.into_iter().peekable();
    let mut tokens = Vec::new();

    while let Some(token) = input.next() {
        match token {
            TokenTree::Ident(ident) => {
                match input.peek() {
                    Some(TokenTree::Ident(next)) if ident == "async" && next == "for" => {
                        tokens.extend(quote_spanned! { ident.span() => #[async] });
                    }
                    _ => tokens.push(ident.into())
                }
            }
            TokenTree::Group(group) => {
                let stream = replace_async_for(group.stream());
                tokens.push(Group::new(group.delimiter(), stream).into());
            }
            _ => tokens.push(token),
        }
    }

    tokens.into_iter().collect()
}

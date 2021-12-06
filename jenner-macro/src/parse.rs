use syn::{parse::Parse, Block, ItemFn, Result, Stmt};

pub struct ExprGenerator {
    pub stmts: Vec<Stmt>,
}

impl Parse for ExprGenerator {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(ExprGenerator {
            stmts: Block::parse_within(input)?,
        })
    }
}

pub struct AttrGenerator {
    pub func: ItemFn,
}

impl Parse for AttrGenerator {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(AttrGenerator {
            func: input.parse()?,
        })
    }
}

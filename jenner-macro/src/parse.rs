use syn::{parse::Parse, ItemFn, Result};

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

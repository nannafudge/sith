use super::{
    Mutate,
    impl_unique_arg,
    impl_to_tokens_wrapped
};
use syn::{
    Ident, Item, Result,
    parse::{
        Parse, ParseStream
    }
};

use quote::format_ident;
use crate::common::macros::error_spanned;

mod with;
pub(crate) use with::*;

#[derive(Clone)]
pub struct ArgName(pub Ident);

impl Parse for ArgName {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(input.parse::<Ident>()?))
    }
}

impl Mutate for ArgName {
    fn mutate(&self, target: &mut Item) -> Result<()> {
        if let Item::Fn(function) = target {
            function.sig.ident = format_ident!("{}_{}", function.sig.ident, self.0);

            return Ok(());
        }
        
        Err(error_spanned!("{}\n ^ not a function", target))
    }
}

impl_unique_arg!(ArgName);
impl_to_tokens_wrapped!(ArgName);
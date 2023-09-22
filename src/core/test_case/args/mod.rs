use crate::common::macros::error_spanned;

use super::{
    Mutate,
    impl_unique_arg,
    impl_to_tokens_arg
};
use syn::{
    Result,
    Ident, Signature,
    parse::{
        Parse, ParseStream
    }
};

use quote::format_ident;

mod with;
pub(crate) use with::*;

#[derive(Clone)]
pub struct ArgName(pub Ident);

impl Parse for ArgName {
    fn parse(input: ParseStream) -> Result<Self> {
        let Result::Ok(name) = input.parse::<Ident>() else {
            return Err(error_spanned!("expected test name", &input.span()));
        };

        Ok(Self(name))
    }
}

impl Mutate for ArgName {
    type Item = Signature;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        target.ident = format_ident!("{}_{}", target.ident, self.0);

        return Ok(());
    }
}

impl_unique_arg!(ArgName);
impl_to_tokens_arg!(ArgName, 0);
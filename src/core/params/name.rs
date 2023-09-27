use quote::format_ident;

use syn::{
    Result,
    Ident, Signature,
    parse::{
        Parse, ParseStream
    }
};
use crate::params::{
    Mutate, macros::*
};

#[derive(Clone)]
pub(crate) struct ParamName(pub Ident);

impl Parse for ParamName {
    fn parse(input: ParseStream) -> Result<Self> {
        let Result::Ok(name) = input.parse::<Ident>() else {
            return Err(input.error("expected test name"));
        };

        Ok(Self(name))
    }
}

impl Mutate for ParamName {
    type Item = Signature;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        target.ident = format_ident!("{}_{}", target.ident, self.0);

        Ok(())
    }
}

impl_unique_param!(ParamName);
impl_to_tokens_param!(ParamName, 0);
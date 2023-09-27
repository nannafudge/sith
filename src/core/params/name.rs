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

#[derive(Debug, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        macros::error_spanned,
        tests::macros::*
    };

    use proc_macro2::Span;
    use quote::{
        quote, ToTokens
    };

    #[test]
    fn parse_name_ident() {
        assert_eq_parsed!(
            syn::parse2::<ParamName>(quote!(test)),
            Ok(ParamName(syn::Ident::new("test", Span::call_site())))
        );
    }

    #[test]
    fn parse_name_type() {
        assert_eq_parsed!(
            syn::parse2::<ParamName>(quote!(usize)),
            Ok(ParamName(syn::Ident::new("usize", Span::call_site())))
        );
    }

    #[test]
    fn parse_name_non_ident() {
        assert_eq_parsed!(
            syn::parse2::<ParamName>(quote!((group))),
            Err(error_spanned!("expected test name"))
        );
    }
}
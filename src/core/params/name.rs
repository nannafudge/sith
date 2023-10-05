use quote::format_ident;

use syn::{
    Result,
    Ident, Signature,
    parse::{
        Parse, ParseStream
    }
};
use crate::{
    common::macros::error_spanned,
    params::{
        Mutate, macros::*
    }
};

#[derive(Clone)]
pub(crate) struct ParamName(pub Ident);

impl Parse for ParamName {
    fn parse(input: ParseStream) -> Result<Self> {
        let Result::Ok(name) = input.parse::<Ident>() else {
            return Err(error_spanned!("expected test name", &input.span()));
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

impl_unique!(ParamName);
impl_param!(debug(ParamName, 0));
impl_param!(to_tokens(ParamName, 0));

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
    fn parse_accepts_ident() {
        assert_eq_parsed!(
            syn::parse2::<ParamName>(quote!(test)),
            Ok(ParamName(syn::Ident::new("test", Span::call_site())))
        );
    }

    #[test]
    fn parse_accepts_type() {
        assert_eq_parsed!(
            syn::parse2::<ParamName>(quote!(usize)),
            Ok(ParamName(syn::Ident::new("usize", Span::call_site())))
        );
    }

    #[test]
    fn parse_returns_error_on_non_ident() {
        assert_eq_parsed!(
            syn::parse2::<ParamName>(quote!((group))),
            Err(error_spanned!("expected test name"))
        );
    }
}
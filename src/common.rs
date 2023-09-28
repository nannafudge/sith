use proc_macro2::{
    Delimiter,
    TokenStream, TokenTree
};
use syn::{
    Attribute, Result,
    parse::{
        ParseStream, Parse
    }
};

use crate::common::macros::error_spanned;

const DELIM_DEBUG: [&str; 4] = ["()", "{}", "[]", "INVIS"];

pub fn attribute_name_to_string(attr: &Attribute) -> String {
    attr.meta.path().segments.iter()
        .next_back()
        .map_or(String::default(), | segment |
            segment.ident.to_string()
        )
}

pub fn parse_group_with_delim(delim: Delimiter, input: ParseStream) -> Result<TokenStream> {
    input.step(| cursor | {
        if let Some((content, _, next)) = cursor.group(delim) {
            return Ok((content.token_stream(), next));
        }

        // isize can be cast as usize in this case - there's no negative values
        Err(
            error_spanned!(
                format!("expected `{}`", DELIM_DEBUG[delim as usize]),
                &cursor.span()
            )
        )
    })
}

pub fn greedy_parse_with_delim<T, D>(input: ParseStream) -> Result<Vec<T>> where
    T: Parse,
    D: Parse
{
    let mut out: Vec<T> = Vec::with_capacity(1);
    while !input.is_empty() {
        out.push(input.parse::<T>()?);
        if !input.is_empty() {
            input.parse::<D>()?;
        }
    }

    Ok(out)
}

pub fn peek_next_tt(input: ParseStream) -> Result<TokenTree> {
    match input.cursor().token_tree() {
        Some((tt, _)) => Ok(tt),
        _ => Err(error_spanned!("expected token", &input.span()))
    }
}

pub fn parse_next_tt(input: ParseStream) -> Result<TokenTree> {
    input.step(| cursor | {
        cursor.token_tree().ok_or(error_spanned!("expected token", &cursor.span()))
    })
}

#[macro_use]
pub(crate) mod macros {
    macro_rules! error_spanned {
        ($error:expr) => {
            syn::Error::new(proc_macro2::Span::call_site(), $error)
        };
        ($error:expr, $spanned:expr $(, $others:expr )*) => {
            syn::Error::new(
                syn::spanned::Spanned::span($spanned)
                $(
                    .join(syn::spanned::Spanned::span($others)).unwrap()
                )*,
                $error
            )
        };
    }

    macro_rules! unwrap_or_err {
        ($target:expr, $($error:tt)+) => {
            if let Err(e) = $target {
                $($error)+.combine(e);
                return $($error)+.to_compile_error();
            } else {
                $target.unwrap()
            }
        };
        ($target:expr) => {
            match $target {
                Ok(t) => t,
                Err(e) => {
                    return e.to_compile_error();
                }
            }
        };
    }

    pub(crate) use error_spanned;
    pub(crate) use unwrap_or_err;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use shim::*;
    use macros::*;

    use proc_macro2::Span;
    use quote::{
        quote, ToTokens
    };

    mod attribute_name_to_string {
        use super::*;

        use syn::AttrStyle;

        #[test]
        fn works_with_single_name_attributes() {
            let attr = construct_attribute!(AttrStyle::Outer, test);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn works_with_pathed_name_attributes() {
            let attr = construct_attribute!(AttrStyle::Outer, my::path::to::test);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn works_with_list_attributes() {
            let attr = construct_attribute!(AttrStyle::Outer, test(one, two));

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn works_with_pathed_list_attributes() {
            let attr = construct_attribute!(AttrStyle::Outer, path::to::my::test(one, two));

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn works_with_name_value_attributes() {
            let attr = construct_attribute!(AttrStyle::Outer, test = 123);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn works_with_pathed_name_value_attributes() {
            let attr = construct_attribute!(AttrStyle::Outer, path::to::my::test = 123);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn returns_empty_string_when_empty_attribute() {
            let attr = construct_attribute!(AttrStyle::Outer);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "");
        }
    }

    mod greedy_parse_with_delim {
        use super::*;

        use core::marker::PhantomData;
        use syn::Token;
        use proc_macro2::{
            Literal, Group
        };

        struct VecLiteralShim<D: Parse, const S: bool>(Vec<Literal>, PhantomData<D>);

        impl<D: Parse, const S: bool> core::fmt::Debug for VecLiteralShim<D, S> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple("VecLiteralShim").field(&self.0).finish()
            }
        }

        impl<D: Parse, const S: bool> ToTokens for VecLiteralShim<D, S> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                self.0.iter().for_each(| item | item.to_tokens(tokens));
            }
        }

        impl<D: Parse, const S: bool> VecLiteralShim<D, S> {
            fn new(inner: Vec<Literal>) -> Self {
                Self(inner, PhantomData)
            }
        }

        impl<D: Parse> Parse for VecLiteralShim<D, true> {
            fn parse(input: ParseStream) -> Result<Self> {
                let out = greedy_parse_with_delim::<Literal, D>(input);
                assert!(input.is_empty(), "greedy_parse: failed to capture all tokens");

                Ok(Self(out?, PhantomData))
            }
        }

        impl<D: Parse> Parse for VecLiteralShim<D, false> {
            fn parse(input: ParseStream) -> Result<Self> {
                let out = greedy_parse_with_delim::<Literal, D>(input);
                Ok(Self(out?, PhantomData))
            }
        }

        type CommaSeperated = VecLiteralShim::<Token![,], true>;
        type GroupSeperated = VecLiteralShim::<Group, true>;

        #[test]
        fn parses_single_values() {
            impl_parse_shim!(CommaSeperated, CommaSeperated::parse);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<CommaSeperated>>(quote!("foo")),
                Ok(CommaSeperated::new(Vec::from([Literal::string("foo")])))
            );
        }

        #[test]
        fn parses_many_values() {
            impl_parse_shim!(CommaSeperated, CommaSeperated::parse);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<CommaSeperated>>(quote!("foo", "bar")),
                Ok(CommaSeperated::new(Vec::from([Literal::string("foo"), Literal::string("bar")])))
            );
        }

        #[test]
        fn accepts_arbitrary_delimiters() {
            impl_parse_shim!(GroupSeperated, GroupSeperated::parse);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<GroupSeperated>>(quote!("foo" (123, 456) "bar" (789))),
                Ok(GroupSeperated::new(Vec::from([Literal::string("foo"), Literal::string("bar")])))
            );
        }

        #[test]
        fn propegates_error_when_invalid_delim() {
            type CommaSeperatedNoEmptyCheck = VecLiteralShim::<Token![,], false>;
            impl_parse_shim!(CommaSeperatedNoEmptyCheck, CommaSeperatedNoEmptyCheck::parse);

            // Error message generation is delegated to syn's implementation here,
            // so we shouldn't explicity test against such in case it changes in the future
            assert!(syn::parse2::<ParseShim<CommaSeperatedNoEmptyCheck>>(quote!("foo";)).is_err());
        }

        #[test]
        fn returns_empty_collection_on_empty_input_tokens() {
            impl_parse_shim!(CommaSeperated, CommaSeperated::parse);
    
            assert_eq_parsed!(
                syn::parse2::<ParseShim<CommaSeperated>>(quote!()),
                Ok(CommaSeperated::new(Vec::new()))
            );
        }
    }

    mod parse_group_with_delim {
        use super::*;

        use proc_macro2::Delimiter;

        fn parse_parenthesis_shim(input: ParseStream) -> Result<TokenStream> {
            parse_group_with_delim(Delimiter::Parenthesis, input)
        }

        fn parse_braces_shim(input: ParseStream) -> Result<TokenStream> {
            parse_group_with_delim(Delimiter::Brace, input)
        }

        fn parse_bracket_shim(input: ParseStream) -> Result<TokenStream> {
            parse_group_with_delim(Delimiter::Bracket, input)
        }

        #[test]
        fn parses_parentheses() {
            impl_parse_shim!(TokenStream, parse_parenthesis_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!((test, 123))),
                Ok(quote!(test, 123))
            );
        }

        #[test]
        fn returns_error_on_invalid_parentheses_tokens() {
            impl_parse_shim!(TokenStream, parse_parenthesis_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!([test, 123])),
                Err(error_spanned!("expected `()`"))
            );
        }

        #[test]
        fn parses_braces() {
            impl_parse_shim!(TokenStream, parse_braces_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!({test, 123})),
                Ok(quote!(test, 123))
            );
        }

        #[test]
        fn returns_error_on_invalid_brace_tokens() {
            impl_parse_shim!(TokenStream, parse_braces_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!([test, 123])),
                Err(error_spanned!("expected `{}`"))
            );
        }

        #[test]
        fn parses_brackets() {
            impl_parse_shim!(TokenStream, parse_bracket_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!([test, 123])),
                Ok(quote!(test, 123))
            );
        }

        #[test]
        fn returns_error_on_invalid_bracket_tokens() {
            impl_parse_shim!(TokenStream, parse_bracket_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!({test, 123})),
                Err(error_spanned!("expected `[]`"))
            );
        }

        #[test]
        fn returns_error_on_no_tokens() {
            impl_parse_shim!(TokenStream, parse_parenthesis_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!()),
                Err(error_spanned!("expected `()`"))
            );
        }
    }

    mod parse_next_tt {
        use super::*;

        use proc_macro2::{
            Group, Delimiter,
            Punct, Spacing,
            Ident, Literal,
            TokenTree
        };

        impl_parse_shim!(TokenTree, parse_next_tt);
    
        #[test]
        fn parses_groups() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!((inner))),
                Ok(Group::new(Delimiter::Parenthesis, quote!(inner)))
            );
        }

        #[test]
        fn parses_idents() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(test)),
                Ok(Ident::new("test", Span::call_site()))
            );
        }

        #[test]
        fn parses_punctuation() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(,)),
                Ok(Punct::new(',', Spacing::Alone))
            );
        }

        #[test]
        fn parses_literals() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!("test")),
                Ok(Literal::string("test"))
            );
        }

        #[test]
        fn returns_error_on_no_tokens() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!()),
                Err(error_spanned!("expected token"))
            );
        }
    }

    mod peek_next_tt {
        use super::*;

        use proc_macro2::{
            Group, Delimiter,
            Punct, Spacing,
            Ident, Literal,
            TokenTree
        };

        // Takes an input stream and asserts that, post peek(),
        // it hasn't been moved
        fn peek_next_tt_spy(input: ParseStream) -> Result<TokenTree> {
            let start = input.fork();
            let out = peek_next_tt(input);
            assert_eq!(
                start.cursor().token_stream().to_string(),
                input.cursor().token_stream().to_string()
            );

            // For some reason, you need to advance
            // the input stream post-parse, otherwise
            // syn errors out if the parse stream hasn't been
            // read from...
            let _ = input.parse::<TokenTree>();

            out
        }

        impl_parse_shim!(TokenTree, peek_next_tt_spy);

        #[test]
        fn parses_groups_without_advancing_input_stream() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!((inner))),
                Ok(Group::new(Delimiter::Parenthesis, quote!(inner)))
            );
        }

        #[test]
        fn parses_idents_without_advancing_input_stream() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(test)),
                Ok(Ident::new("test", Span::call_site()))
            );
        }

        #[test]
        fn parses_punctuation_without_advancing_input_stream() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(,)),
                Ok(Punct::new(',', Spacing::Alone))
            );
        }

        #[test]
        fn parses_literals_without_advancing_input_stream() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!("test")),
                Ok(Literal::string("test"))
            );
        }
    
        #[test]
        fn returns_error_on_no_tokens() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!()),
                Err(error_spanned!("expected token"))
            );
        }
    }

    pub(crate) mod shim {
        macro_rules! impl_parse_shim {
            ($for_type:ty, $use_fn:path) => {
                #[derive(Debug)]
                pub struct ParseShim<T>(pub T);

                impl syn::parse::Parse for ParseShim<$for_type> {
                    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                        Ok(Self($use_fn(input)?))
                    }
                }

                impl quote::ToTokens for ParseShim<$for_type> where $for_type: quote::ToTokens {
                    fn to_tokens(&self, tokens: &mut TokenStream) {
                        quote::ToTokens::to_tokens(&self.0, tokens);
                    }
                }
            };
        }

        pub(crate) use impl_parse_shim;
    }

    pub(crate) mod macros {
        macro_rules! construct_attribute {
            ($style:expr, $($meta:tt)*) => {
                syn::Attribute {
                    pound_token: syn::token::Pound::default(),
                    style: $style,
                    bracket_token: syn::token::Bracket::default(),
                    meta: syn::parse2::<syn::Meta>(quote::quote!($($meta)*)).expect("Invalid attribute meta")
                }
            };
            ($style:expr) => {
                syn::Attribute {
                    pound_token: syn::token::Pound::default(),
                    style: $style,
                    bracket_token: syn::token::Bracket::default(),
                    meta: syn::Meta::Path(syn::Path{
                        leading_colon: None,
                        segments: syn::punctuated::Punctuated::<syn::PathSegment, syn::token::PathSep>::new()
                    })
                }
            };
        }

        macro_rules! assert_eq_parsed {
            ($left:expr, Ok($right:expr)) => {
                match &$left {
                    Ok(left) if left.to_token_stream().to_string().eq(&$right.to_token_stream().to_string()) => {},
                    _ => panic!("assertion failed:\nleft: {:?}\nright: Ok({})", &$left, &$right.to_token_stream())
                };
            };
            ($left:expr, Err($right:expr)) => {
                match &$left {
                    Err(left) if left.to_compile_error().to_string().eq(&$right.to_compile_error().to_string()) => {},
                    _ => panic!("assertion failed:\nleft: {:?}\nright: Err({:?})", &$left, &$right)
                };
            };
        }

        macro_rules! assert_eq_tokens {
            ($left:expr, $right:expr) => {
                if $left.to_token_stream().to_string().ne(&$right.to_token_stream().to_string()) {
                    panic!(
                        "assertion failed:\nleft: {}\nright: Ok({})",
                        &$left.to_token_stream(),
                        &$right.to_token_stream()
                    );
                }
            };
        }

        macro_rules! assert_eq_mutate {
            ($mutator:expr, $target:expr, Ok(())) => {
                if let Err(e) = &$mutator.mutate($target) {
                    panic!("assertion failed:\nleft: Ok(())\nright: Err({:?})", &e)
                }
            };
            ($mutator:expr, $target:expr, Err($right:expr)) => {
                match &$mutator.mutate($target) {
                    Err(e) => {
                        if !e.to_compile_error().to_string().eq(&$right.to_compile_error().to_string()) {
                            panic!("assertion failed:\nleft: {:?}\nright: {:?}", &e, &$right)
                        }
                    },
                    _ => panic!("assertion failed:\nleft: Ok(())\nright: Err({:?})", &$right)
                };
            };
        }

        pub(crate) use assert_eq_mutate;
        pub(crate) use assert_eq_parsed;
        pub(crate) use assert_eq_tokens;
        pub(crate) use construct_attribute;
    }
}
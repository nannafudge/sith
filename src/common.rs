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
        Err(cursor.error(format!("expected '{}'", DELIM_DEBUG[delim as usize])))
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
        _ => Err(input.error("expected token"))
    }
}

pub fn parse_next_tt(input: ParseStream) -> Result<TokenTree> {
    input.step(| cursor | {
        cursor.token_tree().ok_or(input.error("expected token"))
    })
}

#[macro_use]
pub(crate) mod macros {
    macro_rules! error_spanned {
        ($error:literal) => {
            syn::Error::new(proc_macro2::Span::call_site(), $error)
        };
        ($error:literal, $item:expr) => {
            syn::Error::new(syn::spanned::Spanned::span($item), $error)
        };
        (format!($formatter:literal), $item:expr $(, $other_items:expr )*) => {
            syn::Error::new(syn::spanned::Spanned::span($item), &format!(
                $formatter $(, $other_items)*
            ))
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
        fn ident() {
            let attr = construct_attribute!(AttrStyle::Outer, test);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn ident_path() {
            let attr = construct_attribute!(AttrStyle::Outer, my::path::to::test);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn list() {
            let attr = construct_attribute!(AttrStyle::Outer, test(one, two));

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn list_path() {
            let attr = construct_attribute!(AttrStyle::Outer, path::to::my::test(one, two));

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn name_value() {
            let attr = construct_attribute!(AttrStyle::Outer, test = 123);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn name_value_path() {
            let attr = construct_attribute!(AttrStyle::Outer, path::to::my::test = 123);

            assert_eq!(attribute_name_to_string(&attr).as_str(), "test");
        }

        #[test]
        fn empty() {
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
        fn single() {
            impl_parse_shim!(CommaSeperated, CommaSeperated::parse);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<CommaSeperated>>(quote!("foo")),
                Ok(CommaSeperated::new(Vec::from([Literal::string("foo")])))
            );
        }

        #[test]
        fn many() {
            impl_parse_shim!(CommaSeperated, CommaSeperated::parse);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<CommaSeperated>>(quote!("foo", "bar")),
                Ok(CommaSeperated::new(Vec::from([Literal::string("foo"), Literal::string("bar")])))
            );
        }

        #[test]
        fn arbitrary_delims() {
            impl_parse_shim!(GroupSeperated, GroupSeperated::parse);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<GroupSeperated>>(quote!("foo" (123, 456) "bar" (789))),
                Ok(GroupSeperated::new(Vec::from([Literal::string("foo"), Literal::string("bar")])))
            );
        }

        #[test]
        fn invalid_delim() {
            type CommaSeperatedNoEmptyCheck = VecLiteralShim::<Token![,], false>;
            impl_parse_shim!(CommaSeperatedNoEmptyCheck, CommaSeperatedNoEmptyCheck::parse);

            // Error message generation is delegated to syn's implementation here,
            // so we shouldn't explicity test against such in case it changes in the future
            assert!(syn::parse2::<ParseShim<CommaSeperatedNoEmptyCheck>>(quote!("foo";)).is_err());
        }

        #[test]
        fn empty() {
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
        fn parenthesis() {
            impl_parse_shim!(TokenStream, parse_parenthesis_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!((test, 123))),
                Ok(quote!(test, 123))
            );
        }

        #[test]
        fn parenthesis_invalid() {
            impl_parse_shim!(TokenStream, parse_parenthesis_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!([test, 123])),
                Err(error_spanned!("expected '()'"))
            );
        }

        #[test]
        fn braces() {
            impl_parse_shim!(TokenStream, parse_braces_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!({test, 123})),
                Ok(quote!(test, 123))
            );
        }

        #[test]
        fn braces_invalid() {
            impl_parse_shim!(TokenStream, parse_braces_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!([test, 123])),
                Err(error_spanned!("expected '{}'"))
            );
        }

        #[test]
        fn bracket() {
            impl_parse_shim!(TokenStream, parse_bracket_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!([test, 123])),
                Ok(quote!(test, 123))
            );
        }

        #[test]
        fn bracket_invalid() {
            impl_parse_shim!(TokenStream, parse_bracket_shim);

            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenStream>>(quote!({test, 123})),
                Err(error_spanned!("expected '[]'"))
            );
        }

        #[test]
        fn none() {
            impl_parse_shim!(TokenStream, parse_parenthesis_shim);

            assert!(syn::parse2::<ParseShim<TokenStream>>(quote!()).is_err());
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
        fn group() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!((inner))),
                Ok(Group::new(Delimiter::Parenthesis, quote!(inner)))
            );
        }

        #[test]
        fn ident() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(test)),
                Ok(Ident::new("test", Span::call_site()))
            );
        }

        #[test]
        fn punct() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(,)),
                Ok(Punct::new(',', Spacing::Alone))
            );
        }

        #[test]
        fn literal() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!("test")),
                Ok(Literal::string("test"))
            );
        }

        #[test]
        fn empty() {
            // Seems like syn returns its own 'end of input' error, regardless of
            // implementation - in case their error message changes, simply test
            // we propegate the error correctly
            assert!(syn::parse2::<ParseShim<TokenTree>>(quote!()).is_err());
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
        fn group() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!((inner))),
                Ok(Group::new(Delimiter::Parenthesis, quote!(inner)))
            );
        }

        #[test]
        fn ident() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(test)),
                Ok(Ident::new("test", Span::call_site()))
            );
        }

        #[test]
        fn punct() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(,)),
                Ok(Punct::new(',', Spacing::Alone))
            );
        }

        #[test]
        fn literal() {
            assert_eq_parsed!(
                syn::parse2::<ParseShim<TokenTree>>(quote!("test")),
                Ok(Literal::string("test"))
            );
        }
    
        #[test]
        fn empty() {
            assert!(syn::parse2::<ParseShim<TokenTree>>(quote!()).is_err());
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
                    _ => panic!("assertion failed:\nleft: {:?}\nright: Err({})", &$left, &$right.to_compile_error())
                };
            };
        }

        pub(crate) use assert_eq_parsed;
        pub(crate) use construct_attribute;
    }
}
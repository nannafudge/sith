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

pub fn attribute_name_to_str(attr: &Attribute) -> String {
    let segments = attr.meta.path().segments.iter().rev();
    segments.last().map_or(String::default(), | segment | segment.ident.to_string())
}

pub fn parse_group_with_delim(delim: Delimiter, input: ParseStream) -> Result<TokenStream> {
    input.step(| cursor | {
        if let Some((content, _, next)) = cursor.group(delim) {
            return Ok((content.token_stream(), next));
        }

        Err(cursor.error(format!("expected delimiter: {:?}", &delim)))
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

    mod attribute_name_to_str {
        use super::*;

        use syn::{
            AttrStyle, Path, Meta,
            punctuated::Punctuated
        };

        #[test]
        fn empty() {
            let attr = construct_attribute!(
                AttrStyle::Outer,
                Meta::Path(Path{ leading_colon: None, segments: Punctuated::new() })
            );

            assert_eq!(attribute_name_to_str(&attr).as_str(), "");
        }

        #[test]
        fn path() {
            let attr = construct_attribute!(
                AttrStyle::Outer,
                construct_attribute_meta!(test)
            );

            assert_eq!(attribute_name_to_str(&attr).as_str(), "test");
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
            assert_eq_with_shim!(
                syn::parse2::<ParseShim<TokenTree>>(quote!((inner))),
                Ok(Group::new(Delimiter::Parenthesis, quote!(inner)))
            );
        }

        #[test]
        fn ident() {
            assert_eq_with_shim!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(test)),
                Ok(Ident::new("test", Span::call_site()))
            );
        }

        #[test]
        fn punct() {
            assert_eq_with_shim!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(,)),
                Ok(Punct::new(',', Spacing::Alone))
            );
        }

        #[test]
        fn literal() {
            assert_eq_with_shim!(
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
            assert_eq_with_shim!(
                syn::parse2::<ParseShim<TokenTree>>(quote!((inner))),
                Ok(Group::new(Delimiter::Parenthesis, quote!(inner)))
            );
        }

        #[test]
        fn ident() {
            assert_eq_with_shim!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(test)),
                Ok(Ident::new("test", Span::call_site()))
            );
        }

        #[test]
        fn punct() {
            assert_eq_with_shim!(
                syn::parse2::<ParseShim<TokenTree>>(quote!(,)),
                Ok(Punct::new(',', Spacing::Alone))
            );
        }

        #[test]
        fn literal() {
            assert_eq_with_shim!(
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
            };
        }
    
        macro_rules! assert_eq_with_shim {
            ($left:expr, Ok($right:expr)) => {
                match &$left {
                    Ok(left) if left.0.to_token_stream().to_string().eq(&$right.to_token_stream().to_string()) => {},
                    _ => panic!("assertion failed:\nleft: {:?}\nright:{:?}", &$left, &$right)
                };
            };
            ($left:expr, Err($right:expr)) => {
                match &$left {
                    Err(left) if left.to_compile_error().to_string().eq(&$right.to_compile_error().to_string()) => {},
                    _ => panic!("assertion failed:\nleft: {:?}\nright:{:?}", &$left, &$right)
                };
            };
        }

        pub(crate) use impl_parse_shim;
        pub(crate) use assert_eq_with_shim;
    }

    pub(crate) mod macros {
        macro_rules! construct_attribute {
            ($style:expr, $meta:expr) => {
                syn::Attribute {
                    pound_token: syn::token::Pound::default(),
                    style: $style,
                    bracket_token: syn::token::Bracket::default(),
                    meta: $meta
                }
            };
        }

        macro_rules! construct_attribute_meta {
            ($($tokens:tt)*) => {
                syn::parse2::<syn::Meta>(quote::quote!($($tokens)*)).expect("Invalid attribute meta")
            };
        }

        pub(crate) use construct_attribute;
        pub(crate) use construct_attribute_meta;
    }
}
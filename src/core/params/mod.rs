use syn::{
    Result, Attribute,
    Pat, Type, FnArg,
    punctuated::Pair,
    token::Comma,
    parse::{
        ParseStream, Parse
    }
};
use proc_macro2::{
    TokenStream, Delimiter
};
use crate::{
    core::Mutate,
    common::{
        parse_group_with_delim,
        macros::error_spanned
    }
};

pub(crate) mod name;
pub(crate) mod with;
pub(crate) mod init;
pub(crate) mod setup;
pub(crate) mod teardown;

pub(crate) fn parse_param_args<T: Parse>(input: ParseStream) -> Result<T> {
    let param_inner: TokenStream = parse_group_with_delim(Delimiter::Parenthesis, input)?;
    syn::parse2::<T>(param_inner)
}

pub(crate) fn split_rust_fn_input(input: Option<&mut Pair<FnArg, Comma>>) -> Result<(&mut [Attribute], &mut Pat, &mut Type)> {
    match input {
        Some(Pair::Punctuated(param, _)) | Some(Pair::End(param)) => {
            if let FnArg::Typed(typed) = param {
                return Ok((typed.attrs.as_mut_slice(), &mut *typed.pat, &mut *typed.ty));
            }

            Err(error_spanned!("invalid parameter", &param))
        },
        _ => {
            Err(error_spanned!("no corresponding input", &input))
        }
    }
}

#[macro_use]
pub(crate) mod macros {
    macro_rules! impl_unique {
        ($target:ident $(< $generic:tt $(, $generics:tt)? >)?) => {
            impl $(< $generic $(, $generics)? >)? PartialEq for $target $(<$generic $(, $generics)?>)? {
                fn eq(&self, _: &Self) -> bool { true }
            }
            
            impl $(<$generic $(, $generics)?>)? Eq for $target $(<$generic $(, $generics)?>)? {

            }

            impl $(<$generic $(, $generics)?>)? PartialOrd for $target $(<$generic $(, $generics)?>)? {
                fn partial_cmp(&self, _: &Self) -> Option<core::cmp::Ordering> {
                    Some(core::cmp::Ordering::Equal)
                }
            }
            
            impl $(<$generic $(, $generics)?>)? Ord for $target $(<$generic $(, $generics)?>)? {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    self.partial_cmp(other).expect(
                        stringify!($target, ": Unexpected ord result")
                    )
                }
            }
        };
        ($target:ident $(< $generic:tt $(, $generics:tt)? >)?, $($path:tt)+) => {
            impl $(< $generic $(, $generics)? >)? PartialEq for $target $(<$generic $(, $generics)?>)? {
                fn eq(&self, other: &$target) -> bool {
                    self.$($path)+.eq(&other.$($path)+)
                }
            }
            
            impl $(<$generic $(, $generics)?>)? Eq for $target $(<$generic $(, $generics)?>)? {

            }

            impl $(<$generic $(, $generics)?>)? PartialOrd for $target $(<$generic $(, $generics)?>)? {
                fn partial_cmp(&self, other: &$target) -> Option<core::cmp::Ordering> {
                    self.$($path)+.partial_cmp(&other.$($path)+)
                }
            }
            
            impl $(<$generic $(, $generics)?>)? Ord for $target $(<$generic $(, $generics)?>)? {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    self.partial_cmp(other).expect(
                        stringify!($target, ": Unexpected ord result")
                    )
                }
            }
        };
    }

    macro_rules! impl_param {
        (debug($target:ty $(, $field:tt $(. $subfield:tt)* $(($iter_field:tt $(. $iter_subfield:tt)*))? )+)) => {
            impl core::fmt::Debug for $target {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_tuple(stringify!($target))
                    $(
                        .field(
                            &format!("{}", &crate::core::params::macros::impl_param!(
                                @to_token_stream(
                                    self, $field $(. $subfield)* $(($iter_field $(. $iter_subfield)*))?
                                )
                            ))
                        )
                    )+
                    .finish()
                }
            }
        };
        (to_tokens($target:ty $(, $field:tt $(. $subfield:tt)* $(($iter_field:tt $(. $iter_subfield:tt)*))? )+)) => {
            impl quote::ToTokens for $target {
                fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
                    $(
                        crate::core::params::macros::impl_param!(
                            @to_tokens(
                                self, tokens,
                                $field $(. $subfield)* $(($iter_field $(. $iter_subfield)*))?
                            )
                        );
                    )+
                }
            }
        };
        (@to_tokens($self:ident, $tokens:ident, iterable($field:tt $(. $subfield:tt)*))) => {
            $self.$field$(. $subfield)*.iter().for_each(| item | {
                quote::ToTokens::to_tokens(item, $tokens)
            })
        };
        (@to_token_stream($self:ident, iterable($field:tt $(. $subfield:tt)*))) => {
            $self.$field$(. $subfield)*.iter().fold(proc_macro2::TokenStream::new(), | mut acc, item | {
                quote::ToTokens::to_tokens(item, &mut acc);
                acc
            })
        };
        (@to_tokens($self:ident, $tokens:ident, $field:tt $(. $subfield:tt)*)) => {
            quote::ToTokens::to_tokens(&$self.$field$(. $subfield)*, $tokens)
        };
        (@to_token_stream($self:ident, $field:tt $(. $subfield:tt)*)) => {
            quote::ToTokens::to_token_stream(&$self.$field$(. $subfield)*)
        };
    }

    pub(crate) use impl_unique;
    pub(crate) use impl_param;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    mod split_rust_fn_input {
        use super::*;
        use crate::common::tests::macros::construct_attribute;
        
        use quote::ToTokens;
        use syn::{
            Signature, AttrStyle,
            parse_quote
        };

        const EMPTY_ATTRS: [syn::Attribute; 0] = [];

        macro_rules! assert_fn_inputs_eq {
            ($target:expr, Ok($attrs:expr, $pat:expr, $ty:expr)) => {
                match split_rust_fn_input($target.as_mut()) {
                    Ok((attrs, pat, ty)) => {
                        assert_eq!(attrs.len(), $attrs.len());
                        attrs.iter().zip($attrs).for_each(| (left, right) | {
                            assert_eq!(left.to_token_stream().to_string(), right.to_token_stream().to_string());
                        });
                        assert_eq!(pat.to_token_stream().to_string(), $pat);
                        assert_eq!(ty.to_token_stream().to_string(), $ty);
                    },
                    Err(e) => {
                        panic!("{:?}", e.to_compile_error().to_token_stream().to_string());
                    }
                };
            };
            ($target:expr, Err($err:expr)) => {
                match split_rust_fn_input($target.as_mut()) {
                    Ok((attrs, pat, ty)) => {
                        panic!(
                            "Expected err, received {} {} {}",
                            attrs.iter().fold(String::new(), | mut acc, a | {
                                acc += &a.to_token_stream().to_string();
                                acc
                            }),
                            pat.to_token_stream().to_string(),
                            ty.to_token_stream().to_string()
                        );
                    },
                    Err(e) => {
                        assert_eq!(e.to_compile_error().to_string(), $err.to_compile_error().to_string())
                    }
                };
            }
        }

        #[test]
        fn parses_explicit_type_annotation() {
            let mut sig: Signature = parse_quote!{
                fn _test(one: usize)
            };

            assert_fn_inputs_eq!(sig.inputs.pop(), Ok(EMPTY_ATTRS, "one", "usize"));
        }

        #[test]
        fn parses_ducked_type_annotation() {
            let mut sig: Signature = parse_quote!{
                fn _test(one: _)
            };

            assert_fn_inputs_eq!(sig.inputs.pop(), Ok(EMPTY_ATTRS, "one", "_"));
        }

        #[test]
        fn parses_outer_attribute() {
            let mut sig: Signature = parse_quote!{
                fn _test(#[my_attr] one: String)
            };
            let attributes = [
                construct_attribute!(AttrStyle::Outer, my_attr)
            ];

            assert_fn_inputs_eq!(sig.inputs.pop(), Ok(attributes, "one", "String"));
        }

        #[test]
        fn returns_error_on_self_fn_parameter() {
            let mut sig: Signature = parse_quote!{
                fn _test(self)
            };

            assert_fn_inputs_eq!(sig.inputs.pop(), Err(error_spanned!("invalid parameter")));
        }

        #[test]
        fn returns_error_when_no_bindings_provided() {
            assert_fn_inputs_eq!(None, Err(error_spanned!("no corresponding input")));
        }
    }

    pub(crate) mod macros {
        macro_rules! assert_mutator_order {
            ($ty:ident($target:expr) $(, $param:pat)+) => {
                {
                    let params: Vec<&$ty> = $target.iter().collect();
                    let mut param_count = 0;
                    $(
                        #[allow(unused_assignments)]
                        match params[param_count] {
                            $param => {
                                param_count += 1;
                            },
                            _ => {
                                panic!(
                                    "{} should be rank {}, found {:?}",
                                    stringify!($param), param_count,
                                    params[param_count]
                                );
                            }
                        }

                    )+
                }
            };
        }

        pub(crate) use assert_mutator_order;
    }
}
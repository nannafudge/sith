use syn::parse::{
    Result,
    ParseStream, Parse
};
use proc_macro2::{
    TokenStream, Delimiter
};
use crate::{
    core::Mutate,
    common::parse_group_with_delim
};

pub(crate) mod name;
pub(crate) mod with;
pub(crate) mod setup;
pub(crate) mod teardown;

pub(crate) fn parse_param_args<T: Parse>(input: ParseStream) -> Result<T> {
    let arg_inner: TokenStream = parse_group_with_delim(Delimiter::Parenthesis, input)?;
    syn::parse2::<T>(arg_inner)
}

#[macro_use]
pub(crate) mod macros {
    macro_rules! impl_unique_param {
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

    macro_rules! impl_to_tokens_param {
        ($target:ty, iterable($($path:tt)+)) => {
            impl quote::ToTokens for $target {
                fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
                    self.$($path)+.iter().for_each(| item | item.to_tokens(tokens));
                }
            }
        };
        ($target:ty, $($path:tt)+) => {
            impl quote::ToTokens for $target {
                fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
                    self.$($path)+.to_tokens(tokens);
                }
            }
        };
    }

    pub(crate) use impl_unique_param;
    pub(crate) use impl_to_tokens_param;
}
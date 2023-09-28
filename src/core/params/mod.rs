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
    let param_inner: TokenStream = parse_group_with_delim(Delimiter::Parenthesis, input)?;
    syn::parse2::<T>(param_inner)
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
        ($target:ty $(, $field:tt $(. $subfield:tt)* $(($iter_field:tt $(. $iter_subfield:tt)*))? )+ ) => {
            impl quote::ToTokens for $target {
                fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
                    $(
                        impl_param!(
                            @to_tokens(
                                self, tokens,
                                $field $(. $subfield)* $(($iter_field $(. $iter_subfield)*))?
                            )
                        );
                    )+
                }
            }

            impl core::fmt::Debug for $target {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_tuple(stringify!($target))
                    $(
                        .field(
                            &format!("{}", &impl_param!(
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
    }

    pub(crate) use impl_param;
    pub(crate) use impl_unique;
}
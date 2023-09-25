use quote::{
    ToTokens,
    spanned::Spanned
};
use syn::Result;

use crate::common::macros::error_spanned;
use std::collections::BTreeSet;

mod test_case;
mod test_suite;

pub use test_case::{
    TestCase, render_test_case
};
pub use test_suite::{
    TestSuite, render_test_suite
};

type Mutators<T> = BTreeSet<T>;

impl<T: Ord + Spanned + ToTokens> InsertUnique<T> for Mutators<T> {
    fn insert_unique(&mut self, item: T) -> Result<()> {
        let err = Err(error_spanned!("duplicate argument", &item));
        if !self.insert(item) {
            return err;
        }

        Ok(())
    }
}

trait Mutate {
    type Item;

    fn mutate(&self, target: &mut Self::Item) -> Result<()>;
}

trait InsertUnique<T> {
    fn insert_unique(&mut self, item: T) -> Result<()>;
}

#[macro_use]
mod macros {
    macro_rules! impl_unique_arg {
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

    macro_rules! impl_to_tokens_arg {
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

    macro_rules! rustc_test_attribute {
        ($($span:tt)+) => {
            Attribute {
                pound_token: syn::token::Pound::default(),
                style: AttrStyle::Outer,
                bracket_token: syn::token::Bracket::default(),
                meta: syn::Meta::Path(syn::Path::from(Ident::new_raw("test", $($span)+))),
            }
        };
    }

    pub(crate) use impl_unique_arg;
    pub(crate) use impl_to_tokens_arg;
    pub(crate) use rustc_test_attribute;
}
use syn::Result;
use std::collections::BTreeSet;

use quote::{
    ToTokens,
    spanned::Spanned
};

use crate::common::macros::error_spanned;

pub(crate) mod params;
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
        let err = Err(error_spanned!("duplicate parameter", &item));
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

    pub(crate) use rustc_test_attribute;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::common::tests::macros::assert_eq_parsed;

    #[test]
    fn insert_unique_returns_error_if_item_exists() {
        let mut mutators: Mutators<usize> = Mutators::new();

        assert!(mutators.insert_unique(0).is_ok());
        assert_eq_parsed!(
            mutators.insert_unique(0),
            Err(error_spanned!("duplicate parameter"))
        );
    }

    pub(crate) mod macros {
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
    }
}
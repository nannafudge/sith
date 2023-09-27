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
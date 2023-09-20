use super::{
    Mutate,
    impl_unique_arg,
    impl_to_tokens_wrapped
};
use syn::{
    Item, Stmt, Result
};

use crate::common::macros::error_spanned;

#[derive(Clone)]
pub struct ArgSetup(pub Vec<Stmt>);

impl Mutate for ArgSetup {
    fn mutate(&self, target: &mut Item) -> Result<()> {
        if let Item::Fn(function) = target {
            for stmt in self.0.iter().rev() {
                function.block.stmts.insert(0, stmt.clone());
            }

            return Ok(());
        }

        Err(error_spanned!("{}\n ^ not a function", target))
    }
}

impl_unique_arg!(ArgSetup);
impl_to_tokens_wrapped!(ArgSetup: collection);

#[derive(Clone)]
pub struct ArgTeardown(pub Vec<Stmt>);

impl Mutate for ArgTeardown {
    fn mutate(&self, target: &mut Item) -> Result<()> {
        if let Item::Fn(function) = target {
            function.block.stmts.extend(self.0.clone());

            return Ok(());
        }

        Err(error_spanned!("{}\n ^ not a function", target))
    }
}

impl_unique_arg!(ArgTeardown);
impl_to_tokens_wrapped!(ArgTeardown: collection);
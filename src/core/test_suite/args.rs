use super::{
    Mutate,
    impl_unique_arg,
    impl_to_tokens_arg
};
use syn::{
    Stmt, Block, Result
};

#[derive(Clone)]
pub struct ArgSetup(pub Vec<Stmt>);

impl Mutate for ArgSetup {
    type Item = Block;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        for stmt in self.0.iter().rev() {
            target.stmts.insert(0, stmt.clone());
        }

        return Ok(());
    }
}

impl_unique_arg!(ArgSetup);
impl_to_tokens_arg!(ArgSetup, iterable(0));

#[derive(Clone)]
pub struct ArgTeardown(pub Vec<Stmt>);

impl Mutate for ArgTeardown {
    type Item = Block;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        target.stmts.extend(self.0.clone());

        return Ok(());
    }
}

impl_unique_arg!(ArgTeardown);
impl_to_tokens_arg!(ArgTeardown, iterable(0));
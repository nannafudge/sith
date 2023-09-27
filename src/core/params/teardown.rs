use syn::{
    Stmt, Block, Result
};
use crate::params::{
    Mutate, macros::*
};

#[derive(Clone)]
pub struct ParamTeardown(pub Vec<Stmt>);

impl Mutate for ParamTeardown {
    type Item = Block;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        target.stmts.extend(self.0.clone());

        Ok(())
    }
}

impl_unique_param!(ParamTeardown);
impl_to_tokens_param!(ParamTeardown, iterable(0));
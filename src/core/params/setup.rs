use syn::{
    Stmt, Block, Result
};
use crate::params::{
    Mutate, macros::*
};

#[derive(Clone)]
pub struct ParamSetup(pub Vec<Stmt>);

impl Mutate for ParamSetup {
    type Item = Block;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        for stmt in self.0.iter().rev() {
            target.stmts.insert(0, stmt.clone());
        }

        Ok(())
    }
}

impl_unique_param!(ParamSetup);
impl_to_tokens_param!(ParamSetup, iterable(0));
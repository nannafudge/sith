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

impl_unique!(ParamSetup);
impl_param!(ParamSetup, iterable(0));

#[cfg(test)]
mod tests {
    use super::*;

    use syn::token::Brace;
    use quote::{
        quote, ToTokens
    };

    #[test]
    fn mutate_correctly_prepends_statements_preserving_order() {
        let stmts = Vec::from([
            syn::parse2::<Stmt>(quote!(let a = 1;)).unwrap(),
            syn::parse2::<Stmt>(quote!(let b = 2;)).unwrap(),
            syn::parse2::<Stmt>(quote!(let c = 3;)).unwrap()
        ]);
        
        /* let c = 3;
           let b = 2;
           let a = 1; */
        let mut target = Block {
            brace_token: Brace::default(),
            stmts: stmts.clone()
        };
        target.stmts.reverse();

        /* Expected output:
             let a = 1; <-- setup stmts
             let b = 2;
             let c = 3;
             let c = 3; <-- existing stmts
             let b = 2;
             let a = 1; */
        let mut expected = stmts.clone();
        expected.extend(target.stmts.clone());

        assert!(ParamSetup(stmts).mutate(&mut target).is_ok());
        assert_eq!(target.stmts.len(), expected.len());

        expected.iter().zip(target.stmts.iter()).for_each(| (left, right)| {
            assert_eq!(left.to_token_stream().to_string(), right.to_token_stream().to_string())
        });
    }

    #[test]
    fn mutate_works_with_no_parsed_statements() {
        let stmts = Vec::from([
            syn::parse2::<Stmt>(quote!(let a = 1;)).unwrap(),
            syn::parse2::<Stmt>(quote!(let b = 2;)).unwrap(),
            syn::parse2::<Stmt>(quote!(let c = 3;)).unwrap()
        ]);

        let mut target= Block {
            brace_token: Brace::default(),
            stmts: stmts.clone()
        };

        assert!(ParamSetup(Vec::new()).mutate(&mut target).is_ok());
        assert_eq!(target.stmts.len(), stmts.len());
        stmts.iter().zip(target.stmts.iter()).for_each(| (left, right)| {
            assert_eq!(left.to_token_stream().to_string(), right.to_token_stream().to_string())
        });
    }

    #[test]
    fn parameter_is_unique() {
        let first = ParamSetup(Vec::new());
        let second = ParamSetup(Vec::from([
            syn::parse2::<Stmt>(quote!(let a = 1;)).unwrap()
        ]));

        assert!(first.eq(&second));
    }

    #[test]
    fn to_tokens_outputs_internal_contents_literally() {
        let setup = ParamSetup(
            Vec::from([
                syn::parse2::<Stmt>(quote!(let a = 1;)).unwrap(),
                syn::parse2::<Stmt>(quote!(let b = 2;)).unwrap(),
                syn::parse2::<Stmt>(quote!(let c = 3;)).unwrap()
            ])
        );

        let expected = quote!{
            let a = 1;
            let b = 2;
            let c = 3;
        };

        assert_eq!(setup.to_token_stream().to_string(), expected.to_string());
    }
}
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

#[cfg(test)]
mod tests {
    use super::*;

    use syn::token::Brace;
    use quote::{
        quote, ToTokens
    };

    #[test]
    fn mutate_stmt_order() {
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
             let c = 3; <-- existing stmts
             let b = 2;
             let a = 1;
             let a = 1; <-- teardown stmts
             let b = 2;
             let c = 3; */
        let mut expected = target.stmts.clone();
        expected.extend(stmts.clone());

        assert!(ParamTeardown(stmts).mutate(&mut target).is_ok());
        assert_eq!(target.stmts.len(), expected.len());

        expected.iter().zip(target.stmts.iter()).for_each(| (left, right)| {
            assert_eq!(left.to_token_stream().to_string(), right.to_token_stream().to_string())
        });
    }

    #[test]
    fn empty() {
        let stmts = Vec::from([
            syn::parse2::<Stmt>(quote!(let a = 1;)).unwrap(),
            syn::parse2::<Stmt>(quote!(let b = 2;)).unwrap(),
            syn::parse2::<Stmt>(quote!(let c = 3;)).unwrap()
        ]);

        let mut target= Block {
            brace_token: Brace::default(),
            stmts: stmts.clone()
        };

        assert!(ParamTeardown(Vec::new()).mutate(&mut target).is_ok());
        assert_eq!(target.stmts.len(), stmts.len());
        stmts.iter().zip(target.stmts.iter()).for_each(| (left, right)| {
            assert_eq!(left.to_token_stream().to_string(), right.to_token_stream().to_string())
        });
    }

    #[test]
    fn uniqueness() {
        let first = ParamTeardown(Vec::new());
        let second = ParamTeardown(Vec::from([
            syn::parse2::<Stmt>(quote!(let a = 1;)).unwrap()
        ]));

        assert!(first.eq(&second));
    }

    #[test]
    fn to_tokens() {
        let teardown = ParamTeardown(
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

        assert_eq!(teardown.to_token_stream().to_string(), expected.to_string());
    }
}
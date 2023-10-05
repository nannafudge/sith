use std::collections::BTreeSet;

use proc_macro2::{TokenStream, Ident};

use quote::{
    quote, ToTokens, TokenStreamExt
};
use syn::{
    Expr, ExprAssign,
    Result, Stmt, Pat,
    Item, ItemStatic, ItemFn, StaticMutability, token::Mut
};
use crate::{
    params::{
        Mutate, macros::*,
        split_rust_fn_input
    },
    common::{
        macros::error_spanned,
        attribute_name_to_string
    }
};

#[derive(Clone)]
pub struct ParamInit(pub Vec<Stmt>);

impl Mutate for ParamInit {
    type Item = Item;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        fn assignments(stmt: &Stmt) -> Option<Ident> {
            if let Stmt::Expr(Expr::Assign(ExprAssign { left, .. }, ..), _) = stmt {
                if let Expr::Path(ident) = left.as_ref() {
                    return ident.path.get_ident().map(Clone::clone);
                }
            }

            None
        }

        match target {
            Item::Fn(func) if Self::is_init(func) => {
                // Check the #[init] fn body to ensure all declared `statics` are initialized,
                // as each `static` is initially set to MaybeUninit::assume_init() - Otherwise,
                // we could have undefined behavior
                let assignments: BTreeSet<Ident> = func.block.stmts.iter().filter_map(assignments).collect();
                while let Ok((_, Pat::Ident(name), _)) = split_rust_fn_input(func.sig.inputs.pop().as_mut()) {
                    if !assignments.contains(&name.ident) {
                        return Err(error_spanned!("uninitialized value", &name));
                    }
                }

                func.block.stmts.insert(0, syn::parse2::<Stmt>(
                    quote!(INIT.call_once(__INIT);)
                )?);
            },
            Item::Static(decl) if Self::is_init_target(decl)  => {
                decl.mutability = StaticMutability::Mut(Mut::default());
                *decl.expr.as_mut() = syn::parse2::<Expr>(quote!{
                    unsafe { core::mem::MaybeUninit::uninit().assume_init() };
                })?;
            },
            _ => {}
        }

        return Ok(());
    }
}

impl ToTokens for ParamInit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let init_stmts = self.0.iter().fold(TokenStream::new(), | mut acc, stmt | {
            stmt.to_tokens(&mut acc);
            acc
        });

        tokens.append_all(quote!{
            static INIT: std::sync::Once = std::sync::Once::new();

            fn __INIT() {
                unsafe {
                    #init_stmts
                }
            }
        });
    }
}

impl ParamInit {
    pub const IDENT: &'static str = "init";
    pub const INIT_TARGET: &'static str = "init";

    pub fn is_init(item: &ItemFn) -> bool {
        item.attrs.iter()
            .map(attribute_name_to_string)
            .any(| name | name == Self::IDENT)
    }

    pub fn is_init_target(item: &ItemStatic) -> bool {
        let Expr::Path(item) = item.expr.as_ref() else {
            return false;
        };

        item.path.get_ident().is_some_and(| name | *name == Self::INIT_TARGET)
    }
}

impl_unique!(ParamInit);
impl_param!(debug(ParamInit, iterable(0)));

/*#[cfg(test)]
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

        assert!(ParamInit(stmts).mutate(&mut target).is_ok());
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

        assert!(ParamInit(Vec::new()).mutate(&mut target).is_ok());
        assert_eq!(target.stmts.len(), stmts.len());
        stmts.iter().zip(target.stmts.iter()).for_each(| (left, right)| {
            assert_eq!(left.to_token_stream().to_string(), right.to_token_stream().to_string())
        });
    }

    #[test]
    fn parameter_is_unique() {
        let first = ParamInit(Vec::new());
        let second = ParamInit(Vec::from([
            syn::parse2::<Stmt>(quote!(let a = 1;)).unwrap()
        ]));

        assert!(first.eq(&second));
    }

    #[test]
    fn to_tokens_outputs_internal_contents_literally() {
        let setup = ParamInit(
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
}*/
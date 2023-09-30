use quote::ToTokens;

use proc_macro2::{
    TokenStream, TokenTree,
    Group
};
use syn::{
    Ident, Type, Pat,
    ItemFn, Stmt, Result,
    parse::{
        Parse, ParseStream
    },
    buffer::{
        TokenBuffer, Cursor
    }
};
use super::{
    ParamWithInner, split_rust_fn_input
};
use crate::{
    common::macros::error_spanned,
    params::{
        Mutate, macros::*, parse_param_args
    }
};

#[derive(Clone)]
pub(crate) struct ParamVerbatim(TokenStream);

impl Parse for ParamVerbatim {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Ident>();
        Ok(Self(parse_param_args::<TokenStream>(input)?))
    }
}

impl Mutate for ParamVerbatim {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match split_rust_fn_input(target.sig.inputs.pop().as_mut())? {
            (_, Pat::Ident(def), Type::Infer(_)) => {
                for stmt in &mut target.block.stmts {
                    let tokens = recursive_descent_replace(
                        &mut TokenBuffer::new2(stmt.to_token_stream()).begin(),
                        &def.ident,
                        &self.0
                    );
                    *stmt = syn::parse2::<Stmt>(tokens)?;
                }

                Ok(())
            },
            (_, _, ty) => {
                Err(error_spanned!("verbatim inputs must be tagged as `_`", ty))
            }
        }
    }
}

impl From<ParamVerbatim> for ParamWithInner {
    fn from(value: ParamVerbatim) -> Self {
        ParamWithInner::Verbatim(value)
    }
}

impl_param!(ParamVerbatim, 0);

fn recursive_descent_replace(input: &mut Cursor, pattern: &Ident, substitute: &TokenStream) -> TokenStream {
    let mut out = TokenStream::new();
    while let Some((tt, next)) = input.token_tree() {
        match tt {
            TokenTree::Group(item) => {
                let (mut start, _, _) = input.group(item.delimiter()).unwrap();

                Group::new(
                    item.delimiter(),
                    recursive_descent_replace(&mut start, pattern, substitute)
                ).to_tokens(&mut out);
            },
            TokenTree::Ident(item) if item.eq(pattern) => {
                substitute.to_tokens(&mut out);
            },
            _ => {
                tt.to_tokens(&mut out);
            }
        }

        *input = next;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::tests::macros::*,
        core::tests::macros::*
    };
    
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn parse_accepts_empty_tokens() {
        assert_eq_parsed!(
            syn::parse2::<ParamVerbatim>(quote!(verbatim())),
            Ok(quote!())
        );
    }

    #[test]
    fn parse_accepts_arbitrary_tokens() {
        assert_eq_parsed!(
            syn::parse2::<ParamVerbatim>(quote!(verbatim(n *$ ## 1- {}))),
            Ok(quote!(n *$ ## 1- {}))
        );
    }

    #[test]
    fn parse_returns_error_when_missing_param_parenthesis() {
        assert_eq_parsed!(
            syn::parse2::<ParamVerbatim>(quote!(verbatim)),
            Err(error_spanned!("expected `()`"))
        );
    }

    #[test]
    fn mutate_works_when_no_tokens_captured() {
        let mut target: ItemFn = parse_quote!{
            fn foo(r#replace: _) {
                let a = r#replace(0);
            }
        };

        assert_eq_mutate!(
            ParamVerbatim(quote!()),
            &mut target,
            Ok(())
        );

        assert_eq!(target.sig.inputs.len(), 0);
        assert_eq_tokens!(target.block.stmts[0], quote!{
            let a = (0);
        });
    }

    #[test]
    fn mutate_returns_error_when_not_annotated_as_infer_type() {
        let mut target: ItemFn = parse_quote!{
            fn foo(r#replace: usize) {
                let a = Option::r#replace;
            }
        };

        assert_eq_mutate!(
            ParamVerbatim(quote!()),
            &mut target,
            Err(error_spanned!("verbatim inputs must be tagged as `_`"))
        );
    }

    mod recursive_descent_replace {
        use super::*;

        use syn::{
            parse_quote,
            buffer::TokenBuffer
        };

        #[test]
        fn works_within_nested_parenthesis() {
            let target = TokenBuffer::new2(quote!{
                let val: usize = foo(r#replace, bar(r#replace));
            });

            let new = recursive_descent_replace(
                &mut target.begin(),
                &parse_quote!(r#replace),
                &quote!(123)
            );

            assert_eq_tokens!(new, quote!{
                let val: usize = foo(123, bar(123));
            });
        }

        #[test]
        fn works_within_nested_braces() {
            let target = TokenBuffer::new2(quote!{
                let val: Matrix = if true {
                    Matrix::M2 { x: r#replace, y: 1 }
                } else {
                    Matrix::M2 { x: 1, y: r#replace }
                }
            });

            let new = recursive_descent_replace(
                &mut target.begin(),
                &parse_quote!(r#replace),
                &quote!(123)
            );

            assert_eq_tokens!(new, quote!{
                let val: Matrix = if true {
                    Matrix::M2 { x: 123, y: 1 }
                } else {
                    Matrix::M2 { x: 1, y: 123 }
                }
            });
        }

        #[test]
        fn works_within_nested_brackets() {
            let target = TokenBuffer::new2(quote!{
                let tiles: [Chunk<[Tile; r#replace]>; r#replace];
            });

            let new = recursive_descent_replace(
                &mut target.begin(),
                &parse_quote!(r#replace),
                &quote!(64)
            );

            assert_eq_tokens!(new, quote!{
                let tiles: [Chunk<[Tile; 64]>; 64];
            });
        }

        #[test]
        fn works_with_empty_substitution_tokens() {
            let target = TokenBuffer::new2(quote!());
            let new = recursive_descent_replace(
                &mut target.begin(),
                &parse_quote!(r#replace),
                &quote!(64)
            );

            assert_eq_tokens!(new, quote!());
        }

        #[test]
        fn result_is_unchanged_when_no_matches_found() {
            let target = TokenBuffer::new2(quote!{
                let a: usize = usize::MAX;
            });

            let new = recursive_descent_replace(
                &mut target.begin(),
                &parse_quote!(r#replace),
                &quote!(64)
            );

            assert_eq_tokens!(new, quote!{
                let a: usize = usize::MAX;
            });
        }
    }
}
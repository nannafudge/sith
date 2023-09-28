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
    ParamWithInner, parse_rust_fn_input
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
        match parse_rust_fn_input(target.sig.inputs.pop().as_mut())? {
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
                Err(error_spanned!("vertabim(): expected `_`", ty))
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
    
}
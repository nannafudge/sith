use super::{
    Mutate,
    impl_unique_arg,
    impl_to_tokens_arg,
    super::parse_arg_parameterized
};
use crate::common::{
    peek_next_tt,
    greedy_parse_with_delim,
    macros::error_spanned
};
use proc_macro2::{
    TokenStream, TokenTree, Literal, Group
};
use syn::{
    Ident, Type, Pat, Attribute,
    ItemFn, FnArg, Expr, Stmt,
    Token, Result,
    parse::{
        Parse, ParseStream
    },
    buffer::{
        TokenBuffer, Cursor
    },
    token::{
        Comma, Mut
    },
    punctuated::Pair
};

use quote::{ToTokens, TokenStreamExt};

#[derive(Clone)]
struct WithVerbatim(TokenStream);

impl Parse for WithVerbatim {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Ident>();
        Ok(Self(parse_arg_parameterized::<TokenStream>(input)?))
    }
}

impl Mutate for WithVerbatim {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match parse_fn_param(target.sig.inputs.pop().as_mut())? {
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

impl_to_tokens_arg!(WithVerbatim, 0);

#[derive(Clone)]
struct WithAssignment(Option<Mut>, Expr);

impl Parse for WithAssignment {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(input.parse::<Mut>().ok(), input.parse::<Expr>()?))
    }
}

impl Mutate for WithAssignment {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        let mut fn_input = target.sig.inputs.pop();
        let (attrs, Pat::Ident(def), ty) = parse_fn_param(fn_input.as_mut())? else {
            return Err(error_spanned!("expected identifier", &fn_input));
        };

        // If mut override is present, ensure it's set
        if self.0.is_some() {
            def.mutability = self.0;
        }

        let mut tokens = TokenStream::new();
        // Apply defined attributes above the composed `let` statement
        tokens.append_all(attrs);
        quote::quote!(let #def: #ty = #self;).to_tokens(&mut tokens);

        target.block.stmts.insert(0, syn::parse2::<Stmt>(tokens)?);
        return Ok(());
    }
}

impl_to_tokens_arg!(WithAssignment, 1);

#[derive(Clone)]
enum WithExpr {
    // Mutators should be defined in the order they must apply
    Assignment(WithAssignment),
    Verbatim(WithVerbatim)
}

impl Parse for WithExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        let TokenTree::Ident(name) = peek_next_tt(input)? else {
            // By default assume un-named parameters are direct test function inputs
            return Ok(WithExpr::Assignment(input.parse::<WithAssignment>()?));
        };

        match name.to_string().as_bytes() {
            b"verbatim" => {
                return Ok(WithExpr::Verbatim(input.parse::<WithVerbatim>()?));
            },
            _ => {
                // Might be Union/Struct Tuple/Enum Variant - attempt to parse
                return Ok(WithExpr::Assignment(input.parse::<WithAssignment>()?));
            }
        }
    }
}

impl ToTokens for WithExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            WithExpr::Assignment(item) => item.to_tokens(tokens),
            WithExpr::Verbatim(item) => item.to_tokens(tokens)
        }
    }
}

impl Mutate for WithExpr {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match self {
            WithExpr::Assignment(item) => item.mutate(target),
            WithExpr::Verbatim(item) => item.mutate(target)
        }
    }
}

impl From<WithVerbatim> for WithExpr {
    fn from(value: WithVerbatim) -> Self {
        WithExpr::Verbatim(value)
    }
}

impl From<WithAssignment> for WithExpr {
    fn from(value: WithAssignment) -> Self {
        WithExpr::Assignment(value)
    }
}

#[derive(Clone)]
pub(crate) struct ArgWith(Vec<WithExpr>);

impl Parse for ArgWith {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(
            greedy_parse_with_delim::<WithExpr, Token![,]>(input)?
        ))
    }
}

impl Mutate for ArgWith {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        if self.0.len() != target.sig.inputs.len() {
            return Err(
                error_spanned!(
                    format!("with(): {} fn inputs but only {} args declared"),
                    &target.sig.inputs,
                    &Literal::usize_unsuffixed(target.sig.inputs.len()),
                    &Literal::usize_unsuffixed(self.0.len())
                )
            );
        }

        let mut inputs = core::mem::take(&mut target.sig.inputs).into_iter();

        for mutator in &self.0 {
            // Use target.sig.inputs vec as an input queue/stack -
            // with() mutators read from this and apply their mutation to such
            target.sig.inputs.push(inputs.next().unwrap());
            mutator.mutate(target)?;
        }

        return Ok(());
    }
}

impl_unique_arg!(ArgWith);
impl_to_tokens_arg!(ArgWith, iterable(0));

fn recursive_descent_replace<'a>(input: &mut Cursor<'a>, pattern: &Ident, substitute: &TokenStream) -> TokenStream {
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

fn parse_fn_param<'c>(fn_param: Option<&'c mut Pair<FnArg, Comma>>) -> Result<(&mut [Attribute], &mut Pat, &mut Type)> {
    match fn_param {
        Some(Pair::Punctuated(param, _)) | Some(Pair::End(param)) => {
            if let FnArg::Typed(typed) = param {
                return Ok((typed.attrs.as_mut_slice(), &mut *typed.pat, &mut *typed.ty));
            }

            return Err(error_spanned!("invalid parameter", &param));
        },
        _ => {
            Err(error_spanned!("no corresponding input", &fn_param))
        }
    }
}
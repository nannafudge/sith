use super::{
    Mutate,
    impl_unique_arg,
    impl_to_tokens_wrapped,
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
    Ident, Expr, ItemFn, Token,
    FnArg, Result, Stmt, Type, Pat,
    parse::{
        Parse, ParseStream
    },
    buffer::{
        TokenBuffer, Cursor
    },
    token::Comma,
    punctuated::Pair
};

use quote::ToTokens;

#[derive(Clone)]
struct WithVerbatim(TokenStream);

impl Parse for WithVerbatim {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(parse_arg_parameterized::<TokenStream>(input)?))
    }
}

impl Mutate for WithVerbatim {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match parse_fn_arg(target.sig.inputs.pop().as_ref())? {
            (ident, Type::Infer(_)) => {
                for stmt in &mut target.block.stmts {
                    // Unsure if this is faster than string.replace()
                    // TODO: Benchmark
                    let tokens = recursive_descent_replace(
                        &mut TokenBuffer::new2(stmt.to_token_stream()).begin(),
                        ident,
                        &self.0
                    );
                    *stmt = syn::parse2::<Stmt>(tokens)?;
                }

                Ok(())
            },
            (_, ty) => {
                Err(error_spanned!("{}\n ^ vertabim(): expected `_`", ty))
            }
        }
    }
}

impl_to_tokens_wrapped!(WithVerbatim);

#[derive(Clone)]
struct WithAssignment(Expr);

impl Parse for WithAssignment {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl Mutate for WithAssignment {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        let next = target.sig.inputs.pop();
        let (ident, ty) = parse_fn_arg(next.as_ref())?;
        let mut tokens = TokenStream::new();

        syn::token::Let::default().to_tokens(&mut tokens);
        ident.to_tokens(&mut tokens);
        syn::token::Colon::default().to_tokens(&mut tokens);
        ty.to_tokens(&mut tokens);
        syn::token::Eq::default().to_tokens(&mut tokens);
        self.0.to_tokens(&mut tokens);
        syn::token::Semi::default().to_tokens(&mut tokens);

        target.block.stmts.insert(0, syn::parse2::<Stmt>(tokens)?);

        return Ok(());
    }
}

impl_to_tokens_wrapped!(WithAssignment);

#[derive(Clone)]
enum WithExpr {
    Assignment(WithAssignment),
    Verbatim(WithVerbatim)
}

impl Parse for WithExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        // It would be more efficient to use step() directly here - but would
        // also be messy, add more bloat to codebase, and (likely) the performance
        // hit from doing it like this isn't large regardless
        if let (TokenTree::Ident(name), next) = peek_next_tt(input.cursor())? {
            match name.to_string().as_bytes() {
                b"verbatim" => {
                    *(&mut input.cursor()) = next;
                    return Ok(WithExpr::Verbatim(input.parse::<WithVerbatim>()?));
                },
                _ => {}
            }
        }

        Ok(WithExpr::Assignment(input.parse::<WithAssignment>()?))
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

#[derive(Clone)]
pub(crate) struct ArgWith(Vec<WithExpr>);

impl Parse for ArgWith {
    fn parse(input: ParseStream) -> Result<Self> {
        let items: Vec<WithExpr> = greedy_parse_with_delim::<WithExpr, Token![,]>(input)?;
        Ok(Self(items))
    }
}

impl Mutate for ArgWith {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        if self.0.len() != target.sig.inputs.len() {
            return Err(
                error_spanned!(
                    "{}\n ^ expected {} args, found {}",
                    &target.sig.inputs,
                    &Literal::usize_unsuffixed(self.0.len()),
                    &Literal::usize_unsuffixed(target.sig.inputs.len())
                )
            );
        }

        // Steal inputs from signature, leaving the original function sig inputs empty
        let mut inputs = core::mem::take(&mut target.sig.inputs).into_iter();
        // Apply each mutator with its corresponding input
        for mutator in &self.0 {
            target.sig.inputs.push(inputs.next().unwrap());
            mutator.mutate(target)?;
        }

        return Ok(());
    }
}

impl_unique_arg!(ArgWith);
impl_to_tokens_wrapped!(ArgWith: collection);

fn recursive_descent_replace<'a>(input: &mut Cursor<'a>, pattern: &Ident, substitute: &TokenStream) -> TokenStream {
    let mut out = TokenStream::new();
    while let Some((tt, next)) = input.token_tree() {
        match tt {
            TokenTree::Group(item) => {
                let (mut start, _, _) = input.group(item.delimiter())
                    .expect("Invariant: Expected group contents");

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

fn parse_fn_arg<'c>(arg: Option<&'c Pair<FnArg, Comma>>) -> Result<(&'c Ident, &'c Type)> {
    match arg {
        Some(Pair::Punctuated(fn_arg, _)) | Some(Pair::End(fn_arg)) => {
            if let FnArg::Typed(typed_arg) = fn_arg {
                if let Pat::Ident(decl) = typed_arg.pat.as_ref() {
                    return Ok((&decl.ident, typed_arg.ty.as_ref()));
                }
            }
    
            return Err(error_spanned!("{}\n ^ invalid arg", &arg));
        },
        _ => {
            Err(error_spanned!("{:?}\n ^ no corresponding with() input", &arg))
        }
    }
}
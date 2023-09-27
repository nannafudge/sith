use quote::ToTokens;

use proc_macro2::{
    Literal,
    TokenStream, TokenTree
};
use syn::{
    Type, Pat, Attribute,
    ItemFn, FnArg, Result,
    parse::{
        Parse, ParseStream
    },
    punctuated::Pair,
    token::Comma, Token
};
use crate::{
    common::{
        macros::error_spanned,
        greedy_parse_with_delim,
        peek_next_tt
    },
    params::{
        Mutate, macros::*
    }
};

mod assignment;
mod verbatim;

use self::{assignment::*, verbatim::*};

#[derive(Clone)]
enum ParamWithInner {
    // Mutators should be defined in the order they must apply
    Assignment(ParamAssignment),
    Verbatim(ParamVerbatim)
}

impl Parse for ParamWithInner {
    fn parse(input: ParseStream) -> Result<Self> {
        let TokenTree::Ident(name) = peek_next_tt(input)? else {
            // By default assume un-named parameters are direct test function inputs
            return Ok(ParamWithInner::Assignment(input.parse::<ParamAssignment>()?));
        };

        match name.to_string().as_bytes() {
            b"verbatim" => {
                Ok(ParamWithInner::Verbatim(input.parse::<ParamVerbatim>()?))
            },
            _ => {
                // Might be Union/Struct Tuple/Enum Variant - attempt to parse
                Ok(ParamWithInner::Assignment(input.parse::<ParamAssignment>()?))
            }
        }
    }
}

impl ToTokens for ParamWithInner {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            ParamWithInner::Assignment(item) => item.to_tokens(tokens),
            ParamWithInner::Verbatim(item) => item.to_tokens(tokens)
        }
    }
}

impl Mutate for ParamWithInner {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match self {
            ParamWithInner::Assignment(item) => item.mutate(target),
            ParamWithInner::Verbatim(item) => item.mutate(target)
        }
    }
}

#[derive(Clone)]
pub(crate) struct ParamWith(Vec<ParamWithInner>);

impl Parse for ParamWith {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(
            greedy_parse_with_delim::<ParamWithInner, Token![,]>(input)?
        ))
    }
}

impl Mutate for ParamWith {
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

        Ok(())
    }
}

impl_unique_param!(ParamWith);
impl_to_tokens_param!(ParamWith, iterable(0));

fn parse_rust_fn_input(fn_param: Option<&mut Pair<FnArg, Comma>>) -> Result<(&mut [Attribute], &mut Pat, &mut Type)> {
    match fn_param {
        Some(Pair::Punctuated(param, _)) | Some(Pair::End(param)) => {
            if let FnArg::Typed(typed) = param {
                return Ok((typed.attrs.as_mut_slice(), &mut *typed.pat, &mut *typed.ty));
            }

            Err(error_spanned!("invalid parameter", &param))
        },
        _ => {
            Err(error_spanned!("no corresponding input", &fn_param))
        }
    }
}
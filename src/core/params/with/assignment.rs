use proc_macro2::TokenStream;

use syn::{
    ItemFn, Expr, Stmt,
    Pat, Result,
    parse::{
        Parse, ParseStream
    },
    token::Mut
};
use quote::{
    ToTokens, TokenStreamExt
};
use super::{
    ParamWithInner, parse_rust_fn_input
};
use crate::{
    common::macros::error_spanned,
    params::{
        Mutate, macros::*
    }
};

#[derive(Clone)]
pub(crate) struct ParamAssignment(Option<Mut>, Expr);

impl Parse for ParamAssignment {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(input.parse::<Mut>().ok(), input.parse::<Expr>()?))
    }
}

impl Mutate for ParamAssignment {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        let mut fn_input = target.sig.inputs.pop();
        let (attrs, Pat::Ident(def), ty) = parse_rust_fn_input(fn_input.as_mut())? else {
            return Err(error_spanned!("expected identifier", &fn_input));
        };

        // If mut override is present, ensure it's set
        if self.0.is_some() {
            def.mutability = self.0;
        }

        let mut tokens = TokenStream::new();
        // Apply defined attributes above the composed `let` statement
        tokens.append_all(attrs);

        let expr: &Expr = &self.1;
        quote::quote!(let #def: #ty = #expr;).to_tokens(&mut tokens);

        target.block.stmts.insert(0, syn::parse2::<Stmt>(tokens)?);
        Ok(())
    }
}

impl From<ParamAssignment> for ParamWithInner {
    fn from(value: ParamAssignment) -> Self {
        ParamWithInner::Assignment(value)
    }
}

impl_param!(ParamAssignment, 0, 1);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::tests::macros::*;

    use quote::quote;
    
    #[test]
    fn parse_enum_variant() {
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(Option::Some(0))),
            Ok(quote!(Option::Some(0)))
        );
    }

    fn parse_struct_tuple() {

    }

    fn parse_struct_new() {

    }

    fn parse_primitive() {

    }

    fn parse_with_mut_override() {

    }
}
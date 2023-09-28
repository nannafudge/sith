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
    ParamWithInner, split_rust_fn_input
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
        Ok(Self(
            input.parse::<Mut>().ok(),
            input.parse::<Expr>().map_err(|e| {
                error_spanned!("expected input", &e.span())
            })?
        ))
    }
}

impl Mutate for ParamAssignment {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        let mut fn_input = target.sig.inputs.pop();
        let (attrs, Pat::Ident(def), ty) = split_rust_fn_input(fn_input.as_mut())? else {
            // https://doc.rust-lang.org/reference/items/functions.html
            // Sig may only contain: (SelfParam[0..1], FunctionParam[0..n])
            unreachable!("Rust syntax should not allow any `Pat` but `Pat::Ident` in fn inputs");
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
    use crate::common::{
        macros::error_spanned,
        tests::macros::*
    };

    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn parses_primitive_inputs() {
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(0)),
            Ok(quote!(0))
        );
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!("test")),
            Ok(quote!("test"))
        );
    }

    #[test]
    fn parses_enum_variant_inputs() {
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(Option::Some(0))),
            Ok(quote!(Option::Some(0)))
        );
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(Matrix::M3{x: 0, y: 0, z: 0})),
            Ok(quote!(Matrix::M3{x: 0, y: 0, z: 0}))
        );
    }
    
    #[test]
    fn parses_named_tuple_inputs() {
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(
                MyStruct::<'static, str>("test", (0, 1))
            )),
            Ok(quote!(
                MyStruct::<'static, str>("test", (0, 1))
            ))
        );

        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(
                MyStruct::<'static, str>{a: &"test", b:(0, 1)}
            )),
            Ok(quote!(
                MyStruct::<'static, str>{a: &"test", b:(0, 1)}
            ))
        );
    }

    #[test]
    fn parses_inputs_with_instantiation_methods() {
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(
                MyStruct::<'static, str>::new("test", (0, 1))
            )),
            Ok(quote!(
                MyStruct::<'static, str>::new("test", (0, 1))
            ))
        );
    }

    #[test]
    fn parses_ref_inputs() {
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(
                &mut MyStruct::<'static, str>::new("test", (0, 1))
            )),
            Ok(quote!(
                &mut MyStruct::<'static, str>::new("test", (0, 1))
            ))
        );
    }

    #[test]
    fn parses_inputs_with_mut_override() {
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!(mut usize::default())),
            Ok(quote!(mut usize::default()))
        );
    }

    #[test]
    fn parse_returns_error_on_empty() {
        assert_eq_parsed!(
            syn::parse2::<ParamAssignment>(quote!()),
            Err(error_spanned!("expected input"))
        );
    }

    #[test]
    fn mutate_accepts_infer_type() {
        let mut target: ItemFn = parse_quote!{
            fn input(input: _) {}
        };

        let param: ParamAssignment = ParamAssignment(None, parse_quote!("test"));
        assert_eq_mutate!(param, &mut target, Ok(()));

        assert_eq_tokens!(
            target.block.stmts[0],
            quote!(let input: _ = "test";)
        );
    }

    #[test]
    fn mutate_propagates_attributes() {
        let mut target: ItemFn = parse_quote!{
            fn input(#[my_attr] input: bool) {}
        };

        let param: ParamAssignment = ParamAssignment(None, parse_quote!(true));
        assert_eq_mutate!(param, &mut target, Ok(()));

        assert_eq_tokens!(
            target.block.stmts[0],
            quote!{
                #[my_attr]
                let input: bool = true;
            }
        );
    }

    #[test]
    fn mutate_propagates_lifetimes() {
        let mut target_lifetimed: ItemFn = parse_quote!{
            fn input(input: &'static MyStruct) {}
        };

        assert_eq_mutate!(
            ParamAssignment(None, parse_quote!(
                &MyStruct::<'static, str>("test")
            )),
            &mut target_lifetimed, Ok(())
        );

        assert_eq_tokens!(
            target_lifetimed.block.stmts[0],
            quote!(
                let input: &'static MyStruct = &MyStruct::<'static, str>("test");
            )
        );
    }

    #[test]
    fn mutate_propagates_mut_overrides() {
        let mut target_nonexisting_mut: ItemFn = parse_quote!{
            fn input(input: usize) {}
        };

        let param: ParamAssignment = ParamAssignment(parse_quote!(mut), parse_quote!(123));
        assert_eq_mutate!(param, &mut target_nonexisting_mut, Ok(()));

        assert_eq_tokens!(
            target_nonexisting_mut.block.stmts[0],
            quote!(let mut input: usize = 123;)
        );
    }

    #[test]
    fn mutate_propagates_mut_overrides_when_already_defined_on_binding() {
        let mut target_existing_mut: ItemFn = parse_quote!{
            fn input(mut input: usize) {}
        };

        let param: ParamAssignment = ParamAssignment(parse_quote!(mut), parse_quote!(123));
        assert_eq_mutate!(param, &mut target_existing_mut, Ok(()));

        assert_eq_tokens!(
            target_existing_mut.block.stmts[0],
            quote!(let mut input: usize = 123;)
        );
    }

    #[test]
    fn mutate_returns_error_when_no_fn_inputs() {
        let mut target: ItemFn = parse_quote!{
            fn input() {}
        };

        assert_eq_mutate!(
            ParamAssignment(None, parse_quote!(0)), &mut target,
            Err(error_spanned!("no corresponding input"))
        );
    }
}
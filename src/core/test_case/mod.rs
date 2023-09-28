use quote::ToTokens;

use proc_macro2::{
    TokenStream, TokenTree
};
use syn::{
    Attribute, AttrStyle,
    Ident, ItemFn, Token, Result,
    parse::{
        Parse, ParseStream
    },
    spanned::Spanned
};
use crate::{
    common::{
        attribute_name_to_string,
        parse_next_tt,
        macros::{
            unwrap_or_err,
            error_spanned
        }
    },
    core::{
        Mutate, Mutators,
        InsertUnique,
        macros::*
    },
    params::{
        macros::impl_param,
        parse_param_args,
        name::*, with::*
    }
};

#[repr(u8)]
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
enum TestMutator {
    // Mutators should be defined in the order they must apply
    ParamName(ParamName),
    ParamWith(ParamWith)
}

impl Mutate for TestMutator {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match self {
            TestMutator::ParamWith(param) => param.mutate(target),
            TestMutator::ParamName(param) => param.mutate(&mut target.sig)
        }
    }
}

impl ToTokens for TestMutator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            TestMutator::ParamWith(param) => param.to_tokens(tokens),
            TestMutator::ParamName(param) => param.to_tokens(tokens)
        };
    }
}

impl Parse for TestMutator {
    fn parse(input: ParseStream) -> Result<Self> {
        let Ok(TokenTree::Ident(name)) = parse_next_tt(input) else {
            return Err(error_spanned!("expected one of: `name`, `arg(...)`", &input.span()));
        };

        match name.to_string().as_bytes() {
            b"with" => {
                Ok(TestMutator::ParamWith(parse_param_args(input)?))
            },
            _ => {
                // Assume the ident is the test name
                Ok(TestMutator::ParamName(ParamName(name)))
            }
        }
    }
}

#[derive(Clone)]
pub struct TestCase(Mutators<TestMutator>);

impl Mutate for TestCase {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        for mutator in &self.0 {
            mutator.mutate(target)?;
        }

        Ok(())
    }
}

impl Parse for TestCase {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut mutators: Mutators<TestMutator> = Mutators::new();
        
        while !input.is_empty() {
            mutators.insert_unique(input.parse::<TestMutator>()?)?;

            // If more args to be parsed
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self(mutators))
    }
}

impl TestCase {
    pub const SITH_TEST_IDENT: &'static str = "test_case";
    pub const RUSTC_TEST_IDENT: &'static str = "test";
    pub const WASM_TEST_IDENT: &'static str = "wasm_bindgen_test";
}

impl_param!(TestCase, iterable(0));

pub fn render_test_case(test_case_: TestCase, mut target: ItemFn) -> TokenStream {
    let mut out: TokenStream = TokenStream::new();
    let mut test_cases: Vec<TestCase> = vec![test_case_];

    // Search for other test case attributes, plucking such from the fn def if present
    let mut removed_elements: usize = 0;
    for i in 0..target.attrs.len() {
        if attribute_name_to_string(&target.attrs[i - removed_elements]).as_str() != TestCase::SITH_TEST_IDENT {
            continue;
        }

        let attr = target.attrs.remove(i - removed_elements);
        let parsed_test_case = unwrap_or_err!(attr.parse_args_with(TestCase::parse));

        test_cases.push(parsed_test_case);

        // Upon removal, the vec shifts one to
        // the left (and thus - so does the length)
        // So we must adjust index `i` accordingly
        removed_elements += 1;
    }

    // For each test case matched, evaluate each against a fresh instance of the function
    for test_case in test_cases {
        let mut target_fn: ItemFn = target.clone();
        target_fn.attrs.push(rustc_test_attribute!(target.span()));

        match test_case.mutate(&mut target_fn) {
            Ok(()) => target_fn.to_tokens(&mut out),
            Err(e) => e.to_compile_error().to_tokens(&mut out)
        };
    }

    out
}
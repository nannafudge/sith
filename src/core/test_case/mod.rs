use super::{
    InsertUnique,
    Mutate, Mutators,
    macros::*
};
use crate::common::{
    parse_next_tt,
    parse_group_with_delim, 
    attribute_name_to_bytes,
    macros::{
        error_spanned,
        unwrap_or_err
    }
};
use proc_macro2::{
    TokenStream,
    Delimiter
};
use syn::{
    Attribute, AttrStyle,
    Ident, ItemFn, Token, Result,
    parse::{
        Parse, ParseStream
    },
    spanned::Spanned
};

use quote::ToTokens;

mod args;
use args::*;

#[repr(u8)]
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
enum TestMutator {
    // Mutators should be defined in the order they must apply
    ArgName(ArgName),
    ArgWith(ArgWith)
}

impl Mutate for TestMutator {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match self {
            TestMutator::ArgWith(arg) => arg.mutate(target),
            TestMutator::ArgName(arg) => arg.mutate(&mut target.sig)
        }
    }
}

impl ToTokens for TestMutator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            TestMutator::ArgWith(arg) => arg.to_tokens(tokens),
            TestMutator::ArgName(arg) => arg.to_tokens(tokens)
        };
    }
}

impl Parse for TestMutator {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse::<Ident>().map_err(|_| {
            match parse_next_tt(input) {
                Ok(token) => input.error(format!("{}\n ^ unexpected arg", token)),
                Err(e) => e
            }
        })?;

        match name.to_string().as_bytes() {
            b"with" => {
                Ok(TestMutator::ArgWith(parse_arg_parameterized(input)?))
            },
            _ => {
                // Assume the ident is the test name
                Ok(TestMutator::ArgName(ArgName(name)))
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

impl_to_tokens_arg!(TestCase, iterable(0));

fn parse_arg_parameterized<T: Parse>(input: ParseStream) -> Result<T> {
    let arg_inner: TokenStream = parse_group_with_delim(Delimiter::Parenthesis, input)?;
    syn::parse2::<T>(arg_inner)
}

pub fn render_test_case(test_case_: TestCase, mut target: ItemFn) -> TokenStream {
    let mut out: TokenStream = TokenStream::new();
    let mut test_cases: Vec<TestCase> = vec![test_case_];

    // Search for other test case attributes, plucking such from the fn def if present
    let mut removed_elements: usize = 0;
    for i in 0..target.attrs.len() {
        if attribute_name_to_bytes(&target.attrs[i - removed_elements]) != Some(b"test_case") {
            continue;
        }

        let attr = target.attrs.remove(i - removed_elements);
        let parsed_test_case = unwrap_or_err!(
            attr.parse_args_with(TestCase::parse),
            error_spanned!("{}", &attr)
        );

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
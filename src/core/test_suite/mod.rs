use crate::common::attribute_name_to_string;
use super::{
    InsertUnique,
    Mutate, Mutators,
    TestCase, macros::*
};
use syn::{
    Attribute,
    Result, Ident,
    ItemMod, Item, ItemFn,
    parse::{
        Parse, ParseStream
    }, 
    token::{
        Mod, Brace
    }
};
use quote::{
    ToTokens, TokenStreamExt
};
use proc_macro2::TokenStream;

use core::mem::take;

mod args;
use args::*;

#[repr(u8)]
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
enum SuiteMutator {
    // Mutators should be defined in the order they must apply
    Setup(ArgSetup),
    Teardown(ArgTeardown)
}

impl Mutate for SuiteMutator {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match self {
            SuiteMutator::Setup(arg) => arg.mutate(&mut target.block),
            SuiteMutator::Teardown(arg) => arg.mutate(&mut target.block)
        }
    }
}

impl ToTokens for SuiteMutator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            SuiteMutator::Setup(arg) => arg.to_tokens(tokens),
            SuiteMutator::Teardown(arg) => arg.to_tokens(tokens)
        };
    }
}

impl SuiteMutator {
    fn new_from(function: &mut ItemFn) -> Option<SuiteMutator> {
        for attribute in &function.attrs {
            match attribute_name_to_string(attribute).as_str() {
                TestSuite::SETUP_IDENT => {
                    return Some(
                        SuiteMutator::Setup(ArgSetup(take(&mut function.block.stmts)))
                    );
                },
                TestSuite::TEARDOWN_IDENT => {
                    return Some(
                        SuiteMutator::Teardown(ArgTeardown(take(&mut function.block.stmts)))
                    );
                },
                _ => {}
            }
        }

        None
    }
}

#[derive(Clone)]
pub struct TestSuite {
    name: Ident,
    mutators: Option<Mutators<SuiteMutator>>,
    contents: Option<Vec<Item>>
}

impl Mutate for TestSuite {
    type Item = Item;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        let Option::Some(mutators) = &self.mutators else {
            return Ok(());
        };

        let Item::Fn(function) = target else {
            return Ok(());
        };

        if is_test_attribute(&function.attrs) {
            for mutator in mutators {
                mutator.mutate(function)?;
            }
        }

        Ok(())
    }
}

impl Parse for TestSuite {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut target: ItemMod = input.parse::<ItemMod>()?;
        let Some(mut contents) = take(&mut target.content) else {
            return Ok( Self { name: target.ident, mutators: None, contents: None } );
        };

        let mut mutators: Mutators<SuiteMutator> = Mutators::new();

        // TODO: Make suites composable using 'use', where setup/teardown
        // functions are combined into one as an inheritable strategy
        // TODO: Detect #[setup]/#[teardown] on invalid Items, reporting such correctly
        // TODO: Create 'safe remove' iterator type
        let mut removed_elements: usize = 0;
        for i in 0..contents.1.len() {
            let Item::Fn(item) = &mut contents.1[i - removed_elements] else {
                continue;
            };

            if let Some(mutator) = SuiteMutator::new_from(item) {
                mutators.insert_unique(mutator)?;
                contents.1.remove(i - removed_elements);
                removed_elements += 1;
            }
        }

        Ok(Self {
            name: target.ident,
            mutators: Some(mutators),
            contents: Some(contents.1)
        })
    }
}

impl ToTokens for TestSuite {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        render_mod_name(self, tokens);

        let braced: Brace = Brace::default();
        braced.surround(tokens, | suite_inner |{
            if let Some(contents) = &self.contents {
                contents.iter().for_each(| item | item.to_tokens(suite_inner));
            }
        });
    }
}

impl TestSuite {
    pub const SETUP_IDENT: &'static str = "setup";
    pub const TEARDOWN_IDENT: &'static str = "teardown";
}

pub fn render_test_suite(mut test_suite: TestSuite) -> TokenStream {
    let Option::Some(mut contents) = take(&mut test_suite.contents) else {
        return test_suite.to_token_stream();
    };

    // DRY - however... ToTokens doesn't pass-in self as an owned or mutable reference,
    // so mutation must occur outside. It's more optimized to mutate and apply items
    // within the same loop
    let mut suite_out: TokenStream = TokenStream::new();
    render_mod_name(&test_suite, &mut suite_out);
    
    let braced: Brace = Brace::default();
    braced.surround(&mut suite_out, | suite_inner |{
        for item in &mut contents {
            if let Err(e) = test_suite.mutate(item) {
                suite_inner.append_all(e.to_compile_error());
            }

            item.to_tokens(suite_inner);
        }
    });

    suite_out
}

fn render_mod_name(test_suite: &TestSuite, tokens: &mut TokenStream) {
    Mod::default().to_tokens(tokens);
    test_suite.name.to_tokens(tokens);
}

fn is_test_attribute(attributes: &[Attribute]) -> bool {
    attributes.iter()
        .map(attribute_name_to_string)
        .any(| name | {
            name.as_str() == TestCase::SITH_TEST_IDENT ||
            name.as_str() == TestCase::RUSTC_TEST_IDENT ||
            name.as_str() == TestCase::WASM_TEST_IDENT
        })
}

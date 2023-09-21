use crate::common::attribute_name_to_bytes;
use super::{
    InsertUnique,
    Mutate, Mutators,
    macros::*
};
use syn::{
    Result, Ident,
    ItemMod, Item, ItemFn,
    parse::{
        Parse, ParseStream
    }, 
    token::{
        Mod, Brace
    }
};
use quote::{ToTokens, TokenStreamExt};
use proc_macro2::TokenStream;

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

#[derive(Clone)]
pub struct TestSuite {
    name: Ident,
    mutators: Mutators<SuiteMutator>,
    contents: Vec<Item>
}

impl Mutate for TestSuite {
    type Item = Item;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        if let Item::Fn(function) = target {
            let is_test = function.attrs.iter()
                .filter_map(attribute_name_to_bytes)
                .any(| attr | {
                    attr == b"test" || attr == b"test_case"
                });

            if is_test {
                for mutator in &self.mutators {
                    mutator.mutate(function)?;
                }
            }
        }

        Ok(())
    }
}

impl Parse for TestSuite {
    fn parse(input: ParseStream) -> Result<Self> {
        let target: ItemMod = input.parse::<ItemMod>()?;
        if target.content.is_none() {
            return Ok(Self{
                name: target.ident,
                mutators: Mutators::new(),
                contents: Vec::new()
            });
        }

        let mut mutators: Mutators<SuiteMutator> = Mutators::new();
        let mut contents: Vec<Item> = Vec::with_capacity(1);

        // TODO: Make suites composable using 'use', where setup/teardown
        // functions are combined into one as an inheritable strategy
        // TODO: Detect #[setup]/#[teardown] on invalid Items, reporting such correctly
        for item in target.content.expect("Invariant: Empty suite").1 {
            // We need to clone the stmts regardless, to support
            // functions that are tagged as both setup and teardown
            let mut is_suite_arg: bool = false;
            if let Item::Fn(item) = &item {
                let mut attributes = item.attrs.iter().filter_map(attribute_name_to_bytes);
                while let Some(name) = attributes.next() {
                    match name {
                        b"setup" => {
                            mutators.insert_unique(
                                SuiteMutator::Setup(
                                    ArgSetup(item.block.stmts.to_owned())
                                )
                            )?;

                            is_suite_arg = true;
                        },
                        b"teardown" => {
                            mutators.insert_unique(
                                SuiteMutator::Teardown(
                                    ArgTeardown(item.block.stmts.to_owned())
                                )
                            )?;

                            is_suite_arg = true;
                        },
                        _ => {}
                    }
                }
            }

            if !is_suite_arg {
                contents.push(item);
            }
        }

        Ok(Self {
            name: target.ident,
            mutators,
            contents
        })
    }
}

pub fn render_test_suite(mut test_suite: TestSuite) -> TokenStream {
    let mut suite_out: TokenStream = TokenStream::new();
    let mut contents = core::mem::take(&mut test_suite.contents);

    Mod::default().to_tokens(&mut suite_out);
    test_suite.name.to_tokens(&mut suite_out);
    
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
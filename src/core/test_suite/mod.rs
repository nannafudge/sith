use proc_macro2::TokenStream;

use quote::{
    ToTokens, TokenStreamExt
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
use core::{
    fmt::{
        Debug, Formatter
    },
    mem::take
};
use crate::{
    core::{
        Mutate, Mutators,
        InsertUnique, TestCase
    },
    params::{
        setup::*, teardown::*
    },
    common::{
        attribute_name_to_string,
        macros::error_spanned
    }
};

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum SuiteMutator {
    // Mutators should be defined in the order they must apply
    Setup(ParamSetup),
    Teardown(ParamTeardown)
}

impl Mutate for SuiteMutator {
    type Item = ItemFn;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        match self {
            SuiteMutator::Setup(param) => param.mutate(&mut target.block),
            SuiteMutator::Teardown(param) => param.mutate(&mut target.block)
        }
    }
}

impl ToTokens for SuiteMutator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            SuiteMutator::Setup(param) => param.to_tokens(tokens),
            SuiteMutator::Teardown(param) => param.to_tokens(tokens)
        };
    }
}

impl SuiteMutator {
    fn new_from(function: &mut ItemFn) -> Option<SuiteMutator> {
        for attribute in &function.attrs {
            match attribute_name_to_string(attribute).as_str() {
                TestSuite::SETUP_IDENT => {
                    return Some(
                        SuiteMutator::Setup(ParamSetup(take(&mut function.block.stmts)))
                    );
                },
                TestSuite::TEARDOWN_IDENT => {
                    return Some(
                        SuiteMutator::Teardown(ParamTeardown(take(&mut function.block.stmts)))
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
        let Result::Ok(mut target) = input.parse::<ItemMod>() else {
            return Err(error_spanned!("#[test_suite] can only be applied to modules", &input.span()));
        };

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

impl Debug for TestSuite {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestSuite")
            .field("name", &self.name)
            .field("mutators", &self.mutators)
            .field("contents", &self.contents.as_ref().map(| items | {
                items.iter().fold(TokenStream::new(), | mut acc, item | {
                    item.to_tokens(&mut acc);
                    acc
                })
            }))
            .finish()
    }
}

impl TestSuite {
    pub const SETUP_IDENT: &'static str = "setup";
    pub const TEARDOWN_IDENT: &'static str = "teardown";
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{
            macros::error_spanned,
            tests::macros::*
        },
        core::tests::macros::*
    };

    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn parse_consumes_setup_function() {
        assert_eq_parsed!(
            syn::parse2::<TestSuite>(quote!{
                mod my_suite {
                    #[setup]
                    fn setup() {}

                    #[test_case]
                    fn foo() {}
                }
            }),
            Ok(TestSuite {
                name: parse_quote!(my_suite),
                mutators: Some(
                    Mutators::from(
                        [SuiteMutator::Setup(ParamSetup(vec![]))]
                    )
                ),
                contents: Some(vec![
                    parse_quote!{
                        #[test_case]
                        fn foo() {}
                    }
                ])
            })
        );
    }

    #[test]
    fn parse_consumes_teardown_function() {
        assert_eq_parsed!(
            syn::parse2::<TestSuite>(quote!{
                mod my_suite {
                    #[teardown]
                    fn teardown() {}

                    #[test_case]
                    fn foo() {}
                }
            }),
            Ok(TestSuite {
                name: parse_quote!(my_suite),
                mutators: Some(
                    Mutators::from(
                        [SuiteMutator::Teardown(ParamTeardown(vec![]))]
                    )
                ),
                contents: Some(vec![
                    parse_quote!{
                        #[test_case]
                        fn foo() {}
                    }
                ])
            })
        );
    }

    #[test]
    fn parse_works_with_no_setup_and_teardown() {
        assert_eq_parsed!(
            syn::parse2::<TestSuite>(quote!{
                mod my_suite {
                    #[test_case]
                    fn foo() {}
                }
            }),
            Ok(TestSuite {
                name: parse_quote!(my_suite),
                mutators: None,
                contents: Some(vec![
                    parse_quote!{
                        #[test_case]
                        fn foo() {}
                    }
                ])
            })
        );
    }

    #[test]
    fn parse_works_with_empty_modules() {
        assert_eq_parsed!(
            syn::parse2::<TestSuite>(quote!{
                mod my_suite {

                }
            }),
            Ok(TestSuite {
                name: parse_quote!(my_suite),
                mutators: None,
                contents: None
            })
        );
    }

    #[test]
    fn mutate_is_ok_with_no_mutators() {
        let suite = TestSuite {
            name: parse_quote!(my_suite),
            mutators: None,
            contents: None
        };

        let mut items: [Item; 3] = [
            parse_quote!(fn foo() {}),
            parse_quote!(fn bar() {}),
            parse_quote!(fn baz() {})
        ];

        items.iter_mut().for_each(| item |
            assert_eq_mutate!(suite, item, Ok(()))
        );
    }

    #[test]
    fn mutate_applies_setup_and_teardown_in_order() {
        let suite = TestSuite {
            name: parse_quote!(my_suite),
            mutators: Some(
                // Defined the other way around
                // purpose to test the Ord implementation
                Mutators::from(
                    [
                        SuiteMutator::Teardown(ParamTeardown(vec![
                            parse_quote!(let b = 456;)
                        ])),
                        SuiteMutator::Setup(ParamSetup(vec![
                            parse_quote!(let a = 123;)
                        ]))
                    ]
                )
            ),
            contents: None
        };

        let mut test = Item::Fn(parse_quote!(#[test] fn foo() {bar();}));
        assert_eq_mutate!(suite, &mut test, Ok(()));
        assert_eq_tokens!(
            test, quote!{
                #[test]
                fn foo() {
                    let a = 123;
                    bar();
                    let b = 456;
                }
            }
        )
    }

    #[test]
    fn mutate_only_affects_tests() {
        let suite = TestSuite {
            name: parse_quote!(my_suite),
            mutators: Some(
                Mutators::from(
                    [
                        SuiteMutator::Setup(ParamSetup(vec![
                            parse_quote!(let a = 123;)
                        ])),
                        SuiteMutator::Teardown(ParamTeardown(vec![
                            parse_quote!(let b = 456;)
                        ]))
                    ]
                )
            ),
            contents: None
        };

        let mut items: [Item; 9] = [
            parse_quote!(#[wasm_bindgen_test] fn one() {}),
            parse_quote!(const SEED: usize = 123;),
            parse_quote!(use crate::foo::*;),
            parse_quote!(struct Foo;),
            parse_quote!(#[test_case] fn two() {}),
            parse_quote!(enum Bar{ZERO}),
            parse_quote!(trait Baz{}),
            parse_quote!(type Whiskey = Delta;),
            parse_quote!(#[test] fn three() {}),
        ];

        items.iter_mut().for_each(| item |
            assert_eq_mutate!(suite, item, Ok(()))
        );

        assert_eq_tokens!(
            items[0], quote!{
                #[wasm_bindgen_test]
                fn one() {
                    let a = 123;
                    let b = 456;
                }
            }
        );

        assert_eq_tokens!(
            items[4], quote!{
                #[test_case]
                fn two() {
                    let a = 123;
                    let b = 456;
                }
            }
        );

        assert_eq_tokens!(
            items[8], quote!{
                #[test]
                fn three() {
                    let a = 123;
                    let b = 456;
                }
            }
        );
    }

    #[test]
    fn attribute_only_applies_to_modules() {
        assert_eq_parsed!(
            syn::parse2::<TestSuite>(quote!(fn invalid() {})),
            Err(error_spanned!("#[test_suite] can only be applied to modules"))
        );
    }

    #[test]
    fn to_tokens_outputs_parsed_module_as_is() {
        let suite: TestSuite = parse_quote!{
            mod my_suite {
                #[test_case]
                fn my_test() {}
            }
        };

        assert_eq_tokens!(
            suite.to_token_stream(),
            quote!{
                mod my_suite {
                    #[test_case]
                    fn my_test() {}
                }
            }
        )
    }

    #[test]
    fn to_tokens_outputs_parsed_module_even_if_empty() {
        let suite: TestSuite = parse_quote!(mod my_suite {});

        assert_eq_tokens!(
            suite.to_token_stream(),
            quote!(mod my_suite {})
        )
    }

    mod render_mod_name {
        use super::*;
        
        use syn::parse_quote;

        #[test]
        fn outputs_mod_with_test_suite_name() {
            let mut tokens = TokenStream::new();

            render_mod_name(&TestSuite {
                name: parse_quote!(my_suite),
                mutators: None,
                contents: None
            }, &mut tokens);
    
            assert_eq_tokens!(tokens, quote!(mod my_suite));
        }
    }

    mod is_test_attribute {
        use super::*;
        
        use syn::AttrStyle;

        #[test]
        fn recognizes_test_case() {
            assert!(is_test_attribute(
                &[construct_attribute!(AttrStyle::Outer, test_case)]
            ));
        }

        #[test]
        fn recognizes_test() {
            assert!(is_test_attribute(
                &[construct_attribute!(AttrStyle::Outer, test)]
            ));
        }

        #[test]
        fn recognizes_wasm_bindgen_test() {
            assert!(is_test_attribute(
                &[construct_attribute!(AttrStyle::Outer, wasm_bindgen_test)]
            ));
        }

        #[test]
        fn does_not_recognize_other_attributes_named_test() {
            assert!(!
                is_test_attribute(
                    &[
                        construct_attribute!(AttrStyle::Outer, foo_test),
                        construct_attribute!(AttrStyle::Outer, test_bar),
                        construct_attribute!(AttrStyle::Outer, test_),
                        construct_attribute!(AttrStyle::Outer, _test),
                    ]
                )
            );
        }
    }
}
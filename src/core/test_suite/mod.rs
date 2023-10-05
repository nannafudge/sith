use proc_macro2::TokenStream;

use quote::{
    ToTokens, TokenStreamExt,
    quote
};
use syn::{
    Result, Ident,
    ItemMod, Item, ItemFn,
    parse::{
        Parse, ParseStream
    }, 
    token::{
        Mut, Mod, Brace
    },
    Expr, StaticMutability
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
        setup::*, teardown::*, init::*
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
    Teardown(ParamTeardown),
    Init(ParamInit)
}

impl Mutate for SuiteMutator {
    type Item = Item;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        // Done here as match takes ownership, even if
        // matched patterns are empty, i.e.: Pattern(_)
        if let Self::Init(param) = self {
            return param.mutate(target);
        }

        match (self, target) {
            (Self::Setup(param), Item::Fn(func)) => param.mutate(&mut func.block),
            (Self::Teardown(param), Item::Fn(func)) => param.mutate(&mut func.block),
            _ => Ok(())
        }
    }
}

impl ToTokens for SuiteMutator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let SuiteMutator::Init(param) = self {
            param.to_tokens(tokens);
        }
    }
}

impl TryFrom<&mut ItemFn> for SuiteMutator {
    type Error = ();

    fn try_from(function: &mut ItemFn) -> std::result::Result<Self, Self::Error> {
        for attribute in &function.attrs {
            match attribute_name_to_string(attribute).as_str() {
                TestSuite::INIT_IDENT => {
                    return Ok(SuiteMutator::Init(
                        ParamInit(take(&mut function.block.stmts))
                    ));
                },
                TestSuite::SETUP_IDENT => {
                    return Ok(SuiteMutator::Setup(
                        ParamSetup(take(&mut function.block.stmts))
                    ));
                },
                TestSuite::TEARDOWN_IDENT => {
                    return Ok(SuiteMutator::Teardown(
                        ParamTeardown(take(&mut function.block.stmts))
                    ));
                },
                _ => {}
            }
        }

        Err(())
    }
}

#[derive(Clone)]
pub struct TestSuite {
    name: Ident,
    contents: Option<Vec<Item>>,
    mutators: Option<Mutators<SuiteMutator>>
}

impl Mutate for TestSuite {
    type Item = Item;

    fn mutate(&self, target: &mut Self::Item) -> Result<()> {
        if let Option::Some(mutators) = &self.mutators {
            for mutator in mutators {
                mutator.mutate(target)?;
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

        let mut out = TestSuite {
            name: target.ident,
            mutators: None,
            contents: take(&mut target.content).map(|c| c.1)
        };

        let Some(contents) = &mut out.contents else {
            return Ok(out);
        };

        //let mut mutators: Mutators<SuiteMutator> = Mutators::new();
        // TODO: Create 'safe remove' iterator type
        let mut removed_elements: usize = 0;
        // TODO: Detect #[setup]/#[teardown] on invalid Items, reporting such correctly
        for i in 0..contents.len() {
            let real_index = i - removed_elements;
            
            let Item::Fn(func) = &mut contents[real_index] else {
                continue;
            };

            if let Ok(mutator) = SuiteMutator::try_from(func) {
                let mutators = out.mutators.get_or_insert(Mutators::new());
                mutators.insert_unique(mutator)?;
                remove_contents(contents, &mut removed_elements, real_index);
            }
        }

        Ok(out)
    }
}

impl ToTokens for TestSuite {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.render_mod_name(tokens);

        let braced: Brace = Brace::default();
        braced.surround(tokens, | suite_inner |{
            if let Some(mutators) = &self.mutators {
                mutators.iter().for_each(| mutator | mutator.to_tokens(suite_inner));
            }
            if let Some(contents) = &self.contents {
                contents.iter().for_each(| item | item.to_tokens(suite_inner));
            }
        });
    }

    fn into_token_stream(mut self) -> TokenStream {
        let mut suite_out: TokenStream = TokenStream::new();
        self.render_mod_name(&mut suite_out);

        let braced: Brace = Brace::default();
        braced.surround(&mut suite_out, | suite_inner |{
            if let Some(mutators) = &self.mutators {
                mutators.iter().for_each(| mutator | mutator.to_tokens(suite_inner));
            }
            if let Some(contents) = &mut take(&mut self.contents) {
                contents.iter_mut().for_each(| item | {
                    match &self.mutate(item) {
                        Ok(_) => item.to_tokens(suite_inner),
                        Err(e) => suite_inner.append_all(e.to_compile_error())
                    };
                });
            }
        });

        suite_out
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
    pub const INIT_IDENT: &'static str = "init";
    pub const SETUP_IDENT: &'static str = "setup";
    pub const TEARDOWN_IDENT: &'static str = "teardown";

    fn render_mod_name(&self, tokens: &mut TokenStream) {
        Mod::default().to_tokens(tokens);
        self.name.to_tokens(tokens);
    }
}

fn remove_contents(contents: &mut Vec<syn::Item>, removed_elements: &mut usize, index: usize) {
    contents.remove(index);
    *removed_elements += 1;
}

/*pub fn render_test_suite(mut test_suite: TestSuite) -> TokenStream {
    let Option::Some(mut contents) = take(&mut test_suite.contents) else {
        return test_suite.into_token_stream();
    };

    // DRY - however... ToTokens doesn't pass-in self as an owned or mutable reference,
    // so mutation must occur outside. It's more optimized to mutate and apply items
    // within the same loop
    let mut suite_out: TokenStream = TokenStream::new();
    render_mod_name(&test_suite, &mut suite_out);
    
    let braced: Brace = Brace::default();
    braced.surround(&mut suite_out, | suite_inner |{
        if let Some(mutators) = &test_suite.mutators {
            mutators.iter().for_each(| mutator | mutator.to_tokens(suite_inner));
        }
        for item in &mut contents {
            if let Err(e) = test_suite.mutate(item) {
                suite_inner.append_all(e.to_compile_error());
            }

            item.to_tokens(suite_inner);
        }
    });

    suite_out
}*/

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

    #[test]
    fn outputs_mod_with_test_suite_name() {
        let mut tokens = TokenStream::new();
        let suite = TestSuite {
            name: parse_quote!(my_suite),
            mutators: None,
            contents: None
        };

        suite.render_mod_name(&mut tokens);
        assert_eq_tokens!(tokens, quote!(mod my_suite));
    }
}
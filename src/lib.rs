use proc_macro::TokenStream;
use syn::parse_macro_input;

mod common;
mod core;

use crate::core::*;

#[proc_macro_attribute]
pub fn test_suite(_: TokenStream, target: TokenStream) -> TokenStream {
    let test_suite: TestSuite = parse_macro_input!(target as TestSuite);
    render_test_suite(test_suite).into()
}

#[proc_macro_attribute]
pub fn test_case(attr_args: TokenStream, target: TokenStream) -> TokenStream {
    use syn::ItemFn;

    let test_case: TestCase = parse_macro_input!(attr_args as TestCase);
    let test_fn: ItemFn = parse_macro_input!(target as ItemFn);
    render_test_case(test_case, test_fn).into()
}
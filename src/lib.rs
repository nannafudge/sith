use proc_macro::TokenStream;

mod common;
mod core;

use crate::core::*;

macro_rules! parse_token_stream {
    ($target:expr => $type_:ty) => {
        match syn::parse2::<$type_>($target.into()) {
            Ok(t) => t,
            Err(e) => {
                return e.to_compile_error().into();
            }
        }
    };
    ($target:expr => $type_:ty, $message:literal) => {
        match syn::parse2::<$type_>($target.into()) {
            Ok(t) => t,
            Err(e) => {
                let mut out = syn::Error::new(proc_macro2::Span::call_site(), $message);
                out.combine(e);

                return out.to_compile_error().into();
            }
        }
    };
}

#[proc_macro_attribute]
pub fn test_suite(_: TokenStream, target: TokenStream) -> TokenStream {
    let test_suite: TestSuite = parse_token_stream!(target => TestSuite);
    render_test_suite(test_suite).into()
}

#[proc_macro_attribute]
pub fn test_case(attr_args: TokenStream, target: TokenStream) -> TokenStream {
    use syn::ItemFn;

    let test_case: TestCase = parse_token_stream!(attr_args => TestCase);
    let test_fn: ItemFn = parse_token_stream!(target => ItemFn, "#[test_case] can only be applied to functions");
    render_test_case(test_case, test_fn).into()
}
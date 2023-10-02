//! # Sith
//! > **S**imple **I**njectible **T**est **H**arness
//! 
//! Current Features:
//! 
//! ## `#[test_case]`
//! 
//! Functions may be affixed with the `#[test_case]` outer attribute
//! to define a `sith` test. These tests may be parameterized: see 
//! [`test_case`](core::test_case#parameters) for an up-to-date list of 
//! available parameters and features.
//! 
//! ### Example
//! 
//! ```
//! use sith::test_case;
//! 
//! #[test_case]
//! fn unparameterized() {
//!     println!("Hello from a unparameterized test!");
//! }
//! 
//! #[test_case(with(123))]
//! fn parameterized(my_arg: usize) {
//!     println!("my_arg is: {}!", my_arg);
//! }
//! ```
//! 
//! ## `#[test_suite]`
//! 
//! Tests may be aggregated into modules or `suites`. Currently, modules
//! affixed with the `#[test_suite]` attribute may define a common set of 
//! `setup`/`teardown` routines: I.e, pre-test configuration and post-test cleanup.
//! 
//! ### Example
//! 
//! ```
//! #[test_suite]
//! mod suite {
//!     use sith::test_case;
//!     use std::io::Write;
//!
//!     #[setup]
//!     fn setup() {
//!         let handler = std::panic::take_hook();
//!         std::panic::set_hook(Box::new(move | info | {
//!            let _ = std::io::stderr().write_fmt(format_args!("failed with seed: {}\n", SEED));
//!             handler(info);
//!         })); 
//!     }
//! 
//!     #[teardown]
//!     fn teardown() {
//!         // Re-register the default panic hook
//!         let _ = std::panic::take_hook();
//!     }
//! 
//!     #[test_case]
//!     fn unparameterized() {
//!         println!("Hello from a unparameterized test!");
//!     }
//! }
//! 
//! ```

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
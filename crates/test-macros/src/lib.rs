//! Various helper macros used throughout testing wasmtime

use proc_macro::TokenStream;

mod add_variants;
mod flags_test;
mod wasmtime_test;

#[proc_macro_attribute]
pub fn wasmtime_test(attrs: TokenStream, item: TokenStream) -> TokenStream {
    wasmtime_test::run(attrs, item)
}

#[proc_macro_attribute]
pub fn add_variants(attr: TokenStream, item: TokenStream) -> TokenStream {
    add_variants::run(attr, item)
}

#[proc_macro]
pub fn flags_test(input: TokenStream) -> TokenStream {
    flags_test::run(input)
}

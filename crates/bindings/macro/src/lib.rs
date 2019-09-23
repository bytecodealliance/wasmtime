extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate syn;
extern crate quote;

mod attr;
mod r#impl;
mod method;
mod signature;
mod r#trait;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn wasmtime_method(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as attr::TransformAttributes);
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    method::wrap_method(input, attr).into()
}

#[proc_macro_attribute]
pub fn wasmtime_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as attr::TransformAttributes);
    let input = syn::parse_macro_input!(item as syn::ItemTrait);
    r#trait::wrap_trait(input, attr).into()
}

#[proc_macro_attribute]
pub fn wasmtime_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as attr::TransformAttributes);
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    r#impl::wrap_impl(input, attr).into()
}

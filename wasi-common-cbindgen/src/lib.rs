extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ArgCaptured, Pat, PatIdent};
use std::collections::HashMap;

#[proc_macro_attribute]
pub fn wasi_common_cbindgen(attr: TokenStream, function: TokenStream) -> TokenStream {
    assert!(attr.is_empty());

    let function = syn::parse_macro_input!(function as syn::ItemFn);
    let result = quote! {
        #function
    };

    result.into()
}

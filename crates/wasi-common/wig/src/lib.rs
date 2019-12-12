extern crate proc_macro;

mod raw_types;
mod utils;
mod wasi;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro]
pub fn witx_host_types(args: TokenStream) -> TokenStream {
    TokenStream::from(raw_types::gen(
        TokenStream2::from(args),
        raw_types::Mode::Host,
    ))
}

#[proc_macro]
pub fn witx_wasi_types(args: TokenStream) -> TokenStream {
    TokenStream::from(raw_types::gen(
        TokenStream2::from(args),
        raw_types::Mode::Wasi,
    ))
}

#[proc_macro]
pub fn witx_wasi32_types(args: TokenStream) -> TokenStream {
    TokenStream::from(raw_types::gen(
        TokenStream2::from(args),
        raw_types::Mode::Wasi32,
    ))
}

/// A single-use macro in the `wasmtime-wasi` crate.
#[proc_macro]
pub fn define_add_wrappers_to_module(args: TokenStream) -> TokenStream {
    wasi::add_wrappers_to_module(args.into()).into()
}

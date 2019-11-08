extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate witx;

mod raw_types;
mod utils;

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

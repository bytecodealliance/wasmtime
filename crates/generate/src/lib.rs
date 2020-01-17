extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    TokenStream::new()
    // TokenStream::from(raw_types::gen(
    //     TokenStream2::from(args),
    //     raw_types::Mode::Host,
    // ))
}

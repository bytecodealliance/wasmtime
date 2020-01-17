extern crate proc_macro;

mod imp;

use proc_macro::TokenStream;

#[proc_macro]
pub fn from_witx(_args: TokenStream) -> TokenStream {
    TokenStream::from(imp::gen())
}

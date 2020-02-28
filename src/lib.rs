extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as wiggle_generate::Config);
    TokenStream::from(wiggle_generate::generate(config))
}

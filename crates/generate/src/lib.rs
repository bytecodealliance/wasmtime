extern crate proc_macro;

mod imp;
mod parse;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let witx_paths = parse::witx_paths(args).expect("parsing macro arguments");
    let doc = witx::load(&witx_paths).expect("loading witx");
    let out = imp::gen(doc);
    TokenStream::from(out)
}

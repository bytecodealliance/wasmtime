extern crate proc_macro;

mod parse;
mod types;

use heck::SnakeCase;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use types::define_datatype;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let witx_paths = parse::witx_paths(args).expect("parsing macro arguments");
    let doc = witx::load(&witx_paths).expect("loading witx");

    let mut types = TokenStream2::new();
    for namedtype in doc.typenames() {
        let def = define_datatype(&namedtype);
        types.extend(def);
    }

    let mut modules = TokenStream2::new();
    for module in doc.modules() {
        let modname = format_ident!("{}", module.name.as_str().to_snake_case());
        let mut fs = TokenStream2::new();
        for func in module.funcs() {
            let ident = format_ident!("{}", func.name.as_str().to_snake_case());
            fs.extend(quote!(pub fn #ident() { unimplemented!() }));
        }
        modules.extend(quote!(mod #modname { use super::types::*; #fs }));
    }

    TokenStream::from(quote!(mod types { #types } #modules))
}

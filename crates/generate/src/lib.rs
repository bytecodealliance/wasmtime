extern crate proc_macro;

mod funcs;
mod names;
mod parse;
mod types;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use funcs::define_func;
use names::Names;
use types::define_datatype;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let witx_paths = parse::witx_paths(args).expect("parsing macro arguments");

    let names = Names::new(); // TODO parse the names from the invocation of the macro, or from a file?

    let doc = witx::load(&witx_paths).expect("loading witx");

    let mut types = TokenStream2::new();
    for namedtype in doc.typenames() {
        let def = define_datatype(&names, &namedtype);
        types.extend(def);
    }

    let mut modules = TokenStream2::new();
    for module in doc.modules() {
        let modname = names.module(&module.name);

        let mut fs = TokenStream2::new();
        for func in module.funcs() {
            fs.extend(define_func(&names, &func));
        }
        modules.extend(quote!(mod #modname { use super::types::*; #fs }));
    }

    TokenStream::from(quote!(mod types { #types } #modules))
}

extern crate proc_macro;

mod config;
mod funcs;
mod lifetimes;
mod module_trait;
mod names;
mod types;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use config::Config;
use funcs::define_func;
use module_trait::define_module_trait;
use names::Names;
use types::define_datatype;

#[proc_macro]
pub fn from_witx(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as Config);

    let doc = witx::load(&config.witx.paths).expect("loading witx");

    let names = Names::new(config); // TODO parse the names from the invocation of the macro, or from a file?

    let types = doc.typenames().map(|t| define_datatype(&names, &t));

    let modules = doc.modules().map(|module| {
        let modname = names.module(&module.name);
        let fs = module.funcs().map(|f| define_func(&names, &f));
        let modtrait = define_module_trait(&names, &module);
        let ctx_type = names.ctx_type();
        quote!(
            mod #modname {
                use super::#ctx_type;
                use super::types::*;
                #(#fs)*

                #modtrait
            }
        )
    });

    TokenStream::from(quote!(
        mod types {
            #(#types)*
        }
        #(#modules)*
    ))
}

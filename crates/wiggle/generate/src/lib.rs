pub mod config;
mod funcs;
mod lifetimes;
mod module_trait;
mod names;
mod types;

use proc_macro2::TokenStream;
use quote::quote;

pub use config::Config;
pub use funcs::define_func;
pub use module_trait::define_module_trait;
pub use names::Names;
pub use types::define_datatype;

pub fn generate(doc: &witx::Document, config: &Config) -> TokenStream {
    // TODO at some point config should grow more ability to configure name
    // overrides.
    let names = Names::new(&config.ctx.name);

    let types = doc.typenames().map(|t| define_datatype(&names, &t));

    let modules = doc.modules().map(|module| {
        let modname = names.module(&module.name);
        let trait_name = names.trait_name(&module.name);
        let fs = module
            .funcs()
            .map(|f| define_func(&names, &f, quote!(#trait_name)));
        let modtrait = define_module_trait(&names, &module);
        let ctx_type = names.ctx_type();
        quote!(
            pub mod #modname {
                use super::#ctx_type;
                use super::types::*;
                #(#fs)*

                #modtrait
            }
        )
    });

    let metadata = if config.emit_metadata {
        let doc_text = &format!("{}", doc);
        quote! {
            pub mod metadata {
                pub const DOC_TEXT: &str = #doc_text;
                pub fn document() -> wiggle::witx::Document {
                    wiggle::witx::parse(DOC_TEXT).unwrap()
                }
            }
        }
    } else {
        quote!()
    };

    quote!(
        pub mod types {
            #(#types)*
        }
        #(#modules)*
        #metadata
    )
}

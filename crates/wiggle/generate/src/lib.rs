pub mod config;
mod error_transform;
mod funcs;
mod lifetimes;
mod module_trait;
mod names;
mod types;

use lifetimes::anon_lifetime;
use proc_macro2::TokenStream;
use quote::quote;

pub use config::Config;
pub use error_transform::{ErrorTransform, UserErrorType};
pub use funcs::define_func;
pub use module_trait::define_module_trait;
pub use names::Names;
pub use types::define_datatype;

pub fn generate(doc: &witx::Document, names: &Names, errs: &ErrorTransform) -> TokenStream {
    // TODO at some point config should grow more ability to configure name
    // overrides.
    let rt = names.runtime_mod();

    let types = doc.typenames().map(|t| define_datatype(&names, &t));

    let guest_error_methods = doc.error_types().map(|t| {
        let typename = names.type_ref(&t, anon_lifetime());
        let err_method = names.guest_error_conversion_method(&t);
        quote!(fn #err_method(&self, e: #rt::GuestError) -> #typename;)
    });
    let guest_error_conversion = quote! {
        pub trait GuestErrorConversion {
            #(#guest_error_methods)*
        }
    };

    let user_error_methods = errs.iter().map(|errtype| {
        let abi_typename = names.type_ref(&errtype.abi_type(), anon_lifetime());
        let user_typename = errtype.typename();
        let methodname = names.user_error_conversion_method(&errtype);
        quote!(fn #methodname(&self, e: super::#user_typename) -> Result<#abi_typename, #rt::Trap>;)
    });
    let user_error_conversion = quote! {
        pub trait UserErrorConversion {
            #(#user_error_methods)*
        }
    };
    let modules = doc.modules().map(|module| {
        let modname = names.module(&module.name);
        let fs = module
            .funcs()
            .map(|f| define_func(&names, &module, &f, &errs));
        let modtrait = define_module_trait(&names, &module, &errs);
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

    quote!(
        pub mod types {
            use std::convert::TryFrom;

            #(#types)*
            #guest_error_conversion
            #user_error_conversion
        }
        #(#modules)*
    )
}

pub fn generate_metadata(doc: &witx::Document, names: &Names) -> TokenStream {
    let rt = names.runtime_mod();
    let doc_text = &format!("{}", doc);
    quote! {
        pub mod metadata {
            pub const DOC_TEXT: &str = #doc_text;
            pub fn document() -> #rt::witx::Document {
                #rt::witx::parse(DOC_TEXT).unwrap()
            }
        }
    }
}

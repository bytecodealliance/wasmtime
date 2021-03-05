mod codegen_settings;
pub mod config;
mod funcs;
mod lifetimes;
mod module_trait;
mod names;
mod types;

use heck::ShoutySnakeCase;
use lifetimes::anon_lifetime;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

pub use codegen_settings::{CodegenSettings, UserErrorType};
pub use config::Config;
pub use funcs::define_func;
pub use module_trait::define_module_trait;
pub use names::Names;
pub use types::define_datatype;

pub fn generate(doc: &witx::Document, names: &Names, settings: &CodegenSettings) -> TokenStream {
    // TODO at some point config should grow more ability to configure name
    // overrides.
    let rt = names.runtime_mod();

    let types = doc.typenames().map(|t| define_datatype(&names, &t));

    let constants = doc.constants().map(|c| {
        let name = quote::format_ident!(
            "{}_{}",
            c.ty.as_str().to_shouty_snake_case(),
            c.name.as_str().to_shouty_snake_case()
        );
        let ty = names.type_(&c.ty);
        let value = Literal::u64_unsuffixed(c.value);
        quote! {
            pub const #name: #ty = #value;
        }
    });

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

    let user_error_methods = settings.errors.iter().map(|errtype| {
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
            .map(|f| define_func(&names, &module, &f, &settings));
        let modtrait = define_module_trait(&names, &module, &settings);
        quote!(
            pub mod #modname {
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
            #(#constants)*
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

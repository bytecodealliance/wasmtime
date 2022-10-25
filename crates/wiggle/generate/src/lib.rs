pub mod config;
mod funcs;
mod lifetimes;
mod module_trait;
mod names;
mod types;
pub mod wasmtime;

use heck::ToShoutySnakeCase;
use lifetimes::anon_lifetime;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

pub use config::{AsyncConf, CodegenConf, Config, UserErrorType};
pub use funcs::define_func;
pub use module_trait::define_module_trait;
pub use types::define_datatype;

pub fn generate(doc: &witx::Document, conf: &CodegenConf) -> TokenStream {
    // TODO at some point config should grow more ability to configure name
    // overrides.

    let types = doc.typenames().map(|t| define_datatype(&t));

    let constants = doc.constants().map(|c| {
        let name = quote::format_ident!(
            "{}_{}",
            c.ty.as_str().to_shouty_snake_case(),
            c.name.as_str().to_shouty_snake_case()
        );
        let ty = names::type_(&c.ty);
        let value = Literal::u64_unsuffixed(c.value);
        quote! {
            pub const #name: #ty = #value;
        }
    });

    let user_error_methods = conf.errors.iter().map(|errtype| {
        let abi_typename = names::type_ref(&errtype.abi_type(), anon_lifetime());
        let user_typename = errtype.typename();
        let methodname = names::user_error_conversion_method(&errtype);
        quote!(fn #methodname(&mut self, e: super::#user_typename) -> Result<#abi_typename, wasmtime::Trap>;)
    });
    let user_error_conversion = quote! {
        pub trait UserErrorConversion {
            #(#user_error_methods)*
        }
    };
    let modules = doc.modules().map(|module| {
        let modname = names::module(&module.name);
        let fs = module.funcs().map(|f| define_func(&module, &f, &conf));
        let modtrait = define_module_trait(&module, &conf);

        let add_to_linker = if conf.async_.is_sync() {
            crate::wasmtime::link_module(&module, &conf)
        } else {
            let conf = CodegenConf {
                errors: conf.errors.clone(),
                async_: AsyncConf::Blocking,
            };
            let blocking = crate::wasmtime::link_module(&module, &conf);
            let conf = CodegenConf {
                errors: conf.errors,
                async_: AsyncConf::Async,
            };
            let async_ = crate::wasmtime::link_module(&module, &conf);
            quote!( #blocking #async_ )
        };
        quote!(
            pub mod #modname {
                use super::types::*;
                pub use super::types::UserErrorConversion;
                #(#fs)*

                #modtrait

                #add_to_linker
            }
        )
    });

    quote!(
        pub mod types {
            use std::convert::TryFrom;

            #(#types)*
            #(#constants)*
            #user_error_conversion
        }
        #(#modules)*
    )
}

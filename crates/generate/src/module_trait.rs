use proc_macro2::TokenStream;
use quote::quote;

use crate::names::Names;
use crate::types::{anon_lifetime, type_needs_lifetime};
use witx::Module;

pub fn define_module_trait(names: &Names, m: &Module) -> TokenStream {
    let traitname = names.trait_name(&m.name);
    let traitmethods = m.funcs().map(|f| {
        // Check if we're returning an entity anotated with a lifetime,
        // in which case, we'll need to annotate the function itself, and
        // hence will need an explicit lifetime (rather than anonymous)
        let (lifetime, is_anonymous) = if f.results.iter().any(|ret| type_needs_lifetime(&ret.tref))
        {
            (quote!('a), false)
        } else {
            (anon_lifetime(), true)
        };
        let funcname = names.func(&f.name);
        let args = f.params.iter().map(|arg| {
            let arg_name = names.func_param(&arg.name);
            let arg_typename = names.type_ref(&arg.tref, lifetime.clone());
            let arg_type = match arg.tref.type_().passed_by() {
                witx::TypePassedBy::Value { .. } => quote!(#arg_typename),
                witx::TypePassedBy::Pointer { .. } => quote!(&#arg_typename),
                witx::TypePassedBy::PointerLengthPair { .. } => quote!(&#arg_typename),
            };
            quote!(#arg_name: #arg_type)
        });
        let rets = f
            .results
            .iter()
            .skip(1)
            .map(|ret| names.type_ref(&ret.tref, lifetime.clone()));
        let err = f
            .results
            .get(0)
            .map(|err_result| names.type_ref(&err_result.tref, lifetime.clone()))
            .unwrap_or(quote!(()));

        if is_anonymous {
            quote!(fn #funcname(&mut self, #(#args),*) -> Result<(#(#rets),*), #err>;)
        } else {
            quote!(fn #funcname<#lifetime>(&mut self, #(#args),*) -> Result<(#(#rets),*), #err>;)
        }
    });
    quote! {
        pub trait #traitname {
            #(#traitmethods)*
        }
    }
}

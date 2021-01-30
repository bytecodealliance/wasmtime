use proc_macro2::TokenStream;
use quote::quote;

use crate::error_transform::ErrorTransform;
use crate::lifetimes::{anon_lifetime, LifetimeExt};
use crate::names::Names;
use witx::Module;

pub fn passed_by_reference(ty: &witx::Type) -> bool {
    let passed_by = match ty.passed_by() {
        witx::TypePassedBy::Value { .. } => false,
        witx::TypePassedBy::Pointer { .. } | witx::TypePassedBy::PointerLengthPair { .. } => true,
    };
    match ty {
        witx::Type::Builtin(b) => match &*b {
            witx::BuiltinType::String => true,
            _ => passed_by,
        },
        witx::Type::Pointer(_) | witx::Type::ConstPointer(_) | witx::Type::Array(_) => true,
        _ => passed_by,
    }
}

pub fn define_module_trait(names: &Names, m: &Module, errxform: &ErrorTransform) -> TokenStream {
    let traitname = names.trait_name(&m.name);
    let traitmethods = m.funcs().map(|f| {
        // Check if we're returning an entity anotated with a lifetime,
        // in which case, we'll need to annotate the function itself, and
        // hence will need an explicit lifetime (rather than anonymous)
        let (lifetime, is_anonymous) = if f
            .params
            .iter()
            .chain(&f.results)
            .any(|ret| ret.tref.needs_lifetime())
        {
            (quote!('a), false)
        } else {
            (anon_lifetime(), true)
        };
        let funcname = names.func(&f.name);
        let args = f.params.iter().map(|arg| {
            let arg_name = names.func_param(&arg.name);
            let arg_typename = names.type_ref(&arg.tref, lifetime.clone());
            let arg_type = if passed_by_reference(&*arg.tref.type_()) {
                quote!(&#arg_typename)
            } else {
                quote!(#arg_typename)
            };
            quote!(#arg_name: #arg_type)
        });

        let result = if !f.noreturn {
            let rets = f
                .results
                .iter()
                .skip(1)
                .map(|ret| names.type_ref(&ret.tref, lifetime.clone()));
            let err = f
                .results
                .get(0)
                .map(|err_result| {
                    if let Some(custom_err) = errxform.for_abi_error(&err_result.tref) {
                        let tn = custom_err.typename();
                        quote!(super::#tn)
                    } else {
                        names.type_ref(&err_result.tref, lifetime.clone())
                    }
                })
                .unwrap_or(quote!(()));
            quote!( Result<(#(#rets),*), #err> )
        } else {
            let rt = names.runtime_mod();
            quote!(#rt::Trap)
        };

        if is_anonymous {
            quote!(fn #funcname(&self, #(#args),*) -> #result; )
        } else {
            quote!(fn #funcname<#lifetime>(&self, #(#args),*) -> #result;)
        }
    });
    quote! {
        pub trait #traitname {
            #(#traitmethods)*
        }
    }
}

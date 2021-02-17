use proc_macro2::TokenStream;
use quote::quote;

use crate::error_transform::ErrorTransform;
use crate::lifetimes::{anon_lifetime, LifetimeExt};
use crate::names::Names;
use witx::Module;

pub fn passed_by_reference(ty: &witx::Type) -> bool {
    match ty {
        witx::Type::Pointer(_) | witx::Type::ConstPointer(_) | witx::Type::List(_) => true,
        witx::Type::Record(r) => r.bitflags_repr().is_none(),
        witx::Type::Variant(v) => !v.is_enum(),
        _ => false,
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

        let rt = names.runtime_mod();
        let result = match f.results.len() {
            0 if f.noreturn => quote!(#rt::Trap),
            0 => quote!(()),
            1 => {
                let (ok, err) = match &**f.results[0].tref.type_() {
                    witx::Type::Variant(v) => match v.as_expected() {
                        Some(p) => p,
                        None => unimplemented!("anonymous variant ref {:?}", v),
                    },
                    _ => unimplemented!(),
                };

                let ok = match ok {
                    Some(ty) => names.type_ref(ty, lifetime.clone()),
                    None => quote!(()),
                };
                let err = match err {
                    Some(ty) => match errxform.for_abi_error(ty) {
                        Some(custom) => {
                            let tn = custom.typename();
                            quote!(super::#tn)
                        }
                        None => names.type_ref(ty, lifetime.clone()),
                    },
                    None => quote!(()),
                };
                quote!(Result<#ok, #err>)
            }
            _ => unimplemented!(),
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

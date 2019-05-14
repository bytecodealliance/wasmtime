extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{ArgCaptured, FnArg, Pat, PatIdent, Token, Type, TypeReference};

#[proc_macro_attribute]
pub fn wasi_common_cbindgen(attr: TokenStream, function: TokenStream) -> TokenStream {
    assert!(attr.is_empty());

    let function = syn::parse_macro_input!(function as syn::ItemFn);

    // generate C fn name prefixed with __wasi_
    let ident = &function.ident;
    let concatenated = format!("__wasi_{}", ident);
    let c_fn_ident = syn::Ident::new(&concatenated, ident.span());

    // capture input args
    let mut arg_ident = Vec::new();
    let mut arg_type = Vec::new();
    for input in &function.decl.inputs {
        match input {
            FnArg::Captured(ArgCaptured {
                pat: Pat::Ident(pat @ PatIdent { .. }),
                colon_token: _,
                ty,
            }) => {
                // parse arg identifier
                let ident = &pat.ident;
                arg_ident.push(quote!(#ident));
                // parse arg type
                if let Type::Reference(ty @ TypeReference { .. }) = &ty {
                    // if we're here, then we found a &-ref
                    // so substitute it for *mut since we're exporting to C
                    let elem = &ty.elem;
                    arg_type.push(quote!(*mut #elem));
                } else {
                    arg_type.push(quote!(#ty));
                }
            }
            _ => {}
        }
    }

    // capture output arg
    let output = &function.decl.output;

    let result = quote! {
        #function

        pub unsafe extern "C" fn #c_fn_ident(
            #(
                #arg_ident: #arg_type,
            )*
        ) #output {

            
        }
    };

    result.into()
}

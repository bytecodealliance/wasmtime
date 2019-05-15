extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ArgCaptured, FnArg, Pat, PatIdent, Type, TypeReference, TypeSlice};

#[proc_macro_attribute]
pub fn wasi_common_cbindgen(attr: TokenStream, function: TokenStream) -> TokenStream {
    assert!(attr.is_empty());

    let function = syn::parse_macro_input!(function as syn::ItemFn);

    // capture visibility
    let vis = &function.vis;

    // generate C fn name prefixed with __wasi_
    let fn_ident = &function.ident;
    let concatenated = format!("wasi_common_{}", fn_ident);
    let c_fn_ident = syn::Ident::new(&concatenated, fn_ident.span());

    // capture input args
    let mut arg_ident = Vec::new();
    let mut arg_type = Vec::new();
    let mut call_arg_ident = Vec::new();
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
                    let elem = &*ty.elem;
                    if let Type::Slice(elem @ TypeSlice { .. }) = &elem {
                        // slice: &[type] or &mut [type]
                        // in C it requires a signature *mut type
                        let elem = &elem.elem;
                        arg_type.push(quote!(*mut #elem));
                        // since it's a slice, we'll need to do more work here
                        // simple dereferencing is not enough
                        // firstly, we need to add a len arg to C fn
                        // secondly, we need to invoke std::slice::from_raw_parts_mut(..)
                        let concatenated = format!("{}_len", ident);
                        let len_ident = syn::Ident::new(&concatenated, ident.span());
                        call_arg_ident.push(quote! {
                            std::slice::from_raw_parts_mut(#ident, #len_ident)
                        });
                        arg_ident.push(quote!(#len_ident));
                        arg_type.push(quote!(usize));
                    } else {
                        // & or &mut type
                        // so simply substitute with *mut type
                        arg_type.push(quote!(*mut #elem));
                        // we need to properly dereference the substituted raw
                        // pointer if we are to properly call the hostcall fn
                        call_arg_ident.push(quote!(&mut *#ident));
                    }
                } else {
                    arg_type.push(quote!(#ty));
                    // non-&-ref type, so preserve whatever the arg was
                    call_arg_ident.push(quote!(#ident));
                }
            }
            _ => {}
        }
    }

    // capture output arg
    let output = &function.decl.output;

    let result = quote! {
        #function

        #[no_mangle]
        #vis unsafe extern "C" fn #c_fn_ident(
            #(
                #arg_ident: #arg_type,
            )*
        ) #output {
            #fn_ident(#(
                #call_arg_ident,
            )*)
        }
    };

    result.into()
}

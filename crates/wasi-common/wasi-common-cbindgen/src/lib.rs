extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, Pat, PatType, Type, TypeReference, TypeSlice};

fn capture_input_args(
    function: &syn::ItemFn,
) -> (
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
) {
    let mut arg_ident = Vec::new();
    let mut arg_type = Vec::new();
    let mut call_arg_ident = Vec::new();
    for input in &function.sig.inputs {
        match input {
            FnArg::Typed(PatType {
                attrs,
                pat,
                colon_token: _,
                ty,
            }) => {
                // parse arg identifier
                let ident = if let Pat::Ident(ident) = &**pat {
                    &ident.ident
                } else {
                    panic!("expected function input to be an identifier")
                };
                if !attrs.is_empty() {
                    panic!("unsupported attributes on function arg");
                }
                arg_ident.push(quote!(#ident));
                // parse arg type
                if let Type::Reference(ty @ TypeReference { .. }) = &**ty {
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
                        // & or &mut type; substitute with *const or *mut type.
                        // Also, we need to properly dereference the substituted raw
                        // pointer if we are to properly call the hostcall fn.
                        if ty.mutability.is_none() {
                            arg_type.push(quote!(*const #elem));
                            call_arg_ident.push(quote!(&*#ident));
                        } else {
                            arg_type.push(quote!(*mut #elem));
                            call_arg_ident.push(quote!(&mut *#ident));
                        }
                    }
                } else {
                    arg_type.push(quote!(#ty));
                    // non-&-ref type, so preserve whatever the arg was
                    call_arg_ident.push(quote!(#ident));
                }
            }
            _ => {
                unimplemented!("unrecognized function input pattern");
            }
        }
    }

    (arg_ident, arg_type, call_arg_ident)
}

#[proc_macro_attribute]
pub fn wasi_common_cbindgen(attr: TokenStream, function: TokenStream) -> TokenStream {
    assert!(attr.is_empty());

    let function = syn::parse_macro_input!(function as syn::ItemFn);

    // capture visibility
    let vis = &function.vis;

    // generate C fn name prefixed with wasi_common_
    let fn_ident = &function.sig.ident;
    let concatenated = format!("wasi_common_{}", fn_ident);
    let c_fn_ident = syn::Ident::new(&concatenated, fn_ident.span());

    // capture input args
    let (arg_ident, arg_type, call_arg_ident) = capture_input_args(&function);

    // capture output arg
    let output = &function.sig.output;

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

#[proc_macro_attribute]
pub fn wasi_common_cbindgen_old(attr: TokenStream, function: TokenStream) -> TokenStream {
    assert!(attr.is_empty());

    let function = syn::parse_macro_input!(function as syn::ItemFn);

    // capture visibility
    let vis = &function.vis;

    // generate C fn name prefixed with old_wasi_common_
    let fn_ident = &function.sig.ident;
    let concatenated = format!("old_wasi_common_{}", fn_ident);
    let c_fn_ident = syn::Ident::new(&concatenated, fn_ident.span());

    // capture input args
    let (arg_ident, arg_type, call_arg_ident) = capture_input_args(&function);

    // capture output arg
    let output = &function.sig.output;

    let result = quote! {
        #function

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

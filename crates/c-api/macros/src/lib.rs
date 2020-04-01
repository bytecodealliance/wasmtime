//! A set of convenience macros for our wasmtime-c-api crate.
//!
//! These are intended to mirror the macros in the `wasm.h` header file and
//! largely facilitate the `declare_ref` macro.

extern crate proc_macro;

use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::quote;

fn extract_ident(input: proc_macro::TokenStream) -> Ident {
    let input = TokenStream::from(input);
    let i = match input.into_iter().next().unwrap() {
        TokenTree::Ident(i) => i,
        _ => panic!("expected an ident"),
    };
    let name = i.to_string();
    assert!(name.ends_with("_t"));
    return i;
}

#[proc_macro]
pub fn declare_own(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ty = extract_ident(input);
    let name = ty.to_string();
    let delete = quote::format_ident!("{}_delete", &name[..name.len() - 2]);

    (quote! {
        #[no_mangle]
        pub extern fn #delete(_: Box<#ty>) {}
    })
    .into()
}

#[proc_macro]
pub fn declare_ty(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ty = extract_ident(input);
    let name = ty.to_string();
    let copy = quote::format_ident!("{}_copy", &name[..name.len() - 2]);

    (quote! {
        wasmtime_c_api_macros::declare_own!(#ty);

        #[no_mangle]
        pub extern fn #copy(src: &#ty) -> Box<#ty> {
            Box::new(src.clone())
        }
    })
    .into()
}

#[proc_macro]
pub fn declare_ref(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ty = extract_ident(input);
    let name = ty.to_string();
    let prefix = &name[..name.len() - 2];
    let copy = quote::format_ident!("{}_copy", prefix);
    let same = quote::format_ident!("{}_same", prefix);
    let get_host_info = quote::format_ident!("{}_get_host_info", prefix);
    let set_host_info = quote::format_ident!("{}_set_host_info", prefix);
    let set_host_info_final = quote::format_ident!("{}_set_host_info_with_finalizer", prefix);
    let as_ref = quote::format_ident!("{}_as_ref", prefix);
    let as_ref_const = quote::format_ident!("{}_as_ref_const", prefix);

    (quote! {
        wasmtime_c_api_macros::declare_own!(#ty);

        #[no_mangle]
        pub extern fn #copy(src: &#ty) -> Box<#ty> {
            Box::new(src.clone())
        }

        #[no_mangle]
        pub extern fn #same(a: &#ty, b: &#ty) -> bool {
            a.anyref().ptr_eq(&b.anyref())
        }

        #[no_mangle]
        pub extern fn #get_host_info(a: &#ty) -> *mut std::os::raw::c_void {
            crate::r#ref::get_host_info(&a.anyref())
        }

        #[no_mangle]
        pub extern fn #set_host_info(a: &#ty, info: *mut std::os::raw::c_void) {
            crate::r#ref::set_host_info(&a.anyref(), info, None)
        }

        #[no_mangle]
        pub extern fn #set_host_info_final(
            a: &#ty,
            info: *mut std::os::raw::c_void,
            finalizer: Option<extern "C" fn(*mut std::os::raw::c_void)>,
        ) {
            crate::r#ref::set_host_info(&a.anyref(), info, finalizer)
        }

        #[no_mangle]
        pub extern fn #as_ref(a: &#ty) -> Box<crate::wasm_ref_t> {
            let r = a.anyref();
            Box::new(crate::wasm_ref_t { r })
        }

        #[no_mangle]
        pub extern fn #as_ref_const(a: &#ty) -> Box<crate::wasm_ref_t> {
            #as_ref(a)
        }

        // TODO: implement `wasm_ref_as_#name#`
        // TODO: implement `wasm_ref_as_#name#_const`
    })
    .into()
}

use crate::utils;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn define(args: TokenStream) -> TokenStream {
    let path = utils::witx_path_from_args(args);
    let doc = match witx::load(&[&path]) {
        Ok(doc) => doc,
        Err(e) => {
            panic!("error opening file {}: {}", path.display(), e);
        }
    };

    let mut ret = TokenStream::new();

    let old = true;

    for module in doc.modules() {
        for func in module.funcs() {
            // `proc_exit` is special; it's essentially an unwinding primitive,
            // so we implement it in the runtime rather than use the implementation
            // in wasi-common.
            if func.name.as_str() == "proc_exit" {
                continue;
            }

            ret.extend(generate_wrappers(&func, old));
        }
    }

    return ret;
}

fn generate_wrappers(func: &witx::InterfaceFunc, old: bool) -> TokenStream {
    let name = format_ident!("{}", func.name.as_str());
    let mut arg_declarations = Vec::new();
    let mut arg_names = Vec::new();

    for param in func.params.iter() {
        let name = utils::param_name(param);

        if let witx::TypePassedBy::PointerLengthPair = param.tref.type_().passed_by() {
            let ptr = format_ident!("{}_ptr", name);
            let len = format_ident!("{}_len", name);
            arg_declarations.push(quote! { #ptr: super::wasi32::uintptr_t });
            arg_declarations.push(quote! { #len: super::wasi32::size_t });
            arg_names.push(ptr);
            arg_names.push(len);
            continue;
        }

        match &param.tref {
            witx::TypeRef::Name(n) => {
                if n.name.as_str() == "size" {
                    arg_declarations.push(quote! { #name: super::wasi32::size_t });
                } else {
                    let ty_name = format_ident!("__wasi_{}_t", n.name.as_str());
                    arg_declarations.push(quote! { #name: super::wasi::#ty_name });
                }
            }
            witx::TypeRef::Value(v) => match &**v {
                witx::Type::ConstPointer(_) | witx::Type::Pointer(_) => {
                    arg_declarations.push(quote! { #name: super::wasi32::uintptr_t });
                }
                _ => panic!("unexpected value type"),
            },
        }
        arg_names.push(name);
    }

    let mut ret = quote!(());

    for (i, result) in func.results.iter().enumerate() {
        if i == 0 {
            match &result.tref {
                witx::TypeRef::Name(n) => {
                    let ty_name = format_ident!("__wasi_{}_t", n.name.as_str());
                    ret = quote! { super::wasi::#ty_name };
                }
                witx::TypeRef::Value(_) => panic!("unexpected value type"),
            }
            continue;
        }
        let name = utils::param_name(result);
        arg_declarations.push(quote! { #name: super::wasi32::uintptr_t });
        arg_names.push(name);
    }

    let call = quote! {
        super::hostcalls_impl::#name(wasi_ctx, memory, #(#arg_names,)*)
    };
    let body = if func.results.len() == 0 {
        call
    } else {
        quote! {
            let ret = #call
                .err()
                .unwrap_or(super::wasi::WasiError::ESUCCESS);
            log::trace!("     | errno={}", ret);
            ret.as_raw_errno()
        }
    };

    let c_abi_name = if old {
        format_ident!("old_wasi_common_{}", name)
    } else {
        format_ident!("wasi_common_{}", name)
    };

    quote! {
        pub unsafe fn #name(
            wasi_ctx: &mut super::WasiCtx,
            memory: &mut [u8],
            #(#arg_declarations,)*
        ) -> #ret {
            #body
        }

        #[no_mangle]
        pub unsafe fn #c_abi_name(
            wasi_ctx: *mut super::WasiCtx,
            memory: *mut u8,
            memory_len: usize,
            #(#arg_declarations,)*
        ) -> #ret {
            #name(
                &mut *wasi_ctx,
                std::slice::from_raw_parts_mut(memory, memory_len),
                #(#arg_names,)*
            )
        }
    }
}

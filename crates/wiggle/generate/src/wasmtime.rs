use crate::config::{AsyncConf, CodegenConf};
use crate::funcs::func_bounds;
use crate::names;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashSet;

pub fn link_module(module: &witx::Module, config: &CodegenConf) -> TokenStream {
    let send_bound = if !config.async_.is_sync() {
        quote! { + Send, T: Send }
    } else {
        quote! {}
    };

    let mut bodies = Vec::new();
    let mut bounds = HashSet::new();
    for f in module.funcs() {
        bodies.push(generate_func(&module, &f, &config.async_));
        let bound = func_bounds(module, &f, config);
        for b in bound {
            bounds.insert(b);
        }
    }

    let ctx_bound = {
        let bounds = bounds.into_iter();
        quote!( #(#bounds)+* #send_bound )
    };

    let add_to_linker = format_ident!(
        "add_to_linker{}",
        match config.async_ {
            AsyncConf::Sync => "",
            AsyncConf::Blocking => "_blocking",
            AsyncConf::Async => "_async",
        }
    );

    quote! {
        /// Adds all instance items to the specified `Linker`.
        pub fn #add_to_linker<T, U>(
            linker: &mut wasmtime::Linker<T>,
            get_cx: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> anyhow::Result<()>
            where
                U: #ctx_bound #send_bound
        {
            #(#bodies)*
            Ok(())
        }
    }
}

fn generate_func(
    module: &witx::Module,
    func: &witx::InterfaceFunc,
    asyncness: &AsyncConf,
) -> TokenStream {
    let module_str = module.name.as_str();

    let field_str = func.name.as_str();
    let field_ident = names::func(&func.name);

    let (params, results) = func.wasm_signature();

    let arg_names = (0..params.len())
        .map(|i| Ident::new(&format!("arg{}", i), Span::call_site()))
        .collect::<Vec<_>>();
    let arg_decls = params
        .iter()
        .enumerate()
        .map(|(i, ty)| {
            let name = &arg_names[i];
            let wasm = names::wasm_type(*ty);
            quote! { #name: #wasm }
        })
        .collect::<Vec<_>>();

    let ret_ty = match results.len() {
        0 => quote!(()),
        1 => names::wasm_type(results[0]),
        _ => unimplemented!(),
    };

    let await_ = if asyncness.is_sync() {
        quote!()
    } else {
        quote!(.await)
    };

    let body = quote! {
        let mem = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(m)) => m,
            _ => {
                return Err(wasmtime::Trap::new("missing required memory export"));
            }
        };
        let (mem , ctx) = mem.data_and_store_mut(&mut caller);
        let ctx = get_cx(ctx);
        let mem = wiggle::memory::WasmtimeGuestMemory::new(mem);
        #field_ident(ctx, &mem #(, #arg_names)*) #await_
    };

    match asyncness {
        AsyncConf::Async => {
            let wrapper = format_ident!("func_wrap{}_async", params.len());
            quote! {
                linker.#wrapper(
                    #module_str,
                    #field_str,
                    move |mut caller: wasmtime::Caller<'_, T> #(, #arg_decls)*| {
                        Box::new(async move { #body })
                    },
                )?;
            }
        }

        AsyncConf::Blocking => {
            quote! {
                linker.func_wrap(
                    #module_str,
                    #field_str,
                    move |mut caller: wasmtime::Caller<'_, T> #(, #arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
                        let result = async { #body };
                        wiggle::run_in_dummy_executor(result)?
                    },
                )?;
            }
        }

        AsyncConf::Sync => {
            quote! {
                linker.func_wrap(
                    #module_str,
                    #field_str,
                    move |mut caller: wasmtime::Caller<'_, T> #(, #arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
                        #body
                    },
                )?;
            }
        }
    }
}

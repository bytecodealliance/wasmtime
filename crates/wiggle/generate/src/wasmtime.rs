use crate::config::Asyncness;
use crate::funcs::func_bounds;
use crate::{CodegenSettings, Names};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashSet;

pub fn link_module(
    module: &witx::Module,
    names: &Names,
    target_path: Option<&syn::Path>,
    settings: &CodegenSettings,
) -> TokenStream {
    let module_ident = names.module(&module.name);

    let send_bound = if settings.async_.contains_async(module) {
        quote! { + Send, T: Send }
    } else {
        quote! {}
    };

    let mut bodies = Vec::new();
    let mut bounds = HashSet::new();
    for f in module.funcs() {
        let asyncness = settings.async_.get(module.name.as_str(), f.name.as_str());
        bodies.push(generate_func(&module, &f, names, target_path, asyncness));
        let bound = func_bounds(names, module, &f, settings);
        for b in bound {
            bounds.insert(b);
        }
    }

    let ctx_bound = if let Some(target_path) = target_path {
        let bounds = bounds
            .into_iter()
            .map(|b| quote!(#target_path::#module_ident::#b));
        quote!( #(#bounds)+* #send_bound )
    } else {
        let bounds = bounds.into_iter();
        quote!( #(#bounds)+* #send_bound )
    };

    let func_name = if target_path.is_none() {
        format_ident!("add_to_linker")
    } else {
        format_ident!("add_{}_to_linker", module_ident)
    };

    let rt = names.runtime_mod();

    quote! {
        /// Adds all instance items to the specified `Linker`.
        pub fn #func_name<T, U>(
            linker: &mut #rt::wasmtime_crate::Linker<T>,
            get_cx: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
        ) -> #rt::anyhow::Result<()>
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
    names: &Names,
    target_path: Option<&syn::Path>,
    asyncness: Asyncness,
) -> TokenStream {
    let rt = names.runtime_mod();

    let module_str = module.name.as_str();
    let module_ident = names.module(&module.name);

    let field_str = func.name.as_str();
    let field_ident = names.func(&func.name);

    let (params, results) = func.wasm_signature();

    let arg_names = (0..params.len())
        .map(|i| Ident::new(&format!("arg{}", i), Span::call_site()))
        .collect::<Vec<_>>();
    let arg_decls = params
        .iter()
        .enumerate()
        .map(|(i, ty)| {
            let name = &arg_names[i];
            let wasm = names.wasm_type(*ty);
            quote! { #name: #wasm }
        })
        .collect::<Vec<_>>();

    let ret_ty = match results.len() {
        0 => quote!(()),
        1 => names.wasm_type(results[0]),
        _ => unimplemented!(),
    };

    let await_ = if asyncness.is_sync() {
        quote!()
    } else {
        quote!(.await)
    };

    let abi_func = if let Some(target_path) = target_path {
        quote!( #target_path::#module_ident::#field_ident )
    } else {
        quote!( #field_ident )
    };

    let body = quote! {
        let mem = match caller.get_export("memory") {
            Some(#rt::wasmtime_crate::Extern::Memory(m)) => m,
            _ => {
                return Err(#rt::wasmtime_crate::Trap::new("missing required memory export"));
            }
        };
        // Note the unsafety here. Our goal is to simultaneously borrow the
        // memory and custom data from `caller`, and the store it's connected
        // to. Rust will not let us do that, however, because we must call two
        // separate methods (both of which borrow the whole `caller`) and one of
        // our borrows is mutable (the custom data).
        //
        // This operation, however, is safe because these borrows do not overlap
        // and in the process of borrowing them mutability doesn't actually
        // touch anything. This is akin to mutably borrowing two indices in an
        // array, which is safe so long as the indices are separate.
        //
        // TODO: depending on how common this is for other users to run into we
        // may wish to consider adding a dedicated method for this. For now the
        // future of `GuestPtr` may be a bit hazy, so let's just get this
        // working from the previous iteration for now.
        let (ctx, mem) = unsafe {
            let mem = &mut *(mem.data_mut(&mut caller) as *mut [u8]);
            (get_cx(caller.data_mut()), #rt::wasmtime::WasmtimeGuestMemory::new(mem))
        };
        match #abi_func(ctx, &mem #(, #arg_names)*) #await_ {
            Ok(r) => Ok(<#ret_ty>::from(r)),
            Err(#rt::Trap::String(err)) => Err(#rt::wasmtime_crate::Trap::new(err)),
            Err(#rt::Trap::I32Exit(err)) => Err(#rt::wasmtime_crate::Trap::i32_exit(err)),
        }
    };

    match asyncness {
        Asyncness::Async => {
            let wrapper = format_ident!("func_wrap{}_async", params.len());
            quote! {
                linker.#wrapper(
                    #module_str,
                    #field_str,
                    move |mut caller: #rt::wasmtime_crate::Caller<'_, T> #(, #arg_decls)*| {
                        Box::new(async move { #body })
                    },
                )?;
            }
        }

        Asyncness::Blocking => {
            quote! {
                linker.func_wrap(
                    #module_str,
                    #field_str,
                    move |mut caller: #rt::wasmtime_crate::Caller<'_, T> #(, #arg_decls)*| -> Result<#ret_ty, #rt::wasmtime_crate::Trap> {
                        let result = async { #body };
                        #rt::run_in_dummy_executor(result)
                    },
                )?;
            }
        }

        Asyncness::Sync => {
            quote! {
                linker.func_wrap(
                    #module_str,
                    #field_str,
                    move |mut caller: #rt::wasmtime_crate::Caller<'_, T> #(, #arg_decls)*| -> Result<#ret_ty, #rt::wasmtime_crate::Trap> {
                        #body
                    },
                )?;
            }
        }
    }
}

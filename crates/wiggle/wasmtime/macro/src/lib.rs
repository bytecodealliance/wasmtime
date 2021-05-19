use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse_macro_input;
use wiggle_generate::Names;

mod config;

use config::{AsyncConf, Asyncness};

/// Define the structs required to integrate a Wiggle implementation with Wasmtime.
///
/// ## Arguments
///
/// Arguments are provided using struct syntax e.g. `{ arg_name: value }`.
///
/// * `target`: The path of the module where the Wiggle implementation is defined.
/// * `witx` or `witx_literal`: the .witx document where the interface is defined.
///   `witx` takes a list of filesystem paths, e.g. `["/path/to/file1.witx",
///   "./path/to_file2.witx"]`. Relative paths are relative to the root of the crate
///   where the macro is invoked. `witx_literal` takes a string of the witx document, e.g.
///   `"(typename $foo u8)"`.
#[proc_macro]
pub fn wasmtime_integration(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as config::Config);
    let doc = config.load_document();
    let names = Names::new(quote!(wasmtime_wiggle));

    let modules = doc
        .modules()
        .map(|module| generate_module(&module, &names, &config.target.path, &config.async_));
    quote!( #(#modules)* ).into()
}

fn generate_module(
    module: &witx::Module,
    names: &Names,
    target_path: &syn::Path,
    async_conf: &AsyncConf,
) -> TokenStream2 {
    let module_ident = names.module(&module.name);
    let trait_ident = names.trait_name(&module.name);

    let send_bound = if async_conf.contains_async(module) {
        quote! { + Send }
    } else {
        quote! {}
    };

    let bodies = module.funcs().map(|f| {
        let asyncness = async_conf.is_async(module.name.as_str(), f.name.as_str());
        generate_func(&module, &f, names, &target_path, asyncness)
    });

    quote! {
        /// Adds all instance items to the specified `Linker`.
        pub fn add_to_linker<T, C>(linker: &mut wasmtime::Linker<T>) -> anyhow::Result<()>
            where
                T: std::borrow::BorrowMut<C> #send_bound,
                C: #target_path::#module_ident::#trait_ident #send_bound,
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
    target_path: &syn::Path,
    asyncness: Asyncness,
) -> TokenStream2 {
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

    let runtime = names.runtime_mod();

    let body = quote! {
        let mem = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(m)) => m,
            _ => {
                return Err(wasmtime::Trap::new("missing required memory export"));
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
            (caller.data_mut().borrow_mut(), #runtime::WasmtimeGuestMemory::new(mem))
        };
        match #target_path::#module_ident::#field_ident(ctx, &mem #(, #arg_names)*) #await_ {
            Ok(r) => Ok(<#ret_ty>::from(r)),
            Err(wasmtime_wiggle::Trap::String(err)) => Err(wasmtime::Trap::new(err)),
            Err(wasmtime_wiggle::Trap::I32Exit(err)) => Err(wasmtime::Trap::i32_exit(err)),
        }
    };

    match asyncness {
        Asyncness::Async => {
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

        Asyncness::Blocking => {
            quote! {
                linker.func_wrap(
                    #module_str,
                    #field_str,
                    move |mut caller: wasmtime::Caller<'_, T> #(, #arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
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
                    move |mut caller: wasmtime::Caller<'_, T> #(, #arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
                        #body
                    },
                )?;
            }
        }
    }
}

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse_macro_input;
use wiggle_generate::Names;

mod config;

use config::{AsyncConf, ModuleConf, TargetConf};

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
/// * `ctx`: The context struct used for the Wiggle implementation. This must be the same
///   type as the `wasmtime_wiggle::from_witx` macro at `target` was invoked with. However, it
///   must be imported to the current scope so that it is a bare identifier e.g. `CtxType`, not
///   `path::to::CtxType`.
/// * `modules`: Describes how any modules in the witx document will be implemented as Wasmtime
///    instances. `modules` takes a map from the witx module name to a configuration struct, e.g.
///    `foo => { name: Foo }, bar => { name: Bar }` will generate integrations for the modules
///    named `foo` and `bar` in the witx document, as `pub struct Foo` and `pub struct Bar`
///    respectively.
///    The module configuration uses struct syntax with the following fields:
///      * `name`: required, gives the name of the struct which encapsulates the instance for
///         Wasmtime.
///      * `docs`: optional, a doc string that will be used for the definition of the struct.
///      * `function_override`: A map of witx function names to Rust function symbols for
///         functions that should not call the Wiggle-generated functions, but instead use
///         a separate implementation. This is typically used for functions that need to interact
///         with Wasmtime in a manner that Wiggle does not permit, e.g. wasi's `proc_exit` function
///         needs to return a Trap directly to the runtime.
///    Example:
///    `modules: { some_module => { name: SomeTypeName, docs: "Doc string for definition of
///     SomeTypeName here", function_override: { foo => my_own_foo } }`.
///
#[proc_macro]
pub fn wasmtime_integration(args: TokenStream) -> TokenStream {
    let config = parse_macro_input!(args as config::Config);
    let doc = config.load_document();
    let names = Names::new(quote!(wasmtime_wiggle));

    let modules = config.modules.iter().map(|(name, module_conf)| {
        let module = doc
            .module(&witx::Id::new(name))
            .unwrap_or_else(|| panic!("witx document did not contain module named '{}'", name));
        generate_module(
            &module,
            &module_conf,
            &names,
            &config.target,
            &config.ctx.name,
            &config.async_,
        )
    });
    quote!( #(#modules)* ).into()
}

fn generate_module(
    module: &witx::Module,
    module_conf: &ModuleConf,
    names: &Names,
    target_conf: &TargetConf,
    ctx_type: &syn::Type,
    async_conf: &AsyncConf,
) -> TokenStream2 {
    let fields = module.funcs().map(|f| {
        let name_ident = names.func(&f.name);
        quote! { pub #name_ident: wasmtime::Func }
    });
    let get_exports = module.funcs().map(|f| {
        let func_name = f.name.as_str();
        let name_ident = names.func(&f.name);
        quote! { #func_name => Some(&self.#name_ident) }
    });
    let ctor_fields = module.funcs().map(|f| names.func(&f.name));

    let module_name = module.name.as_str();

    let linker_add = module.funcs().map(|f| {
        let func_name = f.name.as_str();
        let name_ident = names.func(&f.name);
        quote! {
            linker.define(#module_name, #func_name, self.#name_ident.clone())?;
        }
    });

    let target_path = &target_conf.path;
    let module_id = names.module(&module.name);
    let target_module = quote! { #target_path::#module_id };

    let mut fns = Vec::new();
    let mut ctor_externs = Vec::new();
    let mut host_funcs = Vec::new();

    #[cfg(not(feature = "async"))]
    let mut requires_dummy_executor = false;

    for f in module.funcs() {
        let is_async = async_conf.is_async(module.name.as_str(), f.name.as_str());
        #[cfg(not(feature = "async"))]
        if is_async {
            requires_dummy_executor = true;
        }
        generate_func(
            &module_id,
            &f,
            names,
            &target_module,
            ctx_type,
            is_async,
            &mut fns,
            &mut ctor_externs,
            &mut host_funcs,
        );
    }

    let type_name = module_conf.name.clone();
    let type_docs = module_conf
        .docs
        .as_ref()
        .map(|docs| quote!( #[doc = #docs] ))
        .unwrap_or_default();
    let constructor_docs = format!(
        "Creates a new [`{}`] instance.

External values are allocated into the `store` provided and
configuration of the instance itself should be all
contained in the `cx` parameter.",
        module_conf.name.to_string()
    );

    let config_adder_definitions = host_funcs.iter().map(|(func_name, body)| {
        let adder_func = format_ident!("add_{}_to_config", names.func(&func_name));
        let docs = format!(
            "Add the host function for `{}` to a config under a given module and field name.",
            func_name.as_str()
        );
        quote! {
            #[doc = #docs]
            pub fn #adder_func(config: &mut wasmtime::Config, module: &str, field: &str) {
                #body
            }
        }
    });
    let config_adder_invocations = host_funcs.iter().map(|(func_name, _body)| {
        let adder_func = format_ident!("add_{}_to_config", names.func(&func_name));
        let module = module.name.as_str();
        let field = func_name.as_str();
        quote! {
            Self::#adder_func(config, #module, #field);
        }
    });

    #[cfg(not(feature = "async"))]
    let dummy_executor = if requires_dummy_executor {
        dummy_executor()
    } else {
        quote!()
    };
    #[cfg(feature = "async")]
    let dummy_executor = quote!();

    quote! {
        #type_docs
        pub struct #type_name {
            #(#fields,)*
        }

        impl #type_name {
            #[doc = #constructor_docs]
            pub fn new(store: &wasmtime::Store, ctx: std::rc::Rc<std::cell::RefCell<#ctx_type>>) -> Self {
                #(#ctor_externs)*

                Self {
                    #(#ctor_fields,)*
                }
            }


            /// Looks up a field called `name` in this structure, returning it
            /// if found.
            ///
            /// This is often useful when instantiating a `wasmtime` instance
            /// where name resolution often happens with strings.
            pub fn get_export(&self, name: &str) -> Option<&wasmtime::Func> {
                match name {
                    #(#get_exports,)*
                    _ => None,
                }
            }

            /// Adds all instance items to the specified `Linker`.
            pub fn add_to_linker(&self, linker: &mut wasmtime::Linker) -> anyhow::Result<()> {
                #(#linker_add)*
                Ok(())
            }

            /// Adds the host functions to the given [`wasmtime::Config`].
            ///
            /// Host functions added to the config expect [`set_context`] to be called.
            ///
            /// Host functions will trap if the context is not set in the calling [`wasmtime::Store`].
            pub fn add_to_config(config: &mut wasmtime::Config) {
                #(#config_adder_invocations)*
            }

            #(#config_adder_definitions)*

            /// Sets the context in the given store.
            ///
            /// Context must be set in the store when using [`add_to_config`] and prior to any
            /// host function being called.
            ///
            /// If the context is already set in the store, the given context is returned as an error.
            pub fn set_context(store: &wasmtime::Store, ctx: #ctx_type) -> Result<(), #ctx_type> {
                store.set(std::rc::Rc::new(std::cell::RefCell::new(ctx))).map_err(|ctx| {
                    match std::rc::Rc::try_unwrap(ctx) {
                        Ok(ctx) => ctx.into_inner(),
                        Err(_) => unreachable!(),
                    }
                })
            }

            #(#fns)*

            #dummy_executor
        }
    }
}

fn generate_func(
    module_ident: &Ident,
    func: &witx::InterfaceFunc,
    names: &Names,
    target_module: &TokenStream2,
    ctx_type: &syn::Type,
    is_async: bool,
    fns: &mut Vec<TokenStream2>,
    ctors: &mut Vec<TokenStream2>,
    host_funcs: &mut Vec<(witx::Id, TokenStream2)>,
) {
    let name_ident = names.func(&func.name);

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

    let async_ = if is_async { quote!(async) } else { quote!() };
    let await_ = if is_async { quote!(.await) } else { quote!() };

    let runtime = names.runtime_mod();
    let fn_ident = format_ident!("{}_{}", module_ident, name_ident);

    fns.push(quote! {
        #async_ fn #fn_ident(caller: &wasmtime::Caller<'_>, ctx: &mut #ctx_type #(, #arg_decls)*) -> Result<#ret_ty, wasmtime::Trap> {
            unsafe {
                let mem = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(m)) => m,
                    _ => {
                        return Err(wasmtime::Trap::new("missing required memory export"));
                    }
                };
                let mem = #runtime::WasmtimeGuestMemory::new(mem);
                match #target_module::#name_ident(ctx, &mem #(, #arg_names)*) #await_ {
                    Ok(r) => Ok(r.into()),
                    Err(wasmtime_wiggle::Trap::String(err)) => Err(wasmtime::Trap::new(err)),
                    Err(wasmtime_wiggle::Trap::I32Exit(err)) => Err(wasmtime::Trap::i32_exit(err)),
                }
            }
        }
    });

    if is_async {
        #[cfg(feature = "async")]
        {
            let wrapper = format_ident!("wrap{}_async", params.len());
            ctors.push(quote! {
            let #name_ident = wasmtime::Func::#wrapper(
                store,
                ctx.clone(),
                move |caller: wasmtime::Caller<'_>, my_ctx: &std::rc::Rc<std::cell::RefCell<_>> #(,#arg_decls)*|
                    -> Box<dyn std::future::Future<Output = Result<#ret_ty, wasmtime::Trap>>> {
                    Box::new(async move { Self::#fn_ident(&caller, &mut my_ctx.borrow_mut() #(, #arg_names)*).await })
                }
            );
        });
        }

        #[cfg(not(feature = "async"))]
        {
            // Emit a synchronous function. Self::#fn_ident returns a Future, so we need to
            // use a dummy executor to let any synchronous code inside there execute correctly. If
            // the future ends up Pending, this func will Trap.
            ctors.push(quote! {
                let my_ctx = ctx.clone();
                let #name_ident = wasmtime::Func::wrap(
                    store,
                    move |caller: wasmtime::Caller #(, #arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
                        Self::run_in_dummy_executor(Self::#fn_ident(&caller, &mut my_ctx.borrow_mut() #(, #arg_names)*))
                    }
                );
            });
        }
    } else {
        ctors.push(quote! {
            let my_ctx = ctx.clone();
            let #name_ident = wasmtime::Func::wrap(
                store,
                move |caller: wasmtime::Caller #(, #arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
                    Self::#fn_ident(&caller, &mut my_ctx.borrow_mut() #(, #arg_names)*)
                }
            );
        });
    }

    let host_wrapper = if is_async {
        #[cfg(feature = "async")]
        {
            let wrapper = format_ident!("wrap{}_host_func_async", params.len());
            quote! {
                config.#wrapper(
                    module,
                    field,
                    move |caller #(,#arg_decls)*|
                        -> Box<dyn std::future::Future<Output = Result<#ret_ty, wasmtime::Trap>>> {
                        Box::new(async move {
                            let ctx = caller.store()
                                .get::<std::rc::Rc<std::cell::RefCell<#ctx_type>>>()
                                .ok_or_else(|| wasmtime::Trap::new("context is missing in the store"))?;
                            let result = Self::#fn_ident(&caller, &mut ctx.borrow_mut() #(, #arg_names)*).await;
                            result
                        })
                    }
                );
            }
        }

        #[cfg(not(feature = "async"))]
        {
            // Emit a synchronous host function. Self::#fn_ident returns a Future, so we need to
            // use a dummy executor to let any synchronous code inside there execute correctly. If
            // the future ends up Pending, this func will Trap.
            quote! {
                config.wrap_host_func(
                    module,
                    field,
                    move |caller: wasmtime::Caller #(, #arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
                        let ctx = caller
                            .store()
                            .get::<std::rc::Rc<std::cell::RefCell<#ctx_type>>>()
                            .ok_or_else(|| wasmtime::Trap::new("context is missing in the store"))?;
                        Self::run_in_dummy_executor(Self::#fn_ident(&caller, &mut ctx.borrow_mut()  #(, #arg_names)*))
                    },
                );
            }
        }
    } else {
        quote! {
            config.wrap_host_func(
                module,
                field,
                move |caller: wasmtime::Caller #(, #arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
                    let ctx = caller
                        .store()
                        .get::<std::rc::Rc<std::cell::RefCell<#ctx_type>>>()
                        .ok_or_else(|| wasmtime::Trap::new("context is missing in the store"))?;
                    Self::#fn_ident(&caller, &mut ctx.borrow_mut()  #(, #arg_names)*)
                },
            );
        }
    };
    host_funcs.push((func.name.clone(), host_wrapper));
}
#[cfg(not(feature = "async"))]
fn dummy_executor() -> TokenStream2 {
    quote! {
        fn run_in_dummy_executor<F: std::future::Future>(future: F) -> F::Output {
            use std::pin::Pin;
            use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

            let mut f = Pin::from(Box::new(future));
            let waker = dummy_waker();
            let mut cx = Context::from_waker(&waker);
            match f.as_mut().poll(&mut cx) {
                Poll::Ready(val) => return val,
                Poll::Pending => {
                    panic!("Cannot wait on pending future: must enable wiggle \"async\" future and execute on an async Store")
                }
            }

            fn dummy_waker() -> Waker {
                return unsafe { Waker::from_raw(clone(5 as *const _)) };

                unsafe fn clone(ptr: *const ()) -> RawWaker {
                    assert_eq!(ptr as usize, 5);
                    const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
                    RawWaker::new(ptr, &VTABLE)
                }

                unsafe fn wake(ptr: *const ()) {
                    assert_eq!(ptr as usize, 5);
                }

                unsafe fn wake_by_ref(ptr: *const ()) {
                    assert_eq!(ptr as usize, 5);
                }

                unsafe fn drop(ptr: *const ()) {
                    assert_eq!(ptr as usize, 5);
                }
            }

        }
    }
}

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
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

    #[cfg(feature = "async")]
    let async_config = config.async_.clone();
    #[cfg(not(feature = "async"))]
    let async_config = AsyncConf::default();

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
            &async_config,
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

    let ctor_externs = module.funcs().map(|f| {
        generate_func(
            &f,
            names,
            &target_module,
            async_conf.is_async(module.name.as_str(), f.name.as_str()),
        )
    });

    let type_name = module_conf.name.clone();
    let type_docs = module_conf
        .docs
        .as_ref()
        .map(|docs| quote!( #[doc = #docs] ))
        .unwrap_or_default();
    let constructor_docs = format!(
        "Creates a new [`{}`] instance.

External values are allocated into the `store` provided and
configuration of the wasi instance itself should be all
contained in the `cx` parameter.",
        module_conf.name.to_string()
    );

    quote! {
        #type_docs
        pub struct #type_name {
            #(#fields,)*
        }

        impl #type_name {
            #[doc = #constructor_docs]
            pub fn new(store: &wasmtime::Store, cx: std::rc::Rc<std::cell::RefCell<#ctx_type>>) -> Self {
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
        }
    }
}

fn generate_func(
    func: &witx::InterfaceFunc,
    names: &Names,
    target_module: &TokenStream2,
    is_async: bool,
) -> TokenStream2 {
    let name_ident = names.func(&func.name);

    let (params, results) = func.wasm_signature();

    let arg_names = (0..params.len())
        .map(|i| Ident::new(&format!("arg{}", i), Span::call_site()))
        .collect::<Vec<_>>();
    let arg_decls = params.iter().enumerate().map(|(i, ty)| {
        let name = &arg_names[i];
        let wasm = names.wasm_type(*ty);
        quote! { #name: #wasm }
    });

    let ret_ty = match results.len() {
        0 => quote!(()),
        1 => names.wasm_type(results[0]),
        _ => unimplemented!(),
    };

    let runtime = names.runtime_mod();

    let await_ = if is_async { quote!(.await) } else { quote!() };

    let closure_body = quote! {
        unsafe {
            let mem = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => {
                    return Err(wasmtime::Trap::new("missing required memory export"));
                }
            };
            let mem = #runtime::WasmtimeGuestMemory::new(mem);
            let result = #target_module::#name_ident(
                &mut *my_cx.borrow_mut(),
                &mem,
                #(#arg_names),*
            ) #await_;
            match result {
                Ok(r) => Ok(r.into()),
                Err(wasmtime_wiggle::Trap::String(err)) => Err(wasmtime::Trap::new(err)),
                Err(wasmtime_wiggle::Trap::I32Exit(err)) => Err(wasmtime::Trap::i32_exit(err)),
            }
        }

    };
    if is_async {
        let wrapper = quote::format_ident!("wrap{}_async", params.len());
        quote! {
        let #name_ident = wasmtime::Func::#wrapper(
            store,
            cx.clone(),
            move |caller: wasmtime::Caller<'_>, my_cx: &Rc<RefCell<_>> #(,#arg_decls)*|
              -> Box<dyn std::future::Future<Output = Result<#ret_ty, wasmtime::Trap>>>
            {
                Box::new(async move { #closure_body })
            }
        );
        }
    } else {
        quote! {
            let my_cx = cx.clone();
            let #name_ident = wasmtime::Func::wrap(
                store,
                move |caller: wasmtime::Caller<'_> #(,#arg_decls)*| -> Result<#ret_ty, wasmtime::Trap> {
                    #closure_body
                }
            );
        }
    }
}

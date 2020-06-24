use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse_macro_input;
use wiggle_generate::Names;

mod config;

use config::{MissingMemoryConf, ModuleConf, TargetConf};

#[proc_macro]
pub fn define_wasmtime_integration(args: TokenStream) -> TokenStream {
    let mut config = parse_macro_input!(args as config::Config);
    config.witx.make_paths_relative_to(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR env var"),
    );
    let doc = config.load_document();
    let names = Names::new(&config.ctx.name, quote!(wasmtime_wiggle));

    let modules = config.modules.iter().map(|(name, module_conf)| {
        let module = doc
            .module(&witx::Id::new(name))
            .unwrap_or_else(|| panic!("witx document did not contain module named '{}'", name));
        generate_module(
            &module,
            &module_conf,
            &names,
            &config.target,
            &config.missing_memory,
        )
    });
    quote!( #(#modules)* ).into()
}

fn generate_module(
    module: &witx::Module,
    module_conf: &ModuleConf,
    names: &Names,
    target_conf: &TargetConf,
    missing_mem_conf: &MissingMemoryConf,
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
        if let Some(func_override) = module_conf.function_override.find(&f.name.as_str()) {
            let name_ident = names.func(&f.name);
            quote! { let #name_ident = wasmtime::Func::wrap(store, #func_override); }
        } else {
            generate_func(&f, names, missing_mem_conf, &target_module)
        }
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

    let ctx_type = names.ctx_type();

    quote! {
        #type_docs
        pub struct #type_name {
            #(#fields,)*
        }

        impl #type_name {
            #[doc = #constructor_docs]
            pub fn new(store: &wasmtime::Store, cx: #ctx_type) -> Self {
                let cx = std::rc::Rc::new(std::cell::RefCell::new(cx));
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
    missing_mem_conf: &MissingMemoryConf,
    target_module: &TokenStream2,
) -> TokenStream2 {
    let missing_mem_err = &missing_mem_conf.err;
    let name_ident = names.func(&func.name);

    let coretype = func.core_type();

    let arg_decls = coretype.args.iter().map(|arg| {
        let name = names.func_core_arg(arg);
        let atom = names.atom_type(arg.repr());
        quote! { #name: #atom }
    });
    let arg_names = coretype.args.iter().map(|arg| names.func_core_arg(arg));

    let (ret_ty, handle_early_error) = if let Some(ret) = &coretype.ret {
        let ret_ty = match ret.signifies {
            witx::CoreParamSignifies::Value(atom) => names.atom_type(atom),
            _ => unreachable!("coretype ret should always be passed by value"),
        };
        (quote! { #ret_ty }, quote! { return e.into(); })
    } else {
        (
            quote! {()},
            quote! { panic!("unrecoverable error in {}: {}", stringify!(#name_ident), e) },
        )
    };

    let runtime = names.runtime_mod();

    quote! {
        let my_cx = cx.clone();
        let #name_ident = wasmtime::Func::wrap(
            store,
            move |caller: wasmtime::Caller<'_> #(,#arg_decls)*| -> #ret_ty {
                unsafe {
                    let mem = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(m)) => m,
                        _ => {
                            log::warn!("callee does not export a memory as \"memory\"");
                            let e = { #missing_mem_err };
                            #handle_early_error
                        }
                    };
                    // Wiggle does not expose any methods for
                    // functions to re-enter the WebAssembly module,
                    // or expose the memory via non-wiggle mechanisms.
                    // Therefore, creating a new BorrowChecker at the
                    // root of each function invocation is correct.
                    let bc = #runtime::BorrowChecker::new();
                    let mem = #runtime::WasmtimeGuestMemory::new( mem, bc );
                    #target_module::#name_ident(
                        &mut my_cx.borrow_mut(),
                        &mem,
                        #(#arg_names),*
                    )
                }
            }
        );
    }
}

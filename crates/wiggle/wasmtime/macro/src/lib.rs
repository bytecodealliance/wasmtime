use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
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

enum Abi {
    I32,
    I64,
    F32,
    F64,
}

fn generate_module(
    module: &witx::Module,
    module_conf: &ModuleConf,
    names: &Names,
    target_conf: &TargetConf,
    missing_mem_conf: &MissingMemoryConf,
) -> TokenStream2 {
    let mut fields = Vec::new();
    let mut get_exports = Vec::new();
    let mut ctor_externs = Vec::new();
    let mut ctor_fields = Vec::new();
    let mut linker_add = Vec::new();

    let runtime = names.runtime_mod();
    let target_path = &target_conf.path;
    let missing_mem_err = &missing_mem_conf.err;

    let module_name = module.name.as_str();
    let module_id = names.module(&module.name);
    for func in module.funcs() {
        let func_name = func.name.as_str();
        let name_ident = names.func(&func.name);
        fields.push(quote! { pub #name_ident: wasmtime::Func });
        get_exports.push(quote! { #func_name => Some(&self.#name_ident) });
        ctor_fields.push(name_ident.clone());
        linker_add.push(quote! {
            linker.define(#module_name, #func_name, self.#name_ident.clone())?;
        });

        if let Some(func_override) = module_conf.function_override.find(func_name) {
            ctor_externs.push(quote! {
                let #name_ident = wasmtime::Func::wrap(store, #func_override);
            });
            continue;
        }

        let mut shim_arg_decls = Vec::new();
        let mut params = Vec::new();
        let mut hostcall_args = Vec::new();

        for param in func.params.iter() {
            let name = names.func_param(&param.name);

            // Registers a new parameter to the shim we're making with the
            // given `name`, the `abi_ty` wasm type
            //
            // This will register a whole bunch of things:
            //
            // * The cranelift type for the parameter
            // * Syntax to specify the actual function parameter
            // * How to actually pass this argument to the host
            //   implementation, converting as necessary.
            let mut add_param = |name: &Ident, abi_ty: Abi| {
                match abi_ty {
                    Abi::I32 => {
                        params.push(quote! { types::I32 });
                        shim_arg_decls.push(quote! { #name: i32 });
                    }
                    Abi::I64 => {
                        params.push(quote! { types::I64 });
                        shim_arg_decls.push(quote! { #name: i64 });
                    }
                    Abi::F32 => {
                        params.push(quote! { types::F32 });
                        shim_arg_decls.push(quote! { #name: f32 });
                    }
                    Abi::F64 => {
                        params.push(quote! { types::F64 });
                        shim_arg_decls.push(quote! { #name: f64 });
                    }
                }
                hostcall_args.push(quote! { #name as _ });
            };

            match &*param.tref.type_() {
                witx::Type::Int(e) => match e.repr {
                    witx::IntRepr::U64 => add_param(&name, Abi::I64),
                    _ => add_param(&name, Abi::I32),
                },

                witx::Type::Enum(e) => match e.repr {
                    witx::IntRepr::U64 => add_param(&name, Abi::I64),
                    _ => add_param(&name, Abi::I32),
                },

                witx::Type::Flags(f) => match f.repr {
                    witx::IntRepr::U64 => add_param(&name, Abi::I64),
                    _ => add_param(&name, Abi::I32),
                },

                witx::Type::Builtin(witx::BuiltinType::Char8)
                | witx::Type::Builtin(witx::BuiltinType::S8)
                | witx::Type::Builtin(witx::BuiltinType::U8)
                | witx::Type::Builtin(witx::BuiltinType::S16)
                | witx::Type::Builtin(witx::BuiltinType::U16)
                | witx::Type::Builtin(witx::BuiltinType::S32)
                | witx::Type::Builtin(witx::BuiltinType::U32)
                | witx::Type::Builtin(witx::BuiltinType::USize) => {
                    add_param(&name, Abi::I32);
                }

                witx::Type::Builtin(witx::BuiltinType::S64)
                | witx::Type::Builtin(witx::BuiltinType::U64) => {
                    add_param(&name, Abi::I64);
                }

                witx::Type::Builtin(witx::BuiltinType::F32) => {
                    add_param(&name, Abi::F32);
                }

                witx::Type::Builtin(witx::BuiltinType::F64) => {
                    add_param(&name, Abi::F64);
                }

                // strings/arrays have an extra ABI parameter for the length
                // of the array passed.
                witx::Type::Builtin(witx::BuiltinType::String) | witx::Type::Array(_) => {
                    add_param(&name, Abi::I32);
                    let len = format_ident!("{}_len", name);
                    add_param(&len, Abi::I32);
                }

                witx::Type::ConstPointer(_) | witx::Type::Handle(_) | witx::Type::Pointer(_) => {
                    add_param(&name, Abi::I32);
                }

                witx::Type::Struct(_) | witx::Type::Union(_) => panic!("unsupported argument type"),
            }
        }

        let mut results = func.results.iter();
        let mut ret_ty = quote! { () };
        let mut cvt_ret = quote! {};
        let mut returns = Vec::new();
        let mut handle_early_error = quote! { panic!("error: {:?}", e) };

        // The first result is returned bare right now...
        if let Some(ret) = results.next() {
            handle_early_error = quote! { return e.into() };
            match &*ret.tref.type_() {
                // Eventually we'll want to add support for more returned
                // types, but for now let's just conform to what `*.witx`
                // definitions currently use.
                witx::Type::Enum(e) => match e.repr {
                    witx::IntRepr::U16 => {
                        returns.push(quote! { types::I32 });
                        ret_ty = quote! { i32 };
                        cvt_ret = quote! { .into() }
                    }
                    other => panic!("unsupported ret enum repr {:?}", other),
                },
                other => panic!("unsupported first return {:?}", other),
            }
        }

        // ... and all remaining results are returned via out-poiners
        for result in results {
            let name = format_ident!("{}", result.name.as_str());
            params.push(quote! { types::I32 });
            shim_arg_decls.push(quote! { #name: i32 });
            hostcall_args.push(quote! { #name });
        }

        ctor_externs.push(quote! {
            let my_cx = cx.clone();
            let #name_ident = wasmtime::Func::wrap(
                store,
                move |caller: wasmtime::Caller<'_> #(,#shim_arg_decls)*| -> #ret_ty {
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
                        #target_path::#module_id::#name_ident(
                            &mut my_cx.borrow_mut(),
                            &mem,
                            #(#hostcall_args),*
                        ) #cvt_ret
                    }
                }
            );
        });
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

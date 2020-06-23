use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse_macro_input;
use wiggle_generate::Names;

mod config;

#[proc_macro]
pub fn define_struct_for_wiggle(args: TokenStream) -> TokenStream {
    let mut config = parse_macro_input!(args as config::Config);
    config.witx.make_paths_relative_to(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR env var"),
    );
    let doc = config.load_document();
    let names = Names::new(&config.ctx.name, quote!(wasmtime_wiggle));

    generate(&doc, &names).into()
}

enum Abi {
    I32,
    I64,
    F32,
    F64,
}

fn generate(doc: &witx::Document, names: &Names) -> TokenStream2 {
    let mut fields = Vec::new();
    let mut get_exports = Vec::new();
    let mut ctor_externs = Vec::new();
    let mut ctor_fields = Vec::new();
    let mut linker_add = Vec::new();

    for module in doc.modules() {
        let module_name = module.name.as_str();
        let module_id = Ident::new(module_name, Span::call_site());
        for func in module.funcs() {
            let name = func.name.as_str();
            let name_ident = Ident::new(name, Span::call_site());
            fields.push(quote! { pub #name_ident: wasmtime::Func });
            get_exports.push(quote! { #name => Some(&self.#name_ident) });
            ctor_fields.push(name_ident.clone());
            linker_add.push(quote! {
                linker.define(#module_name, #name, self.#name_ident.clone())?;
            });
            // `proc_exit` is special; it's essentially an unwinding primitive,
            // so we implement it in the runtime rather than use the implementation
            // in wasi-common.
            if name == "proc_exit" {
                ctor_externs.push(quote! {
                    let #name_ident = wasmtime::Func::wrap(store, crate::wasi_proc_exit);
                });
                continue;
            }

            let mut shim_arg_decls = Vec::new();
            let mut params = Vec::new();
            let mut formats = Vec::new();
            let mut format_args = Vec::new();
            let mut hostcall_args = Vec::new();

            for param in func.params.iter() {
                let name = names.func_param(&param.name);

                // Registers a new parameter to the shim we're making with the
                // given `name`, the `abi_ty` wasm type and `hex` defines
                // whether it's debug-printed in a hex format or not.
                //
                // This will register a whole bunch of things:
                //
                // * The cranelift type for the parameter
                // * Syntax to specify the actual function parameter
                // * How to log the parameter value in a call to `trace!`
                // * How to actually pass this argument to the host
                //   implementation, converting as necessary.
                let mut add_param = |name: &Ident, abi_ty: Abi, hex: bool| {
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
                    formats.push(format!("{}={}", name, if hex { "{:#x}" } else { "{}" },));
                    format_args.push(name.clone());
                    hostcall_args.push(quote! { #name as _ });
                };

                match &*param.tref.type_() {
                    witx::Type::Int(e) => match e.repr {
                        witx::IntRepr::U64 => add_param(&name, Abi::I64, false),
                        _ => add_param(&name, Abi::I32, false),
                    },

                    witx::Type::Enum(e) => match e.repr {
                        witx::IntRepr::U64 => add_param(&name, Abi::I64, false),
                        _ => add_param(&name, Abi::I32, false),
                    },

                    witx::Type::Flags(f) => match f.repr {
                        witx::IntRepr::U64 => add_param(&name, Abi::I64, true),
                        _ => add_param(&name, Abi::I32, true),
                    },

                    witx::Type::Builtin(witx::BuiltinType::Char8)
                    | witx::Type::Builtin(witx::BuiltinType::S8)
                    | witx::Type::Builtin(witx::BuiltinType::U8)
                    | witx::Type::Builtin(witx::BuiltinType::S16)
                    | witx::Type::Builtin(witx::BuiltinType::U16)
                    | witx::Type::Builtin(witx::BuiltinType::S32)
                    | witx::Type::Builtin(witx::BuiltinType::U32)
                    | witx::Type::Builtin(witx::BuiltinType::USize) => {
                        add_param(&name, Abi::I32, false);
                    }

                    witx::Type::Builtin(witx::BuiltinType::S64)
                    | witx::Type::Builtin(witx::BuiltinType::U64) => {
                        add_param(&name, Abi::I64, false);
                    }

                    witx::Type::Builtin(witx::BuiltinType::F32) => {
                        add_param(&name, Abi::F32, false);
                    }

                    witx::Type::Builtin(witx::BuiltinType::F64) => {
                        add_param(&name, Abi::F64, false);
                    }

                    // strings/arrays have an extra ABI parameter for the length
                    // of the array passed.
                    witx::Type::Builtin(witx::BuiltinType::String) | witx::Type::Array(_) => {
                        add_param(&name, Abi::I32, true);
                        let len = format_ident!("{}_len", name);
                        add_param(&len, Abi::I32, false);
                    }

                    witx::Type::ConstPointer(_)
                    | witx::Type::Handle(_)
                    | witx::Type::Pointer(_) => {
                        add_param(&name, Abi::I32, true);
                    }

                    witx::Type::Struct(_) | witx::Type::Union(_) => {
                        panic!("unsupported argument type")
                    }
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
                formats.push(format!("{}={{:#x}}", name));
                format_args.push(name.clone());
                hostcall_args.push(quote! { #name });
            }

            let format_str = format!("{}({})", name, formats.join(", "));
            ctor_externs.push(quote! {
                let my_cx = cx.clone();
                let #name_ident = wasmtime::Func::wrap(
                    store,
                    move |caller: wasmtime::Caller<'_> #(,#shim_arg_decls)*| -> #ret_ty {
                        log::trace!(
                            #format_str,
                            #(#format_args),*
                        );
                        unsafe {
                            let mem = match caller.get_export("memory") {
                                Some(wasmtime::Extern::Memory(m)) => m,
                                _ => {
                                    log::warn!("callee does not export a memory as \"memory\"");
                                    let e = wasi_common::wasi::Errno::Inval;
                                    #handle_early_error
                                }
                            };
                            // Wiggle does not expose any methods for
                            // functions to re-enter the WebAssembly module,
                            // or expose the memory via non-wiggle mechanisms.
                            // Therefore, creating a new BorrowChecker at the
                            // root of each function invocation is correct.
                            let bc = wasmtime_wiggle::BorrowChecker::new();
                            let mem = wasmtime_wiggle::WasmtimeGuestMemory::new( mem, bc );
                            wasi_common::wasi::#module_id::#name_ident(
                                &mut my_cx.borrow_mut(),
                                &mem,
                                #(#hostcall_args),*
                            ) #cvt_ret
                        }
                    }
                );
            });
        }
    }

    quote! {
        /// An instantiated instance of the wasi exports.
        ///
        /// This represents a wasi module which can be used to instantiate other
        /// wasm modules. This structure exports all that various fields of the
        /// wasi instance as fields which can be used to implement your own
        /// instantiation logic, if necessary. Additionally [`Wasi::get_export`]
        /// can be used to do name-based resolution.
        pub struct Wasi {
            #(#fields,)*
        }

        impl Wasi {
            /// Creates a new [`Wasi`] instance.
            ///
            /// External values are allocated into the `store` provided and
            /// configuration of the wasi instance itself should be all
            /// contained in the `cx` parameter.
            pub fn new(store: &wasmtime::Store, cx: WasiCtx) -> Wasi {
                let cx = std::rc::Rc::new(std::cell::RefCell::new(cx));
                #(#ctor_externs)*

                Wasi {
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

            /// Adds all wasi items to the specified `Linker`.
            pub fn add_to_linker(&self, linker: &mut wasmtime::Linker) -> anyhow::Result<()> {
                #(#linker_add)*
                Ok(())
            }
        }
    }
}

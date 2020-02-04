use crate::utils;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

enum Abi {
    I32,
    I64,
    F32,
    F64,
}

/// This is a single-use macro intended to be used in the `wasmtime-wasi` crate.
///
/// This macro will generate a structure, `Wasi`, which will create all the
/// functions necessary to bind wasi and hook everything up via the `wasmtime`
/// crate.
///
/// The generated shim functions here will also `trace!` their arguments for
/// logging purposes. Otherwise this is hopefully somewhat straightforward!
///
/// I'd recommend using `cargo +nightly expand` to explore the output of this
/// macro some more.
pub fn define_struct(args: TokenStream) -> TokenStream {
    let (path, _phase) = utils::witx_path_from_args(args);
    let doc = match witx::load(&[&path]) {
        Ok(doc) => doc,
        Err(e) => {
            panic!("error opening file {}: {}", path, e);
        }
    };

    let mut fields = Vec::new();
    let mut get_exports = Vec::new();
    let mut ctor_externs = Vec::new();
    let mut ctor_fields = Vec::new();

    for module in doc.modules() {
        for func in module.funcs() {
            let name = func.name.as_str();
            let name_ident = Ident::new(func.name.as_str(), Span::call_site());
            fields.push(quote! { pub #name_ident: wasmtime::Func });
            get_exports.push(quote! { #name => Some(&self.#name_ident) });
            ctor_fields.push(name_ident.clone());

            let mut shim_arg_decls = Vec::new();
            let mut params = Vec::new();
            let mut formats = Vec::new();
            let mut format_args = Vec::new();
            let mut hostcall_args = Vec::new();

            for param in func.params.iter() {
                let name = utils::param_name(param);

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
                hostcall_args.push(quote! { #name as u32 });
            }

            let format_str = format!("{}({})", name, formats.join(", "));
            let wrap = format_ident!("wrap{}", shim_arg_decls.len() + 1);
            ctor_externs.push(quote! {
                let my_cx = cx.clone();
                let #name_ident = wasmtime::Func::#wrap(
                    store,
                    move |mem: crate::WasiCallerMemory #(,#shim_arg_decls)*| -> #ret_ty {
                        log::trace!(
                            #format_str,
                            #(#format_args),*
                        );
                        unsafe {
                            let memory = match mem.get() {
                                Ok(e) => e,
                                Err(e) => #handle_early_error,
                            };
                            hostcalls::#name_ident(
                                &mut my_cx.borrow_mut(),
                                memory,
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
        }
    }
}

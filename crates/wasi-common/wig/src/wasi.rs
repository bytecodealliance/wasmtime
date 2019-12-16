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
/// This macro will generate a function, `add_wrappers_to_module`, which will
/// use the input arguments to register all wasi functions inside of a `Module`
/// instance. This will automatically assign the wasm underlying types and
/// perform conversions from the wasm type to the underlying wasi type (often
/// unsigned or of a smaller width).
///
/// The generated shim functions here will also `trace!` their arguments for
/// logging purposes. Otherwise this is hopefully somewhat straightforward!
///
/// I'd recommend using `cargo +nightly expand` to explore the output of this
/// macro some more.
pub fn add_wrappers_to_module(args: TokenStream) -> TokenStream {
    let (path, _phase) = utils::witx_path_from_args(args);
    let doc = match witx::load(&[&path]) {
        Ok(doc) => doc,
        Err(e) => {
            panic!("error opening file {}: {}", path, e);
        }
    };

    let mut add = Vec::new();

    for module in doc.modules() {
        for func in module.funcs() {
            let name = func.name.as_str();
            let name_ident = Ident::new(func.name.as_str(), Span::call_site());

            let mut shim_arg_decls = Vec::new();
            let mut params = Vec::new();
            let mut formats = Vec::new();
            let mut format_args = Vec::new();
            let mut hostcall_args = Vec::new();

            for param in func.params.iter() {
                let name = format_ident!(
                    "{}",
                    match param.name.as_str() {
                        "in" | "type" => format!("r#{}", param.name.as_str()),
                        s => s.to_string(),
                    }
                );

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
                    witx::Type::Enum(e) => match e.repr {
                        witx::IntRepr::U64 => add_param(&name, Abi::I64, false),
                        _ => add_param(&name, Abi::I32, false),
                    },

                    witx::Type::Flags(f) => match f.repr {
                        witx::IntRepr::U64 => add_param(&name, Abi::I64, true),
                        _ => add_param(&name, Abi::I32, true),
                    },

                    witx::Type::Builtin(witx::BuiltinType::S8)
                    | witx::Type::Builtin(witx::BuiltinType::U8)
                    | witx::Type::Builtin(witx::BuiltinType::S16)
                    | witx::Type::Builtin(witx::BuiltinType::U16)
                    | witx::Type::Builtin(witx::BuiltinType::S32)
                    | witx::Type::Builtin(witx::BuiltinType::U32) => {
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
            add.push(quote! {
                let sig = module.signatures.push(translate_signature(
                    ir::Signature {
                        params: vec![#(cranelift_codegen::ir::AbiParam::new(#params)),*],
                        returns: vec![#(cranelift_codegen::ir::AbiParam::new(#returns)),*],
                        call_conv,
                    },
                    pointer_type,
                ));
                let func = module.functions.push(sig);
                module
                    .exports
                    .insert(#name.to_owned(), Export::Function(func));

                unsafe extern "C" fn #name_ident(
                    ctx: *mut wasmtime_runtime::VMContext,
                    #(#shim_arg_decls),*
                ) -> #ret_ty {
                    log::trace!(
                        #format_str,
                        #(#format_args),*
                    );
                    let wasi_ctx = match get_wasi_ctx(&mut *ctx) {
                        Ok(e) => e,
                        Err(e) => #handle_early_error,
                    };
                    let memory = match get_memory(&mut *ctx) {
                        Ok(e) => e,
                        Err(e) => #handle_early_error,
                    };
                    wasi_common::hostcalls::#name_ident(
                        wasi_ctx,
                        memory,
                        #(#hostcall_args),*
                    ) #cvt_ret
                }
                finished_functions.push(#name_ident as *const _);
            });
        }
    }

    quote! {
        pub fn add_wrappers_to_module(
            module: &mut Module,
            finished_functions: &mut PrimaryMap<DefinedFuncIndex, *const wasmtime_runtime::VMFunctionBody>,
            call_conv: isa::CallConv,
            pointer_type: types::Type,
        ) {
            #(#add)*
        }
    }
}

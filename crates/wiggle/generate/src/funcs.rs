use proc_macro2::TokenStream;
use quote::quote;

use crate::config::LoggingConf;
use crate::error_transform::ErrorTransform;
use crate::lifetimes::anon_lifetime;
use crate::module_trait::passed_by_reference;
use crate::names::Names;

pub fn define_func(
    names: &Names,
    func: &witx::InterfaceFunc,
    trait_name: TokenStream,
    errxform: &ErrorTransform,
    logging: &LoggingConf,
) -> TokenStream {
    let funcname = func.name.as_str();

    let ident = names.func(&func.name);
    let rt = names.runtime_mod();
    let ctx_type = names.ctx_type();
    let coretype = func.core_type();

    let params = coretype.args.iter().map(|arg| {
        let name = names.func_core_arg(arg);
        let atom = names.atom_type(arg.repr());
        quote!(#name : #atom)
    });

    let abi_args = quote!(
            ctx: &#ctx_type,
            memory: &dyn #rt::GuestMemory,
            #(#params),*
    );
    let abi_ret = if let Some(ret) = &coretype.ret {
        match ret.signifies {
            witx::CoreParamSignifies::Value(atom) => names.atom_type(atom),
            _ => unreachable!("ret should always be passed by value"),
        }
    } else if func.noreturn {
        // Ideally we would return `quote!(!)` here, but, we'd have to change
        // the error handling logic in all the marshalling code to never return,
        // and instead provide some other way to bail to the context...
        // noreturn func
        unimplemented!("noreturn funcs not supported yet!")
    } else {
        quote!(())
    };

    let err_type = coretype.ret.clone().map(|ret| ret.param.tref);
    let ret_err = coretype
        .ret
        .map(|ret| {
            let name = ret.param.name.as_str();
            let conversion = if let Some(user_err) = errxform.for_abi_error(&ret.param.tref) {
                let method = names.user_error_conversion_method(&user_err);
                quote!(#abi_ret::from(UserErrorConversion::#method(ctx, e)))
            } else {
                quote!(#abi_ret::from(e))
            };
            quote! {
                #[cfg(feature = "trace_log")]
                {
                    log::trace!("     | {}={:?}", #name, e);
                }
                return #conversion;
            }
        })
        .unwrap_or_else(|| quote!(()));

    let error_handling = |location: &str| -> TokenStream {
        if let Some(tref) = &err_type {
            let abi_ret = match tref.type_().passed_by() {
                witx::TypePassedBy::Value(atom) => names.atom_type(atom),
                _ => unreachable!("err should always be passed by value"),
            };
            let err_typename = names.type_ref(&tref, anon_lifetime());
            let err_method = names.guest_error_conversion_method(&tref);
            quote! {
                let e = #rt::GuestError::InFunc { funcname: #funcname, location: #location, err: Box::new(e.into()) };
                let err: #err_typename = GuestErrorConversion::#err_method(ctx, e); // XXX replace with conversion method on trait!
                return #abi_ret::from(err);
            }
        } else {
            quote! {
                panic!("error: {:?}", e)
            }
        }
    };

    let marshal_args = func
        .params
        .iter()
        .map(|p| marshal_arg(names, p, error_handling(p.name.as_str())));
    let trait_args = func.params.iter().map(|param| {
        let name = names.func_param(&param.name);
        if passed_by_reference(&*param.tref.type_()) {
            quote!(&#name)
        } else {
            quote!(#name)
        }
    });

    let (trait_rets, trait_bindings) = if func.results.len() < 2 {
        (quote!({}), quote!(_))
    } else {
        let trait_rets: Vec<_> = func
            .results
            .iter()
            .skip(1)
            .map(|result| names.func_param(&result.name))
            .collect();
        let bindings = quote!((#(#trait_rets),*));
        let names: Vec<_> = func
            .results
            .iter()
            .skip(1)
            .map(|result| {
                let name = names.func_param(&result.name);
                let fmt = match &*result.tref.type_() {
                    witx::Type::Builtin(_)
                    | witx::Type::Enum(_)
                    | witx::Type::Flags(_)
                    | witx::Type::Handle(_)
                    | witx::Type::Int(_) => "{}",
                    _ => "{:?}",
                };
                format!("{}={}", name.to_string(), fmt)
            })
            .collect();
        let trace_fmt = format!("     | result=({})", names.join(","));
        let rets = quote! {
            #[cfg(feature = "trace_log")]
            {
                log::trace!(#trace_fmt, #(#trait_rets),*);
            }
            (#(#trait_rets),*)
        };
        (rets, bindings)
    };

    // Return value pointers need to be validated before the api call, then
    // assigned to afterwards. marshal_result returns these two statements as a pair.
    let marshal_rets = func
        .results
        .iter()
        .skip(1)
        .map(|result| marshal_result(names, result, &error_handling));
    let marshal_rets_pre = marshal_rets.clone().map(|(pre, _post)| pre);
    let marshal_rets_post = marshal_rets.map(|(_pre, post)| post);

    let success = if let Some(ref err_type) = err_type {
        let err_typename = names.type_ref(&err_type, anon_lifetime());
        quote! {
            let success:#err_typename = #rt::GuestErrorType::success();
            #[cfg(feature = "trace_log")]
            {
                log::trace!("     | errno={}", success);
            }
            #abi_ret::from(success)
        }
    } else {
        quote!()
    };

    let log_args = logging.args(&func, names);

    quote!(pub fn #ident(#abi_args) -> #abi_ret {
        #(#marshal_args)*
        #(#marshal_rets_pre)*
        #log_args
        let #trait_bindings  = match #trait_name::#ident(ctx, #(#trait_args),*) {
            Ok(#trait_bindings) => { #trait_rets },
            Err(e) => { #ret_err },
        };
        #(#marshal_rets_post)*
        #success
    })
}

fn marshal_arg(
    names: &Names,
    param: &witx::InterfaceFuncParam,
    error_handling: TokenStream,
) -> TokenStream {
    let rt = names.runtime_mod();
    let tref = &param.tref;
    let interface_typename = names.type_ref(&tref, anon_lifetime());

    let try_into_conversion = {
        let name = names.func_param(&param.name);
        quote! {
            let #name: #interface_typename = {
                use ::std::convert::TryInto;
                match #name.try_into() {
                    Ok(a) => a,
                    Err(e) => {
                        #error_handling
                    }
                }
            };
        }
    };

    let read_conversion = {
        let pointee_type = names.type_ref(tref, anon_lifetime());
        let arg_name = names.func_ptr_binding(&param.name);
        let name = names.func_param(&param.name);
        quote! {
            let #name = match #rt::GuestPtr::<#pointee_type>::new(memory, #arg_name as u32).read() {
                Ok(r) => r,
                Err(e) => {
                    #error_handling
                }
            };
        }
    };

    match &*tref.type_() {
        witx::Type::Enum(_e) => try_into_conversion,
        witx::Type::Flags(_f) => try_into_conversion,
        witx::Type::Int(_i) => try_into_conversion,
        witx::Type::Builtin(b) => match b {
            witx::BuiltinType::U8 | witx::BuiltinType::U16 | witx::BuiltinType::Char8 => {
                try_into_conversion
            }
            witx::BuiltinType::S8 | witx::BuiltinType::S16 => {
                let name = names.func_param(&param.name);
                quote! {
                    let #name: #interface_typename = match (#name as i32).try_into() {
                        Ok(a) => a,
                        Err(e) => {
                            #error_handling
                        }
                    }
                }
            }
            witx::BuiltinType::U32
            | witx::BuiltinType::S32
            | witx::BuiltinType::U64
            | witx::BuiltinType::S64
            | witx::BuiltinType::USize
            | witx::BuiltinType::F32
            | witx::BuiltinType::F64 => {
                let name = names.func_param(&param.name);
                quote! {
                    let #name = #name as #interface_typename;
                }
            }
            witx::BuiltinType::String => {
                let lifetime = anon_lifetime();
                let ptr_name = names.func_ptr_binding(&param.name);
                let len_name = names.func_len_binding(&param.name);
                let name = names.func_param(&param.name);
                quote! {
                    let #name = #rt::GuestPtr::<#lifetime, str>::new(memory, (#ptr_name as u32, #len_name as u32));
                }
            }
        },
        witx::Type::Pointer(pointee) | witx::Type::ConstPointer(pointee) => {
            let pointee_type = names.type_ref(pointee, anon_lifetime());
            let name = names.func_param(&param.name);
            quote! {
                let #name = #rt::GuestPtr::<#pointee_type>::new(memory, #name as u32);
            }
        }
        witx::Type::Struct(_) => read_conversion,
        witx::Type::Array(arr) => {
            let pointee_type = names.type_ref(arr, anon_lifetime());
            let ptr_name = names.func_ptr_binding(&param.name);
            let len_name = names.func_len_binding(&param.name);
            let name = names.func_param(&param.name);
            quote! {
                let #name = #rt::GuestPtr::<[#pointee_type]>::new(memory, (#ptr_name as u32, #len_name as u32));
            }
        }
        witx::Type::Union(_u) => read_conversion,
        witx::Type::Handle(_h) => {
            let name = names.func_param(&param.name);
            let handle_type = names.type_ref(tref, anon_lifetime());
            quote!( let #name = #handle_type::from(#name); )
        }
    }
}

fn marshal_result<F>(
    names: &Names,
    result: &witx::InterfaceFuncParam,
    error_handling: F,
) -> (TokenStream, TokenStream)
where
    F: Fn(&str) -> TokenStream,
{
    let rt = names.runtime_mod();
    let tref = &result.tref;

    let write_val_to_ptr = {
        let pointee_type = names.type_ref(tref, anon_lifetime());
        // core type is given func_ptr_binding name.
        let ptr_name = names.func_ptr_binding(&result.name);
        let ptr_err_handling = error_handling(&format!("{}:result_ptr_mut", result.name.as_str()));
        let pre = quote! {
            let #ptr_name = #rt::GuestPtr::<#pointee_type>::new(memory, #ptr_name as u32);
        };
        // trait binding returns func_param name.
        let val_name = names.func_param(&result.name);
        let post = quote! {
            if let Err(e) = #ptr_name.write(#val_name) {
                #ptr_err_handling
            }
        };
        (pre, post)
    };

    match &*tref.type_() {
        witx::Type::Builtin(b) => match b {
            witx::BuiltinType::String => unimplemented!("string result types"),
            _ => write_val_to_ptr,
        },
        witx::Type::Pointer { .. } | witx::Type::ConstPointer { .. } | witx::Type::Array { .. } => {
            unimplemented!("pointer/array result types")
        }
        _ => write_val_to_ptr,
    }
}

impl LoggingConf {
    fn args(&self, func: &witx::InterfaceFunc, names: &Names) -> TokenStream {
        match self {
            Self::Log { cfg_feature } => {
                let (placeholders, args): (Vec<_>, Vec<_>) = func
                    .params
                    .iter()
                    .map(|param| {
                        let name = names.func_param(&param.name);
                        let fmt = if passed_by_reference(&*param.tref.type_()) {
                            "{:?}"
                        } else {
                            "{}"
                        };
                        (format!("{}={}", name.to_string(), fmt), quote!(#name))
                    })
                    .unzip();
                let trace_fmt = format!(
                    "{}({})",
                    names.func(&func.name).to_string(),
                    placeholders.join(",")
                );
                let trace_stmt = quote!(log::trace!(#trace_fmt, #(#args),*););
                if let Some(feature) = cfg_feature {
                    quote! {
                        #[cfg(feature = #feature)]
                        {
                            #trace_stmt
                        }
                    }
                } else {
                    trace_stmt
                }
            }
            Self::Tracing => {
                let args = func.params.iter().map(|param| {
                    let name = names.func_param(&param.name);
                    quote!( #name = #name )
                });
                let func_name = names.func(&func.name).to_string();
                quote! {
                    tracing::debug!(function = #func_name, #(#args),*, "marshalled arguments");
                }
            }
        }
    }
}

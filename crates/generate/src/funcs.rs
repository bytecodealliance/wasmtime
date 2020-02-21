use proc_macro2::TokenStream;
use quote::quote;

use crate::names::Names;
use crate::types::{anon_lifetime, struct_is_copy};

pub fn define_func(names: &Names, func: &witx::InterfaceFunc) -> TokenStream {
    let funcname = func.name.as_str();

    let ident = names.func(&func.name);
    let ctx_type = names.ctx_type();
    let coretype = func.core_type();

    let params = coretype.args.iter().map(|arg| match arg.signifies {
        witx::CoreParamSignifies::Value(atom) => {
            let atom = names.atom_type(atom);
            let name = names.func_param(&arg.param.name);
            quote!(#name : #atom)
        }
        witx::CoreParamSignifies::PointerTo => {
            let atom = names.atom_type(witx::AtomType::I32);
            let name = names.func_ptr_binding(&arg.param.name);
            quote!(#name: #atom)
        }
        witx::CoreParamSignifies::LengthOf => {
            let atom = names.atom_type(witx::AtomType::I32);
            let name = names.func_len_binding(&arg.param.name);
            quote!(#name: #atom)
        }
    });

    let abi_args = quote!(
            ctx: &mut #ctx_type, memory: &mut wiggle_runtime::GuestMemory,
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

    let err_type = coretype.ret.map(|ret| ret.param.tref);
    let err_val = err_type
        .clone()
        .map(|_res| quote!(#abi_ret::from(e)))
        .unwrap_or_else(|| quote!(()));

    let error_handling = |location: &str| -> TokenStream {
        if let Some(tref) = &err_type {
            let abi_ret = match tref.type_().passed_by() {
                witx::TypePassedBy::Value(atom) => names.atom_type(atom),
                _ => unreachable!("err should always be passed by value"),
            };
            let err_typename = names.type_ref(&tref, anon_lifetime());
            quote! {
                let e = wiggle_runtime::GuestError::InFunc { funcname: #funcname, location: #location, err: Box::new(e) };
                let err: #err_typename = wiggle_runtime::GuestErrorType::from_error(e, ctx);
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
        match param.tref.type_().passed_by() {
            witx::TypePassedBy::Value { .. } => quote!(#name),
            witx::TypePassedBy::Pointer { .. } => quote!(&#name),
            witx::TypePassedBy::PointerLengthPair { .. } => quote!(&#name),
        }
    });

    let (trait_rets, trait_bindings) = if func.results.len() < 2 {
        (quote!({}), quote!(_))
    } else {
        let trait_rets = func
            .results
            .iter()
            .skip(1)
            .map(|result| names.func_param(&result.name));
        let tuple = quote!((#(#trait_rets),*));
        (tuple.clone(), tuple)
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
            let success:#err_typename = wiggle_runtime::GuestErrorType::success();
            #abi_ret::from(success)
        }
    } else {
        quote!()
    };

    quote!(pub fn #ident(#abi_args) -> #abi_ret {
        #(#marshal_args)*
        #(#marshal_rets_pre)*
        let #trait_bindings  = match ctx.#ident(#(#trait_args),*) {
            Ok(#trait_bindings) => #trait_rets,
            Err(e) => { return #err_val; },
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
    let tref = &param.tref;
    let interface_typename = names.type_ref(&tref, anon_lifetime());

    let try_into_conversion = {
        let name = names.func_param(&param.name);
        quote! {
            use ::std::convert::TryInto;
            let #name: #interface_typename = match #name.try_into() {
                Ok(a) => a,
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
                    let num_elems = match memory.ptr::<u32>(#len_name as u32) {
                        Ok(p) => match p.as_ref() {
                            Ok(r) => r,
                            Err(e) => {
                                #error_handling
                            }
                        }
                        Err(e) => {
                            #error_handling
                        }
                    };
                    let #name: wiggle_runtime::GuestString<#lifetime> = match memory.ptr::<u8>(#ptr_name as u32) {
                        Ok(p) => match p.array(*num_elems) {
                            Ok(s) => s.into(),
                            Err(e) => {
                                #error_handling
                            }
                        }
                        Err(e) => {
                            #error_handling
                        }
                    };
                }
            }
        },
        witx::Type::Pointer(pointee) => {
            let pointee_type = names.type_ref(pointee, anon_lifetime());
            let name = names.func_param(&param.name);
            quote! {
                let #name = match memory.ptr_mut::<#pointee_type>(#name as u32) {
                    Ok(p) => p,
                    Err(e) => {
                        #error_handling
                    }
                };
            }
        }
        witx::Type::ConstPointer(pointee) => {
            let pointee_type = names.type_ref(pointee, anon_lifetime());
            let name = names.func_param(&param.name);
            quote! {
                let #name = match memory.ptr::<#pointee_type>(#name as u32) {
                    Ok(p) => p,
                    Err(e) => {
                        #error_handling
                    }
                };
            }
        }
        witx::Type::Struct(s) if struct_is_copy(&s) => {
            let pointee_type = names.type_ref(tref, anon_lifetime());
            let arg_name = names.func_ptr_binding(&param.name);
            let name = names.func_param(&param.name);
            quote! {
                let #name = match memory.ptr::<#pointee_type>(#arg_name as u32) {
                    Ok(p) => match p.as_ref() {
                        Ok(r) => r,
                        Err(e) => {
                            #error_handling
                        }
                    },
                    Err(e) => {
                        #error_handling
                    }
                };
            }
        }
        witx::Type::Struct(s) if !struct_is_copy(&s) => {
            let pointee_type = names.type_ref(tref, anon_lifetime());
            let arg_name = names.func_ptr_binding(&param.name);
            let name = names.func_param(&param.name);
            quote! {
                let #name = match memory.ptr_mut::<#pointee_type>(#arg_name as u32) {
                    Ok(p) => match p.read_ptr_from_guest() {
                        Ok(r) => r,
                        Err(e) => {
                            #error_handling
                        }
                    },
                    Err(e) => {
                        #error_handling
                    }
                };
            }
        }
        witx::Type::Array(arr) => {
            let pointee_type = names.type_ref(arr, anon_lifetime());
            let ptr_name = names.func_ptr_binding(&param.name);
            let len_name = names.func_len_binding(&param.name);
            let name = names.func_param(&param.name);
            quote! {
                let num_elems = match memory.ptr::<u32>(#len_name as u32) {
                    Ok(p) => match p.as_ref() {
                        Ok(r) => r,
                        Err(e) => {
                            #error_handling
                        }
                    }
                    Err(e) => {
                        #error_handling
                    }
                };
                let #name = match memory.ptr::<#pointee_type>(#ptr_name as u32) {
                    Ok(p) => match p.array(*num_elems) {
                        Ok(s) => s,
                        Err(e) => {
                            #error_handling
                        }
                    }
                    Err(e) => {
                        #error_handling
                    }
                };
            }
        }
        _ => unimplemented!("argument type marshalling"),
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
    let tref = &result.tref;

    let write_val_to_ptr = {
        let pointee_type = names.type_ref(tref, anon_lifetime());
        // core type is given func_ptr_binding name.
        let ptr_name = names.func_ptr_binding(&result.name);
        let ptr_err_handling = error_handling(&format!("{}:result_ptr_mut", result.name.as_str()));
        let ref_err_handling = error_handling(&format!("{}:result_ref_mut", result.name.as_str()));
        let pre = quote! {
            let mut #ptr_name = match memory.ptr_mut::<#pointee_type>(#ptr_name as u32) {
                Ok(p) => match p.as_ref_mut() {
                    Ok(r) => r,
                    Err(e) => {
                        #ref_err_handling
                    }
                },
                Err(e) => {
                    #ptr_err_handling
                }
            };
        };
        // trait binding returns func_param name.
        let val_name = names.func_param(&result.name);
        let post = quote! {
            *#ptr_name = #val_name;
        };
        (pre, post)
    };

    match &*tref.type_() {
        witx::Type::Builtin(b) => match b {
            witx::BuiltinType::U8
            | witx::BuiltinType::S8
            | witx::BuiltinType::U16
            | witx::BuiltinType::S16
            | witx::BuiltinType::U32
            | witx::BuiltinType::S32
            | witx::BuiltinType::U64
            | witx::BuiltinType::S64
            | witx::BuiltinType::F32
            | witx::BuiltinType::F64
            | witx::BuiltinType::USize
            | witx::BuiltinType::Char8 => write_val_to_ptr,
            witx::BuiltinType::String => unimplemented!("string types"),
        },
        witx::Type::Enum(_) | witx::Type::Flags(_) | witx::Type::Int(_) => write_val_to_ptr,
        _ => unimplemented!("marshal result"),
    }
}

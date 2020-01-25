use proc_macro2::TokenStream;
use quote::quote;

use crate::names::Names;

// FIXME need to template what argument is required to an import function - some context
// struct (e.g. WasiCtx) should be provided at the invocation of the `gen` proc macro.
//
pub fn define_func(names: &Names, func: &witx::InterfaceFunc) -> TokenStream {
    let ident = names.func(&func.name);

    let arg_signature = |param: &witx::InterfaceFuncParam| -> TokenStream {
        let name = names.func_param(&param.name);
        match param.tref.type_().passed_by() {
            witx::TypePassedBy::Value(atom) => {
                let atom = names.atom_type(atom);
                quote!(#name: #atom)
            }
            witx::TypePassedBy::Pointer => {
                let atom = names.atom_type(witx::AtomType::I32);
                quote!(#name: #atom)
            }
            witx::TypePassedBy::PointerLengthPair => {
                let atom = names.atom_type(witx::AtomType::I32);
                let len_name = names.func_len_param(&param.name);
                quote!(#name: #atom, #len_name: #atom)
            }
        }
    };

    let params = func
        .params
        .iter()
        .chain(func.results.iter().skip(1))
        .map(arg_signature);
    let abi_args = quote!(
            ctx: &mut WasiCtx, memory: ::memory::GuestMemory,
            #(#params),*
    );
    let abi_ret = if let Some(first_result) = func.results.get(0) {
        match first_result.tref.type_().passed_by() {
            witx::TypePassedBy::Value(atom) => names.atom_type(atom),
            _ => unreachable!("first result should always be passed by value"),
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

    let err_type = func
        .results
        .get(0)
        .map(|res| names.type_ref(&res.tref))
        .unwrap_or_else(|| abi_ret.clone());
    let err_val = func
        .results
        .get(0)
        .map(|_res| quote!(#abi_ret::from(e)))
        .unwrap_or_else(|| quote!(()));

    let marshal_args = func
        .params
        .iter()
        .map(|p| marshal_arg(names, p, func.results.get(0).map(|r| &r.tref)));
    let trait_args = func
        .params
        .iter()
        .map(|param| names.func_param(&param.name));

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

    let marshal_rets = func
        .results
        .iter()
        .skip(1)
        .map(|_result| quote! { unimplemented!("convert result..."); });

    quote!(pub fn #ident(#abi_args) -> #abi_ret {
        #(#marshal_args)*
        let #trait_bindings  = match ctx.#ident(#(#trait_args),*) {
            Ok(#trait_bindings) => #trait_rets,
            Err(e) => { return #err_val; },
        };
        #(#marshal_rets)*
        let success:#err_type = ::memory::GuestErrorType::success();
        #abi_ret::from(success)
    })
}

fn marshal_arg(
    names: &Names,
    param: &witx::InterfaceFuncParam,
    error_type: Option<&witx::TypeRef>,
) -> TokenStream {
    let tref = &param.tref;
    let interface_typename = names.type_ref(&tref);
    let name = names.func_param(&param.name);

    let error_handling: TokenStream = {
        if let Some(tref) = error_type {
            let abi_ret = match tref.type_().passed_by() {
                witx::TypePassedBy::Value(atom) => names.atom_type(atom),
                _ => unreachable!("err should always be passed by value"),
            };
            let err_typename = names.type_ref(&tref);
            quote! {
                let err: #err_typename = ::memory::GuestErrorType::from_error(e, ctx);
                return #abi_ret::from(err);
            }
        } else {
            quote! {
                panic!("error: {:?}", e)
            }
        }
    };

    let try_into_conversion = quote! {
        use ::std::convert::TryInto;
        let #name: #interface_typename = match #name.try_into() {
            Ok(a) => a,
            Err(e) => {
                #error_handling
            }
        };
    };

    match &*tref.type_() {
        witx::Type::Enum(_e) => try_into_conversion,
        witx::Type::Builtin(b) => match b {
            witx::BuiltinType::U8 | witx::BuiltinType::U16 | witx::BuiltinType::Char8 => {
                try_into_conversion
            }
            witx::BuiltinType::S8 | witx::BuiltinType::S16 => quote! {
                let #name: #interface_typename = match (#name as i32).try_into() {
                    Ok(a) => a,
                    Err(e) => {
                        #error_handling
                    }
                }
            },
            witx::BuiltinType::U32
            | witx::BuiltinType::S32
            | witx::BuiltinType::U64
            | witx::BuiltinType::S64
            | witx::BuiltinType::USize
            | witx::BuiltinType::F32
            | witx::BuiltinType::F64 => quote! {
                let #name = #name as #interface_typename;
            },
            witx::BuiltinType::String => unimplemented!("string types unimplemented"),
        },
        witx::Type::Pointer(pointee) => {
            let pointee_type = names.type_ref(pointee);
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
            let pointee_type = names.type_ref(pointee);
            quote! {
                let #name = match memory.ptr::<#pointee_type>(#name as u32) {
                    Ok(p) => p,
                    Err(e) => {
                        #error_handling
                    }
                };
            }
        }
        _ => unimplemented!("argument type marshalling"),
    }
}

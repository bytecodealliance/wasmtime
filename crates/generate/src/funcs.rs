use proc_macro2::{Ident, TokenStream};
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
        .map(|param| match param.tref.type_().passed_by() {
            witx::TypePassedBy::Value(_atom) => {
                // FIXME atom -> param.tref can be either an `as` conversion, or `try_from`
                let name = names.func_param(&param.name);
                let interface_type = names.type_ref(&param.tref);
                quote!( let #name = #name as #interface_type; )
            }
            _ => unimplemented!(),
        });
    let trait_args = func
        .params
        .iter()
        .map(|param| names.func_param(&param.name));

    let trait_rets = func
        .results
        .iter()
        .skip(1)
        .map(|result| names.func_param(&result.name))
        .collect::<Vec<Ident>>();
    let (trait_rets, trait_bindings) = if trait_rets.is_empty() {
        (quote!({}), quote!(_))
    } else {
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
        let success:#err_type = ::memory::GuestError::success();
        #abi_ret::from(success)
    })
}

use proc_macro2::TokenStream;
use quote::quote;

use crate::names::Names;

// FIXME need to template what argument is required to an import function - some context
// struct (e.g. WasiCtx) should be provided at the invocation of the `gen` proc macro.
//
// Additionally - need to template how to transform GuestValueError and MemoryError into
// the error type returned! From impl is a good start, but what if we want to log
// a more informative error? Maybe the conversion should be a (generated, for each by-value first
// return type) trait impled by WasiCtx?
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
            wasi_ctx: &mut WasiCtx, memory: GuestMemory,
            #(#params),*
    );
    let abi_ret = if let Some(first_result) = func.results.get(0) {
        match first_result.tref.type_().passed_by() {
            witx::TypePassedBy::Value(atom) => names.atom_type(atom),
            _ => unreachable!("first result should always be passed by value"),
        }
    } else if func.noreturn {
        quote!(!)
    } else {
        quote!(())
    };

    quote!(pub fn #ident(#abi_args) -> #abi_ret { unimplemented!() })
}

use proc_macro2::TokenStream;
use quote::quote;

use crate::names::Names;

// FIXME need to template what arguments are required to an import function - some context
// struct (e.g. WasiCtx) should be provided at the invocation of the `gen` proc macro.
// Rather than pass in memory as a `&mut [u8]` as today, require a `GuestMemory<'a>` be
// passed in.
pub fn define_func(names: &Names, func: &witx::InterfaceFunc) -> TokenStream {
    let ident = names.func(&func.name);
    let mut args = quote!(wasi_ctx: &mut WasiCtx, memory: GuestMemory,);

    let arg_signature = |param: &witx::InterfaceFuncParam| -> TokenStream {
        let name = names.func_param(&param.name);
        match param.tref.type_().passed_by() {
            witx::TypePassedBy::Value(atom) => {
                let atom = names.atom_type(atom);
                quote!(#name: #atom,)
            }
            witx::TypePassedBy::Pointer => {
                let atom = names.atom_type(witx::AtomType::I32);
                quote!(#name: #atom,)
            }
            witx::TypePassedBy::PointerLengthPair => {
                let atom = names.atom_type(witx::AtomType::I32);
                let len_name = names.func_len_param(&param.name);
                quote!(#name: #atom, #len_name, #atom,)
            }
        }
    };

    for param in func.params.iter() {
        args.extend(arg_signature(param));
    }

    if let Some(arg_results) = func.results.get(1..) {
        for result in arg_results {
            args.extend(arg_signature(result))
        }
    }

    let ret = if let Some(first_result) = func.results.get(0) {
        match first_result.tref.type_().passed_by() {
            witx::TypePassedBy::Value(atom) => names.atom_type(atom),
            _ => unreachable!("first result should always be passed by value"),
        }
    } else if func.noreturn {
        quote!(!)
    } else {
        quote!(())
    };

    quote!(pub fn #ident(#args) -> #ret { unimplemented!() })
}

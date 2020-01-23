use crate::names::Names;
use heck::SnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use witx::{Document, TypeRef};

/// The context struct needs to implement a trait for converting memory and value errors into the
/// witx doc's error types.
///
// XXX im rethinking this. maybe each error type should impl
// pub trait WitxErrorType {
//   type Context;
//   fn is_success(&self) -> bool;
//   fn from_memory_error(memory_error: MemoryError, ctx: &mut Context) -> Self;
//   fn from_value_error(value_error: GuestValueError, ctx: &mut Context) -> Self;
// }
//
// where Context is mapped to their wasi ctx.
// It seems less "magic" to leave that impl up to the user, and the error message may be simpler?
//
pub fn define_error_trait(names: &Names, doc: &Document) -> TokenStream {
    // All non-anonymous first return types are used to pass errors.
    let error_typenames = doc
        .modules()
        .flat_map(|m| {
            m.funcs()
                .filter_map(|f| {
                    f.results.get(0).and_then(|r| match &r.tref {
                        TypeRef::Name(nt) => Some(nt.name.clone()),
                        _ => None,
                    })
                })
                .collect::<HashSet<witx::Id>>()
        })
        .collect::<HashSet<witx::Id>>();

    let methods = error_typenames.iter().map(|typename| {
        let tname = names.type_(typename);
        let methodfragment = typename.as_str().to_snake_case();
        let success = format_ident!("success_to_{}", methodfragment);
        let memory_error = format_ident!("memory_error_to_{}", methodfragment);
        let value_error = format_ident!("value_error_to_{}", methodfragment);

        quote! {
            fn #success(&mut self) -> #tname;
            fn #memory_error(&mut self, err: ::memory::MemoryError) -> #tname;
            fn #value_error(&mut self, err: ::memory::GuestValueError) -> #tname;
        }
    });

    quote!(
        pub trait WitxErrorConversion {
            #(#methods)*
        }
    )
}

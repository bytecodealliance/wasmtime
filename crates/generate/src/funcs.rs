use proc_macro2::TokenStream;
use quote::quote;

use crate::names::Names;

pub fn define_func(names: &Names, func: &witx::InterfaceFunc) -> TokenStream {
    let ident = names.func(&func.name);
    let mut args = TokenStream::new();

    for param in func.params.iter() {
        let name = names.func_param(&param.name);
        let type_ = names.type_ref(&param.tref);
        args.extend(quote!(#name: #type_,));
    }

    let mut rets = TokenStream::new();
    for result in func.results.iter() {
        let type_ = names.type_ref(&result.tref);
        rets.extend(quote!(#type_,));
    }

    quote!(pub fn #ident(#args) -> (#rets) { unimplemented!() })
}

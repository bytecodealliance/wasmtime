use proc_macro2::TokenStream;
use quote::quote;

use crate::names::Names;
use witx::Module;

pub fn define_module_trait(names: &Names, m: &Module) -> TokenStream {
    let traitname = names.trait_name(&m.name);
    let traitmethods = m.funcs().map(|f| {
        let funcname = names.func(&f.name);
        let args = f.params.iter().map(|arg| {
            let arg_name = names.func_param(&arg.name);
            let arg_type = names.type_ref(&arg.tref);
            quote!(#arg_name: #arg_type)
        });
        let rets = f
            .results
            .iter()
            .skip(1)
            .map(|ret| names.type_ref(&ret.tref));
        let err = f
            .results
            .get(0)
            .map(|err_result| names.type_ref(&err_result.tref))
            .unwrap_or(quote!(()));
        quote!(fn #funcname(&mut self, #(#args),*) -> Result<(#(#rets),*), #err>;)
    });
    quote! {
        pub trait #traitname {
            #(#traitmethods)*
        }
    }
}

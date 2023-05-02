use proc_macro::TokenStream;
use quote::quote;

include!(concat!(env!("OUT_DIR"), "/components.rs"));

fn compiled_components(components: &[(&str, &str)]) -> TokenStream {
    let globals = components.iter().map(|(stem, file)| {
        let global = quote::format_ident!("{}_COMPONENT", stem.to_uppercase());
        quote! {
            lazy_static::lazy_static! {
                static ref #global: wasmtime::component::Component = {
                    wasmtime::component::Component::from_file(&ENGINE, #file).unwrap()
                };
            }
        }
    });

    let cases = components.iter().map(|(stem, _file)| {
        let global = quote::format_ident!("{}_COMPONENT", stem.to_uppercase());
        quote! {
            #stem => #global.clone()
        }
    });
    quote! {
        #(#globals)*
        fn get_component(s: &str) -> wasmtime::component::Component {
            match s {
                #(#cases),*,
                _ => panic!("no such component: {}", s),
            }
        }
    }
    .into()
}

#[proc_macro]
pub fn command_components(_input: TokenStream) -> TokenStream {
    compiled_components(COMMAND_COMPONENTS)
}

#[proc_macro]
pub fn reactor_components(_input: TokenStream) -> TokenStream {
    compiled_components(REACTOR_COMPONENTS)
}

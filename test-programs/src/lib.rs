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

fn compiled_modules(modules: &[(&str, &str)]) -> TokenStream {
    let globals = modules.iter().map(|(stem, file)| {
        let global = quote::format_ident!("{}_MODULE", stem.to_uppercase());
        quote! {
            lazy_static::lazy_static! {
                static ref #global: wasmtime::Module = {
                    wasmtime::Module::from_file(&ENGINE, #file).unwrap()
                };
            }
        }
    });

    let cases = modules.iter().map(|(stem, _file)| {
        let global = quote::format_ident!("{}_MODULE", stem.to_uppercase());
        quote! {
            #stem => #global.clone()
        }
    });
    quote! {
        #(#globals)*
        fn get_module(s: &str) -> wasmtime::Module {
            match s {
                #(#cases),*,
                _ => panic!("no such component: {}", s),
            }
        }
    }
    .into()
}

#[proc_macro]
pub fn command_tests_components(_input: TokenStream) -> TokenStream {
    compiled_components(COMMAND_TESTS_COMPONENTS)
}

#[proc_macro]
pub fn wasi_tests_modules(_input: TokenStream) -> TokenStream {
    compiled_modules(WASI_TESTS_MODULES)
}

#[proc_macro]
pub fn wasi_tests_components(_input: TokenStream) -> TokenStream {
    compiled_components(WASI_TESTS_COMPONENTS)
}

#[proc_macro]
pub fn reactor_tests_components(_input: TokenStream) -> TokenStream {
    compiled_components(REACTOR_TESTS_COMPONENTS)
}

//! > **⚠️ Warning ⚠️**: this crate is an internal-only crate for the Wasmtime
//! > project and is not intended for general use. APIs are not strictly
//! > reviewed for safety and usage outside of Wasmtime may have bugs. If
//! > you're interested in using this feel free to file an issue on the
//! > Wasmtime repository to start a discussion about doing so, but otherwise
//! > be aware that your usage of this crate is not supported.

use syn::{DeriveInput, Error, parse_macro_input};

mod bindgen;
mod component;
mod host_heap_usage;

#[proc_macro_derive(HostHeapUsage, attributes(component))]
pub fn host_heap_usage(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    host_heap_usage::expand_host_heap_usage(&parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(FixedHostHeapUsage, attributes(component))]
pub fn fixed_host_heap_usage(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    host_heap_usage::expand_fixed_host_heap_usage(&parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Lift, attributes(component))]
pub fn lift(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    component::expand(
        &component::LiftExpander,
        &parse_macro_input!(input as DeriveInput),
    )
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

#[proc_macro_derive(Lower, attributes(component))]
pub fn lower(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    component::expand(
        &component::LowerExpander,
        &parse_macro_input!(input as DeriveInput),
    )
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

#[proc_macro_derive(ComponentType, attributes(component))]
pub fn component_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    component::expand(
        &component::ComponentTypeExpander,
        &parse_macro_input!(input as DeriveInput),
    )
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

#[proc_macro]
pub fn flags(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    component::expand_flags(&parse_macro_input!(input as component::Flags))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn bindgen(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bindgen::expand(&mut parse_macro_input!(input as bindgen::Config))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

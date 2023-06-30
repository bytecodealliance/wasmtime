//! This crate defines macros to easily define and use functions with a
//! versioned suffix, to facilitate using multiple versions of the same
//! crate that generate assembly.

use quote::ToTokens;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn version(value: impl std::fmt::Display) -> String {
    format!("{}_{}", value, VERSION.replace('.', "_"))
}

#[proc_macro_attribute]
pub fn versioned_export(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut function = syn::parse_macro_input!(item as syn::ItemFn);

    let export_name = syn::LitStr::new(
        version(&function.sig.ident).as_str(),
        proc_macro2::Span::call_site(),
    );
    function
        .attrs
        .push(syn::parse_quote! { #[export_name = #export_name] });

    function.to_token_stream().into()
}

#[proc_macro_attribute]
pub fn versioned_link(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut function = syn::parse_macro_input!(item as syn::ForeignItemFn);

    let link_name = syn::LitStr::new(
        version(&function.sig.ident).as_str(),
        proc_macro2::Span::call_site(),
    );
    function
        .attrs
        .push(syn::parse_quote! { #[link_name = #link_name] });

    function.to_token_stream().into()
}

#[proc_macro]
pub fn versioned_stringify_ident(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = syn::parse_macro_input!(item as syn::Ident);

    syn::LitStr::new(version(ident).as_str(), proc_macro2::Span::call_site())
        .to_token_stream()
        .into()
}

#[proc_macro]
pub fn versioned_suffix(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    if !item.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "`versioned_suffix!` accepts no input",
        )
        .to_compile_error()
        .into();
    };

    syn::LitStr::new(version("").as_str(), proc_macro2::Span::call_site())
        .to_token_stream()
        .into()
}

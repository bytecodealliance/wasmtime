//! This crate defines macros to easily define and use functions with a
//! versioned suffix, to facilitate using multiple versions of the same
//! crate that generate assembly.
//!
//! This crate contains a `enabled` feature making it easy to expose
//! use this versioned name logic conditionally with a feature.

#[cfg(feature = "enabled")]
use quote::ToTokens;

#[cfg(feature = "enabled")]
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn version(value: impl std::fmt::Display) -> String {
    format!("{}_{}", value, VERSION.replace('.', "_"))
}

#[cfg(feature = "enabled")]
struct VersionedExportInput {
    aliases: Vec<syn::Ident>,
}

#[cfg(feature = "enabled")]
impl syn::parse::Parse for VersionedExportInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(aliases);

        let aliases = if input.is_empty() {
            Vec::new()
        } else {
            input.parse::<aliases>()?;
            input.parse::<syn::Token![=]>()?;
            let content;
            syn::bracketed!(content in input);
            syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated(&content)?
                .into_iter()
                .collect()
        };

        Ok(Self { aliases })
    }
}

#[proc_macro_attribute]
pub fn versioned_export(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut function = syn::parse_macro_input!(item as syn::ItemFn);
    let input = syn::parse_macro_input!(attr as VersionedExportInput);

    let export_name = syn::LitStr::new(
        version(&function.sig.ident).as_str(),
        proc_macro2::Span::call_site(),
    );
    function
        .attrs
        .push(syn::parse_quote! { #[export_name = #export_name] });

    let mut stream = function.to_token_stream();

    input.aliases.iter().for_each(|alias| {
        let attrs: Option<syn::Attribute> = if cfg!(feature = "enabled") {
            let export_name =
                syn::LitStr::new(version(alias).as_str(), proc_macro2::Span::call_site());
            Some(syn::parse_quote! { #[export_name = #export_name] })
        } else {
            None
        };

        let vis = &function.vis;
        let unsafety = &function.sig.unsafety;
        let inputs = &function.sig.inputs;
        let output = &function.sig.output;
        let function_ident = &function.sig.ident;

        // it would be much easier to simply do `use #function_ident as #alias`, but the
        // #[export_name = ...] attribute cannot be used there so there is not way to
        // apply versioning to the exported symbol
        let item_static: syn::ItemStatic = syn::parse_quote! {
            #[allow(non_upper_case_globals)]
            #attrs
            #vis static #alias: #unsafety fn(#inputs) #output = #function_ident;
        };

        stream.extend(item_static.to_token_stream());
    });

    stream.into()
}

#[cfg(feature = "enabled")]
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

#[cfg(not(feature = "enabled"))]
#[proc_macro_attribute]
pub fn versioned_link(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    item
}

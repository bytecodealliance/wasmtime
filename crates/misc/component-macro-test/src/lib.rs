use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn add_variants(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    expand_variants(
        &parse_macro_input!(attr as syn::LitInt),
        parse_macro_input!(item as syn::ItemEnum),
    )
    .unwrap_or_else(syn::Error::into_compile_error)
    .into()
}

fn expand_variants(count: &syn::LitInt, mut ty: syn::ItemEnum) -> syn::Result<TokenStream> {
    let count = count
        .base10_digits()
        .parse::<usize>()
        .map_err(|_| syn::Error::new(count.span(), "expected unsigned integer"))?;

    ty.variants = (0..count)
        .map(|index| syn::Variant {
            attrs: Vec::new(),
            ident: syn::Ident::new(&format!("V{}", index), Span::call_site()),
            fields: syn::Fields::Unit,
            discriminant: None,
        })
        .collect();

    Ok(quote!(#ty))
}

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Error, Result, Token, parse_macro_input};

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

#[derive(Debug)]
struct FlagsTest {
    name: String,
    flag_count: usize,
}

impl Parse for FlagsTest {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse::<syn::Ident>()?.to_string();
        input.parse::<Token![,]>()?;
        let flag_count = input.parse::<syn::LitInt>()?.base10_parse()?;

        Ok(Self { name, flag_count })
    }
}

#[proc_macro]
pub fn flags_test(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_flags_test(&parse_macro_input!(input as FlagsTest))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_flags_test(test: &FlagsTest) -> Result<TokenStream> {
    let name = format_ident!("{}", test.name);
    let flags = (0..test.flag_count)
        .map(|index| {
            let name = format_ident!("F{}", index);
            quote!(const #name;)
        })
        .collect::<TokenStream>();

    let expanded = quote! {
        wasmtime::component::flags! {
            #name {
                #flags
            }
        }
    };

    Ok(expanded)
}

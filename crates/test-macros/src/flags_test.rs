use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Error, Result, Token, parse_macro_input};

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

pub fn run(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(&parse_macro_input!(input as FlagsTest))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand(test: &FlagsTest) -> Result<TokenStream> {
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

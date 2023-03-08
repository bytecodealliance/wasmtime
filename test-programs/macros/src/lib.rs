use proc_macro::TokenStream;
use quote::quote;

include!(concat!(env!("OUT_DIR"), "/components.rs"));

#[proc_macro]
pub fn command_tests(_input: TokenStream) -> TokenStream {
    let tests = COMMAND_COMPONENTS.iter().map(|(stem, file)| {
        let name = quote::format_ident!("{}", stem);
        let runner = quote::format_ident!("run_{}", stem);
        quote! {
            #[test_log::test(tokio::test)]
            async fn #name() -> anyhow::Result<()> {
                let (store, inst) = instantiate(#file).await?;
                #runner(store, inst).await
            }
        }
    });
    quote!(#(#tests)*).into()
}

#[proc_macro]
pub fn reactor_tests(_input: TokenStream) -> TokenStream {
    let tests = REACTOR_COMPONENTS.iter().map(|(stem, file)| {
        let name = quote::format_ident!("{}", stem);
        let runner = quote::format_ident!("run_{}", stem);
        quote! {
            #[test_log::test(tokio::test)]
            async fn #name() -> anyhow::Result<()> {
                let (store, inst) = instantiate(#file).await?;
                #runner(store, inst).await
            }
        }
    });
    quote!(#(#tests)*).into()
}

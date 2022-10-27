use proc_macro::TokenStream;
use quote::quote;

include!(concat!(env!("OUT_DIR"), "/components.rs"));

#[proc_macro]
pub fn tests(_input: TokenStream) -> TokenStream {
    let tests = COMPONENTS.iter().map(|(stem, file)| {
        let name = quote::format_ident!("{}", stem);
        let runner = quote::format_ident!("run_{}", stem);
        quote! {
            #[test_log::test]
            fn #name() -> anyhow::Result<()> {
                let (store, inst) = instantiate(#file)?;
                #runner(store, inst)
            }
        }
    });
    quote!(#(#tests)*).into()
}

use proc_macro::TokenStream;
use quote::quote;

include!(concat!(env!("OUT_DIR"), "/components.rs"));

#[proc_macro]
pub fn tests(_input: TokenStream) -> TokenStream {
    let tests = COMPONENTS.iter().map(|(stem, file)| {
        let name = quote::format_ident!("{}", stem);
        quote! {
            #[test]
            fn #name() {
                run(#file).unwrap()
            }
        }
    });
    quote!(#(#tests)*).into()
}

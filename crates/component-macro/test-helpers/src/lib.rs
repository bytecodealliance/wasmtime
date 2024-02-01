use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;

#[proc_macro]
pub fn foreach(input: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let mut cwd = std::env::current_dir().unwrap();
    cwd.push("crates/component-macro/tests/codegen");
    let mut result = Vec::new();
    for f in cwd.read_dir().unwrap() {
        let f = f.unwrap().path();
        if f.extension().and_then(|s| s.to_str()) == Some("wit") || f.is_dir() {
            let name = f.file_stem().unwrap().to_str().unwrap();
            let ident = Ident::new(&name.replace("-", "_"), Span::call_site());
            let path = f.to_str().unwrap();
            result.push(quote! {
                #input!(#ident #name #path);
            });
        }
    }
    (quote!( #(#result)*)).into()
}

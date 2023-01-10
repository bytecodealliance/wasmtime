extern crate proc_macro;

use std::{ffi::OsStr, fs, path::Path};

use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

/// Generate a test case for every .wat file in the filetests directory.
/// This should only be used from the filetests crate.
#[proc_macro_attribute]
pub fn generate_file_tests(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input as a function.
    let input = proc_macro2::TokenStream::from(input);

    let fn_ast: ItemFn =
        syn::parse(input.clone().into()).expect("Failed to parse tokens as function");

    // Get the function's name and body.
    let name = &fn_ast.sig.ident;

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let test_file_entries = fs::read_dir(manifest_dir.join("../filetests/filetests"))
        .expect(format!("Failed to read directory: {:?}", manifest_dir).as_str());

    // Create a list of test cases by opening every .wat file in the directory.
    let test_cases = test_file_entries
        .map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        })
        .filter(|path| path.extension() == Some(OsStr::new("wat")))
        .map(|path| {
            let file_stem = path.file_stem().unwrap().to_string_lossy();
            let full = path.to_str().expect("Path for file was empty");
            let test_name = proc_macro2::Ident::new(
                &format!("winch_filetests_{}", file_stem),
                proc_macro2::Span::call_site(),
            );
            quote! {
                #[test]
                fn #test_name() {
                    #name(#full);
                }
            }
        });

    // Assemble the output by combining the function and test cases.
    let output = quote! {
        #input

        #(#test_cases)*
    };

    output.into()
}

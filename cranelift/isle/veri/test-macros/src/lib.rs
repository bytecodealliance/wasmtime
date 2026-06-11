extern crate proc_macro;

use std::{env, ffi::OsStr, fs, path::PathBuf, str::FromStr};

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn file_tests(attrs: TokenStream, input: TokenStream) -> TokenStream {
    let mut path = ".".to_string();
    let mut ext = "test".to_string();
    let attr_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("path") {
            path = meta.value()?.parse::<LitStr>()?.value();
            Ok(())
        } else if meta.path.is_ident("ext") {
            ext = meta.value()?.parse::<LitStr>()?.value();
            Ok(())
        } else {
            Err(meta.error("unknown attribute key"))
        }
    });
    parse_macro_input!(attrs with attr_parser);
    let input = parse_macro_input!(input as ItemFn);

    expand(path, ext, input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(path: String, ext: String, input: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let func_ident = &input.sig.ident;
    let func_name = func_ident.to_string();

    // Locate test data directory.
    let crate_dir = PathBuf::from_str(
        &env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR environment variable must be set"),
    )
    .expect("CARGO_MANIFEST_DIR should be a valid path");
    let test_data_dir = crate_dir.join(path);

    // Collect files with requested extension.
    let mut paths = Vec::new();
    for entry in fs::read_dir(test_data_dir).expect("should be able to read test data directory") {
        let entry = entry.expect("invalid directory entry");
        if entry.path().extension() == Some(OsStr::new(&ext)) {
            paths.push(entry.path());
        }
    }

    if paths.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "no test case files found",
        ));
    }

    // Generate one test case per file.
    let test_cases = paths.iter().map(|path| {
        let full = path
            .to_str()
            .expect("test file path should be valid string");
        let test_case_name = path
            .file_stem()
            .expect("test data path should have a file name")
            .to_str()
            .expect("test file name should be valid string");
        let test_name = format!("{func_name}_{test_case_name}");
        let test_ident =
            proc_macro2::Ident::new(test_name.as_str(), proc_macro2::Span::call_site());
        quote! {
            #[test]
            fn #test_ident() {
                #func_ident(#full);
            }
        }
    });

    // Combine the function and test cases.
    Ok(quote! {
        #input

        #(#test_cases)*
    })
}

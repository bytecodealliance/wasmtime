extern crate proc_macro;

use std::{collections::HashMap, env, ffi::OsStr, fs, path::PathBuf, str::FromStr};

use proc_macro::TokenStream;
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, Expr, ExprLit, ItemFn, Lit, Meta};

#[proc_macro_error]
#[proc_macro_attribute]
pub fn file_tests(attrs: TokenStream, input: TokenStream) -> TokenStream {
    // Parse attributes.
    let metas = parse_macro_input!(attrs with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let mut attrs = parse_attrs(&metas);

    let relative_path = attrs.remove("path").unwrap_or(".".to_string());
    let ext = attrs.remove("ext").unwrap_or("test".to_string());
    if !attrs.is_empty() {
        let keys: String = attrs.keys().cloned().collect::<Vec<_>>().join(", ");
        abort_call_site!(format!("unknown keys: {keys}"));
    }

    // Parse the input as a function.
    let input = proc_macro2::TokenStream::from(input);

    let func_ast: ItemFn =
        syn::parse(input.clone().into()).expect("should be able to parse tokens as function");

    let func_ident = &func_ast.sig.ident;
    let func_name = func_ident.to_string();

    // Locate test data directory.
    let crate_dir = PathBuf::from_str(
        &env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR environment variable must be set"),
    )
    .expect("CARGO_MANIFEST_DIR should be a valid path");
    let test_data_dir = crate_dir.join(relative_path);

    // Collect files with requested extension.
    let mut paths = Vec::new();
    for entry in fs::read_dir(test_data_dir).expect("should be able to read test data directory") {
        let entry = entry.expect("invalid directory entry");
        if entry.path().extension() == Some(OsStr::new(&ext)) {
            paths.push(entry.path());
        }
    }

    if paths.is_empty() {
        abort_call_site!("no test case files found");
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

    // Combining the function and test cases.
    let output = quote! {
        #input

        #(#test_cases)*
    };

    output.into()
}

fn parse_attrs(metas: &Punctuated<Meta, syn::Token![,]>) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    for meta in metas.iter() {
        if let Meta::NameValue(n) = meta {
            let key = n.path.get_ident().unwrap().to_string();
            match &n.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => {
                    attrs.insert(key, s.value());
                }
                _ => abort_call_site!("attribute values must be string"),
            }
        }
    }
    attrs
}

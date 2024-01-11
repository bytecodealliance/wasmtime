use std::path::Path;

use glob::glob;
use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

fn get_test_name_for_root(root: &Path, path: &Path) -> String {
    let test_name = path
        .strip_prefix(root)
        .unwrap()
        .to_str()
        .unwrap()
        .replace("/", "_")
        .replace("\\", "_")
        .replace(".wat", "");

    format!("winch_filetests_{}", test_name)
}

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

    let filetests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../filetests/filetests");

    let test_file_entries = glob(format!("{}/**/*.wat", filetests_dir.to_str().unwrap()).as_str())
        .expect("Failed to read glob pattern");

    // Create a list of test cases by opening every .wat file in the directory.
    let test_cases = test_file_entries.map(|entry| {
        let path = entry.expect("Failed to read glob entry");

        let full = path.to_str().expect("Path for file was empty");

        let test_name = proc_macro2::Ident::new(
            &get_test_name_for_root(&filetests_dir, &path),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_test_name_for_root_unix() {
        let root = Path::new("/home/user/Documents/winch/filetests/filetests");
        let path = Path::new("/home/user/Documents/winch/filetests/filetests/simd/simple.wat");

        let test_name = get_test_name_for_root(root, path);

        assert_eq!(test_name, "winch_filetests_simd_simple");
    }
}

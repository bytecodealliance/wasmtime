use proc_macro2::{Literal, TokenStream, TokenTree};

/// Given the input tokens to a macro invocation, return the path to the
/// witx file to process.
pub(crate) fn witx_path_from_args(args: TokenStream) -> (String, String) {
    let mut strings = Vec::new();

    for arg in args {
        if let TokenTree::Literal(literal) = arg {
            let parsed = parse_string_literal(literal);

            strings.push(parsed);
        } else {
            panic!("arguments must be string literals");
        }
    }

    if strings.len() != 2 {
        panic!("expected two string literals");
    }

    let phase = &strings[0];
    let id = &strings[1];
    let path = witx_path(phase, id);

    (path, phase.clone())
}

fn witx_path(phase: &str, id: &str) -> String {
    let root = env!("CARGO_MANIFEST_DIR");
    format!("{}/WASI/phases/{}/witx/{}.witx", root, phase, id)
}

// Convert a `Literal` holding a string literal into the `String`.
//
// FIXME: It feels like there should be an easier way to do this.
fn parse_string_literal(literal: Literal) -> String {
    let s = literal.to_string();
    assert!(
        s.starts_with('"') && s.ends_with('"'),
        "string literal must be enclosed in double-quotes"
    );

    let trimmed = s[1..s.len() - 1].to_owned();
    assert!(
        !trimmed.contains('"'),
        "string literal must not contain embedded quotes for now"
    );
    assert!(
        !trimmed.contains('\\'),
        "string literal must not contain embedded backslashes for now"
    );

    trimmed
}

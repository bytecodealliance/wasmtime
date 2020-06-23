
use proc_macro2::{Ident, Literal, TokenStream, TokenTree};
use std::path::PathBuf;

/// Given the input tokens to a macro invocation, return the path to the
/// witx file to process.
pub(crate) fn witx_path_from_args(args: TokenStream) -> PathBuf {
    let mut strings = Vec::new();

    for arg in args {
        if let TokenTree::Literal(literal) = arg {
            let parsed = parse_string_literal(literal);

            strings.push(parsed);
        } else {
            panic!("arguments must be string literals");
        }
    }

    if strings.len() != 1 {
        panic!("expected one string literals");
    }
    let root = PathBuf::from(std::env::var("WASI_ROOT").unwrap());
    return root.join(&strings[0]);
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

pub fn param_name(param: &witx::InterfaceFuncParam) -> Ident {
    quote::format_ident!(
        "{}",
        match param.name.as_str() {
            "in" | "type" => format!("r#{}", param.name.as_str()),
            s => s.to_string(),
        }
    )
}

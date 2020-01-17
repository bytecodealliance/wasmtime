use anyhow::{bail, Result};
use proc_macro2::{Literal, TokenStream, TokenTree};

pub fn witx_paths(args: TokenStream) -> Result<Vec<String>> {
    let arg_strings = args
        .into_iter()
        .map(|arg| match arg {
            TokenTree::Literal(lit) => string_literal(lit),
            _ => bail!("expected string literal, got: {:?}", arg),
        })
        .collect::<Result<Vec<String>>>()?;

    if arg_strings.is_empty() {
        bail!("expected at least one argument");
    }
    Ok(arg_strings)
}

fn string_literal(literal: Literal) -> Result<String> {
    let s = literal.to_string();
    if !s.starts_with('"') || !s.ends_with('"') {
        bail!("string literal must be enclosed in double quotes");
    }

    let trimmed = s[1..s.len() - 1].to_owned();
    if trimmed.contains('"') {
        bail!("string literal must not contain quotes");
    }
    if trimmed.contains('\\') {
        bail!("string literal must not contain backslashes");
    }
    Ok(trimmed)
}

use std::path::PathBuf;

use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, LitStr, Result, Token,
};

pub struct Config {
    _bracket_token: token::Bracket,
    path_lits: Punctuated<LitStr, Token![,]>,
}

impl Config {
    pub fn witx_paths(&self) -> Vec<PathBuf> {
        self.path_lits
            .iter()
            .map(|lit| PathBuf::from(lit.value()))
            .collect()
    }
}

impl Parse for Config {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Config {
            _bracket_token: bracketed!(content in input),
            path_lits: content.parse_terminated(Parse::parse)?,
        })
    }
}

/*
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
*/

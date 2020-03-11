use std::path::PathBuf;

use proc_macro2::Span;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, Ident, LitStr, Result, Token,
};

#[derive(Debug, Clone)]
pub struct Config {
    pub witx: WitxConf,
    pub ctx: CtxConf,
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    Witx(WitxConf),
    Ctx(CtxConf),
}

impl ConfigField {
    pub fn parse_pair(ident: &str, value: ParseStream, err_loc: Span) -> Result<Self> {
        match ident {
            "witx" => Ok(ConfigField::Witx(value.parse()?)),
            "ctx" => Ok(ConfigField::Ctx(value.parse()?)),
            _ => Err(Error::new(err_loc, "expected `witx` or `ctx`")),
        }
    }
}

impl Parse for ConfigField {
    fn parse(input: ParseStream) -> Result<Self> {
        let id: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        Self::parse_pair(id.to_string().as_ref(), input, id.span())
    }
}

impl Config {
    pub fn build(fields: impl Iterator<Item = ConfigField>, err_loc: Span) -> Result<Self> {
        let mut witx = None;
        let mut ctx = None;
        for f in fields {
            match f {
                ConfigField::Witx(c) => {
                    witx = Some(c);
                }
                ConfigField::Ctx(c) => {
                    ctx = Some(c);
                }
            }
        }
        Ok(Config {
            witx: witx
                .take()
                .ok_or_else(|| Error::new(err_loc, "`witx` field required"))?,
            ctx: ctx
                .take()
                .ok_or_else(|| Error::new(err_loc, "`ctx` field required"))?,
        })
    }
}

impl Parse for Config {
    fn parse(input: ParseStream) -> Result<Self> {
        let contents;
        let _lbrace = braced!(contents in input);
        let fields: Punctuated<ConfigField, Token![,]> =
            contents.parse_terminated(ConfigField::parse)?;
        Ok(Config::build(fields.into_iter(), input.span())?)
    }
}

#[derive(Debug, Clone)]
pub struct WitxConf {
    pub paths: Vec<PathBuf>,
}

impl Parse for WitxConf {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = bracketed!(content in input);
        let path_lits: Punctuated<LitStr, Token![,]> = content.parse_terminated(Parse::parse)?;
        let paths: Vec<PathBuf> = path_lits
            .iter()
            .map(|lit| PathBuf::from(lit.value()))
            .collect();
        Ok(WitxConf { paths })
    }
}

#[derive(Debug, Clone)]
pub struct CtxConf {
    pub name: Ident,
}

impl Parse for CtxConf {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(CtxConf {
            name: input.parse()?,
        })
    }
}

use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    pub errors: ErrorConf,
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    Witx(WitxConf),
    Ctx(CtxConf),
    Error(ErrorConf),
}

impl ConfigField {
    pub fn parse_pair(ident: &str, value: ParseStream, err_loc: Span) -> Result<Self> {
        match ident {
            "witx" => Ok(ConfigField::Witx(value.parse()?)),
            "ctx" => Ok(ConfigField::Ctx(value.parse()?)),
            "errors" => Ok(ConfigField::Error(value.parse()?)),
            _ => Err(Error::new(err_loc, "expected `witx`, `ctx`, or `errors`")),
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
        let mut errors = None;
        for f in fields {
            match f {
                ConfigField::Witx(c) => {
                    if witx.is_some() {
                        return Err(Error::new(err_loc, "duplicate `witx` field"));
                    }
                    witx = Some(c);
                }
                ConfigField::Ctx(c) => {
                    if ctx.is_some() {
                        return Err(Error::new(err_loc, "duplicate `ctx` field"));
                    }
                    ctx = Some(c);
                }
                ConfigField::Error(c) => {
                    if errors.is_some() {
                        return Err(Error::new(err_loc, "duplicate `errors` field"));
                    }
                    errors = Some(c);
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
            errors: errors.take().unwrap_or_default(),
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

impl WitxConf {
    pub fn make_paths_relative_to<P: AsRef<Path>>(&mut self, root: P) {
        self.paths.iter_mut().for_each(|p| {
            if !p.is_absolute() {
                *p = PathBuf::from(root.as_ref()).join(p.clone());
            }
        });
    }
}

impl Parse for WitxConf {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = bracketed!(content in input);
        let path_lits: Punctuated<LitStr, Token![,]> = content.parse_terminated(Parse::parse)?;
        let paths = path_lits
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

#[derive(Clone, Default, Debug)]
/// Map from abi error type to rich error type
pub struct ErrorConf(HashMap<Ident, ErrorConfField>);

impl ErrorConf {
    pub fn iter(&self) -> impl Iterator<Item = (&Ident, &ErrorConfField)> {
        self.0.iter()
    }
}

impl Parse for ErrorConf {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = braced!(content in input);
        let items: Punctuated<ErrorConfField, Token![,]> =
            content.parse_terminated(Parse::parse)?;
        let mut m = HashMap::new();
        for i in items {
            match m.insert(i.abi_error.clone(), i.clone()) {
                None => {}
                Some(prev_def) => {
                    return Err(Error::new(
                        i.err_loc,
                        format!(
                        "duplicate definition of rich error type for {:?}: previously defined at {:?}",
                        i.abi_error, prev_def.err_loc,
                    ),
                    ))
                }
            }
        }
        Ok(ErrorConf(m))
    }
}

#[derive(Clone)]
pub struct ErrorConfField {
    pub abi_error: Ident,
    pub rich_error: syn::Path,
    pub err_loc: Span,
}

impl std::fmt::Debug for ErrorConfField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorConfField")
            .field("abi_error", &self.abi_error)
            .field("rich_error", &"(...)")
            .field("err_loc", &self.err_loc)
            .finish()
    }
}

impl Parse for ErrorConfField {
    fn parse(input: ParseStream) -> Result<Self> {
        let err_loc = input.span();
        let abi_error = input.parse::<Ident>()?;
        let _arrow: Token![=>] = input.parse()?;
        let rich_error = input.parse::<syn::Path>()?;
        Ok(ErrorConfField {
            abi_error,
            rich_error,
            err_loc,
        })
    }
}

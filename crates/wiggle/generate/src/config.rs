use {
    proc_macro2::Span,
    std::{
        iter::FromIterator,
        path::{Path, PathBuf},
    },
    syn::{
        braced, bracketed,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        Error, Ident, LitStr, Result, Token,
    },
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
            "witx" => Ok(ConfigField::Witx(WitxConf::Paths(value.parse()?))),
            "witx_literal" => Ok(ConfigField::Witx(WitxConf::Literal(value.parse()?))),
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

    /// Load the `witx` document for the configuration.
    ///
    /// # Panics
    ///
    /// This method will panic if the paths given in the `witx` field were not valid documents.
    pub fn load_document(&self) -> witx::Document {
        self.witx.load_document()
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

/// The witx document(s) that will be loaded from a [`Config`](struct.Config.html).
///
/// A witx interface definition can be provided either as a collection of relative paths to
/// documents, or as a single inlined string literal. Note that `(use ...)` directives are not
/// permitted when providing a string literal.
#[derive(Debug, Clone)]
pub enum WitxConf {
    /// A collection of paths pointing to witx files.
    Paths(Paths),
    /// A single witx document, provided as a string literal.
    Literal(Literal),
}

impl WitxConf {
    /// Load the `witx` document.
    ///
    /// # Panics
    ///
    /// This method will panic if the paths given in the `witx` field were not valid documents, or
    /// if any of the given documents were not syntactically valid.
    pub fn load_document(&self) -> witx::Document {
        match self {
            Self::Paths(paths) => witx::load(paths.as_ref()).expect("loading witx"),
            Self::Literal(doc) => witx::parse(doc.as_ref()).expect("parsing witx"),
        }
    }

    /// If using the [`Paths`][paths] syntax, make all paths relative to a root directory.
    ///
    /// [paths]: enum.WitxConf.html#variant.Paths
    pub fn make_paths_relative_to<P: AsRef<Path>>(&mut self, root: P) {
        if let Self::Paths(paths) = self {
            paths.as_mut().iter_mut().for_each(|p| {
                if !p.is_absolute() {
                    *p = PathBuf::from(root.as_ref()).join(p.clone());
                }
            });
        }
    }
}

/// A collection of paths, pointing to witx documents.
#[derive(Debug, Clone)]
pub struct Paths(Vec<PathBuf>);

impl Paths {
    /// Create a new, empty collection of paths.
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for Paths {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl AsRef<[PathBuf]> for Paths {
    fn as_ref(&self) -> &[PathBuf] {
        self.0.as_ref()
    }
}

impl AsMut<[PathBuf]> for Paths {
    fn as_mut(&mut self) -> &mut [PathBuf] {
        self.0.as_mut()
    }
}

impl FromIterator<PathBuf> for Paths {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = PathBuf>,
    {
        Self(iter.into_iter().collect())
    }
}

impl Parse for Paths {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = bracketed!(content in input);
        let path_lits: Punctuated<LitStr, Token![,]> = content.parse_terminated(Parse::parse)?;
        Ok(path_lits
            .iter()
            .map(|lit| PathBuf::from(lit.value()))
            .collect())
    }
}

/// A single witx document, provided as a string literal.
#[derive(Debug, Clone)]
pub struct Literal(String);

impl AsRef<str> for Literal {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Parse for Literal {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(input.parse::<syn::LitStr>()?.value()))
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

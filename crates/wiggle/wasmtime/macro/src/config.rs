use {
    proc_macro2::{Span, TokenStream},
    syn::{
        braced,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        Error, Ident, Path, Result, Token,
    },
    wiggle_generate::config::{CtxConf, WitxConf},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub target: TargetConf,
    pub witx: WitxConf,
    pub ctx: CtxConf,
    pub instance: InstanceConf,
    pub missing_memory: MissingMemoryConf,
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    Target(TargetConf),
    Witx(WitxConf),
    Ctx(CtxConf),
    Instance(InstanceConf),
    MissingMemory(MissingMemoryConf),
}

mod kw {
    syn::custom_keyword!(target);
    syn::custom_keyword!(witx);
    syn::custom_keyword!(witx_literal);
    syn::custom_keyword!(ctx);
    syn::custom_keyword!(instance);
    syn::custom_keyword!(name);
    syn::custom_keyword!(docs);
    syn::custom_keyword!(missing_memory);
}

impl Parse for ConfigField {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::target) {
            input.parse::<kw::target>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Target(input.parse()?))
        } else if lookahead.peek(kw::witx) {
            input.parse::<kw::witx>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Witx(WitxConf::Paths(input.parse()?)))
        } else if lookahead.peek(kw::witx_literal) {
            input.parse::<kw::witx_literal>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Witx(WitxConf::Literal(input.parse()?)))
        } else if lookahead.peek(kw::ctx) {
            input.parse::<kw::ctx>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Ctx(input.parse()?))
        } else if lookahead.peek(kw::instance) {
            input.parse::<kw::instance>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Instance(input.parse()?))
        } else if lookahead.peek(kw::missing_memory) {
            input.parse::<kw::missing_memory>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::MissingMemory(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Config {
    pub fn build(fields: impl Iterator<Item = ConfigField>, err_loc: Span) -> Result<Self> {
        let mut target = None;
        let mut witx = None;
        let mut ctx = None;
        let mut instance = None;
        let mut missing_memory = None;
        for f in fields {
            match f {
                ConfigField::Target(c) => {
                    if target.is_some() {
                        return Err(Error::new(err_loc, "duplicate `target` field"));
                    }
                    target = Some(c);
                }
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
                ConfigField::Instance(c) => {
                    if instance.is_some() {
                        return Err(Error::new(err_loc, "duplicate `instance` field"));
                    }
                    instance = Some(c);
                }
                ConfigField::MissingMemory(c) => {
                    if missing_memory.is_some() {
                        return Err(Error::new(err_loc, "duplicate `missing_memory` field"));
                    }
                    missing_memory = Some(c);
                }
            }
        }
        Ok(Config {
            target: target
                .take()
                .ok_or_else(|| Error::new(err_loc, "`target` field required"))?,
            witx: witx
                .take()
                .ok_or_else(|| Error::new(err_loc, "`witx` field required"))?,
            ctx: ctx
                .take()
                .ok_or_else(|| Error::new(err_loc, "`ctx` field required"))?,
            instance: instance
                .take()
                .ok_or_else(|| Error::new(err_loc, "`instance` field required"))?,
            missing_memory: missing_memory
                .take()
                .ok_or_else(|| Error::new(err_loc, "`missing_memory` field required"))?,
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

#[derive(Debug, Clone)]
pub struct TargetConf {
    pub path: Path,
}

impl Parse for TargetConf {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(TargetConf {
            path: input.parse()?,
        })
    }
}

enum InstanceConfField {
    Name(Ident),
    Docs(String),
}

impl Parse for InstanceConfField {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::name) {
            input.parse::<kw::name>()?;
            input.parse::<Token![:]>()?;
            Ok(InstanceConfField::Name(input.parse()?))
        } else if lookahead.peek(kw::docs) {
            input.parse::<kw::docs>()?;
            input.parse::<Token![:]>()?;
            let docs: syn::LitStr = input.parse()?;
            Ok(InstanceConfField::Docs(docs.value()))
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstanceConf {
    pub name: Ident,
    pub docs: Option<String>,
}

impl InstanceConf {
    fn build(fields: impl Iterator<Item = InstanceConfField>, err_loc: Span) -> Result<Self> {
        let mut name = None;
        let mut docs = None;
        for f in fields {
            match f {
                InstanceConfField::Name(c) => {
                    if name.is_some() {
                        return Err(Error::new(err_loc, "duplicate `name` field"));
                    }
                    name = Some(c);
                }
                InstanceConfField::Docs(c) => {
                    if docs.is_some() {
                        return Err(Error::new(err_loc, "duplicate `docs` field"));
                    }
                    docs = Some(c);
                }
            }
        }
        Ok(InstanceConf {
            name: name
                .take()
                .ok_or_else(|| Error::new(err_loc, "`name` field required"))?,
            docs,
        })
    }
}

impl Parse for InstanceConf {
    fn parse(input: ParseStream) -> Result<Self> {
        let contents;
        let _lbrace = braced!(contents in input);
        let fields: Punctuated<InstanceConfField, Token![,]> =
            contents.parse_terminated(InstanceConfField::parse)?;
        Ok(InstanceConf::build(fields.into_iter(), input.span())?)
    }
}

#[derive(Debug, Clone)]
pub struct MissingMemoryConf {
    pub err: TokenStream,
}
impl Parse for MissingMemoryConf {
    fn parse(input: ParseStream) -> Result<Self> {
        let contents;
        let _lbrace = braced!(contents in input);
        Ok(MissingMemoryConf {
            err: contents.parse()?,
        })
    }
}

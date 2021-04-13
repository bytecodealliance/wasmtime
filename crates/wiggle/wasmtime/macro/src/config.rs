use wiggle_generate::config::AsyncConfField;
use {
    proc_macro2::Span,
    std::collections::HashMap,
    syn::{
        braced,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        Error, Ident, Path, Result, Token,
    },
    wiggle_generate::config::WitxConf,
};
#[derive(Debug, Clone)]
pub struct Config {
    pub target: TargetConf,
    pub witx: WitxConf,
    pub ctx: CtxConf,
    pub modules: ModulesConf,
    pub async_: AsyncConf,
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    Target(TargetConf),
    Witx(WitxConf),
    Ctx(CtxConf),
    Modules(ModulesConf),
    Async(AsyncConf),
}

mod kw {
    syn::custom_keyword!(target);
    syn::custom_keyword!(witx);
    syn::custom_keyword!(witx_literal);
    syn::custom_keyword!(ctx);
    syn::custom_keyword!(modules);
    syn::custom_keyword!(name);
    syn::custom_keyword!(docs);
    syn::custom_keyword!(function_override);
    syn::custom_keyword!(block_on);
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
        } else if lookahead.peek(kw::modules) {
            input.parse::<kw::modules>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Modules(input.parse()?))
        } else if lookahead.peek(Token![async]) {
            input.parse::<Token![async]>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Async(input.parse()?))
        } else if lookahead.peek(kw::block_on) {
            input.parse::<kw::block_on>()?;
            input.parse::<Token![:]>()?;
            let mut async_conf: AsyncConf = input.parse()?;
            async_conf.blocking = true;
            Ok(ConfigField::Async(async_conf))
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
        let mut modules = None;
        let mut async_ = None;
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
                ConfigField::Modules(c) => {
                    if modules.is_some() {
                        return Err(Error::new(err_loc, "duplicate `modules` field"));
                    }
                    modules = Some(c);
                }
                ConfigField::Async(c) => {
                    if async_.is_some() {
                        return Err(Error::new(err_loc, "duplicate `async` field"));
                    }
                    async_ = Some(c);
                }
            }
        }
        Ok(Config {
            target: target.ok_or_else(|| Error::new(err_loc, "`target` field required"))?,
            witx: witx.ok_or_else(|| Error::new(err_loc, "`witx` field required"))?,
            ctx: ctx.ok_or_else(|| Error::new(err_loc, "`ctx` field required"))?,
            modules: modules.ok_or_else(|| Error::new(err_loc, "`modules` field required"))?,
            async_: async_.unwrap_or_default(),
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
pub struct CtxConf {
    pub name: syn::Type,
}

impl Parse for CtxConf {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(CtxConf {
            name: input.parse()?,
        })
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

enum ModuleConfField {
    Name(Ident),
    Docs(String),
}

impl Parse for ModuleConfField {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::name) {
            input.parse::<kw::name>()?;
            input.parse::<Token![:]>()?;
            Ok(ModuleConfField::Name(input.parse()?))
        } else if lookahead.peek(kw::docs) {
            input.parse::<kw::docs>()?;
            input.parse::<Token![:]>()?;
            let docs: syn::LitStr = input.parse()?;
            Ok(ModuleConfField::Docs(docs.value()))
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleConf {
    pub name: Ident,
    pub docs: Option<String>,
}

impl ModuleConf {
    fn build(fields: impl Iterator<Item = ModuleConfField>, err_loc: Span) -> Result<Self> {
        let mut name = None;
        let mut docs = None;
        for f in fields {
            match f {
                ModuleConfField::Name(c) => {
                    if name.is_some() {
                        return Err(Error::new(err_loc, "duplicate `name` field"));
                    }
                    name = Some(c);
                }
                ModuleConfField::Docs(c) => {
                    if docs.is_some() {
                        return Err(Error::new(err_loc, "duplicate `docs` field"));
                    }
                    docs = Some(c);
                }
            }
        }
        Ok(ModuleConf {
            name: name.ok_or_else(|| Error::new(err_loc, "`name` field required"))?,
            docs,
        })
    }
}

impl Parse for ModuleConf {
    fn parse(input: ParseStream) -> Result<Self> {
        let contents;
        let _lbrace = braced!(contents in input);
        let fields: Punctuated<ModuleConfField, Token![,]> =
            contents.parse_terminated(ModuleConfField::parse)?;
        Ok(ModuleConf::build(fields.into_iter(), input.span())?)
    }
}

#[derive(Debug, Clone)]
pub struct ModulesConf {
    pub mods: HashMap<String, ModuleConf>,
}

impl ModulesConf {
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ModuleConf)> {
        self.mods.iter()
    }
}

impl Parse for ModulesConf {
    fn parse(input: ParseStream) -> Result<Self> {
        let contents;
        let _lbrace = braced!(contents in input);
        let fields: Punctuated<(String, ModuleConf), Token![,]> =
            contents.parse_terminated(|i| {
                let name = i.parse::<Ident>()?.to_string();
                i.parse::<Token![=>]>()?;
                let val = i.parse()?;
                Ok((name, val))
            })?;
        Ok(ModulesConf {
            mods: fields.into_iter().collect(),
        })
    }
}

#[derive(Clone, Default, Debug)]
/// Modules and funcs that have async signatures
pub struct AsyncConf {
    blocking: bool,
    functions: HashMap<String, Vec<String>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Asyncness {
    /// Wiggle function is synchronous, wasmtime Func is synchronous
    Sync,
    /// Wiggle function is asynchronous, but wasmtime Func is synchronous
    Blocking,
    /// Wiggle function and wasmtime Func are asynchronous.
    Async,
}

impl Asyncness {
    pub fn is_sync(&self) -> bool {
        match self {
            Asyncness::Sync => true,
            _ => false,
        }
    }
}

impl AsyncConf {
    pub fn is_async(&self, module: &str, function: &str) -> Asyncness {
        if self
            .functions
            .get(module)
            .and_then(|fs| fs.iter().find(|f| *f == function))
            .is_some()
        {
            if self.blocking {
                Asyncness::Blocking
            } else {
                Asyncness::Async
            }
        } else {
            Asyncness::Sync
        }
    }
}

impl Parse for AsyncConf {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _ = braced!(content in input);
        let items: Punctuated<AsyncConfField, Token![,]> =
            content.parse_terminated(Parse::parse)?;
        let mut functions: HashMap<String, Vec<String>> = HashMap::new();
        use std::collections::hash_map::Entry;
        for i in items {
            let function_names = i
                .function_names
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>();
            match functions.entry(i.module_name.to_string()) {
                Entry::Occupied(o) => o.into_mut().extend(function_names),
                Entry::Vacant(v) => {
                    v.insert(function_names);
                }
            }
        }
        Ok(AsyncConf {
            functions,
            blocking: false,
        })
    }
}

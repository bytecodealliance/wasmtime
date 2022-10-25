use {
    anyhow::anyhow,
    proc_macro2::{Span, TokenStream},
    quote::quote,
    std::rc::Rc,
    std::{collections::HashMap, iter::FromIterator, path::PathBuf},
    syn::{
        braced, bracketed,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        Error, Ident, LitStr, Result, Token,
    },
    witx::{Document, Id, NamedType, TypeRef},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub witx: WitxConf,
    pub errors: ErrorConf,
    pub async_: AsyncConf,
}

mod kw {
    syn::custom_keyword!(witx);
    syn::custom_keyword!(witx_literal);
    syn::custom_keyword!(block_on);
    syn::custom_keyword!(errors);
    syn::custom_keyword!(target);
}

#[derive(Debug, Clone)]
pub enum ConfigField {
    Witx(WitxConf),
    Error(ErrorConf),
    Async(AsyncConf),
}

impl Parse for ConfigField {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::witx) {
            input.parse::<kw::witx>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Witx(WitxConf::Paths(input.parse()?)))
        } else if lookahead.peek(kw::witx_literal) {
            input.parse::<kw::witx_literal>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Witx(WitxConf::Literal(input.parse()?)))
        } else if lookahead.peek(kw::errors) {
            input.parse::<kw::errors>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Error(input.parse()?))
        } else if lookahead.peek(Token![async]) {
            input.parse::<Token![async]>()?;
            Ok(ConfigField::Async(AsyncConf::Async))
        } else if lookahead.peek(kw::block_on) {
            input.parse::<kw::block_on>()?;
            input.parse::<Token![:]>()?;
            Ok(ConfigField::Async(AsyncConf::Blocking))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Config {
    pub fn build(fields: impl Iterator<Item = ConfigField>, err_loc: Span) -> Result<Self> {
        let mut witx = None;
        let mut errors = None;
        let mut async_ = None;
        for f in fields {
            match f {
                ConfigField::Witx(c) => {
                    if witx.is_some() {
                        return Err(Error::new(err_loc, "duplicate `witx` field"));
                    }
                    witx = Some(c);
                }
                ConfigField::Error(c) => {
                    if errors.is_some() {
                        return Err(Error::new(err_loc, "duplicate `errors` field"));
                    }
                    errors = Some(c);
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
            witx: witx
                .take()
                .ok_or_else(|| Error::new(err_loc, "`witx` field required"))?,
            errors: errors.take().unwrap_or_default(),
            async_: async_.take().unwrap_or_default(),
        })
    }

    /// Load the `witx` document for the configuration.
    ///
    /// # Panics
    ///
    /// This method will panic if the paths given in the `witx` field were not valid documents.
    pub fn load_document(&self) -> Document {
        self.witx.load_document()
    }

    pub fn codegen_conf(&self) -> anyhow::Result<CodegenConf> {
        CodegenConf::new(&self.errors, &self.async_, &self.witx.load_document())
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

pub struct CodegenConf {
    pub errors: ErrorTransform,
    pub async_: AsyncConf,
}

impl CodegenConf {
    pub fn new(error_conf: &ErrorConf, async_: &AsyncConf, doc: &Document) -> anyhow::Result<Self> {
        let errors = ErrorTransform::new(error_conf, doc)?;
        Ok(Self {
            errors,
            async_: async_.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ErrorTransform {
    m: Vec<UserErrorType>,
}

impl ErrorTransform {
    pub fn empty() -> Self {
        Self { m: Vec::new() }
    }
    pub fn new(conf: &ErrorConf, doc: &Document) -> anyhow::Result<Self> {
        let mut richtype_identifiers = HashMap::new();
        let m = conf.iter().map(|(ident, field)|
            if let Some(abi_type) = doc.typename(&Id::new(ident.to_string())) {
                    if let Some(ident) = field.rich_error.get_ident() {
                        if let Some(prior_def) = richtype_identifiers.insert(ident.clone(), field.err_loc.clone())
                         {
                            return Err(anyhow!(
                                    "duplicate rich type identifier of {:?} not allowed. prior definition at {:?}",
                                    ident, prior_def
                                ));
                        }
                        Ok(UserErrorType {
                            abi_type,
                            rich_type: field.rich_error.clone(),
                            method_fragment: ident.to_string()
                        })
                    } else {
                        return Err(anyhow!(
                            "rich error type must be identifier for now - TODO add ability to provide a corresponding identifier: {:?}",
                            field.err_loc
                        ))
                    }
                }
                else { Err(anyhow!("No witx typename \"{}\" found", ident.to_string())) }
        ).collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self { m })
    }

    pub fn iter(&self) -> impl Iterator<Item = &UserErrorType> {
        self.m.iter()
    }

    pub fn for_abi_error(&self, tref: &TypeRef) -> Option<&UserErrorType> {
        match tref {
            TypeRef::Name(nt) => self.for_name(nt),
            TypeRef::Value { .. } => None,
        }
    }

    pub fn for_name(&self, nt: &NamedType) -> Option<&UserErrorType> {
        self.m.iter().find(|u| u.abi_type.name == nt.name)
    }
}

#[derive(Debug, Clone)]
pub struct UserErrorType {
    abi_type: Rc<NamedType>,
    rich_type: syn::Path,
    method_fragment: String,
}

impl UserErrorType {
    pub fn abi_type(&self) -> TypeRef {
        TypeRef::Name(self.abi_type.clone())
    }
    pub fn typename(&self) -> TokenStream {
        let t = &self.rich_type;
        quote!(#t)
    }
    pub fn method_fragment(&self) -> &str {
        &self.method_fragment
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
    pub fn load_document(&self) -> Document {
        match self {
            Self::Paths(paths) => witx::load(paths.as_ref()).expect("loading witx"),
            Self::Literal(doc) => witx::parse(doc.as_ref()).expect("parsing witx"),
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

        let expanded_paths = path_lits
            .iter()
            .map(|lit| {
                PathBuf::from(
                    shellexpand::env(&lit.value())
                        .expect("shell expansion")
                        .as_ref(),
                )
            })
            .collect::<Vec<PathBuf>>();

        Ok(Paths(expanded_paths))
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum AsyncConf {
    /// Wiggle function is synchronous, wasmtime Func is synchronous
    #[default]
    Sync,
    /// Wiggle function is asynchronous, but wasmtime Func is synchronous
    Blocking,
    /// Wiggle function and wasmtime Func are asynchronous.
    Async,
}

impl AsyncConf {
    pub fn is_async(&self) -> bool {
        match self {
            Self::Async => true,
            _ => false,
        }
    }
    pub fn is_blocking(&self) -> bool {
        match self {
            Self::Blocking => true,
            _ => false,
        }
    }
    pub fn is_sync(&self) -> bool {
        match self {
            Self::Sync => true,
            _ => false,
        }
    }
}

use proc_macro2::{Span, TokenStream};
use std::path::{Path, PathBuf};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{braced, token, Ident, Token};
use wasmtime_wit_bindgen::{Opts, TrappableError};
use wit_parser::{PackageId, Resolve, UnresolvedPackage, WorldId};

pub struct Config {
    opts: Opts,
    resolve: Resolve,
    world: WorldId,
    files: Vec<PathBuf>,
}

pub fn expand(input: &Config) -> Result<TokenStream> {
    if !cfg!(feature = "async") && input.opts.async_ {
        return Err(Error::new(
            Span::call_site(),
            "cannot enable async bindings unless `async` crate feature is active",
        ));
    }

    let src = input.opts.generate(&input.resolve, input.world);
    let mut contents = src.parse::<TokenStream>().unwrap();

    // Include a dummy `include_str!` for any files we read so rustc knows that
    // we depend on the contents of those files.
    for file in input.files.iter() {
        contents.extend(
            format!("const _: &str = include_str!(r#\"{}\"#);\n", file.display())
                .parse::<TokenStream>()
                .unwrap(),
        );
    }

    Ok(contents)
}

enum Source {
    Path(String),
    Inline(String),
}

impl Parse for Config {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let call_site = Span::call_site();
        let mut opts = Opts::default();
        let mut source = None;
        let mut world = None;

        if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let fields = Punctuated::<Opt, Token![,]>::parse_terminated(&content)?;
            for field in fields.into_pairs() {
                match field.into_value() {
                    Opt::Path(s) => {
                        if source.is_some() {
                            return Err(Error::new(s.span(), "cannot specify second source"));
                        }
                        source = Some(Source::Path(s.value()));
                    }
                    Opt::World(s) => {
                        if world.is_some() {
                            return Err(Error::new(s.span(), "cannot specify second world"));
                        }
                        world = Some(s.value());
                    }
                    Opt::Inline(s) => {
                        if source.is_some() {
                            return Err(Error::new(s.span(), "cannot specify second source"));
                        }
                        source = Some(Source::Inline(s.value()));
                    }
                    Opt::Tracing(val) => opts.tracing = val,
                    Opt::Async(val) => opts.async_ = val,
                    Opt::TrappableErrorType(val) => opts.trappable_error_type = val,
                }
            }
        } else {
            world = input.parse::<Option<syn::LitStr>>()?.map(|s| s.value());
            if input.parse::<Option<syn::token::In>>()?.is_some() {
                source = Some(Source::Path(input.parse::<syn::LitStr>()?.value()));
            }
        }
        let (resolve, pkg, files) =
            parse_source(&source).map_err(|err| Error::new(call_site, format!("{err:?}")))?;
        let world = resolve
            .select_world(pkg, world.as_deref())
            .map_err(|e| Error::new(call_site, format!("{e:?}")))?;
        Ok(Config {
            opts,
            resolve,
            world,
            files,
        })
    }
}

fn parse_source(source: &Option<Source>) -> anyhow::Result<(Resolve, PackageId, Vec<PathBuf>)> {
    let mut resolve = Resolve::default();
    let mut files = Vec::new();
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut parse = |path: &Path| -> anyhow::Result<_> {
        if path.is_dir() {
            let (pkg, sources) = resolve.push_dir(&path)?;
            files = sources;
            Ok(pkg)
        } else {
            let pkg = UnresolvedPackage::parse_file(path)?;
            files.extend(pkg.source_files().map(|s| s.to_owned()));
            resolve.push(pkg, &Default::default())
        }
    };
    let pkg = match source {
        Some(Source::Inline(s)) => resolve.push(
            UnresolvedPackage::parse("macro-input".as_ref(), &s)?,
            &Default::default(),
        )?,
        Some(Source::Path(s)) => parse(&root.join(&s))?,
        None => parse(&root.join("wit"))?,
    };

    Ok((resolve, pkg, files))
}

mod kw {
    syn::custom_keyword!(inline);
    syn::custom_keyword!(path);
    syn::custom_keyword!(tracing);
    syn::custom_keyword!(trappable_error_type);
    syn::custom_keyword!(world);
}

enum Opt {
    World(syn::LitStr),
    Path(syn::LitStr),
    Inline(syn::LitStr),
    Tracing(bool),
    Async(bool),
    TrappableErrorType(Vec<TrappableError>),
}

impl Parse for Opt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::path) {
            input.parse::<kw::path>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Path(input.parse()?))
        } else if l.peek(kw::inline) {
            input.parse::<kw::inline>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Inline(input.parse()?))
        } else if l.peek(kw::world) {
            input.parse::<kw::world>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::World(input.parse()?))
        } else if l.peek(kw::tracing) {
            input.parse::<kw::tracing>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Tracing(input.parse::<syn::LitBool>()?.value))
        } else if l.peek(Token![async]) {
            input.parse::<Token![async]>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Async(input.parse::<syn::LitBool>()?.value))
        } else if l.peek(kw::trappable_error_type) {
            input.parse::<kw::trappable_error_type>()?;
            input.parse::<Token![:]>()?;
            let contents;
            let _lbrace = braced!(contents in input);
            let fields: Punctuated<(String, String, String), Token![,]> =
                contents.parse_terminated(trappable_error_field_parse)?;
            Ok(Opt::TrappableErrorType(
                fields
                    .into_iter()
                    .map(|(wit_owner, wit_name, rust_name)| TrappableError {
                        wit_owner: Some(wit_owner),
                        wit_name,
                        rust_name,
                    })
                    .collect(),
            ))
        } else {
            Err(l.error())
        }
    }
}

fn trappable_error_field_parse(input: ParseStream<'_>) -> Result<(String, String, String)> {
    // Accept a Rust identifier or a string literal. This is required
    // because not all wit identifiers are Rust identifiers, so we can
    // smuggle the invalid ones inside quotes.
    fn ident_or_str(input: ParseStream<'_>) -> Result<String> {
        let l = input.lookahead1();
        if l.peek(syn::LitStr) {
            Ok(input.parse::<syn::LitStr>()?.value())
        } else if l.peek(syn::Ident) {
            Ok(input.parse::<syn::Ident>()?.to_string())
        } else {
            Err(l.error())
        }
    }

    let interface = ident_or_str(input)?;
    input.parse::<Token![::]>()?;
    let type_ = ident_or_str(input)?;
    input.parse::<Token![:]>()?;
    let rust_type = input.parse::<Ident>()?.to_string();
    Ok((interface, type_, rust_type))
}

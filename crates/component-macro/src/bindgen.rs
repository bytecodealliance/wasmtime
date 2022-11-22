use proc_macro2::{Span, TokenStream};
use std::path::{Path, PathBuf};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token;
use syn::Token;
use wasmtime_wit_bindgen::Opts;
use wit_parser::World;

#[derive(Default)]
pub struct Config {
    opts: Opts, // ...
    world: World,
    files: Vec<String>,
}

pub fn expand(input: &Config) -> Result<TokenStream> {
    let src = input.opts.generate(&input.world);
    let mut contents = src.parse::<TokenStream>().unwrap();

    // Include a dummy `include_str!` for any files we read so rustc knows that
    // we depend on the contents of those files.
    let cwd = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    for file in input.files.iter() {
        contents.extend(
            format!(
                "const _: &str = include_str!(r#\"{}\"#);\n",
                Path::new(&cwd).join(file).display()
            )
            .parse::<TokenStream>()
            .unwrap(),
        );
    }

    Ok(contents)
}

impl Parse for Config {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let call_site = Span::call_site();
        let mut world = None;
        let mut ret = Config::default();

        if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let fields = Punctuated::<Opt, Token![,]>::parse_terminated(&content)?;
            for field in fields.into_pairs() {
                match field.into_value() {
                    Opt::Path(path) => {
                        if world.is_some() {
                            return Err(Error::new(path.span(), "cannot specify second world"));
                        }
                        world = Some(ret.parse(path)?);
                    }
                    Opt::Inline(span, w) => {
                        if world.is_some() {
                            return Err(Error::new(span, "cannot specify second world"));
                        }
                        world = Some(w);
                    }
                    Opt::Tracing(val) => ret.opts.tracing = val,
                    Opt::Async(val) => ret.opts.async_ = val,
                }
            }
        } else {
            let s = input.parse::<syn::LitStr>()?;
            world = Some(ret.parse(s)?);
        }
        ret.world = world.ok_or_else(|| {
            Error::new(
                call_site,
                "must specify a `*.wit` file to generate bindings for",
            )
        })?;
        Ok(ret)
    }
}

impl Config {
    fn parse(&mut self, path: syn::LitStr) -> Result<World> {
        let span = path.span();
        let path = path.value();
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let path = manifest_dir.join(path);
        self.files.push(path.to_str().unwrap().to_string());
        World::parse_file(path).map_err(|e| Error::new(span, e))
    }
}

mod kw {
    syn::custom_keyword!(path);
    syn::custom_keyword!(inline);
    syn::custom_keyword!(tracing);
}

enum Opt {
    Path(syn::LitStr),
    Inline(Span, World),
    Tracing(bool),
    Async(bool),
}

impl Parse for Opt {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::path) {
            input.parse::<kw::path>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Path(input.parse()?))
        } else if l.peek(kw::inline) {
            let span = input.parse::<kw::inline>()?.span;
            input.parse::<Token![:]>()?;
            let s = input.parse::<syn::LitStr>()?;
            let world =
                World::parse("<macro-input>", &s.value()).map_err(|e| Error::new(s.span(), e))?;
            Ok(Opt::Inline(span, world))
        } else if l.peek(kw::tracing) {
            input.parse::<kw::tracing>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Tracing(input.parse::<syn::LitBool>()?.value))
        } else if l.peek(Token![async]) {
            input.parse::<Token![async]>()?;
            input.parse::<Token![:]>()?;
            Ok(Opt::Async(input.parse::<syn::LitBool>()?.value))
        } else {
            Err(l.error())
        }
    }
}

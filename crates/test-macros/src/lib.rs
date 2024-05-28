//! Wasmtime test macro.
//!
//! This macro is a helper to define tests that exercise multiple configuration
//! combinations for Wasmtime. Currently, only compiler strategies are
//! supported.
//!
//! Usage
//!
//! #[wasmtime_test(strategies(Cranelift, Winch))]
//! fn my_test(config: &mut Config) -> Result<()> {
//!    Ok(())
//! }
use proc_macro::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, token, Attribute, Ident, Result, ReturnType, Signature, Visibility,
};

/// Test configuration.
struct TestConfig {
    /// Supported compiler strategies.
    strategies: Vec<(String, Ident)>,
}

impl TestConfig {
    /// Validate the test configuration.
    /// Only the number of strategies is validated, as this avoid expansions of
    /// empty strategies or more strategies than supported.
    ///
    /// The supported strategies are validated inline when parsing.
    fn validate(&self) -> anyhow::Result<()> {
        if self.strategies.len() > 2 {
            Err(anyhow::anyhow!("Expected at most 2 strategies"))
        } else if self.strategies.len() == 0 {
            Err(anyhow::anyhow!("Expected at least 1 strategy"))
        } else {
            Ok(())
        }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self { strategies: vec![] }
    }
}

/// A generic function body represented as a braced [`TokenStream`].
struct Block {
    brace: token::Brace,
    rest: proc_macro2::TokenStream,
}

impl Parse for Block {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            brace: braced!(content in input),
            rest: content.parse()?,
        })
    }
}

impl ToTokens for Block {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.brace.surround(tokens, |tokens| {
            tokens.append_all(self.rest.clone());
        });
    }
}

/// Custom function parser.
/// Parses the function's attributes, visibility and signature, leaving the
/// block as an opaque [`TokenStream`].
struct Fn {
    attrs: Vec<Attribute>,
    visibility: Visibility,
    sig: Signature,
    body: Block,
}

impl Parse for Fn {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let visibility: Visibility = input.parse()?;
        let sig: Signature = input.parse()?;
        let body: Block = input.parse()?;

        Ok(Self {
            attrs,
            visibility,
            sig,
            body,
        })
    }
}

impl ToTokens for Fn {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for attr in &self.attrs {
            attr.to_tokens(tokens);
        }
        self.visibility.to_tokens(tokens);
        self.sig.to_tokens(tokens);
        self.body.to_tokens(tokens);
    }
}

#[proc_macro_attribute]
pub fn wasmtime_test(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut test_config = TestConfig::default();

    let config_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("strategies") {
            meta.parse_nested_meta(|meta| {
                if meta.path.is_ident("Winch") || meta.path.is_ident("Cranelift") {
                    let id = meta.path.require_ident()?.clone();
                    test_config.strategies.push((id.to_string(), id));
                    Ok(())
                } else {
                    Err(meta.error("Unknown strategy"))
                }
            })?;

            test_config.validate().map_err(|e| meta.error(e))
        } else {
            Err(meta.error("Unsupported attributes"))
        }
    });

    parse_macro_input!(attrs with config_parser);

    match expand(&test_config, parse_macro_input!(item as Fn)) {
        Ok(tok) => tok,
        Err(e) => e.into_compile_error().into(),
    }
}

fn expand(test_config: &TestConfig, func: Fn) -> Result<TokenStream> {
    let mut tests = vec![quote! { #func }];
    let attrs = &func.attrs;

    for (strategy_name, ident) in &test_config.strategies {
        // Winch currently only offers support for x64.
        let target = if strategy_name == "Winch" {
            quote! { #[cfg(target_arch = "x86_64")] }
        } else {
            quote! {}
        };
        let func_name = &func.sig.ident;
        let ret = match &func.sig.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, ty) => quote! { -> #ty },
        };
        let test_name = Ident::new(
            &format!("{}_{}", strategy_name.to_lowercase(), func_name),
            func_name.span(),
        );
        let tok = quote! {
            #[test]
            #target
            #(#attrs)*
            fn #test_name() #ret {
                let mut config = Config::new();
                config.strategy(Strategy::#ident);
                #func_name(&mut config)
            }
        };

        tests.push(tok);
    }
    Ok(quote! {
        #(#tests)*
    }
    .into())
}

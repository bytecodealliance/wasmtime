//! Wasmtime test macro.
//!
//! This macro is a helper to define tests that exercise multiple configuration
//! combinations for Wasmtime. Currently compiler strategies and wasm features
//! are supported.
//!
//! Usage
//!
//! To exclude a compiler strategy:
//!
//! ```rust
//! #[wasmtime_test(strategies(not(Winch)))]
//! fn my_test(config: &mut Config) -> Result<()> {
//!    Ok(())
//! }
//! ```
//!
//! To explicitly indicate that a wasm features is needed
//! ```
//! #[wasmtime_test(wasm_features(gc))]
//! fn my_wasm_gc_test(config: &mut Config) -> Result<()> {
//!   Ok(())
//! }
//! ```
//!
//! If the specified wasm feature is disabled by default, the macro will enable
//! the feature in the configuration passed to the test.
//!
//! If the wasm feature is not supported by any of the compiler strategies, no
//! tests will be generated for such strategy.
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    braced,
    meta::ParseNestedMeta,
    parse::{Parse, ParseStream},
    parse_macro_input, token, Attribute, Ident, Result, ReturnType, Signature, Visibility,
};

/// Test configuration.
struct TestConfig {
    /// Supported compiler strategies.
    strategies: Vec<Ident>,
    /// Known WebAssembly features that will be turned on by default in the
    /// resulting Config.
    /// The identifiers in this list are features that are off by default in
    /// Wasmtime's Config, which will be explicitly turned on for a given test.
    wasm_features: Vec<Ident>,
    /// Flag to track if there are Wasm features not supported by Winch.
    wasm_features_unsupported_by_winch: bool,
    /// The test attribute to use. Defaults to `#[test]`.
    test_attribute: Option<proc_macro2::TokenStream>,
}

impl TestConfig {
    fn strategies_from(&mut self, meta: &ParseNestedMeta) -> Result<()> {
        meta.parse_nested_meta(|meta| {
            if meta.path.is_ident("not") {
                meta.parse_nested_meta(|meta| {
                    if meta.path.is_ident("Winch") || meta.path.is_ident("Cranelift") {
                        let id = meta.path.require_ident()?.clone();
                        self.strategies.retain(|s| *s != id);
                        Ok(())
                    } else {
                        Err(meta.error("Unknown strategy"))
                    }
                })
            } else {
                Err(meta.error("Unknown identifier"))
            }
        })?;

        if self.strategies.len() == 0 {
            Err(meta.error("Expected at least one strategy"))
        } else {
            Ok(())
        }
    }

    fn wasm_features_from(&mut self, meta: &ParseNestedMeta) -> Result<()> {
        meta.parse_nested_meta(|meta| {
            if meta.path.is_ident("gc") || meta.path.is_ident("function_references") {
                let feature = meta.path.require_ident()?.clone();
                self.wasm_features.push(feature.clone());
                self.wasm_features_unsupported_by_winch = true;
                Ok(())
            } else if meta.path.is_ident("simd")
                || meta.path.is_ident("relaxed_simd")
                || meta.path.is_ident("reference_types")
                || meta.path.is_ident("tail_call")
                || meta.path.is_ident("threads")
            {
                self.wasm_features_unsupported_by_winch = true;
                Ok(())
            } else {
                Err(meta.error("Unsupported wasm feature"))
            }
        })?;

        if self.wasm_features.len() > 2 {
            return Err(meta.error("Expected at most 2 off-by-default wasm features"));
        }

        if self.wasm_features_unsupported_by_winch {
            self.strategies.retain(|s| s.to_string() != "Winch");
        }

        Ok(())
    }

    fn test_attribute_from(&mut self, meta: &ParseNestedMeta) -> Result<()> {
        let v: syn::LitStr = meta.value()?.parse()?;
        self.test_attribute = Some(v.value().parse()?);
        Ok(())
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            strategies: vec![
                Ident::new("Cranelift", Span::call_site()),
                Ident::new("Winch", Span::call_site()),
            ],
            wasm_features: vec![],
            wasm_features_unsupported_by_winch: false,
            test_attribute: None,
        }
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
            test_config.strategies_from(&meta)
        } else if meta.path.is_ident("wasm_features") {
            test_config.wasm_features_from(&meta)
        } else if meta.path.is_ident("with") {
            test_config.test_attribute_from(&meta)
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
    let mut tests = if test_config.strategies.len() == 1
        && test_config.strategies.get(0).map(|s| s.to_string()) == Some("Winch".to_string())
    {
        vec![quote! {
            // This prevents dead code warning when the macro is invoked as:
            //     #[wasmtime_test(strategies(not(Cranelift))]
            // Given that Winch only fully supports x86_64.
            #[allow(dead_code)]
            #func
        }]
    } else {
        vec![quote! { #func }]
    };
    let attrs = &func.attrs;

    let test_attr = test_config
        .test_attribute
        .clone()
        .unwrap_or_else(|| quote! { #[test] });

    for ident in &test_config.strategies {
        let strategy_name = ident.to_string();
        // Winch currently only offers support for x64.
        let target = if strategy_name == "Winch" {
            quote! { #[cfg(target_arch = "x86_64")] }
        } else {
            quote! {}
        };
        let (asyncness, await_) = if func.sig.asyncness.is_some() {
            (quote! { async }, quote! { .await })
        } else {
            (quote! {}, quote! {})
        };
        let func_name = &func.sig.ident;
        let ret = match &func.sig.output {
            ReturnType::Default => quote! {},
            ReturnType::Type(_, ty) => quote! { -> #ty },
        };
        let test_name = Ident::new(
            &format!("{}_{}", strategy_name.to_lowercase(), func_name),
            func_name.span(),
        );

        let config_setup = test_config.wasm_features.iter().map(|f| {
            let method_name = Ident::new(&format!("wasm_{f}"), f.span());
            quote! {
                config.#method_name(true);
            }
        });

        let tok = quote! {
            #test_attr
            #target
            #(#attrs)*
            #asyncness fn #test_name() #ret {
                let mut config = Config::new();
                config.strategy(Strategy::#ident);
                #(#config_setup)*
                #func_name(&mut config) #await_
            }
        };

        tests.push(tok);
    }
    Ok(quote! {
        #(#tests)*
    }
    .into())
}

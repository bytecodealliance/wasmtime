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
use quote::quote;
use syn::{
    parenthesized, parse::Parse, parse_macro_input, Ident, ItemFn, Result, ReturnType, Token,
};

#[proc_macro_attribute]
pub fn wasmtime_test(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut strategies: Vec<(String, Ident)> = vec![];

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("strategies") {
            let v = meta.input;
            let content;
            parenthesized!(content in v);
            strategies = content
                .parse_terminated(Ident::parse, Token![,])?
                .into_iter()
                .map(|v| (v.to_string(), v))
                .collect();

            if strategies.len() > 2 {
                return Err(meta.error("Expected at most 2 strategies"));
            }

            if strategies.is_empty() {
                return Err(meta.error("Expected at least 1 strategy"));
            }

            Ok(())
        } else {
            Err(meta.error("Unsupported attributes"))
        }
    });

    parse_macro_input!(attrs with parser);

    match expand(&strategies, parse_macro_input!(item as ItemFn)) {
        Ok(tok) => tok,
        Err(e) => e.into_compile_error().into(),
    }
}

fn expand(strategies: &[(String, Ident)], func: ItemFn) -> Result<TokenStream> {
    let mut tests = vec![quote! { #func }];
    let attrs = &func.attrs;

    for (strategy_name, ident) in strategies {
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

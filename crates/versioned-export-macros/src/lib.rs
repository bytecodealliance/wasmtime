//! This crate defines macros to easily define and use functions with a
//! versioned suffix, to facilitate using multiple versions of the same
//! crate that generate assembly.
//!
//! This crate contains a `enabled` feature making it easy to expose
//! use this versioned name logic conditionally with a feature.

#[cfg(feature = "enabled")]
use quote::ToTokens;

#[cfg(feature = "enabled")]
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "enabled")]
#[proc_macro_attribute]
pub fn versioned_fn(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut function = syn::parse_macro_input!(item as syn::ItemFn);

    function.sig.ident = version_ident(&function.sig.ident);

    function.to_token_stream().into()
}

#[cfg(not(feature = "enabled"))]
#[proc_macro_attribute]
pub fn versioned_fn(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    item
}

#[cfg(feature = "enabled")]
#[proc_macro]
pub fn versioned_ident(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = syn::parse_macro_input!(item as syn::Ident);
    version_ident(ident).to_token_stream().into()
}

#[cfg(not(feature = "enabled"))]
#[proc_macro]
pub fn versioned_ident(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    item
}

#[cfg(feature = "enabled")]
fn version_ident(ident: impl quote::IdentFragment) -> syn::Ident {
    quote::format_ident!("{}_{}", ident, VERSION.replace('.', "_"))
}

#[cfg(feature = "enabled")]
#[proc_macro]
pub fn versioned_str(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let s = syn::parse_macro_input!(item as syn::LitStr);
    syn::LitStr::new(
        format!("{}_{}", s.value(), VERSION.replace('.', "_")).as_str(),
        proc_macro2::Span::call_site(),
    )
    .to_token_stream()
    .into()
}

#[cfg(not(feature = "enabled"))]
#[proc_macro]
pub fn versioned_str(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    item
}

#[cfg(feature = "enabled")]
struct Function {
    attrs: Vec<syn::Attribute>,
    ident: syn::Ident,
    paren_token: syn::token::Paren,
    args: syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    output: syn::ReturnType,
    semi_token: syn::token::Semi,
}

#[cfg(feature = "enabled")]
impl syn::parse::Parse for Function {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let ident = input.parse()?;
        let content;
        let paren_token = syn::parenthesized!(content in input);
        let args = content.parse_terminated(syn::FnArg::parse, syn::Token![,])?;
        let output = input.parse()?;
        let semi_token = input.parse()?;
        Ok(Self {
            attrs,
            ident,
            paren_token,
            args,
            output,
            semi_token,
        })
    }
}

#[cfg(feature = "enabled")]
impl ToTokens for Function {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.attrs.iter().for_each(|attr| attr.to_tokens(tokens));
        self.ident.to_tokens(tokens);
        self.paren_token
            .surround(tokens, |tokens| self.args.to_tokens(tokens));
        self.output.to_tokens(tokens);
        self.semi_token.to_tokens(tokens);
    }
}

#[cfg(feature = "enabled")]
struct Functions(Vec<Function>);

#[cfg(feature = "enabled")]
impl syn::parse::Parse for Functions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut functions = Vec::new();

        while !input.is_empty() {
            functions.push(input.parse()?);
        }

        Ok(Self(functions))
    }
}

#[cfg(feature = "enabled")]
#[proc_macro_attribute]
pub fn versioned_foreach_builtin_function(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut mac = syn::parse_macro_input!(item as syn::Macro);
    let exported = syn::parse_macro_input!(attr as syn::LitBool);

    if exported.value && matches!(mac.delimiter, syn::MacroDelimiter::Brace(_)) {
        let functions: syn::Result<Functions> = mac.parse_body();

        match functions {
            Ok(functions) => {
                mac.tokens = functions
                    .0
                    .into_iter()
                    .map(|mut function| {
                        function.ident = version_ident(&function.ident);
                        function.to_token_stream()
                    })
                    .collect();
            }
            Err(error) => {
                return error.to_compile_error().into();
            }
        }
    }

    mac.to_token_stream().into()
}

#[cfg(not(feature = "enabled"))]
#[proc_macro_attribute]
pub fn versioned_foreach_builtin_function(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    item
}

#[cfg(feature = "enabled")]
#[proc_macro]
pub fn versioned_use(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut item = syn::parse_macro_input!(item as syn::ItemUse);
    if let syn::UseTree::Rename(use_rename) = &mut item.tree {
        use_rename.ident = version_ident(&use_rename.ident);
        use_rename.rename = version_ident(&use_rename.rename);
    }
    item.to_token_stream().into()
}

#[cfg(not(feature = "enabled"))]
#[proc_macro]
pub fn versioned_use(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    item
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "enabled")]
    #[test]
    fn functions_parse_test() {
        let tokens = quote::quote! {
            /// Returns an index for wasm's `memory.grow` builtin function.
            memory32_grow(vmctx: vmctx, delta: i64, index: i32) -> pointer;
            /// Returns an index for wasm's `table.copy` when both tables are locally
            /// defined.
            table_copy(vmctx: vmctx, dst_index: i32, src_index: i32, dst: i32, src: i32, len: i32);
        };

        let functions: super::Functions = syn::parse2(tokens).expect("Error parsing");

        assert_eq!(2, functions.0.len());

        let first = &functions.0[0];
        assert_eq!(first.ident, "memory32_grow");
        assert_eq!(first.args.len(), 3);
        assert!(matches!(first.output, syn::ReturnType::Type(_, _)));

        let second = &functions.0[1];
        assert_eq!(second.ident, "table_copy");
        assert_eq!(second.args.len(), 6);
        assert!(matches!(second.output, syn::ReturnType::Default));
    }
}

extern crate proc_macro;

use crate::proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::DeriveInput;
use syn::Error;
use syn::{parse_macro_input, Ident, Result};

mod child_nodes;
mod into_dyn_ast_ref;
mod operator;
mod span;

#[proc_macro_derive(PeepmaticOperator, attributes(peepmatic))]
pub fn operator(input: TokenStream) -> TokenStream {
    operator::derive_operator(input)
}

#[proc_macro_derive(Ast, attributes(peepmatic))]
pub fn derive_ast(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let span_impl = match span::derive_span(&input) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };

    let child_nodes_impl = match child_nodes::derive_child_nodes(&input) {
        Ok(c) => c,
        Err(e) => return e.to_compile_error().into(),
    };

    let into_dyn_ast_ref_impl = match into_dyn_ast_ref::derive_into_dyn_ast_ref(&input) {
        Ok(n) => n,
        Err(e) => return e.to_compile_error().into(),
    };

    let expanded = quote! {
        #span_impl
        #child_nodes_impl
        #into_dyn_ast_ref_impl
    };

    TokenStream::from(expanded)
}

#[derive(Default)]
pub(crate) struct PeepmaticOpts {
    // `ChildNodes` options.
    skip_child: bool,
    flatten: bool,

    // `From<&'a Self> for DynAstRef<'a>` options.
    no_into_dyn_node: bool,

    // Peepmatic operator options.
    immediates_paren: syn::token::Paren,
    immediates: Vec<syn::Ident>,
    params_paren: syn::token::Paren,
    params: Vec<syn::Ident>,
    result: Option<syn::Ident>,
}

impl Parse for PeepmaticOpts {
    fn parse(input: ParseStream) -> Result<Self> {
        enum Attr {
            Immediates(syn::token::Paren, Vec<syn::Ident>),
            Params(syn::token::Paren, Vec<syn::Ident>),
            Result(syn::Ident),
            NoIntoDynNode,
            SkipChild,
            Flatten,
        }

        let attrs = Punctuated::<_, syn::token::Comma>::parse_terminated(input)?;
        let mut ret = PeepmaticOpts::default();
        for attr in attrs {
            match attr {
                Attr::Immediates(paren, imms) => {
                    ret.immediates_paren = paren;
                    ret.immediates = imms;
                }
                Attr::Params(paren, ps) => {
                    ret.params_paren = paren;
                    ret.params = ps;
                }
                Attr::Result(r) => ret.result = Some(r),
                Attr::NoIntoDynNode => ret.no_into_dyn_node = true,
                Attr::SkipChild => ret.skip_child = true,
                Attr::Flatten => ret.flatten = true,
            }
        }

        return Ok(ret);

        impl Parse for Attr {
            fn parse(input: ParseStream) -> Result<Self> {
                let attr: Ident = input.parse()?;
                if attr == "immediates" {
                    let inner;
                    let paren = syn::parenthesized!(inner in input);
                    let imms = Punctuated::<_, syn::token::Comma>::parse_terminated(&inner)?;
                    return Ok(Attr::Immediates(paren, imms.into_iter().collect()));
                }
                if attr == "params" {
                    let inner;
                    let paren = syn::parenthesized!(inner in input);
                    let params = Punctuated::<_, syn::token::Comma>::parse_terminated(&inner)?;
                    return Ok(Attr::Params(paren, params.into_iter().collect()));
                }
                if attr == "result" {
                    let inner;
                    syn::parenthesized!(inner in input);
                    return Ok(Attr::Result(syn::Ident::parse(&inner)?));
                }
                if attr == "skip_child" {
                    return Ok(Attr::SkipChild);
                }
                if attr == "no_into_dyn_node" {
                    return Ok(Attr::NoIntoDynNode);
                }
                if attr == "flatten" {
                    return Ok(Attr::Flatten);
                }
                return Err(Error::new(attr.span(), "unexpected attribute"));
            }
        }
    }
}

fn peepmatic_attrs(attrs: &mut Vec<syn::Attribute>) -> TokenStream {
    let mut ret = proc_macro2::TokenStream::new();
    let ident = syn::Path::from(syn::Ident::new("peepmatic", Span::call_site()));
    for i in (0..attrs.len()).rev() {
        if attrs[i].path != ident {
            continue;
        }
        let attr = attrs.remove(i);
        let group = match attr.tokens.into_iter().next().unwrap() {
            proc_macro2::TokenTree::Group(g) => g,
            _ => panic!("#[peepmatic(...)] expected"),
        };
        ret.extend(group.stream());
        ret.extend(quote! { , });
    }
    return ret.into();
}

impl PeepmaticOpts {
    pub(crate) fn from_attrs(attrs: &mut Vec<syn::Attribute>) -> syn::Result<Self> {
        syn::parse(peepmatic_attrs(attrs))
    }
}

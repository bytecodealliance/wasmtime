//! Implementation of the `#[peepmatic]` macro for the `Operator` AST node.

use crate::proc_macro::TokenStream;
use crate::PeepmaticOpts;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::DeriveInput;
use syn::Error;
use syn::{parse_macro_input, Result};

pub fn derive_operator(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let variants = match get_enum_variants(&input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let arity = match create_arity(&variants) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let num_operators = variants.len();
    let type_methods = create_type_methods(&variants);
    let parse_impl = create_parse_impl(&input.ident, &variants);
    let display_impl = create_display_impl(&input.ident, &variants);
    let try_from_u32_impl = create_try_from_u32_impl(&input.ident, &variants);
    let ident = &input.ident;

    let expanded = quote! {
        impl #ident {
            #arity
            #type_methods

            /// Get the total number of different operators.
            pub const fn num_operators() -> usize {
                #num_operators
            }
        }

        #display_impl
        #try_from_u32_impl
        #parse_impl
    };

    // eprintln!("{}", expanded);
    TokenStream::from(expanded)
}

fn get_enum_variants(input: &DeriveInput) -> Result<Vec<OperatorVariant>> {
    let en = match &input.data {
        syn::Data::Enum(en) => en,
        syn::Data::Struct(_) => {
            panic!("can only put #[peepmatic] on an enum; found it on a struct")
        }
        syn::Data::Union(_) => panic!("can only put #[peepmatic] on an enum; found it on a union"),
    };
    en.variants
        .iter()
        .cloned()
        .map(|mut variant| {
            Ok(OperatorVariant {
                opts: PeepmaticOpts::from_attrs(&mut variant.attrs)?,
                syn: variant,
            })
        })
        .collect()
}

struct OperatorVariant {
    syn: syn::Variant,
    opts: PeepmaticOpts,
}

fn create_arity(variants: &[OperatorVariant]) -> Result<impl quote::ToTokens> {
    let mut imm_arities = vec![];
    let mut params_arities = vec![];

    for v in variants {
        let variant = &v.syn.ident;

        let imm_arity = v.opts.immediates.len();
        if imm_arity > std::u8::MAX as usize {
            return Err(Error::new(
                v.opts.immediates_paren.span,
                "cannot have more than u8::MAX immediates",
            ));
        }
        let imm_arity = imm_arity as u8;

        imm_arities.push(quote! {
            Self::#variant => #imm_arity,
        });

        let params_arity = v.opts.params.len();
        if params_arity > std::u8::MAX as usize {
            return Err(Error::new(
                v.opts.params_paren.span,
                "cannot have more than u8::MAX params",
            ));
        }
        let params_arity = params_arity as u8;

        params_arities.push(quote! {
            Self::#variant => #params_arity,
        });
    }

    Ok(quote! {
        /// Get the number of immediates that this operator has.
        pub fn immediates_arity(&self) -> u8 {
            match *self {
                #( #imm_arities )*
            }
        }

        /// Get the number of parameters that this operator takes.
        pub fn params_arity(&self) -> u8 {
            match *self {
                #( #params_arities )*
            }
        }
    })
}

fn create_type_methods(variants: &[OperatorVariant]) -> impl quote::ToTokens {
    let mut result_types = vec![];
    let mut imm_types = vec![];
    let mut param_types = vec![];

    for v in variants {
        let variant = &v.syn.ident;

        let result_ty = v.opts.result.as_ref().unwrap_or_else(|| {
            panic!(
                "must define #[peepmatic(result(..))] on operator `{}`",
                variant
            )
        });
        result_types.push(quote! {
            Self::#variant => {
                context.#result_ty(span)
            }
        });

        let imm_tys = match &v.opts.immediates[..] {
            [] => quote! {},
            [ty, rest @ ..] => {
                let rest = rest.iter().map(|ty| {
                    quote! { .chain(::std::iter::once(context.#ty(span))) }
                });
                quote! {
                    types.extend(::std::iter::once(context.#ty(span))#( #rest )*);
                }
            }
        };
        imm_types.push(quote! {
            Self::#variant => {
                #imm_tys
            }
        });

        let param_tys = match &v.opts.params[..] {
            [] => quote! {},
            [ty, rest @ ..] => {
                let rest = rest.iter().map(|ty| {
                    quote! { .chain(::std::iter::once(context.#ty(span))) }
                });
                quote! {
                    types.extend(::std::iter::once(context.#ty(span))#( #rest )*);
                }
            }
        };
        param_types.push(quote! {
            Self::#variant => {
                #param_tys
            }
        });
    }

    quote! {
        /// Get the result type of this operator.
        #[cfg(feature = "construct")]
        pub fn result_type<'a, C>(
            &self,
            context: &mut C,
            span: wast::Span,
        ) -> C::TypeVariable
        where
            C: 'a + TypingContext<'a>,
        {
            match *self {
                #( #result_types )*
            }
        }

        /// Get the immediate types of this operator.
        #[cfg(feature = "construct")]
        pub fn immediate_types<'a, C>(
            &self,
            context: &mut C,
            span: wast::Span,
            types: &mut impl Extend<C::TypeVariable>,
        )
        where
            C: 'a + TypingContext<'a>,
        {
            match *self {
                #( #imm_types )*
            }
        }

        /// Get the parameter types of this operator.
        #[cfg(feature = "construct")]
        pub fn param_types<'a, C>(
            &self,
            context: &mut C,
            span: wast::Span,
            types: &mut impl Extend<C::TypeVariable>,
        )
        where
            C: 'a + TypingContext<'a>,
        {
            match *self {
                #( #param_types )*
            }
        }
    }
}

fn snake_case(s: &str) -> String {
    let mut t = String::with_capacity(s.len() + 1);
    for (i, ch) in s.chars().enumerate() {
        if i != 0 && ch.is_uppercase() {
            t.push('_');
        }
        t.extend(ch.to_lowercase());
    }
    t
}

fn create_parse_impl(ident: &syn::Ident, variants: &[OperatorVariant]) -> impl quote::ToTokens {
    let token_defs = variants.iter().map(|v| {
        let tok = snake_case(&v.syn.ident.to_string());
        let tok = Ident::new(&tok, Span::call_site());
        quote! {
            wast::custom_keyword!(#tok);
        }
    });

    let parses = variants.iter().map(|v| {
        let tok = snake_case(&v.syn.ident.to_string());
        let tok = Ident::new(&tok, Span::call_site());
        let ident = &v.syn.ident;
        quote! {
            if p.peek::<#tok>() {
                p.parse::<#tok>()?;
                return Ok(Self::#ident);
            }
        }
    });

    let expected = format!("expected {}", ident);

    quote! {
        #[cfg(feature = "construct")]
        impl<'a> wast::parser::Parse<'a> for #ident {
            fn parse(p: wast::parser::Parser<'a>) -> wast::parser::Result<Self> {
                #( #token_defs )*

                #( #parses )*

                Err(p.error(#expected))
            }
        }
    }
}

fn create_display_impl(ident: &syn::Ident, variants: &[OperatorVariant]) -> impl quote::ToTokens {
    let displays = variants.iter().map(|v| {
        let variant = &v.syn.ident;
        let snake = snake_case(&v.syn.ident.to_string());
        quote! {
            Self::#variant => write!(f, #snake),
        }
    });

    quote! {
        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #( #displays )*
                }
            }
        }
    }
}

fn create_try_from_u32_impl(
    ident: &syn::Ident,
    variants: &[OperatorVariant],
) -> impl quote::ToTokens {
    let matches = variants.iter().map(|v| {
        let variant = &v.syn.ident;
        quote! {
            x if Self::#variant as u32 == x => Ok(Self::#variant),
        }
    });

    let error_msg = format!("value is not an `{}`", ident);

    quote! {
        impl std::convert::TryFrom<u32> for #ident {
            type Error = &'static str;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                match value {
                    #( #matches )*
                    _ => Err(#error_msg)
                }
            }
        }
    }
}

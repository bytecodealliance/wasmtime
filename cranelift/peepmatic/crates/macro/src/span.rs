use quote::quote;
use syn::DeriveInput;
use syn::{parse_quote, GenericParam, Generics, Result};

pub fn derive_span(input: &DeriveInput) -> Result<impl quote::ToTokens> {
    let ty = &input.ident;

    let body = match &input.data {
        syn::Data::Struct(_) => quote! { self.span },
        syn::Data::Enum(e) => {
            let variants = e.variants.iter().map(|v| match v.fields {
                syn::Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {
                    let variant = &v.ident;
                    quote! { #ty::#variant(x) => x.span(), }
                }
                _ => panic!(
                    "derive(Ast) on enums only supports variants with a single, unnamed field"
                ),
            });
            quote! {
                match self {
                    #( #variants )*
                }
            }
        }
        syn::Data::Union(_) => panic!("derive(Ast) can only be used with structs and enums, not unions"),
    };

    let generics = add_span_trait_bounds(input.generics.clone());
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics Span for #ty #ty_generics #where_clause {
            #[inline]
            fn span(&self) -> wast::Span {
                #body
            }
        }
    })
}

// Add a bound `T: Span` to every type parameter `T`.
fn add_span_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(Span));
        }
    }
    generics
}

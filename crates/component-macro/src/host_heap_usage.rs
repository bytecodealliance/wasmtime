use crate::component::{ComponentAttr, default_wasmtime_crate};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Fields, Result, parse_quote};

fn find_wasmtime_crate(attrs: &[syn::Attribute]) -> Result<syn::Path> {
    let mut wasmtime_crate = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("component")) {
        if let ComponentAttr::WasmtimeCrate(c) = attr.parse_args()? {
            wasmtime_crate = Some(c);
        }
    }
    Ok(wasmtime_crate.unwrap_or_else(default_wasmtime_crate))
}

fn collect_variants<'a>(input: &'a DeriveInput) -> Result<Vec<(TokenStream, &'a Fields)>> {
    match &input.data {
        Data::Struct(data) => Ok(vec![(quote!(Self), &data.fields)]),
        Data::Enum(data) => Ok(data
            .variants
            .iter()
            .map(|v| {
                let ident = &v.ident;
                (quote!(Self::#ident), &v.fields)
            })
            .collect()),
        Data::Union(_) => Err(Error::new_spanned(input, "unions are not supported")),
    }
}

fn type_mentions_ident(ty: &syn::Type, ident: &syn::Ident) -> bool {
    match ty {
        syn::Type::Path(p) => {
            if p.qself.is_none() && p.path.segments.first().is_some_and(|s| s.ident == *ident) {
                return true;
            }
            p.qself
                .as_ref()
                .is_some_and(|q| type_mentions_ident(&q.ty, ident))
                || p.path.segments.iter().any(|s| match &s.arguments {
                    syn::PathArguments::None => false,
                    syn::PathArguments::AngleBracketed(a) => a.args.iter().any(|arg| match arg {
                        syn::GenericArgument::Type(t) => type_mentions_ident(t, ident),
                        syn::GenericArgument::AssocType(b) => type_mentions_ident(&b.ty, ident),
                        _ => false,
                    }),
                    syn::PathArguments::Parenthesized(_) => false,
                })
        }
        syn::Type::Array(a) => type_mentions_ident(&a.elem, ident),
        syn::Type::Slice(s) => type_mentions_ident(&s.elem, ident),
        syn::Type::Reference(r) => type_mentions_ident(&r.elem, ident),
        syn::Type::Ptr(p) => type_mentions_ident(&p.elem, ident),
        syn::Type::Tuple(t) => t.elems.iter().any(|e| type_mentions_ident(e, ident)),
        syn::Type::Paren(p) => type_mentions_ident(&p.elem, ident),
        syn::Type::Group(g) => type_mentions_ident(&g.elem, ident),
        _ => false,
    }
}

fn field_where_clause(
    input: &DeriveInput,
    variants: &[(TokenStream, &Fields)],
    field_bound: &syn::Path,
    bound_all_fields: bool,
) -> syn::WhereClause {
    let self_ident = &input.ident;
    let type_params: Vec<_> = input.generics.type_params().map(|p| &p.ident).collect();
    let mut where_clause = input
        .generics
        .where_clause
        .clone()
        .unwrap_or_else(|| parse_quote!(where));
    for (_, fields) in variants {
        for field in fields.iter() {
            let ty = &field.ty;
            if type_mentions_ident(ty, self_ident) {
                for param in &type_params {
                    if type_mentions_ident(ty, param) {
                        where_clause
                            .predicates
                            .push(parse_quote!(#param: #field_bound));
                    }
                }
            } else if bound_all_fields
                || type_params
                    .iter()
                    .any(|param| type_mentions_ident(ty, param))
            {
                where_clause
                    .predicates
                    .push(parse_quote!(#ty: #field_bound));
            }
        }
    }
    where_clause
}

pub fn expand_host_heap_usage(input: &DeriveInput) -> Result<TokenStream> {
    let wasmtime_crate = find_wasmtime_crate(&input.attrs)?;
    let variants = collect_variants(input)?;
    let bound = parse_quote!(#wasmtime_crate::component::HostHeapUsage);
    let where_clause = field_where_clause(input, &variants, &bound, false);

    let arms = variants.iter().map(|(path, fields)| {
        let bindings: Vec<_> = (0..fields.len()).map(|i| format_ident!("f{i}")).collect();
        let pattern = match fields {
            Fields::Named(f) => {
                let names = f.named.iter().map(|f| &f.ident);
                quote!(#path { #(#names: ref #bindings),* })
            }
            Fields::Unnamed(_) => quote!(#path(#(ref #bindings),*)),
            Fields::Unit => quote!(#path),
        };
        quote! {
            #pattern => 0 #(+ #wasmtime_crate::component::HostHeapUsage::heap_size_of_children(#bindings))*,
        }
    });

    let ident = &input.ident;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #bound for #ident #ty_generics #where_clause {
            fn heap_size_of_children(&self) -> usize {
                match *self {
                    #(#arms)*
                }
            }
        }
    })
}

pub fn expand_fixed_host_heap_usage(input: &DeriveInput) -> Result<TokenStream> {
    let wasmtime_crate = find_wasmtime_crate(&input.attrs)?;
    let variants = collect_variants(input)?;
    let field_bound = parse_quote!(#wasmtime_crate::component::FixedHostHeapField);
    let where_clause = field_where_clause(input, &variants, &field_bound, true);
    let ident = &input.ident;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #wasmtime_crate::component::FixedHostHeapUsage for #ident #ty_generics #where_clause {}
    })
}

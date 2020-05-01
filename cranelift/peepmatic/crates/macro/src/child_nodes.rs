use quote::quote;
use syn::DeriveInput;
use syn::{parse_quote, GenericParam, Generics, Result};

pub fn derive_child_nodes(input: &DeriveInput) -> Result<impl quote::ToTokens> {
    let children = get_child_nodes(&input.data)?;
    let name = &input.ident;
    let generics = add_trait_bounds(input.generics.clone());
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ChildNodes<'a, 'a> for #name #ty_generics #where_clause {
            fn child_nodes(&'a self, children: &mut impl Extend<DynAstRef<'a>>) {
                #children
            }
        }
    })
}

fn get_child_nodes(data: &syn::Data) -> Result<impl quote::ToTokens> {
    match data {
        syn::Data::Struct(s) => {
            let mut fields = vec![];

            match &s.fields {
                syn::Fields::Named(n) => {
                    for f in n.named.iter() {
                        let opts = crate::PeepmaticOpts::from_attrs(&mut f.attrs.clone())?;

                        if opts.skip_child {
                            continue;
                        }

                        let field_name = f.ident.as_ref().unwrap();
                        if opts.flatten {
                            fields.push(quote! {
                                self.#field_name.iter().map(DynAstRef::from)
                            });
                        } else {
                            fields.push(quote! {
                                std::iter::once(DynAstRef::from(&self.#field_name))
                            });
                        }
                    }
                }
                syn::Fields::Unnamed(u) => {
                    for (i, f) in u.unnamed.iter().enumerate() {
                        let opts = crate::PeepmaticOpts::from_attrs(&mut f.attrs.clone())?;
                        if opts.skip_child {
                            continue;
                        }
                        if opts.flatten {
                            return Err(syn::Error::new(
                                u.paren_token.span,
                                "#[peepmatic(flatten)] is only allowed with named fields",
                            ));
                        }
                        fields.push(quote! {
                            std::iter::once(DynAstRef::from(&self.#i))
                        });
                    }
                }
                syn::Fields::Unit => {}
            }

            Ok(match fields.as_slice() {
                [] => quote! { let _ = children; },
                [f, rest @ ..] => {
                    let rest = rest.iter().map(|f| {
                        quote! {
                            .chain(#f)
                        }
                    });
                    quote! {
                        children.extend( #f #( #rest )* );
                    }
                }
            })
        }
        syn::Data::Enum(e) => {
            let mut match_arms = vec![];
            for v in e.variants.iter() {
                match v.fields {
                    syn::Fields::Unnamed(ref u) if u.unnamed.len() == 1 => {
                        let variant = &v.ident;
                        match_arms.push(quote! {
                            Self::#variant(x) => children.extend(Some(x.into())),
                        });
                    }
                    _ => panic!("#[derive(ChildNodes)] only supports enums whose variants all ahve a single unnamed field")
                }
            }
            Ok(quote! {
                match self {
                    #( #match_arms )*
                }
            })
        }
        syn::Data::Union(_) => panic!("#[derive(ChildNodes)] is not supported on unions"),
    }
}

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(ChildNodes<'a, 'a>));
        }
    }
    generics
}

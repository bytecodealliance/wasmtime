use proc_macro2::{Literal, TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Error, Result};

#[derive(Debug)]
enum Style {
    Record,
    Variant,
    Enum,
    Union,
}

fn find_style(input: &DeriveInput) -> Result<Style> {
    let mut style = None;

    for attribute in &input.attrs {
        if attribute.path.leading_colon.is_some() || attribute.path.segments.len() != 1 {
            continue;
        }

        let ident = &attribute.path.segments[0].ident;

        if "component" != &ident.to_string() {
            continue;
        }

        let syntax_error = || {
            Err(Error::new_spanned(
                &attribute.tokens,
                "expected `component(<style>)` syntax",
            ))
        };

        let style_string = if let [TokenTree::Group(group)] =
            &attribute.tokens.clone().into_iter().collect::<Vec<_>>()[..]
        {
            if let [TokenTree::Ident(style)] = &group.stream().into_iter().collect::<Vec<_>>()[..] {
                style.to_string()
            } else {
                return syntax_error();
            }
        } else {
            return syntax_error();
        };

        if style.is_some() {
            return Err(Error::new(ident.span(), "duplicate `component` attribute"));
        }

        style = Some(match style_string.as_ref() {
            "record" => Style::Record,
            "variant" => Style::Variant,
            "enum" => Style::Enum,
            "union" => Style::Union,
            "flags" => {
                return Err(Error::new_spanned(
                    &attribute.tokens,
                    "`flags` not allowed here; \
                     use `wasmtime::component::flags!` macro to define `flags` types",
                ))
            }
            _ => {
                return Err(Error::new_spanned(
                    &attribute.tokens,
                    "unrecognized component type keyword \
                     (expected `record`, `variant`, `enum`, or `union`)",
                ))
            }
        });
    }

    style.ok_or_else(|| Error::new_spanned(input, "missing `component` attribute"))
}

fn find_rename(field: &syn::Field) -> Result<Option<Literal>> {
    let mut name = None;

    for attribute in &field.attrs {
        if attribute.path.leading_colon.is_some() || attribute.path.segments.len() != 1 {
            continue;
        }

        let ident = &attribute.path.segments[0].ident;

        if "component" != &ident.to_string() {
            continue;
        }

        let syntax_error = || {
            Err(Error::new_spanned(
                &attribute.tokens,
                "expected `component(name = <name literal>)` syntax",
            ))
        };

        let name_literal = if let [TokenTree::Group(group)] =
            &attribute.tokens.clone().into_iter().collect::<Vec<_>>()[..]
        {
            match &group.stream().into_iter().collect::<Vec<_>>()[..] {
                [TokenTree::Ident(key), TokenTree::Punct(op), TokenTree::Literal(literal)]
                    if "name" == &key.to_string() && '=' == op.as_char() =>
                {
                    literal.clone()
                }
                _ => return syntax_error(),
            }
        } else {
            return syntax_error();
        };

        if name.is_some() {
            return Err(Error::new(ident.span(), "duplicate field rename attribute"));
        }

        name = Some(name_literal);
    }

    Ok(name)
}

fn add_trait_bounds(generics: &syn::Generics) -> syn::Generics {
    let mut generics = generics.clone();
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(parse_quote!(wasmtime::component::ComponentType));
        }
    }
    generics
}

trait Expander {
    fn expand_record(&self, input: &DeriveInput, fields: &syn::FieldsNamed) -> Result<TokenStream>;

    // TODO: expand_variant, expand_enum, etc.
}

fn expand(expander: &dyn Expander, input: DeriveInput) -> Result<TokenStream> {
    match find_style(&input)? {
        Style::Record => expand_record(expander, input),

        style => Err(Error::new_spanned(input, format!("todo: expand {style:?}"))),
    }
}

fn expand_record(expander: &dyn Expander, input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    let body = if let Data::Struct(body) = &input.data {
        body
    } else {
        return Err(Error::new(
            name.span(),
            "`record` component types can only be derived for `struct`s",
        ));
    };

    match &body.fields {
        syn::Fields::Named(fields) => expander.expand_record(&input, fields),

        syn::Fields::Unnamed(_) | syn::Fields::Unit => Err(Error::new(
            name.span(),
            "`record` component types can only be derived for `struct`s with named fields",
        )),
    }
}

#[proc_macro_derive(Lift, attributes(component))]
pub fn lift(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(&LiftExpander, parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

struct LiftExpander;

impl Expander for LiftExpander {
    fn expand_record(&self, input: &DeriveInput, fields: &syn::FieldsNamed) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut lifts = TokenStream::new();
        let mut loads = TokenStream::new();

        for syn::Field { ty, ident, .. } in &fields.named {
            lifts.extend(quote!(#ident: <#ty as wasmtime::component::Lift>::lift(
                    store, options, &src.#ident
                )?,));

            loads.extend(quote!(#ident: <#ty as wasmtime::component::Lift>::load(
                    memory,
                    &bytes
                        [#internal::next_field::<#ty>(&mut offset)..]
                        [..<#ty as wasmtime::component::ComponentType>::size()]
                )?,));
        }

        let name = &input.ident;
        let generics = add_trait_bounds(&input.generics);
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let expanded = quote! {
            unsafe impl #impl_generics wasmtime::component::Lift for #name #ty_generics #where_clause {
                #[inline]
                fn lift(
                    store: &#internal::StoreOpaque,
                    options: &#internal::Options,
                    src: &Self::Lower,
                ) -> #internal::anyhow::Result<Self> {
                    Ok(Self {
                        #lifts
                    })
                }

                #[inline]
                fn load(memory: &#internal::Memory, bytes: &[u8]) -> #internal::anyhow::Result<Self> {
                    debug_assert!(
                        (bytes.as_ptr() as usize)
                            % (<Self as wasmtime::component::ComponentType>::align() as usize)
                            == 0
                    );
                    let mut offset = 0;
                    Ok(Self {
                        #loads
                    })
                }
            }
        };

        Ok(expanded)
    }
}

#[proc_macro_derive(Lower, attributes(component))]
pub fn lower(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(&LowerExpander, parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

struct LowerExpander;

impl Expander for LowerExpander {
    fn expand_record(&self, input: &DeriveInput, fields: &syn::FieldsNamed) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut lowers = TokenStream::new();
        let mut stores = TokenStream::new();

        for syn::Field { ty, ident, .. } in &fields.named {
            lowers.extend(quote!(wasmtime::component::Lower::lower(
                    &self.#ident, store, options, #internal::map_maybe_uninit!(dst.#ident)
                )?;));

            stores.extend(quote!(wasmtime::component::Lower::store(
                    &self.#ident, memory, #internal::next_field::<#ty>(&mut offset)
                )?;));
        }

        let name = &input.ident;
        let generics = add_trait_bounds(&input.generics);
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let expanded = quote! {
            unsafe impl #impl_generics wasmtime::component::Lower for #name #ty_generics #where_clause {
                #[inline]
                fn lower<T>(
                    &self,
                    store: &mut wasmtime::StoreContextMut<T>,
                    options: &#internal::Options,
                    dst: &mut std::mem::MaybeUninit<Self::Lower>,
                ) -> #internal::anyhow::Result<()> {
                    #lowers
                    Ok(())
                }

                #[inline]
                fn store<T>(
                    &self,
                    memory: &mut #internal::MemoryMut<'_, T>,
                    mut offset: usize
                ) -> #internal::anyhow::Result<()> {
                    #stores
                    Ok(())
                }
            }
        };

        Ok(expanded)
    }
}

#[proc_macro_derive(ComponentType, attributes(component))]
pub fn component_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(
        &ComponentTypeExpander,
        parse_macro_input!(input as DeriveInput),
    )
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

struct ComponentTypeExpander;

impl Expander for ComponentTypeExpander {
    fn expand_record(&self, input: &DeriveInput, fields: &syn::FieldsNamed) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut field_names_and_checks = TokenStream::new();
        let mut lower_field_declarations = TokenStream::new();
        let mut sizes = TokenStream::new();
        let mut unique_types = HashSet::new();

        for field @ syn::Field { ty, ident, .. } in &fields.named {
            lower_field_declarations
                .extend(quote!(#ident: <#ty as wasmtime::component::ComponentType>::Lower,));

            let literal = find_rename(field)?
                .unwrap_or_else(|| Literal::string(&ident.as_ref().unwrap().to_string()));

            field_names_and_checks.extend(
                quote!((#literal, <#ty as wasmtime::component::ComponentType>::typecheck),),
            );

            sizes.extend(quote!(#internal::next_field::<#ty>(&mut size);));

            unique_types.insert(ty);
        }

        let alignments = unique_types
            .into_iter()
            .map(|ty| {
                quote!(align = align.max(
                        <#ty as wasmtime::component::ComponentType>::align()
                    );)
            })
            .collect::<TokenStream>();

        let name = &input.ident;
        let generics = add_trait_bounds(&input.generics);
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let lower = format_ident!("_Lower{}", name);

        let expanded = quote! {
            #[doc(hidden)]
            #[derive(Clone, Copy)]
            #[repr(C)]
            pub struct #lower {
                #lower_field_declarations
            }

            unsafe impl #impl_generics wasmtime::component::ComponentType for #name #ty_generics #where_clause {
                type Lower = #lower;

                #[inline]
                fn typecheck(
                    ty: &#internal::InterfaceType,
                    types: &#internal::ComponentTypes,
                ) -> #internal::anyhow::Result<()> {
                    #internal::typecheck_record(ty, types, &[#field_names_and_checks])
                }

                #[inline]
                fn size() -> usize {
                    let mut size = 0;
                    #sizes
                    size
                }

                #[inline]
                fn align() -> u32 {
                    let mut align = 1;
                    #alignments
                    align
                }
            }
        };

        Ok(quote!(const _: () = { #expanded };))
    }
}

use proc_macro2::{Literal, TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::collections::HashSet;
use std::fmt;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Error, Result};

#[derive(Debug, Copy, Clone)]
enum VariantStyle {
    Variant,
    Enum,
    Union,
}

impl fmt::Display for VariantStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Variant => "variant",
            Self::Enum => "enum",
            Self::Union => "union",
        })
    }
}

#[derive(Debug, Copy, Clone)]
enum Style {
    Record,
    Variant(VariantStyle),
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
            "variant" => Style::Variant(VariantStyle::Variant),
            "enum" => Style::Variant(VariantStyle::Enum),
            "union" => Style::Variant(VariantStyle::Union),
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

fn find_rename(attributes: &[syn::Attribute]) -> Result<Option<Literal>> {
    let mut name = None;

    for attribute in attributes {
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

fn add_trait_bounds(generics: &syn::Generics, bound: syn::TypeParamBound) -> syn::Generics {
    let mut generics = generics.clone();
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(bound.clone());
        }
    }
    generics
}

#[derive(Debug, Copy, Clone)]
enum DiscriminantSize {
    Size1,
    Size2,
    Size4,
}

impl DiscriminantSize {
    fn quote(self, discriminant: usize) -> TokenStream {
        match self {
            Self::Size1 => {
                let discriminant = u8::try_from(discriminant).unwrap();
                quote!(#discriminant)
            }
            Self::Size2 => {
                let discriminant = u16::try_from(discriminant).unwrap();
                quote!(#discriminant)
            }
            Self::Size4 => {
                let discriminant = u32::try_from(discriminant).unwrap();
                quote!(#discriminant)
            }
        }
    }
}

impl From<DiscriminantSize> for u32 {
    fn from(size: DiscriminantSize) -> u32 {
        match size {
            DiscriminantSize::Size1 => 1,
            DiscriminantSize::Size2 => 2,
            DiscriminantSize::Size4 => 4,
        }
    }
}

fn discriminant_size(case_count: usize) -> Option<DiscriminantSize> {
    if case_count <= 0xFF {
        Some(DiscriminantSize::Size1)
    } else if case_count <= 0xFFFF {
        Some(DiscriminantSize::Size2)
    } else if case_count <= 0xFFFF_FFFF {
        Some(DiscriminantSize::Size4)
    } else {
        None
    }
}

struct VariantCase<'a> {
    attrs: &'a [syn::Attribute],
    ident: &'a syn::Ident,
    ty: Option<&'a syn::Type>,
}

trait Expander {
    fn expand_record(&self, input: &DeriveInput, fields: &syn::FieldsNamed) -> Result<TokenStream>;

    fn expand_variant(
        &self,
        input: &DeriveInput,
        discriminant_size: DiscriminantSize,
        cases: &[VariantCase],
        style: VariantStyle,
    ) -> Result<TokenStream>;
}

fn expand(expander: &dyn Expander, input: &DeriveInput) -> Result<TokenStream> {
    match find_style(input)? {
        Style::Record => expand_record(expander, input),
        Style::Variant(style) => expand_variant(expander, input, style),
    }
}

fn expand_record(expander: &dyn Expander, input: &DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    let body = if let Data::Struct(body) = &input.data {
        body
    } else {
        return Err(Error::new(
            name.span(),
            "`record` component types can only be derived for Rust `struct`s",
        ));
    };

    match &body.fields {
        syn::Fields::Named(fields) => expander.expand_record(input, fields),

        syn::Fields::Unnamed(_) | syn::Fields::Unit => Err(Error::new(
            name.span(),
            "`record` component types can only be derived for `struct`s with named fields",
        )),
    }
}

fn expand_variant(
    expander: &dyn Expander,
    input: &DeriveInput,
    style: VariantStyle,
) -> Result<TokenStream> {
    let name = &input.ident;

    let body = if let Data::Enum(body) = &input.data {
        body
    } else {
        return Err(Error::new(
            name.span(),
            format!(
                "`{}` component types can only be derived for Rust `enum`s",
                style
            ),
        ));
    };

    if body.variants.is_empty() {
        return Err(Error::new(
            name.span(),
            format!("`{}` component types can only be derived for Rust `enum`s with at least one variant", style),
        ));
    }

    let discriminant_size = discriminant_size(body.variants.len()).ok_or_else(|| {
        Error::new(
            input.ident.span(),
            "`enum`s with more than 2^32 variants are not supported",
        )
    })?;

    let cases = body
        .variants
        .iter()
        .map(
            |syn::Variant {
                 attrs,
                 ident,
                 fields,
                 ..
             }| {
                Ok(VariantCase {
                    attrs,
                    ident,
                    ty: match fields {
                        syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                            Some(&fields.unnamed[0].ty)
                        }
                        syn::Fields::Unit => None,
                        _ => {
                            return Err(Error::new(
                                name.span(),
                                format!(
                                    "`{}` component types can only be derived for Rust `enum`s \
                                     containing variants with {}",
                                    style,
                                    match style {
                                        VariantStyle::Variant => "at most one unnamed field each",
                                        VariantStyle::Enum => "no fields",
                                        VariantStyle::Union => "exactly one unnamed field each",
                                    }
                                ),
                            ))
                        }
                    },
                })
            },
        )
        .collect::<Result<Vec<_>>>()?;

    expander.expand_variant(input, discriminant_size, &cases, style)
}

#[proc_macro_derive(Lift, attributes(component))]
pub fn lift(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(&LiftExpander, &parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

struct LiftExpander;

impl Expander for LiftExpander {
    fn expand_record(&self, input: &DeriveInput, fields: &syn::FieldsNamed) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut lifts = TokenStream::new();
        let mut loads = TokenStream::new();

        for syn::Field { ident, ty, .. } in &fields.named {
            lifts.extend(quote!(#ident: <#ty as wasmtime::component::Lift>::lift(
                store, options, &src.#ident
            )?,));

            loads.extend(quote!(#ident: <#ty as wasmtime::component::Lift>::load(
                memory,
                &bytes
                    [#internal::next_field::<#ty>(&mut offset)..]
                    [..<#ty as wasmtime::component::ComponentType>::SIZE32]
            )?,));
        }

        let name = &input.ident;
        let generics = add_trait_bounds(&input.generics, parse_quote!(wasmtime::component::Lift));
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
                            % (<Self as wasmtime::component::ComponentType>::ALIGN32 as usize)
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

    fn expand_variant(
        &self,
        input: &DeriveInput,
        discriminant_size: DiscriminantSize,
        cases: &[VariantCase],
        _style: VariantStyle,
    ) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut lifts = TokenStream::new();
        let mut loads = TokenStream::new();

        for (index, VariantCase { ident, ty, .. }) in cases.iter().enumerate() {
            let index_u32 = u32::try_from(index).unwrap();

            let index_quoted = discriminant_size.quote(index);

            if let Some(ty) = ty {
                lifts.extend(
                    quote!(#index_u32 => Self::#ident(<#ty as wasmtime::component::Lift>::lift(
                        store, options, unsafe { &src.payload.#ident }
                    )?),),
                );

                loads.extend(
                    quote!(#index_quoted => Self::#ident(<#ty as wasmtime::component::Lift>::load(
                        memory, &payload[..<#ty as wasmtime::component::ComponentType>::SIZE32]
                    )?),),
                );
            } else {
                lifts.extend(quote!(#index_u32 => Self::#ident,));

                loads.extend(quote!(#index_quoted => Self::#ident,));
            }
        }

        let name = &input.ident;
        let generics = add_trait_bounds(&input.generics, parse_quote!(wasmtime::component::Lift));
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let from_bytes = match discriminant_size {
            DiscriminantSize::Size1 => quote!(bytes[0]),
            DiscriminantSize::Size2 => quote!(u16::from_le_bytes(bytes[0..2].try_into()?)),
            DiscriminantSize::Size4 => quote!(u32::from_le_bytes(bytes[0..4].try_into()?)),
        };

        let payload_offset = u32::from(discriminant_size) as usize;

        let expanded = quote! {
            unsafe impl #impl_generics wasmtime::component::Lift for #name #ty_generics #where_clause {
                #[inline]
                fn lift(
                    store: &#internal::StoreOpaque,
                    options: &#internal::Options,
                    src: &Self::Lower,
                ) -> #internal::anyhow::Result<Self> {
                    Ok(match src.tag.get_u32() {
                        #lifts
                        discrim => #internal::anyhow::bail!("unexpected discriminant: {}", discrim),
                    })
                }

                #[inline]
                fn load(memory: &#internal::Memory, bytes: &[u8]) -> #internal::anyhow::Result<Self> {
                    let align = <Self as wasmtime::component::ComponentType>::ALIGN32;
                    debug_assert!((bytes.as_ptr() as usize) % (align as usize) == 0);
                    let discrim = #from_bytes;
                    let payload = &bytes[#internal::align_to(#payload_offset, align)..];
                    Ok(match discrim {
                        #loads
                        discrim => #internal::anyhow::bail!("unexpected discriminant: {}", discrim),
                    })
                }
            }
        };

        Ok(expanded)
    }
}

#[proc_macro_derive(Lower, attributes(component))]
pub fn lower(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(&LowerExpander, &parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

struct LowerExpander;

impl Expander for LowerExpander {
    fn expand_record(&self, input: &DeriveInput, fields: &syn::FieldsNamed) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut lowers = TokenStream::new();
        let mut stores = TokenStream::new();

        for syn::Field { ident, ty, .. } in &fields.named {
            lowers.extend(quote!(wasmtime::component::Lower::lower(
                &self.#ident, store, options, #internal::map_maybe_uninit!(dst.#ident)
            )?;));

            stores.extend(quote!(wasmtime::component::Lower::store(
                &self.#ident, memory, #internal::next_field::<#ty>(&mut offset)
            )?;));
        }

        let name = &input.ident;
        let generics = add_trait_bounds(&input.generics, parse_quote!(wasmtime::component::Lower));
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
                    debug_assert!(offset % (<Self as wasmtime::component::ComponentType>::ALIGN32 as usize) == 0);
                    #stores
                    Ok(())
                }
            }
        };

        Ok(expanded)
    }

    fn expand_variant(
        &self,
        input: &DeriveInput,
        discriminant_size: DiscriminantSize,
        cases: &[VariantCase],
        _style: VariantStyle,
    ) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut lowers = TokenStream::new();
        let mut stores = TokenStream::new();

        for (index, VariantCase { ident, ty, .. }) in cases.iter().enumerate() {
            let index_u32 = u32::try_from(index).unwrap();

            let index_quoted = discriminant_size.quote(index);

            let pattern;
            let lower;
            let store;

            if ty.is_some() {
                pattern = quote!(Self::#ident(value));
                lower = quote!(value.lower(store, options, #internal::map_maybe_uninit!(dst.payload.#ident)));
                store = quote!(value.store(
                    memory,
                    offset + #internal::align_to(1, <Self as wasmtime::component::ComponentType>::ALIGN32)
                ));
            } else {
                pattern = quote!(Self::#ident);
                lower = quote!(Ok(()));
                store = quote!(Ok(()));
            }

            lowers.extend(quote!(#pattern => {
                #internal::map_maybe_uninit!(dst.tag).write(wasmtime::ValRaw::i32(#index_u32 as i32));
                #lower
            }));

            let discriminant_size = u32::from(discriminant_size) as usize;

            stores.extend(quote!(#pattern => {
                *memory.get::<#discriminant_size>(offset) = #index_quoted.to_le_bytes();
                #store
            }));
        }

        let name = &input.ident;
        let generics = add_trait_bounds(&input.generics, parse_quote!(wasmtime::component::Lower));
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
                    // See comment in <Result<T, E> as Lower>::lower for why we zero out the payload here
                    unsafe {
                        #internal::map_maybe_uninit!(dst.payload)
                            .as_mut_ptr()
                            .write_bytes(0u8, 1);
                    }

                    match self {
                        #lowers
                    }
                }

                #[inline]
                fn store<T>(
                    &self,
                    memory: &mut #internal::MemoryMut<'_, T>,
                    mut offset: usize
                ) -> #internal::anyhow::Result<()> {
                    debug_assert!(offset % (<Self as wasmtime::component::ComponentType>::ALIGN32 as usize) == 0);
                    match self {
                        #stores
                    }
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
        &parse_macro_input!(input as DeriveInput),
    )
    .unwrap_or_else(Error::into_compile_error)
    .into()
}

struct ComponentTypeExpander;

impl Expander for ComponentTypeExpander {
    fn expand_record(&self, input: &DeriveInput, fields: &syn::FieldsNamed) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut field_names_and_checks = TokenStream::new();
        let mut lower_generic_params = TokenStream::new();
        let mut lower_generic_args = TokenStream::new();
        let mut lower_field_declarations = TokenStream::new();
        let mut sizes = TokenStream::new();
        let mut unique_types = HashSet::new();

        for (
            index,
            syn::Field {
                attrs, ident, ty, ..
            },
        ) in fields.named.iter().enumerate()
        {
            let name = find_rename(attrs)?
                .unwrap_or_else(|| Literal::string(&ident.as_ref().unwrap().to_string()));

            let generic = format_ident!("T{}", index);

            lower_generic_params.extend(quote!(#generic: Copy,));
            lower_generic_args.extend(quote!(<#ty as wasmtime::component::ComponentType>::Lower,));

            lower_field_declarations.extend(quote!(#ident: #generic,));

            field_names_and_checks
                .extend(quote!((#name, <#ty as wasmtime::component::ComponentType>::typecheck),));

            sizes.extend(quote!(
                size = #internal::align_to(size, <#ty as wasmtime::component::ComponentType>::ALIGN32);
                size += <#ty as wasmtime::component::ComponentType>::SIZE32;
            ));

            unique_types.insert(ty);
        }

        let alignments = unique_types
            .into_iter()
            .map(|ty| {
                let align = quote!(<#ty as wasmtime::component::ComponentType>::ALIGN32);
                quote!(if #align > align {
                    align = #align;
                })
            })
            .collect::<TokenStream>();

        let name = &input.ident;
        let generics = add_trait_bounds(
            &input.generics,
            parse_quote!(wasmtime::component::ComponentType),
        );
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let lower = format_ident!("Lower{}", name);

        // You may wonder why we make the types of all the fields of the #lower struct generic.  This is to work
        // around the lack of [perfect derive support in
        // rustc](https://smallcultfollowing.com/babysteps//blog/2022/04/12/implied-bounds-and-perfect-derive/#what-is-perfect-derive)
        // as of this writing.
        //
        // If the struct we're deriving a `ComponentType` impl for has any generic parameters, then #lower needs
        // generic parameters too.  And if we just copy the parameters and bounds from the impl to #lower, then the
        // `#[derive(Clone, Copy)]` will fail unless the original generics were declared with those bounds, which
        // we don't want to require.
        //
        // Alternatively, we could just pass the `Lower` associated type of each generic type as arguments to
        // #lower, but that would require distinguishing between generic and concrete types when generating
        // #lower_field_declarations, which would require some form of symbol resolution.  That doesn't seem worth
        // the trouble.

        let expanded = quote! {
            #[doc(hidden)]
            #[derive(Clone, Copy)]
            #[repr(C)]
            pub struct #lower <#lower_generic_params> {
                #lower_field_declarations
            }

            unsafe impl #impl_generics wasmtime::component::ComponentType for #name #ty_generics #where_clause {
                type Lower = #lower <#lower_generic_args>;

                const SIZE32: usize = {
                    let mut size = 0;
                    #sizes
                    size
                };

                const ALIGN32: u32 = {
                    let mut align = 1;
                    #alignments
                    align
                };

                #[inline]
                fn typecheck(
                    ty: &#internal::InterfaceType,
                    types: &#internal::ComponentTypes,
                ) -> #internal::anyhow::Result<()> {
                    #internal::typecheck_record(ty, types, &[#field_names_and_checks])
                }
            }
        };

        Ok(quote!(const _: () = { #expanded };))
    }

    fn expand_variant(
        &self,
        input: &DeriveInput,
        discriminant_size: DiscriminantSize,
        cases: &[VariantCase],
        style: VariantStyle,
    ) -> Result<TokenStream> {
        let internal = quote!(wasmtime::component::__internal);

        let mut case_names_and_checks = TokenStream::new();
        let mut lower_payload_generic_params = TokenStream::new();
        let mut lower_payload_generic_args = TokenStream::new();
        let mut lower_payload_case_declarations = TokenStream::new();
        let mut lower_generic_args = TokenStream::new();
        let mut sizes = TokenStream::new();
        let mut unique_types = HashSet::new();

        for (index, VariantCase { attrs, ident, ty }) in cases.iter().enumerate() {
            let rename = find_rename(attrs)?;

            if let (Some(_), VariantStyle::Union) = (&rename, style) {
                return Err(Error::new(
                    ident.span(),
                    "renaming `union` cases is not permitted; only the type is used",
                ));
            }

            let name = rename.unwrap_or_else(|| Literal::string(&ident.to_string()));

            if let Some(ty) = ty {
                sizes.extend({
                    let size = quote!(<#ty as wasmtime::component::ComponentType>::SIZE32);
                    quote!(if #size > size {
                        size = #size;
                    })
                });

                case_names_and_checks.extend(match style {
                    VariantStyle::Variant => {
                        quote!((#name, <#ty as wasmtime::component::ComponentType>::typecheck),)
                    }
                    VariantStyle::Union => {
                        quote!(<#ty as wasmtime::component::ComponentType>::typecheck,)
                    }
                    VariantStyle::Enum => {
                        return Err(Error::new(
                            ident.span(),
                            "payloads are not permitted for `enum` cases",
                        ))
                    }
                });

                let generic = format_ident!("T{}", index);

                lower_payload_generic_params.extend(quote!(#generic: Copy,));
                lower_payload_generic_args.extend(quote!(#generic,));
                lower_payload_case_declarations.extend(quote!(#ident: #generic,));
                lower_generic_args
                    .extend(quote!(<#ty as wasmtime::component::ComponentType>::Lower,));

                unique_types.insert(ty);
            } else {
                case_names_and_checks.extend(match style {
                    VariantStyle::Variant => {
                        quote!((#name, <() as wasmtime::component::ComponentType>::typecheck),)
                    }
                    VariantStyle::Union => {
                        quote!(<() as wasmtime::component::ComponentType>::typecheck,)
                    }
                    VariantStyle::Enum => quote!(#name,),
                });
            }
        }

        if lower_payload_case_declarations.is_empty() {
            lower_payload_case_declarations.extend(quote!(_dummy: ()));
        }

        let alignments = unique_types
            .into_iter()
            .map(|ty| {
                let align = quote!(<#ty as wasmtime::component::ComponentType>::ALIGN32);
                quote!(if #align > align {
                    align = #align;
                })
            })
            .collect::<TokenStream>();

        let typecheck = match style {
            VariantStyle::Variant => quote!(typecheck_variant),
            VariantStyle::Union => quote!(typecheck_union),
            VariantStyle::Enum => quote!(typecheck_enum),
        };

        let name = &input.ident;
        let generics = add_trait_bounds(
            &input.generics,
            parse_quote!(wasmtime::component::ComponentType),
        );
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let lower = format_ident!("Lower{}", name);
        let lower_payload = format_ident!("LowerPayload{}", name);
        let discriminant_size = u32::from(discriminant_size);

        // You may wonder why we make the types of all the fields of the #lower struct and #lower_payload union
        // generic.  This is to work around a [normalization bug in
        // rustc](https://github.com/rust-lang/rust/issues/90903) such that the compiler does not understand that
        // e.g. `<i32 as ComponentType>::Lower` is `Copy` despite the bound specified in `ComponentType`'s
        // definition.
        //
        // See also the comment in `Self::expand_record` above for another reason why we do this.

        let expanded = quote! {
            #[doc(hidden)]
            #[derive(Clone, Copy)]
            #[repr(C)]
            pub struct #lower<#lower_payload_generic_params> {
                tag: wasmtime::ValRaw,
                payload: #lower_payload<#lower_payload_generic_args>
            }

            #[doc(hidden)]
            #[allow(non_snake_case)]
            #[derive(Clone, Copy)]
            #[repr(C)]
            union #lower_payload<#lower_payload_generic_params> {
                #lower_payload_case_declarations
            }

            unsafe impl #impl_generics wasmtime::component::ComponentType for #name #ty_generics #where_clause {
                type Lower = #lower<#lower_generic_args>;

                #[inline]
                fn typecheck(
                    ty: &#internal::InterfaceType,
                    types: &#internal::ComponentTypes,
                ) -> #internal::anyhow::Result<()> {
                    #internal::#typecheck(ty, types, &[#case_names_and_checks])
                }

                const SIZE32: usize = {
                    let mut size = 0;
                    #sizes
                    #internal::align_to(#discriminant_size as usize, Self::ALIGN32) + size
                };

                const ALIGN32: u32 = {
                    let mut align = #discriminant_size;
                    #alignments
                    align
                };
            }
        };

        Ok(quote!(const _: () = { #expanded };))
    }
}

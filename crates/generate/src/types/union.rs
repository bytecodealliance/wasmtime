use crate::lifetimes::{anon_lifetime, LifetimeExt};
use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;
use witx::Layout;

pub(super) fn define_union(names: &Names, name: &witx::Id, u: &witx::UnionDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = u.mem_size_align().size as u32;
    let align = u.mem_size_align().align as u32;
    let ulayout = u.union_layout();
    let contents_offset = ulayout.contents_offset as u32;

    let lifetime = quote!('a);

    let variants = u.variants.iter().map(|v| {
        let var_name = names.enum_variant(&v.name);
        if let Some(tref) = &v.tref {
            let var_type = names.type_ref(&tref, lifetime.clone());
            quote!(#var_name(#var_type))
        } else {
            quote!(#var_name)
        }
    });

    let tagname = names.type_(&u.tag.name);

    let read_variant = u.variants.iter().map(|v| {
        let variantname = names.enum_variant(&v.name);
        if let Some(tref) = &v.tref {
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #tagname::#variantname => {
                    let variant_ptr = location.cast::<#varianttype>(#contents_offset).expect("union variant ptr validated");
                    let variant_val = <#varianttype as wiggle_runtime::GuestType>::read(&variant_ptr)?;
                    Ok(#ident::#variantname(variant_val))
                }
            }
        } else {
            quote! { #tagname::#variantname => Ok(#ident::#variantname), }
        }
    });

    let write_variant = u.variants.iter().map(|v| {
        let variantname = names.enum_variant(&v.name);
        let write_tag = quote! {
            let tag_ptr = location.cast::<#tagname>(0).expect("union tag ptr TODO error report");
            let mut tag_ref = tag_ptr.as_ref_mut().expect("union tag ref TODO error report");
            *tag_ref = #tagname::#variantname;
        };
        if let Some(tref) = &v.tref {
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #ident::#variantname(contents) => {
                    #write_tag
                    let variant_ptr = location.cast::<#varianttype>(#contents_offset).expect("union variant ptr validated");
                    <#varianttype as wiggle_runtime::GuestType>::write(&contents, &variant_ptr);
                }
            }
        } else {
            quote! {
                #ident::#variantname => {
                    #write_tag
                }
            }
        }
    });
    let validate = union_validate(names, ident.clone(), u, &ulayout);

    if !u.needs_lifetime() {
        // Type does not have a lifetime parameter:
        quote! {
            #[derive(Clone, Debug, PartialEq)]
            pub enum #ident {
                #(#variants),*
            }

            impl<'a> wiggle_runtime::GuestType<'a> for #ident {
                fn size() -> u32 {
                    #size
                }

                fn align() -> u32 {
                    #align
                }

                fn name() -> String {
                    stringify!(#ident).to_owned()
                }

                fn validate(ptr: &wiggle_runtime::GuestPtr<'a, #ident>) -> Result<(), wiggle_runtime::GuestError> {
                    #validate
                }

                fn read(location: &wiggle_runtime::GuestPtr<'a, #ident>)
                        -> Result<Self, wiggle_runtime::GuestError> {
                    <#ident as wiggle_runtime::GuestType>::validate(location)?;
                    let tag = *location.cast::<#tagname>(0).expect("validated tag ptr").as_ref().expect("validated tag ref");
                    match tag {
                        #(#read_variant)*
                    }

                }

                fn write(&self, location: &wiggle_runtime::GuestPtrMut<'a, #ident>) {
                    match self {
                        #(#write_variant)*
                    }
                }
            }
        }
    } else {
        quote! {
            #[derive(Clone)]
            pub enum #ident<#lifetime> {
                #(#variants),*
            }

            impl<#lifetime> wiggle_runtime::GuestType<#lifetime> for #ident<#lifetime> {
                fn size() -> u32 {
                    #size
                }

                fn align() -> u32 {
                    #align
                }

                fn name() -> String {
                    stringify!(#ident).to_owned()
                }

                fn validate(ptr: &wiggle_runtime::GuestPtr<#lifetime, #ident<#lifetime>>) -> Result<(), wiggle_runtime::GuestError> {
                    #validate
                }

                fn read(location: &wiggle_runtime::GuestPtr<#lifetime, #ident<#lifetime>>)
                        -> Result<Self, wiggle_runtime::GuestError> {
                    <#ident as wiggle_runtime::GuestType>::validate(location)?;
                    let tag = *location.cast::<#tagname>(0).expect("validated tag ptr").as_ref().expect("validated tag ref");
                    match tag {
                        #(#read_variant)*
                    }

                }

                fn write(&self, location: &wiggle_runtime::GuestPtrMut<#lifetime, #ident<#lifetime>>) {
                    match self {
                        #(#write_variant)*
                    }
                }
            }
        }
    }
}

fn union_validate(
    names: &Names,
    typename: TokenStream,
    u: &witx::UnionDatatype,
    ulayout: &witx::UnionLayout,
) -> TokenStream {
    let tagname = names.type_(&u.tag.name);
    let contents_offset = ulayout.contents_offset as u32;

    let with_err = |f: &str| -> TokenStream {
        quote!(|e| wiggle_runtime::GuestError::InDataField {
            typename: stringify!(#typename).to_owned(),
            field: #f.to_owned(),
            err: Box::new(e),
        })
    };

    let tag_err = with_err("<tag>");
    let variant_validation = u.variants.iter().map(|v| {
        let err = with_err(v.name.as_str());
        let variantname = names.enum_variant(&v.name);
        if let Some(tref) = &v.tref {
            let lifetime = anon_lifetime();
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #tagname::#variantname => {
                    let variant_ptr = ptr.cast::<#varianttype>(#contents_offset).map_err(#err)?;
                    <#varianttype as wiggle_runtime::GuestType>::validate(&variant_ptr).map_err(#err)?;
                }
            }
        } else {
            quote! { #tagname::#variantname => {} }
        }
    });

    quote! {
        let tag = *ptr.cast::<#tagname>(0).map_err(#tag_err)?.as_ref().map_err(#tag_err)?;
        match tag {
            #(#variant_validation)*
        }
        Ok(())
    }
}

use crate::lifetimes::LifetimeExt;
use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;
use witx::Layout;

pub(super) fn define_union(names: &Names, name: &witx::Id, u: &witx::UnionDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = u.mem_size_align().size as u32;
    let align = u.mem_size_align().align as usize;
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
                    let variant_ptr = location.cast::<u8>().add(#contents_offset)?;
                    let variant_val = <#varianttype as wiggle_runtime::GuestType>::read(&variant_ptr.cast())?;
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
            location.cast().write(#tagname::#variantname)?;
        };
        if let Some(tref) = &v.tref {
            let varianttype = names.type_ref(tref, lifetime.clone());
            quote! {
                #ident::#variantname(contents) => {
                    #write_tag
                    let variant_ptr = location.cast::<u8>().add(#contents_offset)?;
                    <#varianttype as wiggle_runtime::GuestType>::write(&variant_ptr.cast(), contents)?;
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

    let (enum_lifetime, extra_derive) = if u.needs_lifetime() {
        (quote!(<'a>), quote!())
    } else {
        (quote!(), quote!(, PartialEq))
    };

    quote! {
        #[derive(Clone, Debug #extra_derive)]
        pub enum #ident #enum_lifetime {
            #(#variants),*
        }

        impl<'a> wiggle_runtime::GuestType<'a> for #ident #enum_lifetime {
            fn guest_size() -> u32 {
                #size
            }

            fn guest_align() -> usize {
                #align
            }

            fn read(location: &wiggle_runtime::GuestPtr<'a, Self>)
                -> Result<Self, wiggle_runtime::GuestError>
            {
                let tag = location.cast().read()?;
                match tag {
                    #(#read_variant)*
                }

            }

            fn write(location: &wiggle_runtime::GuestPtr<'_, Self>, val: Self)
                -> Result<(), wiggle_runtime::GuestError>
            {
                match val {
                    #(#write_variant)*
                }
                Ok(())
            }
        }
    }
}

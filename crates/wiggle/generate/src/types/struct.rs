use crate::lifetimes::{anon_lifetime, LifetimeExt};
use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;
use witx::Layout;

pub(super) fn define_struct(
    names: &Names,
    name: &witx::Id,
    s: &witx::StructDatatype,
) -> TokenStream {
    let rt = names.runtime_mod();
    let ident = names.type_(name);
    let size = s.mem_size_align().size as u32;
    let align = s.mem_size_align().align as usize;

    let member_names = s.members.iter().map(|m| names.struct_member(&m.name));
    let member_decls = s.members.iter().map(|m| {
        let name = names.struct_member(&m.name);
        let type_ = match &m.tref {
            witx::TypeRef::Name(nt) => names.type_(&nt.name),
            witx::TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => names.builtin_type(*builtin, quote!('a)),
                witx::Type::Pointer(pointee) | witx::Type::ConstPointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, quote!('a));
                    quote!(#rt::GuestPtr<'a, #pointee_type>)
                }
                _ => unimplemented!("other anonymous struct members"),
            },
        };
        quote!(pub #name: #type_)
    });

    let member_reads = s.member_layout().into_iter().map(|ml| {
        let name = names.struct_member(&ml.member.name);
        let offset = ml.offset as u32;
        let location = quote!(location.cast::<u8>().add(#offset)?.cast());
        match &ml.member.tref {
            witx::TypeRef::Name(nt) => {
                let type_ = names.type_(&nt.name);
                quote! {
                    let #name = <#type_ as #rt::GuestType>::read(&#location)?;
                }
            }
            witx::TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => {
                    let type_ = names.builtin_type(*builtin, anon_lifetime());
                    quote! {
                    let #name = <#type_ as #rt::GuestType>::read(&#location)?;
                    }
                }
                witx::Type::Pointer(pointee) | witx::Type::ConstPointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote! {
                        let #name = <#rt::GuestPtr::<#pointee_type> as #rt::GuestType>::read(&#location)?;
                    }
                }
                _ => unimplemented!("other anonymous struct members"),
            },
        }
    });

    let member_writes = s.member_layout().into_iter().map(|ml| {
        let name = names.struct_member(&ml.member.name);
        let offset = ml.offset as u32;
        quote! {
            #rt::GuestType::write(
                &location.cast::<u8>().add(#offset)?.cast(),
                val.#name,
            )?;
        }
    });

    let (struct_lifetime, extra_derive) = if s.needs_lifetime() {
        (quote!(<'a>), quote!())
    } else {
        (quote!(), quote!(, PartialEq))
    };

    let transparent = if s.is_transparent() {
        let member_validate = s.member_layout().into_iter().map(|ml| {
            let offset = ml.offset;
            let typename = names.type_ref(&ml.member.tref, anon_lifetime());
            quote! {
                // SAFETY: caller has validated bounds and alignment of `location`.
                // member_layout gives correctly-aligned pointers inside that area.
                #typename::validate(
                    unsafe { (location as *mut u8).add(#offset) as *mut _ }
                )?;
            }
        });

        quote! {
            unsafe impl<'a> #rt::GuestTypeTransparent<'a> for #ident {
                #[inline]
                fn validate(location: *mut #ident) -> Result<(), #rt::GuestError> {
                    #(#member_validate)*
                    Ok(())
                }
            }
        }
    } else {
        quote!()
    };

    quote! {
        #[derive(Clone, Debug #extra_derive)]
        pub struct #ident #struct_lifetime {
            #(#member_decls),*
        }

        impl<'a> #rt::GuestType<'a> for #ident #struct_lifetime {
            fn guest_size() -> u32 {
                #size
            }

            fn guest_align() -> usize {
                #align
            }

            fn read(location: &#rt::GuestPtr<'a, Self>) -> Result<Self, #rt::GuestError> {
                #(#member_reads)*
                Ok(#ident { #(#member_names),* })
            }

            fn write(location: &#rt::GuestPtr<'_, Self>, val: Self) -> Result<(), #rt::GuestError> {
                #(#member_writes)*
                Ok(())
            }
        }

        #transparent
    }
}

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
    if !s.needs_lifetime() {
        define_copy_struct(names, name, s)
    } else {
        define_ptr_struct(names, name, s)
    }
}

fn define_copy_struct(names: &Names, name: &witx::Id, s: &witx::StructDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = s.mem_size_align().size as u32;
    let align = s.mem_size_align().align as u32;

    let member_decls = s.members.iter().map(|m| {
        let name = names.struct_member(&m.name);
        let type_ = names.type_ref(&m.tref, anon_lifetime());
        quote!(pub #name: #type_)
    });
    let member_valids = s.member_layout().into_iter().map(|ml| {
        let type_ = names.type_ref(&ml.member.tref, anon_lifetime());
        let offset = ml.offset as u32;
        let fieldname = names.struct_member(&ml.member.name);
        quote! {
            #type_::validate(
                &ptr.cast(#offset).map_err(|e|
                    wiggle_runtime::GuestError::InDataField{
                        typename: stringify!(#ident).to_owned(),
                        field: stringify!(#fieldname).to_owned(),
                        err: Box::new(e),
                    })?
                ).map_err(|e|
                    wiggle_runtime::GuestError::InDataField {
                        typename: stringify!(#ident).to_owned(),
                        field: stringify!(#fieldname).to_owned(),
                        err: Box::new(e),
                    })?;
        }
    });

    quote! {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident {
            #(#member_decls),*
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

            fn validate(ptr: &wiggle_runtime::GuestPtr<#ident>) -> Result<(), wiggle_runtime::GuestError> {
                #(#member_valids)*
                Ok(())
            }

            fn read(location: &wiggle_runtime::GuestPtr<'a, #ident>) -> Result<#ident, wiggle_runtime::GuestError> {
                let r = location.as_ref()?;
                Ok(*r)
            }

            fn write(&self, location: &wiggle_runtime::GuestPtrMut<'a, Self>) {
                unsafe { (location.as_raw() as *mut #ident).write(*self) };
            }
        }

        impl<'a> wiggle_runtime::GuestTypeTransparent<'a> for #ident {}
    }
}

fn define_ptr_struct(names: &Names, name: &witx::Id, s: &witx::StructDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = s.mem_size_align().size as u32;
    let align = s.mem_size_align().align as u32;

    let member_names = s.members.iter().map(|m| names.struct_member(&m.name));
    let member_decls = s.members.iter().map(|m| {
        let name = names.struct_member(&m.name);
        let type_ = match &m.tref {
            witx::TypeRef::Name(nt) => names.type_(&nt.name),
            witx::TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => names.builtin_type(*builtin, quote!('a)),
                witx::Type::Pointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, quote!('a));
                    quote!(wiggle_runtime::GuestPtrMut<'a, #pointee_type>)
                }
                witx::Type::ConstPointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, quote!('a));
                    quote!(wiggle_runtime::GuestPtr<'a, #pointee_type>)
                }
                _ => unimplemented!("other anonymous struct members"),
            },
        };
        quote!(pub #name: #type_)
    });
    let member_valids = s.member_layout().into_iter().map(|ml| {
        let type_ = match &ml.member.tref {
            witx::TypeRef::Name(nt) => names.type_(&nt.name),
            witx::TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => names.builtin_type(*builtin, quote!('a)),
                witx::Type::Pointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote!(wiggle_runtime::GuestPtrMut::<#pointee_type>)
                }
                witx::Type::ConstPointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote!(wiggle_runtime::GuestPtr::<#pointee_type>)
                }
                _ => unimplemented!("other anonymous struct members"),
            },
        };
        let offset = ml.offset as u32;
        let fieldname = names.struct_member(&ml.member.name);
        quote! {
            #type_::validate(
                &ptr.cast(#offset).map_err(|e|
                    wiggle_runtime::GuestError::InDataField{
                        typename: stringify!(#ident).to_owned(),
                        field: stringify!(#fieldname).to_owned(),
                        err: Box::new(e),
                    })?
                ).map_err(|e|
                    wiggle_runtime::GuestError::InDataField {
                        typename: stringify!(#ident).to_owned(),
                        field: stringify!(#fieldname).to_owned(),
                        err: Box::new(e),
                    })?;
        }
    });

    let member_reads = s.member_layout().into_iter().map(|ml| {
        let name = names.struct_member(&ml.member.name);
        let offset = ml.offset as u32;
        match &ml.member.tref {
            witx::TypeRef::Name(nt) => {
                let type_ = names.type_(&nt.name);
                quote! {
                    let #name = <#type_ as wiggle_runtime::GuestType>::read(&location.cast(#offset)?)?;
                }
            }
            witx::TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => {
                    let type_ = names.builtin_type(*builtin, anon_lifetime());
                    quote! {
                    let #name = <#type_ as wiggle_runtime::GuestType>::read(&location.cast(#offset)?)?;
                    }
                }
                witx::Type::Pointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote! {
                        let #name = <wiggle_runtime::GuestPtrMut::<#pointee_type> as wiggle_runtime::GuestType>::read(&location.cast(#offset)?)?;
                    }
                }
                witx::Type::ConstPointer(pointee) => {
                    let pointee_type = names.type_ref(&pointee, anon_lifetime());
                    quote! {
                        let #name = <wiggle_runtime::GuestPtr::<#pointee_type> as wiggle_runtime::GuestType>::read(&location.cast(#offset)?)?;
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
            self.#name.write(&location.cast(#offset).expect("cast to inner member"));
        }
    });

    quote! {
        #[derive(Clone)]
        pub struct #ident<'a> {
            #(#member_decls),*
        }

        impl<'a> wiggle_runtime::GuestType<'a> for #ident<'a> {
            fn size() -> u32 {
                #size
            }

            fn align() -> u32 {
                #align
            }

            fn name() -> String {
                stringify!(#ident).to_owned()
            }

            fn validate(ptr: &wiggle_runtime::GuestPtr<'a, #ident<'a>>) -> Result<(), wiggle_runtime::GuestError> {
                #(#member_valids)*
                Ok(())
            }

            fn read(location: &wiggle_runtime::GuestPtr<'a, #ident<'a>>) -> Result<#ident<'a>, wiggle_runtime::GuestError> {
                #(#member_reads)*
                Ok(#ident { #(#member_names),* })
            }

            fn write(&self, location: &wiggle_runtime::GuestPtrMut<'a, Self>) {
                #(#member_writes)*
            }
        }
    }
}

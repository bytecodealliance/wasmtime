use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;
use witx::Layout;

pub fn define_datatype(names: &Names, namedtype: &witx::NamedType) -> TokenStream {
    match &namedtype.tref {
        witx::TypeRef::Name(alias_to) => define_alias(names, &namedtype.name, &alias_to),
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Enum(e) => define_enum(names, &namedtype.name, &e),
            witx::Type::Int(_) => unimplemented!("int types"),
            witx::Type::Flags(_) => unimplemented!("flag types"),
            witx::Type::Struct(s) => {
                if struct_is_copy(s) {
                    define_copy_struct(names, &namedtype.name, &s)
                } else {
                    unimplemented!("non-Copy struct")
                }
            }
            witx::Type::Union(_) => unimplemented!("union types"),
            witx::Type::Handle(_h) => unimplemented!("handle types"),
            witx::Type::Builtin(b) => define_builtin(names, &namedtype.name, *b),
            witx::Type::Pointer { .. } => unimplemented!("pointer types"),
            witx::Type::ConstPointer { .. } => unimplemented!("constpointer types"),
            witx::Type::Array { .. } => unimplemented!("array types"),
        },
    }
}

fn define_alias(names: &Names, name: &witx::Id, to: &witx::NamedType) -> TokenStream {
    let ident = names.type_(name);
    let to = names.type_(&to.name);

    quote!(pub type #ident = #to;)
}

fn define_enum(names: &Names, name: &witx::Id, e: &witx::EnumDatatype) -> TokenStream {
    let ident = names.type_(&name);

    let repr = int_repr_tokens(e.repr);
    let abi_repr = atom_token(match e.repr {
        witx::IntRepr::U8 | witx::IntRepr::U16 | witx::IntRepr::U32 => witx::AtomType::I32,
        witx::IntRepr::U64 => witx::AtomType::I64,
    });

    let variant_names = e.variants.iter().map(|v| names.enum_variant(&v.name));
    let tryfrom_repr_cases = e.variants.iter().enumerate().map(|(n, v)| {
        let variant_name = names.enum_variant(&v.name);
        quote!(#n => Ok(#ident::#variant_name))
    });
    let to_repr_cases = e.variants.iter().enumerate().map(|(n, v)| {
        let variant_name = names.enum_variant(&v.name);
        quote!(#ident::#variant_name => #n as #repr)
    });

    quote! {
        #[repr(#repr)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub enum #ident {
            #(#variant_names),*
        }

        impl ::std::convert::TryFrom<#repr> for #ident {
            type Error = ::memory::GuestError;
            fn try_from(value: #repr) -> Result<#ident, ::memory::GuestError> {
                match value as usize {
                    #(#tryfrom_repr_cases),*,
                    _ => Err(::memory::GuestError::InvalidEnumValue(stringify!(#ident))),
                }
            }
        }

        impl ::std::convert::TryFrom<#abi_repr> for #ident {
            type Error = ::memory::GuestError;
            fn try_from(value: #abi_repr) -> Result<#ident, ::memory::GuestError> {
                #ident::try_from(value as #repr)
            }
        }

        impl From<#ident> for #repr {
            fn from(e: #ident) -> #repr {
                match e {
                    #(#to_repr_cases),*
                }
            }
        }

        impl From<#ident> for #abi_repr {
            fn from(e: #ident) -> #abi_repr {
                #repr::from(e) as #abi_repr
            }
        }

        impl ::memory::GuestType for #ident {
            fn size() -> u32 {
                ::std::mem::size_of::<#repr>() as u32
            }
            fn align() -> u32 {
                ::std::mem::align_of::<#repr>() as u32
            }
            fn name() -> String {
                stringify!(#ident).to_owned()
            }
            fn validate<'a>(location: &::memory::GuestPtr<'a, #ident>) -> Result<(), ::memory::GuestError> {
                use ::std::convert::TryFrom;
                let raw: #repr = unsafe { (location.as_raw() as *const #repr).read() };
                let _ = #ident::try_from(raw)?;
                Ok(())
            }
        }

        impl ::memory::GuestTypeCopy for #ident {}
        impl ::memory::GuestTypeClone for #ident {
            fn read_from_guest(location: &::memory::GuestPtr<#ident>) -> Result<#ident, ::memory::GuestError> {
                use ::std::convert::TryFrom;
                let raw: #repr = unsafe { (location.as_raw() as *const #repr).read() };
                let val = #ident::try_from(raw)?;
                Ok(val)
            }
            fn write_to_guest(&self, location: &::memory::GuestPtrMut<#ident>) {
                let val: #repr = #repr::from(*self);
                unsafe { (location.as_raw() as *mut #repr).write(val) };
            }
        }

    }
}

fn define_builtin(names: &Names, name: &witx::Id, builtin: witx::BuiltinType) -> TokenStream {
    let ident = names.type_(name);
    let built = names.builtin_type(builtin);
    quote!(pub type #ident = #built;)
}

pub fn struct_is_copy(s: &witx::StructDatatype) -> bool {
    s.members.iter().all(|m| match &*m.tref.type_() {
        witx::Type::Struct(s) => struct_is_copy(&s),
        witx::Type::Builtin(b) => match &*b {
            witx::BuiltinType::String => false,
            _ => true,
        },
        witx::Type::ConstPointer { .. }
        | witx::Type::Pointer { .. }
        | witx::Type::Array { .. }
        | witx::Type::Union { .. } => false,
        witx::Type::Enum { .. }
        | witx::Type::Int { .. }
        | witx::Type::Flags { .. }
        | witx::Type::Handle { .. } => true,
    })
}

fn define_copy_struct(names: &Names, name: &witx::Id, s: &witx::StructDatatype) -> TokenStream {
    let ident = names.type_(name);
    let size = s.mem_size_align().size as u32;
    let align = s.mem_size_align().align as u32;

    let member_decls = s.members.iter().map(|m| {
        let name = names.struct_member(&m.name);
        let type_ = names.type_ref(&m.tref);
        quote!(pub #name: #type_)
    });
    let member_valids = s.member_layout().into_iter().map(|ml| {
        let type_ = names.type_ref(&ml.member.tref);
        let offset = ml.offset as u32;
        quote!( #type_::validate(&ptr.cast(#offset)?)?; )
    });

    quote! {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub struct #ident {
            #(#member_decls),*
        }

        impl ::memory::GuestType for #ident {
            fn size() -> u32 {
                #size
            }
            fn align() -> u32 {
                #align
            }
            fn name() -> String {
                stringify!(#ident).to_owned()
            }
            fn validate(ptr: &::memory::GuestPtr<#ident>) -> Result<(), ::memory::GuestError> {
                #(#member_valids)*
                Ok(())
            }
        }
        impl ::memory::GuestTypeCopy for #ident {}
    }
}

fn int_repr_tokens(int_repr: witx::IntRepr) -> TokenStream {
    match int_repr {
        witx::IntRepr::U8 => quote!(u8),
        witx::IntRepr::U16 => quote!(u16),
        witx::IntRepr::U32 => quote!(u32),
        witx::IntRepr::U64 => quote!(u64),
    }
}
fn atom_token(atom: witx::AtomType) -> TokenStream {
    match atom {
        witx::AtomType::I32 => quote!(i32),
        witx::AtomType::I64 => quote!(i64),
        witx::AtomType::F32 => quote!(f32),
        witx::AtomType::F64 => quote!(f64),
    }
}

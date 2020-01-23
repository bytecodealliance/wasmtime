use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;

pub fn define_datatype(names: &Names, namedtype: &witx::NamedType) -> TokenStream {
    match &namedtype.tref {
        witx::TypeRef::Name(alias_to) => define_alias(names, &namedtype.name, &alias_to),
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Enum(e) => define_enum(names, &namedtype.name, &e),
            witx::Type::Int(_) => unimplemented!("int types"),
            witx::Type::Flags(_) => unimplemented!("flag types"),
            witx::Type::Struct(_) => unimplemented!("struct types"),
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

    let variant_names = e.variants.iter().map(|v| names.enum_variant(&v.name));
    let tryfrom_repr_cases = e.variants.iter().enumerate().map(|(n, v)| {
        let variant_name = names.enum_variant(&v.name);
        let n = n as u32;
        quote!(#n => Ok(#ident::#variant_name))
    });
    let to_repr_cases = e.variants.iter().enumerate().map(|(n, v)| {
        let variant_name = names.enum_variant(&v.name);
        let n = n as u32;
        quote!(#ident::#variant_name => #n)
    });

    quote! {
        #[repr(#repr)]
        #[derive(Copy, Clone, Debug, ::std::hash::Hash, Eq, PartialEq)]
        pub enum #ident {
            #(#variant_names),*
        }

        impl ::std::convert::TryFrom<#repr> for #ident {
            type Error = ::memory::GuestValueError;
            fn try_from(value: #repr) -> Result<#ident, ::memory::GuestValueError> {
                match value {
                    #(#tryfrom_repr_cases),*,
                    _ => Err(::memory::GuestValueError::InvalidEnum(stringify!(#ident))),
                }
            }
        }

        impl From<#ident> for #repr {
            fn from(e: #ident) -> #repr {
                match e {
                    #(#to_repr_cases),*
                }
            }
        }

        impl ::memory::GuestType for #ident {
            fn size() -> u32 {
                ::std::mem::size_of::<#repr>() as u32
            }
            fn name() -> &'static str {
                stringify!(#ident)
            }
        }

        impl ::memory::GuestTypeCopy for #ident {
            fn read_val(src: ::memory::GuestPtr<#ident>) -> Result<#ident, ::memory::GuestValueError> {
                use ::std::convert::TryInto;
                let val = unsafe { ::std::ptr::read_unaligned(src.ptr() as *const #repr) };
                val.try_into()
            }
            fn write_val(val: #ident, dest: ::memory::GuestPtrMut<#ident>) {
                let val: #repr = val.into();
                unsafe {
                    ::std::ptr::write_unaligned(dest.ptr_mut() as *mut #repr, val)
                };
            }
        }
    }
}

fn define_builtin(names: &Names, name: &witx::Id, builtin: witx::BuiltinType) -> TokenStream {
    let ident = names.type_(name);
    let built = names.builtin_type(builtin);
    quote!(pub type #ident = #built;)
}

fn int_repr_tokens(int_repr: witx::IntRepr) -> TokenStream {
    match int_repr {
        witx::IntRepr::U8 => quote!(u8),
        witx::IntRepr::U16 => quote!(u16),
        witx::IntRepr::U32 => quote!(u32),
        witx::IntRepr::U64 => quote!(u64),
    }
}

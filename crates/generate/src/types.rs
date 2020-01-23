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

    let mut output = TokenStream::new();
    let repr = int_repr_tokens(e.repr);
    output.extend(quote!(#[repr(#repr)]));
    output.extend(quote!(#[derive(Copy, Clone, Debug, std::hash::Hash, Eq, PartialEq)]));

    let mut variants = TokenStream::new();
    let mut to_repr_cases = TokenStream::new();
    let mut tryfrom_repr_cases = TokenStream::new();
    for (n, variant) in e.variants.iter().enumerate() {
        let n = n as u32;
        let variant_name = names.enum_variant(&variant.name);
        variants.extend(quote!(#variant_name,));
        tryfrom_repr_cases.extend(quote!(#n => Ok(#ident::#variant_name),));
        to_repr_cases.extend(quote!(#ident::#variant_name => #n,));
    }

    tryfrom_repr_cases
        .extend(quote!(_ => Err(::memory::GuestValueError::InvalidEnum(stringify!(#ident)))));

    output.extend(quote!(pub enum #ident {
        #variants
    }

    impl ::std::convert::TryFrom<#repr> for #ident {
        type Error = ::memory::GuestValueError;
        fn try_from(value: #repr) -> Result<#ident, ::memory::GuestValueError> {
            match value {
                #tryfrom_repr_cases
            }
        }
    }

    impl From<#ident> for #repr {
        fn from(e: #ident) -> #repr {
            match e {
                #to_repr_cases
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
    ));

    output
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

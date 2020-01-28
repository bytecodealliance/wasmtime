use heck::{CamelCase, SnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use witx::{AtomType, BuiltinType, Id, TypeRef};

#[derive(Debug, Clone)]
pub struct Names {
    // FIXME: overrides go in here, so we can map e.g. 2big => TooBig
}

impl Names {
    pub fn new() -> Names {
        Names {}
    }
    pub fn type_(&self, id: &Id) -> TokenStream {
        let ident = format_ident!("{}", id.as_str().to_camel_case());
        quote!(#ident)
    }
    pub fn builtin_type(&self, b: BuiltinType) -> TokenStream {
        match b {
            BuiltinType::String => quote!(String),
            BuiltinType::U8 => quote!(u8),
            BuiltinType::U16 => quote!(u16),
            BuiltinType::U32 => quote!(u32),
            BuiltinType::U64 => quote!(u64),
            BuiltinType::S8 => quote!(i8),
            BuiltinType::S16 => quote!(i16),
            BuiltinType::S32 => quote!(i32),
            BuiltinType::S64 => quote!(i64),
            BuiltinType::F32 => quote!(f32),
            BuiltinType::F64 => quote!(f64),
            BuiltinType::Char8 => quote!(char),
            BuiltinType::USize => quote!(usize),
        }
    }
    pub fn atom_type(&self, atom: AtomType) -> TokenStream {
        match atom {
            AtomType::I32 => quote!(i32),
            AtomType::I64 => quote!(i64),
            AtomType::F32 => quote!(f32),
            AtomType::F64 => quote!(f64),
        }
    }

    pub fn type_ref(&self, tref: &TypeRef) -> TokenStream {
        match tref {
            TypeRef::Name(nt) => {
                let ident = self.type_(&nt.name);
                quote!(#ident)
            }
            TypeRef::Value(ty) => match &**ty {
                witx::Type::Builtin(builtin) => self.builtin_type(*builtin),
                witx::Type::Pointer(pointee) => {
                    let pointee_type = self.type_ref(&pointee);
                    quote!(::memory::GuestPtrMut<#pointee_type>)
                }
                witx::Type::ConstPointer(pointee) => {
                    let pointee_type = self.type_ref(&pointee);
                    quote!(::memory::GuestPtr<#pointee_type>)
                }
                _ => unimplemented!("anonymous type ref"),
            },
        }
    }

    pub fn enum_variant(&self, id: &Id) -> Ident {
        // FIXME this is a hack - just a proof of concept.
        if id.as_str().starts_with('2') {
            format_ident!("TooBig")
        } else {
            format_ident!("{}", id.as_str().to_camel_case())
        }
    }

    pub fn struct_member(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_snake_case())
    }

    pub fn module(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_snake_case())
    }

    pub fn trait_name(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_camel_case())
    }

    pub fn func(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_snake_case())
    }

    pub fn func_param(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_snake_case())
    }

    /// For when you need a {name}_ptr binding for passing a value by reference:
    pub fn func_ptr_binding(&self, id: &Id) -> Ident {
        format_ident!("{}_ptr", id.as_str().to_snake_case())
    }

    /// For when you need a {name}_len binding for passing an array:
    pub fn func_len_binding(&self, id: &Id) -> Ident {
        format_ident!("{}_len", id.as_str().to_snake_case())
    }
}

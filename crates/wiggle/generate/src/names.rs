use heck::{CamelCase, ShoutySnakeCase, SnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use witx::{AtomType, BuiltinType, Id, Type, TypeRef};

use crate::lifetimes::LifetimeExt;

pub struct Names {
    ctx_type: Ident,
}

impl Names {
    pub fn new(ctx_type: &Ident) -> Names {
        Names {
            ctx_type: ctx_type.clone(),
        }
    }
    pub fn ctx_type(&self) -> Ident {
        self.ctx_type.clone()
    }
    pub fn type_(&self, id: &Id) -> TokenStream {
        let ident = format_ident!("{}", id.as_str().to_camel_case());
        quote!(#ident)
    }
    pub fn builtin_type(&self, b: BuiltinType, lifetime: TokenStream) -> TokenStream {
        match b {
            BuiltinType::String => quote!(wiggle::GuestPtr<#lifetime, str>),
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
            BuiltinType::Char8 => quote!(u8),
            BuiltinType::USize => quote!(u32),
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

    pub fn type_ref(&self, tref: &TypeRef, lifetime: TokenStream) -> TokenStream {
        match tref {
            TypeRef::Name(nt) => {
                let ident = self.type_(&nt.name);
                if nt.tref.needs_lifetime() {
                    quote!(#ident<#lifetime>)
                } else {
                    quote!(#ident)
                }
            }
            TypeRef::Value(ty) => match &**ty {
                Type::Builtin(builtin) => self.builtin_type(*builtin, lifetime.clone()),
                Type::Pointer(pointee) | Type::ConstPointer(pointee) => {
                    let pointee_type = self.type_ref(&pointee, lifetime.clone());
                    quote!(wiggle::GuestPtr<#lifetime, #pointee_type>)
                }
                Type::Array(pointee) => {
                    let pointee_type = self.type_ref(&pointee, lifetime.clone());
                    quote!(wiggle::GuestPtr<#lifetime, [#pointee_type]>)
                }
                _ => unimplemented!("anonymous type ref {:?}", tref),
            },
        }
    }

    pub fn enum_variant(&self, id: &Id) -> Ident {
        // FIXME this is a hack - just a proof of concept.
        if id.as_str().starts_with('2') {
            format_ident!("TooBig")
        } else if id.as_str() == "type" {
            format_ident!("Type")
        } else {
            format_ident!("{}", id.as_str().to_camel_case())
        }
    }

    pub fn flag_member(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_shouty_snake_case())
    }

    pub fn int_member(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_shouty_snake_case())
    }

    pub fn struct_member(&self, id: &Id) -> Ident {
        // FIXME this is a hack - just a proof of concept.
        if id.as_str() == "type" {
            format_ident!("type_")
        } else {
            format_ident!("{}", id.as_str().to_snake_case())
        }
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
        // FIXME this is a hack - just a proof of concept.
        if id.as_str() == "in" {
            format_ident!("in_")
        } else {
            format_ident!("{}", id.as_str().to_snake_case())
        }
    }

    pub fn func_core_arg(&self, arg: &witx::CoreParamType) -> Ident {
        match arg.signifies {
            witx::CoreParamSignifies::Value { .. } => self.func_param(&arg.param.name),
            witx::CoreParamSignifies::PointerTo => self.func_ptr_binding(&arg.param.name),
            witx::CoreParamSignifies::LengthOf => self.func_len_binding(&arg.param.name),
        }
    }

    /// For when you need a {name}_ptr binding for passing a value by reference:
    pub fn func_ptr_binding(&self, id: &Id) -> Ident {
        format_ident!("{}_ptr", id.as_str().to_snake_case())
    }

    /// For when you need a {name}_len binding for passing an array:
    pub fn func_len_binding(&self, id: &Id) -> Ident {
        format_ident!("{}_len", id.as_str().to_snake_case())
    }

    pub fn guest_error_conversion_method(&self, tref: &TypeRef) -> Ident {
        match tref {
            TypeRef::Name(nt) => format_ident!("into_{}", nt.name.as_str().to_snake_case()),
            TypeRef::Value(ty) => match &**ty {
                Type::Builtin(b) => match b {
                    BuiltinType::String => unreachable!("error type must be atom"),
                    BuiltinType::U8 => format_ident!("into_u8"),
                    BuiltinType::U16 => format_ident!("into_u16"),
                    BuiltinType::U32 => format_ident!("into_u32"),
                    BuiltinType::U64 => format_ident!("into_u64"),
                    BuiltinType::S8 => format_ident!("into_i8"),
                    BuiltinType::S16 => format_ident!("into_i16"),
                    BuiltinType::S32 => format_ident!("into_i32"),
                    BuiltinType::S64 => format_ident!("into_i64"),
                    BuiltinType::F32 => format_ident!("into_f32"),
                    BuiltinType::F64 => format_ident!("into_f64"),
                    BuiltinType::Char8 => format_ident!("into_char8"),
                    BuiltinType::USize => format_ident!("into_usize"),
                },
                _ => panic!("unexpected anonymous error type: {:?}", ty),
            },
        }
    }
}

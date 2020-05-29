use heck::{CamelCase, ShoutySnakeCase, SnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use witx::{AtomType, BuiltinType, Id, Type, TypeRef};

use crate::{lifetimes::LifetimeExt, UserErrorType};

pub struct Names {
    ctx_type: Ident,
    runtime_mod: TokenStream,
}

impl Names {
    pub fn new(ctx_type: &Ident, runtime_mod: TokenStream) -> Names {
        Names {
            ctx_type: ctx_type.clone(),
            runtime_mod,
        }
    }
    pub fn ctx_type(&self) -> Ident {
        self.ctx_type.clone()
    }
    pub fn runtime_mod(&self) -> TokenStream {
        self.runtime_mod.clone()
    }
    pub fn type_(&self, id: &Id) -> TokenStream {
        let ident = format_ident!("{}", id.as_str().to_camel_case());
        quote!(#ident)
    }
    pub fn builtin_type(&self, b: BuiltinType, lifetime: TokenStream) -> TokenStream {
        match b {
            BuiltinType::String => {
                let rt = self.runtime_mod();
                quote!(#rt::GuestPtr<#lifetime, str>)
            }
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
                    let rt = self.runtime_mod();
                    let pointee_type = self.type_ref(&pointee, lifetime.clone());
                    quote!(#rt::GuestPtr<#lifetime, #pointee_type>)
                }
                Type::Array(pointee) => {
                    let rt = self.runtime_mod();
                    let pointee_type = self.type_ref(&pointee, lifetime.clone());
                    quote!(#rt::GuestPtr<#lifetime, [#pointee_type]>)
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

    fn builtin_name(b: &BuiltinType) -> &'static str {
        match b {
            BuiltinType::String => "string",
            BuiltinType::U8 => "u8",
            BuiltinType::U16 => "u16",
            BuiltinType::U32 => "u32",
            BuiltinType::U64 => "u64",
            BuiltinType::S8 => "i8",
            BuiltinType::S16 => "i16",
            BuiltinType::S32 => "i32",
            BuiltinType::S64 => "i64",
            BuiltinType::F32 => "f32",
            BuiltinType::F64 => "f64",
            BuiltinType::Char8 => "char8",
            BuiltinType::USize => "usize",
        }
    }

    fn snake_typename(tref: &TypeRef) -> String {
        match tref {
            TypeRef::Name(nt) => nt.name.as_str().to_snake_case(),
            TypeRef::Value(ty) => match &**ty {
                Type::Builtin(b) => Self::builtin_name(&b).to_owned(),
                _ => panic!("unexpected anonymous type: {:?}", ty),
            },
        }
    }

    pub fn guest_error_conversion_method(&self, tref: &TypeRef) -> Ident {
        let suffix = Self::snake_typename(tref);
        format_ident!("into_{}", suffix)
    }

    pub fn user_error_conversion_method(&self, user_type: &UserErrorType) -> Ident {
        let abi_type = Self::snake_typename(&user_type.abi_type());
        format_ident!(
            "{}_from_{}",
            abi_type,
            user_type.method_fragment().to_snake_case()
        )
    }
}

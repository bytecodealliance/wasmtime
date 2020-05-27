use crate::lifetimes::LifetimeExt;
use escaping::{escape_id, handle_2big_enum_variant, NamingConvention};
use heck::{ShoutySnakeCase, SnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use witx::{AtomType, BuiltinType, Id, Type, TypeRef};

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
        let ident = escape_id(id, NamingConvention::CamelCase);
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

    /// Convert an enum variant from its [`Id`][witx] name to its Rust [`Ident`][id] representation.
    ///
    /// [id]: https://docs.rs/proc-macro2/*/proc_macro2/struct.Ident.html
    /// [witx]: https://docs.rs/witx/*/witx/struct.Id.html
    pub fn enum_variant(&self, id: &Id) -> Ident {
        handle_2big_enum_variant(id).unwrap_or_else(|| escape_id(id, NamingConvention::CamelCase))
    }

    pub fn flag_member(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_shouty_snake_case())
    }

    pub fn int_member(&self, id: &Id) -> Ident {
        format_ident!("{}", id.as_str().to_shouty_snake_case())
    }

    /// Convert a struct member from its [`Id`][witx] name to its Rust [`Ident`][id] representation.
    ///
    /// [id]: https://docs.rs/proc-macro2/*/proc_macro2/struct.Ident.html
    /// [witx]: https://docs.rs/witx/*/witx/struct.Id.html
    pub fn struct_member(&self, id: &Id) -> Ident {
        escape_id(id, NamingConvention::SnakeCase)
    }

    /// Convert a module name from its [`Id`][witx] name to its Rust [`Ident`][id] representation.
    ///
    /// [id]: https://docs.rs/proc-macro2/*/proc_macro2/struct.Ident.html
    /// [witx]: https://docs.rs/witx/*/witx/struct.Id.html
    pub fn module(&self, id: &Id) -> Ident {
        escape_id(id, NamingConvention::SnakeCase)
    }

    /// Convert a trait name from its [`Id`][witx] name to its Rust [`Ident`][id] representation.
    ///
    /// [id]: https://docs.rs/proc-macro2/*/proc_macro2/struct.Ident.html
    /// [witx]: https://docs.rs/witx/*/witx/struct.Id.html
    pub fn trait_name(&self, id: &Id) -> Ident {
        escape_id(id, NamingConvention::CamelCase)
    }

    /// Convert a function name from its [`Id`][witx] name to its Rust [`Ident`][id] representation.
    ///
    /// [id]: https://docs.rs/proc-macro2/*/proc_macro2/struct.Ident.html
    /// [witx]: https://docs.rs/witx/*/witx/struct.Id.html
    pub fn func(&self, id: &Id) -> Ident {
        escape_id(id, NamingConvention::SnakeCase)
    }

    /// Convert a parameter name from its [`Id`][witx] name to its Rust [`Ident`][id] representation.
    ///
    /// [id]: https://docs.rs/proc-macro2/*/proc_macro2/struct.Ident.html
    /// [witx]: https://docs.rs/witx/*/witx/struct.Id.html
    pub fn func_param(&self, id: &Id) -> Ident {
        escape_id(id, NamingConvention::SnakeCase)
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

/// Identifier escaping utilities.
///
/// This module most importantly exports an `escape_id` function that can be used to properly
/// escape tokens that conflict with strict and reserved keywords, as of Rust's 2018 edition.
///
/// Weak keywords are not included as their semantic rules do not have the same implications as
/// those of strict and reserved keywords. `union` for example, is permitted as the name of a
/// variable. `dyn` was promoted to a strict keyword beginning in the 2018 edition.
mod escaping {
    use {
        heck::{CamelCase, SnakeCase},
        proc_macro2::Ident,
        quote::format_ident,
        witx::Id,
    };

    /// Identifier naming convention.
    ///
    /// Because shouty snake case values (identifiers that look `LIKE_THIS`) cannot potentially
    /// conflict with any Rust keywords, this enum only include snake and camel case variants.
    pub enum NamingConvention {
        /// Snake case. Used to denote values `LikeThis`.
        CamelCase,
        /// Snake case. Used to denote values `like_this`.
        SnakeCase,
    }

    /// Given a witx [`Id`][witx] and a [`NamingConvention`][naming], return a [`Ident`] word of
    /// Rust syntax that accounts for escaping both strict and reserved keywords. If an identifier
    /// would have conflicted with a keyword, a trailing underscode will be appended.
    ///
    /// [id]: https://docs.rs/proc-macro2/*/proc_macro2/struct.Ident.html
    /// [naming]: enum.NamingConvention.html
    /// [witx]: https://docs.rs/witx/*/witx/struct.Id.html
    pub fn escape_id(id: &Id, conv: NamingConvention) -> Ident {
        use NamingConvention::{CamelCase, SnakeCase};
        match (conv, id.as_str()) {
            // For camel-cased identifiers, `Self` is the only potential keyword conflict.
            (CamelCase, "self") => format_ident!("Self_"),
            (CamelCase, s) => format_ident!("{}", s.to_camel_case()),
            // Snake-cased identifiers are where the bulk of conflicts can occur.
            (SnakeCase, s) => {
                let s = s.to_snake_case();
                if STRICT.iter().chain(RESERVED).any(|k| *k == s) {
                    // If the camel-cased string matched any strict or reserved keywords, then
                    // append a trailing underscore to the identifier we generate.
                    format_ident!("{}_", s)
                } else {
                    format_ident!("{}", s) // Otherwise, use the string as is.
                }
            }
        }
    }

    /// Strict keywords.
    ///
    /// >  Strict keywords cannot be used as the names of:
    /// >    * Items
    /// >    * Variables and function parameters
    /// >    * Fields and variants
    /// >    * Type parameters
    /// >    * Lifetime parameters or loop labels
    /// >    * Macros or attributes
    /// >    * Macro placeholders
    /// >    * Crates
    /// >
    /// > - <cite>[The Rust Reference][ref]</cite>
    ///
    /// This list also includes keywords that were introduced in the 2018 edition of Rust.
    ///
    /// [ref]: https://doc.rust-lang.org/reference/keywords.html#strict-keywords
    const STRICT: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "true", "type", "unsafe", "use", "where", "while",
    ];

    /// Reserved keywords.
    ///
    /// > These keywords aren't used yet, but they are reserved for future use. They have the same
    /// > restrictions as strict keywords. The reasoning behind this is to make current programs
    /// > forward compatible with future versions of Rust by forbidding them to use these keywords.
    /// >
    /// > - <cite>[The Rust Reference][ref]</cite>
    ///
    /// This list also includes keywords that were introduced in the 2018 edition of Rust.
    ///
    /// [ref]: https://doc.rust-lang.org/reference/keywords.html#reserved-keywords
    const RESERVED: &[&str] = &[
        "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try", "typeof",
        "unsized", "virtual", "yield",
    ];

    /// Handle WASI's [`errno::2big`][err] variant.
    ///
    /// This is an unfortunate edge case that must account for when generating `enum` variants.
    /// This will only return `Some(_)` if the given witx identifier *is* `2big`, otherwise this
    /// function will return `None`.
    ///
    /// [err]: https://github.com/WebAssembly/WASI/blob/master/phases/snapshot/docs.md#-errno-enumu16
    pub fn handle_2big_enum_variant(id: &Id) -> Option<Ident> {
        if id.as_str() == "2big" {
            Some(format_ident!("TooBig"))
        } else {
            None
        }
    }
}

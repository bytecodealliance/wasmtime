//! Translate witx types to Rust.

use crate::utils;
use heck::ShoutySnakeCase;
use proc_macro2::{Delimiter, Group, Literal, TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::convert::TryFrom;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Host,
    Wasi32,
    Wasi,
}

impl Mode {
    pub fn include_target_types(&self) -> bool {
        match self {
            Mode::Host | Mode::Wasi32 => true,
            Mode::Wasi => false,
        }
    }
}

pub fn gen(args: TokenStream, mode: Mode) -> TokenStream {
    let mut output = TokenStream::new();

    let (path, _phase) = utils::witx_path_from_args(args);
    let doc = match witx::load(&[&path]) {
        Ok(doc) => doc,
        Err(e) => {
            panic!("error opening file {}: {}", path, e);
        }
    };

    gen_datatypes(&mut output, &doc, mode);

    output
}

fn gen_datatypes(output: &mut TokenStream, doc: &witx::Document, mode: Mode) {
    for datatype in doc.datatypes() {
        if mode.include_target_types() != type_has_target_size(doc, &datatype) {
            continue;
        }

        gen_datatype(output, doc, mode, &datatype);
    }
}

fn gen_datatype(
    output: &mut TokenStream,
    doc: &witx::Document,
    mode: Mode,
    datatype: &witx::Datatype,
) {
    match &datatype.variant {
        witx::DatatypeVariant::Alias(a) => {
            if a.name.as_str() == "size" {
                let wasi_name = format_ident!("__wasi_{}_t", a.name.as_str());
                match mode {
                    Mode::Host => output.extend(quote!(pub type #wasi_name = usize;)),
                    Mode::Wasi => panic!("size has target-specific size"),
                    Mode::Wasi32 => output.extend(quote!(pub type #wasi_name = u32;)),
                }
            } else {
                let wasi_name = format_ident!("__wasi_{}_t", a.name.as_str());
                let to = ident_tokens(mode, &a.to);
                output.extend(quote!(pub type #wasi_name = #to;));
            }
        }
        witx::DatatypeVariant::Enum(e) => {
            let wasi_name = format_ident!("__wasi_{}_t", e.name.as_str());
            let repr = int_repr_tokens(e.repr);
            output.extend(quote!(pub type #wasi_name = #repr;));
            for (index, variant) in e.variants.iter().enumerate() {
                let value_name = format_ident!(
                    "__WASI_{}_{}",
                    e.name.as_str().to_shouty_snake_case(),
                    variant.name.as_str().to_shouty_snake_case()
                );
                let index_name = Literal::usize_unsuffixed(index);
                output.extend(quote!(pub const #value_name: #wasi_name = #index_name;));
            }
        }
        witx::DatatypeVariant::Flags(f) => {
            let wasi_name = format_ident!("__wasi_{}_t", f.name.as_str());
            let repr = int_repr_tokens(f.repr);
            output.extend(quote!(pub type #wasi_name = #repr;));
            for (index, flag) in f.flags.iter().enumerate() {
                let value_name = format_ident!(
                    "__WASI_{}_{}",
                    f.name.as_str().to_shouty_snake_case(),
                    flag.name.as_str().to_shouty_snake_case()
                );
                let flag_value = Literal::u128_unsuffixed(
                    1u128
                        .checked_shl(u32::try_from(index).expect("flag value overflow"))
                        .expect("flag value overflow"),
                );
                output.extend(quote!(pub const #value_name: #wasi_name = #flag_value;));
            }
        }
        witx::DatatypeVariant::Struct(s) => {
            output.extend(quote!(#[repr(C)]));

            // Types which contain unions can't trivially implement Debug,
            // Hash, or Eq, because the type itself doesn't record which
            // union member is active.
            if struct_has_union(&doc, s) {
                output.extend(quote!(#[derive(Copy, Clone)]));
                output.extend(quote!(#[allow(missing_debug_implementations)]));
            } else {
                output.extend(quote!(#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]));
            }

            let wasi_name = format_ident!("__wasi_{}_t", s.name.as_str());
            output.extend(quote!(pub struct #wasi_name));

            let mut inner = TokenStream::new();
            for member in &s.members {
                let member_name = format_ident!("r#{}", member.name.as_str());
                let member_type = ident_tokens(mode, &member.type_);
                inner.extend(quote!(pub #member_name: #member_type,));
            }
            let braced = Group::new(Delimiter::Brace, inner);
            output.extend(TokenStream::from(TokenTree::Group(braced)));
        }
        witx::DatatypeVariant::Union(u) => {
            output.extend(quote!(#[repr(C)]));
            output.extend(quote!(#[derive(Copy, Clone)]));
            output.extend(quote!(#[allow(missing_debug_implementations)]));

            let wasi_name = format_ident!("__wasi_{}_t", u.name.as_str());
            output.extend(quote!(pub union #wasi_name));

            let mut inner = TokenStream::new();
            for variant in &u.variants {
                let variant_name = format_ident!("r#{}", variant.name.as_str());
                let variant_type = ident_tokens(mode, &variant.type_);
                inner.extend(quote!(pub #variant_name: #variant_type,));
            }
            let braced = Group::new(Delimiter::Brace, inner);
            output.extend(TokenStream::from(TokenTree::Group(braced)));
        }
        witx::DatatypeVariant::Handle(a) => {
            let wasi_name = format_ident!("__wasi_{}_t", a.name.as_str());
            output.extend(quote!(pub type #wasi_name = u32;));
        }
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

fn builtin_tokens(mode: Mode, builtin: witx::BuiltinType) -> TokenStream {
    match builtin {
        witx::BuiltinType::String => match mode {
            Mode::Host => quote!((*const u8, usize)),
            Mode::Wasi => panic!("strings have target-specific size"),
            Mode::Wasi32 => quote!((u32, u32)),
        },
        witx::BuiltinType::U8 => quote!(u8),
        witx::BuiltinType::U16 => quote!(u16),
        witx::BuiltinType::U32 => quote!(u32),
        witx::BuiltinType::U64 => quote!(u64),
        witx::BuiltinType::S8 => quote!(i8),
        witx::BuiltinType::S16 => quote!(i16),
        witx::BuiltinType::S32 => quote!(i32),
        witx::BuiltinType::S64 => quote!(i64),
        witx::BuiltinType::F32 => quote!(f32),
        witx::BuiltinType::F64 => quote!(f64),
    }
}

fn ident_tokens(mode: Mode, ident: &witx::DatatypeIdent) -> TokenStream {
    match ident {
        witx::DatatypeIdent::Builtin(builtin) => builtin_tokens(mode, *builtin),
        witx::DatatypeIdent::Ident(ident) => TokenStream::from(TokenTree::Ident(format_ident!(
            "__wasi_{}_t",
            ident.name.as_str()
        ))),
        witx::DatatypeIdent::Pointer(pointee) => {
            let pointee = ident_tokens(mode, pointee);
            match mode {
                Mode::Host => quote!(*mut #pointee),
                Mode::Wasi => panic!("pointers have target-specific size"),
                Mode::Wasi32 => quote!(u32),
            }
        }
        witx::DatatypeIdent::ConstPointer(pointee) => {
            let pointee = ident_tokens(mode, pointee);
            match mode {
                Mode::Host => quote!(*const #pointee),
                Mode::Wasi => panic!("pointers have target-specific size"),
                Mode::Wasi32 => quote!(u32),
            }
        }
        witx::DatatypeIdent::Array(element) => {
            let element_name = ident_tokens(mode, element);
            match mode {
                Mode::Host => quote!((*const #element_name, usize)),
                Mode::Wasi => panic!("arrays have target-specific size"),
                Mode::Wasi32 => quote!((u32, u32)),
            }
        }
    }
}

/// Test whether the given struct contains any union members.
fn struct_has_union(doc: &witx::Document, s: &witx::StructDatatype) -> bool {
    s.members.iter().any(|member| match &member.type_ {
        witx::DatatypeIdent::Ident(ident) => match &doc.datatype(&ident.name).unwrap().variant {
            witx::DatatypeVariant::Union(_) => true,
            witx::DatatypeVariant::Struct(s) => struct_has_union(doc, &s),
            _ => false,
        },
        _ => false,
    })
}

/// Test whether the given type has a target-specific size.
fn type_has_target_size(doc: &witx::Document, type_: &witx::Datatype) -> bool {
    match &type_.variant {
        witx::DatatypeVariant::Alias(a) => {
            a.name.as_str() == "size" || ident_has_target_size(doc, &a.to)
        }
        witx::DatatypeVariant::Enum(_) => false,
        witx::DatatypeVariant::Flags(_) => false,
        witx::DatatypeVariant::Struct(s) => s
            .members
            .iter()
            .any(|m| ident_has_target_size(doc, &m.type_)),
        witx::DatatypeVariant::Union(u) => u
            .variants
            .iter()
            .any(|v| ident_has_target_size(doc, &v.type_)),
        witx::DatatypeVariant::Handle(_) => false,
    }
}

/// Test whether the given type ident has a target-specific size.
fn ident_has_target_size(doc: &witx::Document, ident: &witx::DatatypeIdent) -> bool {
    match ident {
        witx::DatatypeIdent::Ident(ident) => {
            type_has_target_size(doc, &doc.datatype(&ident.name).unwrap())
        }
        witx::DatatypeIdent::Builtin(builtin) => {
            if let witx::BuiltinType::String = builtin {
                true
            } else {
                false
            }
        }
        witx::DatatypeIdent::Pointer(_) | witx::DatatypeIdent::ConstPointer(_) => true,
        witx::DatatypeIdent::Array(element) => ident_has_target_size(doc, element),
    }
}

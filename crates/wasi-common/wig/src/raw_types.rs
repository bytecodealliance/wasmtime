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

    let path = utils::witx_path_from_args(args);
    let doc = match witx::load(&[&path]) {
        Ok(doc) => doc,
        Err(e) => {
            panic!("error opening file {}: {}", path.display(), e);
        }
    };

    gen_datatypes(&mut output, &doc, mode);

    output
}

fn gen_datatypes(output: &mut TokenStream, doc: &witx::Document, mode: Mode) {
    let mut test_contents = TokenStream::new();
    for namedtype in doc.typenames() {
        if mode.include_target_types() != namedtype_has_target_size(&namedtype) {
            continue;
        }
        gen_datatype(output, &mut test_contents, mode, &namedtype);
    }
    match mode {
        Mode::Wasi | Mode::Wasi32 => output.extend(quote! {
            #[cfg(test)]
            mod test {
                use super::*;
                #test_contents
            }
        }),
        Mode::Host => {} // Don't emit tests for host reprs - the layout is different
    }
}

fn gen_datatype(
    output: &mut TokenStream,
    test_contents: &mut TokenStream,
    mode: Mode,
    namedtype: &witx::NamedType,
) {
    let wasi_name = format_ident!("__wasi_{}_t", namedtype.name.as_str());
    let (size, align) = {
        use witx::Layout;
        let sa = namedtype.type_().mem_size_align();
        (sa.size, sa.align)
    };
    let mut test_code = quote! {
        assert_eq!(::std::mem::size_of::<#wasi_name>(), #size, concat!("Size of: ", stringify!(#wasi_name)));
        assert_eq!(::std::mem::align_of::<#wasi_name>(), #align, concat!("Align of: ", stringify!(#wasi_name)));
    };
    match &namedtype.tref {
        witx::TypeRef::Name(alias_to) => {
            let to = tref_tokens(mode, &alias_to.tref);
            output.extend(quote!(pub type #wasi_name = #to;));
        }
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Int(_) => panic!("unsupported int datatype"),
            witx::Type::Enum(e) => {
                let repr = int_repr_tokens(e.repr);
                output.extend(quote!(pub type #wasi_name = #repr;));
                for (index, variant) in e.variants.iter().enumerate() {
                    let value_name = format_ident!(
                        "__WASI_{}_{}",
                        namedtype.name.as_str().to_shouty_snake_case(),
                        variant.name.as_str().to_shouty_snake_case()
                    );
                    let index_name = Literal::usize_unsuffixed(index);
                    output.extend(quote!(pub const #value_name: #wasi_name = #index_name;));
                }
            }
            witx::Type::Flags(f) => {
                let repr = int_repr_tokens(f.repr);
                output.extend(quote!(pub type #wasi_name = #repr;));
                for (index, flag) in f.flags.iter().enumerate() {
                    let value_name = format_ident!(
                        "__WASI_{}_{}",
                        namedtype.name.as_str().to_shouty_snake_case(),
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
            witx::Type::Struct(s) => {
                output.extend(quote!(#[repr(C)]));
                // Types which contain unions can't trivially implement Debug,
                // Hash, or Eq, because the type itself doesn't record which
                // union member is active.
                if struct_has_union(&s) {
                    output.extend(quote!(#[derive(Copy, Clone)]));
                    output.extend(quote!(#[allow(missing_debug_implementations)]));
                } else {
                    output.extend(quote!(#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]));
                }

                output.extend(quote!(pub struct #wasi_name));

                let mut inner = TokenStream::new();
                for ml in s.member_layout().iter() {
                    let member_name = format_ident!("r#{}", ml.member.name.as_str());
                    let member_type = tref_tokens(mode, &ml.member.tref);
                    let offset = ml.offset;
                    inner.extend(quote!(pub #member_name: #member_type,));
                    test_code.extend(quote!{
                        assert_eq!(
                            unsafe { &(*(::std::ptr::null::<#wasi_name>())).#member_name as *const _ as usize },
                            #offset,
                            concat!(
                                "Offset of field: ",
                                stringify!(#wasi_name),
                                "::",
                                stringify!(#member_name),
                            )
                        );
                    });
                }
                let braced = Group::new(Delimiter::Brace, inner);
                output.extend(TokenStream::from(TokenTree::Group(braced)));
            }
            witx::Type::Union(u) => {
                let u_name = format_ident!("__wasi_{}_u_t", namedtype.name.as_str());
                output.extend(quote!(#[repr(C)]));
                output.extend(quote!(#[derive(Copy, Clone)]));
                output.extend(quote!(#[allow(missing_debug_implementations)]));

                output.extend(quote!(pub union #u_name));

                let mut inner = TokenStream::new();
                for variant in &u.variants {
                    let variant_name = format_ident!("r#{}", variant.name.as_str());
                    if let Some(ref tref) = variant.tref {
                        let variant_type = tref_tokens(mode, tref);
                        inner.extend(quote!(pub #variant_name: #variant_type,));
                    } else {
                        inner.extend(quote!(pub #variant_name: (),));
                    }
                }
                let braced = Group::new(Delimiter::Brace, inner);
                output.extend(TokenStream::from(TokenTree::Group(braced)));

                output.extend(quote!(#[repr(C)]));
                output.extend(quote!(#[derive(Copy, Clone)]));
                output.extend(quote!(#[allow(missing_debug_implementations)]));

                output.extend(quote!(pub struct #wasi_name));
                let tag_name = format_ident!("__wasi_{}_t", u.tag.name.as_str());
                let inner = quote!(pub tag: #tag_name, pub u: #u_name,);
                output.extend(TokenStream::from(TokenTree::Group(Group::new(
                    Delimiter::Brace,
                    inner,
                ))));
            }
            witx::Type::Handle(_h) => {
                output.extend(quote!(pub type #wasi_name = u32;));
            }
            witx::Type::Builtin(b) => {
                if namedtype.name.as_str() == "size" {
                    match mode {
                        Mode::Host => output.extend(quote!(pub type #wasi_name = usize;)),
                        Mode::Wasi => panic!("size has target-specific size"),
                        Mode::Wasi32 => output.extend(quote!(pub type #wasi_name = u32;)),
                    }
                } else {
                    let b_type = builtin_tokens(mode, *b);
                    output.extend(quote!(pub type #wasi_name = #b_type;));
                }
            }
            witx::Type::Pointer { .. }
            | witx::Type::ConstPointer { .. }
            | witx::Type::Array { .. } => {
                let tref_tokens = tref_tokens(mode, &namedtype.tref);
                output.extend(quote!(pub type #wasi_name = #tref_tokens;));
            }
        },
    }

    if namedtype.name.as_str() == "errno" {
        // Generate strerror for errno type
        gen_errno_strerror(output, namedtype);
    }

    let test_func_name = format_ident!("wig_test_layout_{}", namedtype.name.as_str());
    test_contents.extend(quote! {
        #[test]
        fn #test_func_name() {
            #test_code
        }
    });
}

fn gen_errno_strerror(output: &mut TokenStream, namedtype: &witx::NamedType) {
    let inner = match &namedtype.tref {
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Enum(e) => e,
            x => panic!("expected Enum('errno'), instead received {:?}", x),
        },
        x => panic!("expected Enum('errno'), instead received {:?}", x),
    };
    let mut inner_group = TokenStream::new();
    for variant in &inner.variants {
        let value_name = format_ident!(
            "__WASI_ERRNO_{}",
            variant.name.as_str().to_shouty_snake_case()
        );
        let docs = variant.docs.trim();
        inner_group.extend(quote!(#value_name => #docs,));
    }
    output.extend(
        quote!(pub fn strerror(errno: __wasi_errno_t) -> &'static str {
            match errno {
                #inner_group
                other => panic!("Undefined errno value {:?}", other),
            }
        }),
    );
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
        witx::BuiltinType::Char8 => quote!(i8),
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
        witx::BuiltinType::USize => match mode {
            Mode::Host => quote!(usize),
            Mode::Wasi => panic!("usize has target-specific size"),
            Mode::Wasi32 => quote!(u32),
        },
    }
}

fn tref_tokens(mode: Mode, tref: &witx::TypeRef) -> TokenStream {
    match tref {
        witx::TypeRef::Name(n) => TokenStream::from(TokenTree::Ident(format_ident!(
            "__wasi_{}_t",
            n.name.as_str()
        ))),
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Builtin(b) => builtin_tokens(mode, *b),
            witx::Type::Pointer(pointee) => {
                let pointee = tref_tokens(mode, pointee);
                match mode {
                    Mode::Host => quote!(*mut #pointee),
                    Mode::Wasi => panic!("pointers have target-specific size"),
                    Mode::Wasi32 => quote!(u32),
                }
            }
            witx::Type::ConstPointer(pointee) => {
                let pointee = tref_tokens(mode, pointee);
                match mode {
                    Mode::Host => quote!(*const #pointee),
                    Mode::Wasi => panic!("pointers have target-specific size"),
                    Mode::Wasi32 => quote!(u32),
                }
            }
            witx::Type::Array(element) => {
                let element_name = tref_tokens(mode, element);
                match mode {
                    Mode::Host => quote!((*const #element_name, usize)),
                    Mode::Wasi => panic!("arrays have target-specific size"),
                    Mode::Wasi32 => quote!((u32, u32)),
                }
            }
            t => panic!("cannot give name to anonymous type {:?}", t),
        },
    }
}

/// Test whether the given struct contains any union members.
fn struct_has_union(s: &witx::StructDatatype) -> bool {
    s.members.iter().any(|member| match &*member.tref.type_() {
        witx::Type::Union { .. } => true,
        witx::Type::Struct(s) => struct_has_union(&s),
        _ => false,
    })
}

/// Test whether the type referred to has a target-specific size.
fn tref_has_target_size(tref: &witx::TypeRef) -> bool {
    match tref {
        witx::TypeRef::Name(nt) => namedtype_has_target_size(&nt),
        witx::TypeRef::Value(t) => type_has_target_size(&t),
    }
}

/// Test whether the given named type has a target-specific size.
fn namedtype_has_target_size(nt: &witx::NamedType) -> bool {
    if nt.name.as_str() == "size" {
        true
    } else {
        tref_has_target_size(&nt.tref)
    }
}

/// Test whether the given type has a target-specific size.
fn type_has_target_size(ty: &witx::Type) -> bool {
    match ty {
        witx::Type::Builtin(witx::BuiltinType::String) => true,
        witx::Type::Pointer { .. } | witx::Type::ConstPointer { .. } => true,
        witx::Type::Array(elem) => tref_has_target_size(elem),
        witx::Type::Struct(s) => s.members.iter().any(|m| tref_has_target_size(&m.tref)),
        witx::Type::Union(u) => u
            .variants
            .iter()
            .any(|v| v.tref.as_ref().map(tref_has_target_size).unwrap_or(false)),
        _ => false,
    }
}

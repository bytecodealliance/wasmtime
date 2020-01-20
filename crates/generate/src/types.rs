use heck::{CamelCase, MixedCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn define_datatype(namedtype: &witx::NamedType) -> TokenStream {
    match &namedtype.tref {
        witx::TypeRef::Name(alias_to) => define_alias(&namedtype.name, &alias_to),
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Enum(e) => define_enum(&namedtype.name, &e),
            witx::Type::Int(_) => unimplemented!("int types"),
            witx::Type::Flags(_) => unimplemented!("flag types"),
            witx::Type::Struct(_) => unimplemented!("struct types"),
            witx::Type::Union(_) => unimplemented!("union types"),
            witx::Type::Handle(_h) => unimplemented!("handle types"),
            witx::Type::Builtin(b) => define_builtin(&namedtype.name, &b),
            witx::Type::Pointer { .. } => unimplemented!("pointer types"),
            witx::Type::ConstPointer { .. } => unimplemented!("constpointer types"),
            witx::Type::Array { .. } => unimplemented!("array types"),
        },
    }
}

fn define_alias(name: &witx::Id, to: &witx::NamedType) -> TokenStream {
    let ident = format_ident!("{}", name.as_str().to_camel_case());
    let to = format_ident!("{}", to.name.as_str().to_camel_case());

    quote!(pub type #ident = #to;)
}

fn define_enum(name: &witx::Id, e: &witx::EnumDatatype) -> TokenStream {
    let ident = format_ident!("{}", name.as_str().to_camel_case());
    let mut output = TokenStream::new();
    let repr = int_repr_tokens(e.repr);
    output.extend(quote!(#[repr(#repr)]));
    output.extend(quote!(#[derive(Copy, Clone, Debug, std::hash::Hash, Eq, PartialEq)]));

    let mut inner = TokenStream::new();
    for variant in &e.variants {
        let value_name = if name.as_str() == "errno" {
            // FIXME discussion point!
            format_ident!("E{}", variant.name.as_str().to_mixed_case())
        } else {
            format_ident!("{}", variant.name.as_str().to_camel_case())
        };
        inner.extend(quote!(#value_name,));
    }

    output.extend(quote!(pub enum #ident {
        #inner
    }));

    output
}

fn define_builtin(name: &witx::Id, builtin: &witx::BuiltinType) -> TokenStream {
    let ident = format_ident!("{}", name.as_str().to_camel_case());
    let prim = match builtin {
        witx::BuiltinType::String => quote!(String),
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
        witx::BuiltinType::Char8 => quote!(char),
        witx::BuiltinType::USize => quote!(usize),
    };
    quote!(pub type #ident = #prim;)
}

fn int_repr_tokens(int_repr: witx::IntRepr) -> TokenStream {
    match int_repr {
        witx::IntRepr::U8 => quote!(u8),
        witx::IntRepr::U16 => quote!(u16),
        witx::IntRepr::U32 => quote!(u32),
        witx::IntRepr::U64 => quote!(u64),
    }
}

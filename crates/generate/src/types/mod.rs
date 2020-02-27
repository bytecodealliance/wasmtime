mod r#enum;
mod flags;
mod handle;
mod int;
mod r#struct;
mod union;

use crate::lifetimes::LifetimeExt;
use crate::names::Names;

use proc_macro2::TokenStream;
use quote::quote;

pub fn define_datatype(names: &Names, namedtype: &witx::NamedType) -> TokenStream {
    match &namedtype.tref {
        witx::TypeRef::Name(alias_to) => define_alias(names, &namedtype.name, &alias_to),
        witx::TypeRef::Value(v) => match &**v {
            witx::Type::Enum(e) => r#enum::define_enum(names, &namedtype.name, &e),
            witx::Type::Int(i) => int::define_int(names, &namedtype.name, &i),
            witx::Type::Flags(f) => flags::define_flags(names, &namedtype.name, &f),
            witx::Type::Struct(s) => r#struct::define_struct(names, &namedtype.name, &s),
            witx::Type::Union(u) => union::define_union(names, &namedtype.name, &u),
            witx::Type::Handle(h) => handle::define_handle(names, &namedtype.name, &h),
            witx::Type::Builtin(b) => define_builtin(names, &namedtype.name, *b),
            witx::Type::Pointer(p) => define_witx_pointer(
                names,
                &namedtype.name,
                quote!(wiggle_runtime::GuestPtrMut),
                p,
            ),
            witx::Type::ConstPointer(p) => {
                define_witx_pointer(names, &namedtype.name, quote!(wiggle_runtime::GuestPtr), p)
            }
            witx::Type::Array(arr) => define_witx_array(names, &namedtype.name, &arr),
        },
    }
}

fn define_alias(names: &Names, name: &witx::Id, to: &witx::NamedType) -> TokenStream {
    let ident = names.type_(name);
    let rhs = names.type_(&to.name);
    if to.tref.needs_lifetime() {
        quote!(pub type #ident<'a> = #rhs<'a>;)
    } else {
        quote!(pub type #ident = #rhs;)
    }
}

fn define_builtin(names: &Names, name: &witx::Id, builtin: witx::BuiltinType) -> TokenStream {
    let ident = names.type_(name);
    let built = names.builtin_type(builtin, quote!('a));
    if builtin.needs_lifetime() {
        quote!(pub type #ident<'a> = #built;)
    } else {
        quote!(pub type #ident = #built;)
    }
}

fn define_witx_pointer(
    names: &Names,
    name: &witx::Id,
    pointer_type: TokenStream,
    pointee: &witx::TypeRef,
) -> TokenStream {
    let ident = names.type_(name);
    let pointee_type = names.type_ref(pointee, quote!('a));

    quote!(pub type #ident<'a> = #pointer_type<'a, #pointee_type>;)
}

fn define_witx_array(names: &Names, name: &witx::Id, arr_raw: &witx::TypeRef) -> TokenStream {
    let ident = names.type_(name);
    let pointee_type = names.type_ref(arr_raw, quote!('a));
    quote!(pub type #ident<'a> = wiggle_runtime::GuestArray<'a, #pointee_type>;)
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

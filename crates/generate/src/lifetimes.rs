use proc_macro2::TokenStream;
use quote::quote;

pub trait LifetimeExt {
    fn is_transparent(&self) -> bool;
    fn needs_lifetime(&self) -> bool;
}

impl LifetimeExt for witx::TypeRef {
    fn is_transparent(&self) -> bool {
        self.type_().is_transparent()
    }
    fn needs_lifetime(&self) -> bool {
        self.type_().needs_lifetime()
    }
}

impl LifetimeExt for witx::Type {
    fn is_transparent(&self) -> bool {
        match self {
            witx::Type::Builtin(b) => b.is_transparent(),
            witx::Type::Struct(s) => s.is_transparent(),
            witx::Type::Enum { .. }
            | witx::Type::Flags { .. }
            | witx::Type::Int { .. }
            | witx::Type::Handle { .. } => true,
            witx::Type::Union { .. }
            | witx::Type::Pointer { .. }
            | witx::Type::ConstPointer { .. }
            | witx::Type::Array { .. } => false,
        }
    }
    fn needs_lifetime(&self) -> bool {
        match self {
            witx::Type::Builtin(b) => b.needs_lifetime(),
            witx::Type::Struct(s) => s.needs_lifetime(),
            witx::Type::Union(u) => u.needs_lifetime(),
            witx::Type::Enum { .. }
            | witx::Type::Flags { .. }
            | witx::Type::Int { .. }
            | witx::Type::Handle { .. } => false,
            witx::Type::Pointer { .. }
            | witx::Type::ConstPointer { .. }
            | witx::Type::Array { .. } => true,
        }
    }
}

impl LifetimeExt for witx::BuiltinType {
    fn is_transparent(&self) -> bool {
        !self.needs_lifetime()
    }
    fn needs_lifetime(&self) -> bool {
        match self {
            witx::BuiltinType::String => true,
            _ => false,
        }
    }
}

impl LifetimeExt for witx::StructDatatype {
    fn is_transparent(&self) -> bool {
        self.members.iter().all(|m| m.tref.is_transparent())
    }
    fn needs_lifetime(&self) -> bool {
        self.members.iter().any(|m| m.tref.needs_lifetime())
    }
}

impl LifetimeExt for witx::UnionDatatype {
    fn is_transparent(&self) -> bool {
        false
    }
    fn needs_lifetime(&self) -> bool {
        self.variants
            .iter()
            .any(|m| m.tref.as_ref().map(|t| t.needs_lifetime()).unwrap_or(false))
    }
}

pub fn anon_lifetime() -> TokenStream {
    quote!('_)
}

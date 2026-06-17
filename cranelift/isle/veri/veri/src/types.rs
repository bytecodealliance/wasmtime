use std::cmp::Ordering;

use anyhow::Result;
use cranelift_isle::{
    ast::{Ident, ModelType},
    lexer::Pos,
    sema::{self, BuiltinType, Sym, TypeEnv, TypeId, VariantId},
};
use num_bigint::BigUint;

/// Width of a bit vector.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Width {
    Unknown,
    Bits(usize),
}

impl Width {
    pub fn as_bits(&self) -> Option<usize> {
        match self {
            Width::Unknown => None,
            Width::Bits(bits) => Some(*bits),
        }
    }
}

impl PartialOrd for Width {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Width::Unknown, Width::Unknown) => Some(Ordering::Equal),
            (Width::Unknown, Width::Bits(_)) => Some(Ordering::Less),
            (Width::Bits(_), Width::Unknown) => Some(Ordering::Greater),
            (Width::Bits(l), Width::Bits(r)) if l == r => Some(Ordering::Equal),
            (Width::Bits(_), Width::Bits(_)) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Type {
    Unspecified,
    Unknown,
    BitVector(Width),
    Int,
    Bool,
    Unit,
}

impl Type {
    pub fn is_concrete(&self) -> bool {
        match self {
            Type::Unspecified => true,
            Type::Unknown | Type::BitVector(Width::Unknown) => false,
            Type::BitVector(Width::Bits(_)) | Type::Int | Type::Bool | Type::Unit => true,
        }
    }

    pub fn as_bit_vector_width(&self) -> Option<&Width> {
        match self {
            Type::BitVector(w) => Some(w),
            _ => None,
        }
    }

    pub fn is_compatible_with(&self, other: &Type) -> bool {
        matches!(
            (self, other),
            (Type::Unknown, _)
                | (_, Type::Unknown)
                | (Type::Unspecified, Type::Unspecified)
                | (Type::Unit, Type::Unit)
                | (Type::Bool, Type::Bool)
                | (Type::Int, Type::Int)
                | (Type::BitVector(_), Type::BitVector(_))
        )
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Type::Unspecified => write!(f, "\u{2a33}"),
            Type::Unknown => write!(f, "unk"),
            Type::BitVector(Width::Bits(w)) => write!(f, "bv {w}"),
            Type::BitVector(Width::Unknown) => write!(f, "bv _"),
            Type::Int => write!(f, "int"),
            Type::Bool => write!(f, "bool"),
            Type::Unit => write!(f, "unit"),
        }
    }
}

impl PartialOrd for Type {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            // Unspecified is equal to itself, but otherwise incomparible.
            (Type::Unspecified, Type::Unspecified) => Some(Ordering::Equal),
            (Type::Unspecified, _) | (_, Type::Unspecified) => None,

            (Type::Unknown, Type::Unknown) => Some(Ordering::Equal),
            (Type::Unknown, _) => Some(Ordering::Less),
            (_, Type::Unknown) => Some(Ordering::Greater),
            (Type::BitVector(l), Type::BitVector(r)) => l.partial_cmp(r),
            (Type::Int, Type::Int) => Some(Ordering::Equal),
            (Type::Bool, Type::Bool) => Some(Ordering::Equal),
            (Type::Unit, Type::Unit) => Some(Ordering::Equal),
            (_, _) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Compound {
    Primitive(Type),
    Struct(Vec<Field>),
    Enum(Enum),
    // TODO(mbm): intern name identifier
    Named(Ident),
}

#[derive(Debug, Clone)]
pub struct Field {
    // TODO(mbm): intern name identifier
    pub name: Ident,
    pub ty: Compound,
}

impl Field {
    fn from_struct_field(field: &sema::StructField, tyenv: &TypeEnv) -> Self {
        let ty = &tyenv.types[field.ty.index()];
        Self {
            name: Ident(tyenv.syms[field.name.index()].clone(), Pos::default()),
            ty: Compound::named_from_isle(ty, tyenv),
        }
    }

    fn from_tuple_field(index: usize, field: &sema::TupleField, tyenv: &TypeEnv) -> Self {
        let ty = &tyenv.types[field.ty.index()];
        Self {
            name: Ident(index.to_string(), Pos::default()),
            ty: Compound::named_from_isle(ty, tyenv),
        }
    }

    pub fn from_isle_fields(fields: &sema::Fields, tyenv: &TypeEnv) -> Vec<Self> {
        match fields {
            sema::Fields::Unit => Vec::new(),
            sema::Fields::Struct(s) => s
                .fields
                .iter()
                .map(|f| Self::from_struct_field(f, tyenv))
                .collect(),
            sema::Fields::Tuple(t) => t
                .fields
                .iter()
                .enumerate()
                .map(|(i, f)| Self::from_tuple_field(i, f, tyenv))
                .collect(),
        }
    }

    /// Resolve any named types.
    pub fn resolve<F>(&self, lookup: &mut F) -> Result<Self>
    where
        F: FnMut(&Ident) -> Result<Compound>,
    {
        Ok(Field {
            name: self.name.clone(),
            ty: self.ty.resolve(lookup)?,
        })
    }
}

/// Look up the name of a field in an ISLE `Fields` by index. For tuple fields,
/// the synthesized name matches the convention used elsewhere (the index as a
/// decimal string).
pub fn field_name_by_index(fields: &sema::Fields, index: usize, tyenv: &TypeEnv) -> String {
    match fields {
        sema::Fields::Unit => panic!("unit fields cannot be indexed"),
        sema::Fields::Struct(s) => tyenv.syms[s.fields[index].name.index()].clone(),
        sema::Fields::Tuple(_) => index.to_string(),
    }
}

/// Look up the type of a field in an ISLE `Fields` by index.
pub fn field_type_by_index(fields: &sema::Fields, index: usize) -> TypeId {
    match fields {
        sema::Fields::Unit => panic!("unit fields cannot be indexed"),
        sema::Fields::Struct(s) => s.fields[index].ty,
        sema::Fields::Tuple(t) => t.fields[index].ty,
    }
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: Ident,
    pub id: VariantId,
    pub fields: Vec<Field>,
}

impl Variant {
    fn from_isle(variant: &sema::Variant, tyenv: &TypeEnv) -> Self {
        Self {
            name: Ident(tyenv.syms[variant.name.index()].clone(), variant.pos),
            id: variant.id,
            fields: Field::from_isle_fields(&variant.fields, tyenv),
        }
    }

    pub fn is_unit(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn ty(&self) -> Compound {
        Compound::Struct(self.fields.clone())
    }

    /// Resolve any named types.
    pub fn resolve<F>(&self, lookup: &mut F) -> Result<Self>
    where
        F: FnMut(&Ident) -> Result<Compound>,
    {
        Ok(Variant {
            name: self.name.clone(),
            id: self.id,
            fields: self
                .fields
                .iter()
                .map(|f| f.resolve(lookup))
                .collect::<Result<_>>()?,
        })
    }
}

impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_unit() {
            write!(f, "{name}", name = self.name.0)
        } else {
            write!(f, "{name} {ty}", name = self.name.0, ty = self.ty())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: Ident,
    pub id: TypeId,
    pub variants: Vec<Variant>,
}

impl Enum {
    pub fn from_isle(
        name: Sym,
        id: TypeId,
        variants: &[sema::Variant],
        pos: Pos,
        tyenv: &TypeEnv,
    ) -> Self {
        Self {
            name: Ident(tyenv.syms[name.index()].clone(), pos),
            id,
            variants: variants
                .iter()
                .map(|v| Variant::from_isle(v, tyenv))
                .collect(),
        }
    }

    /// Resolve any named types.
    pub fn resolve<F>(&self, lookup: &mut F) -> Result<Self>
    where
        F: FnMut(&Ident) -> Result<Compound>,
    {
        Ok(Self {
            name: self.name.clone(),
            id: self.id,
            variants: self
                .variants
                .iter()
                .map(|v| v.resolve(lookup))
                .collect::<Result<_>>()?,
        })
    }
}

impl Compound {
    pub fn from_ast(model: &ModelType) -> Self {
        match model {
            ModelType::Unspecified => Self::Primitive(Type::Unspecified),
            ModelType::Auto => Self::Primitive(Type::Unknown),
            ModelType::Int => Self::Primitive(Type::Int),
            ModelType::Bool => Self::Primitive(Type::Bool),
            ModelType::Unit => Self::Primitive(Type::Unit),
            ModelType::BitVec(None) => Self::Primitive(Type::BitVector(Width::Unknown)),
            ModelType::BitVec(Some(bits)) => Self::Primitive(Type::BitVector(Width::Bits(*bits))),
            ModelType::Struct(fields) => Self::Struct(
                fields
                    .iter()
                    .map(|m| Field {
                        name: m.name.clone(),
                        ty: Self::from_ast(&m.ty),
                    })
                    .collect(),
            ),
            ModelType::Named(name) => Self::Named(name.clone()),
        }
    }

    /// Derive a type corresponding to the given ISLE type, if possible. For
    /// ISLE internal enumerations, this will build the corresponding VeriISLE
    /// enum representation.
    pub fn from_isle(ty: &sema::Type, tyenv: &TypeEnv) -> Option<Self> {
        match ty {
            sema::Type::Enum {
                name,
                id,
                variants,
                pos,
                ..
            } if !variants.is_empty() => Some(Self::Enum(Enum::from_isle(
                *name, *id, variants, *pos, tyenv,
            ))),
            sema::Type::Struct { fields, .. } => {
                Some(Self::Struct(Field::from_isle_fields(fields, tyenv)))
            }
            _ => None,
        }
    }

    /// Build a named reference to the given ISLE type.
    pub fn named_from_isle(ty: &sema::Type, tyenv: &TypeEnv) -> Self {
        match ty {
            sema::Type::Builtin(BuiltinType::Bool) => Self::Primitive(Type::Bool),
            sema::Type::Builtin(b) => Self::Primitive(Type::BitVector(Width::Bits(b.to_usize()))),
            _ => Self::Named(Ident(
                ty.name(tyenv).to_string(),
                ty.pos().expect("expected position"),
            )),
        }
    }

    pub fn as_primitive(&self) -> Option<&Type> {
        match self {
            Compound::Primitive(ty) => Some(ty),
            _ => None,
        }
    }

    pub fn as_enum(&self) -> Option<&Enum> {
        match self {
            Compound::Enum(e) => Some(e),
            _ => None,
        }
    }

    /// Resolve any named types.
    pub fn resolve<F>(&self, lookup: &mut F) -> Result<Self>
    where
        F: FnMut(&Ident) -> Result<Compound>,
    {
        match self {
            Compound::Primitive(_) => Ok(self.clone()),
            Compound::Struct(fields) => Ok(Compound::Struct(
                fields
                    .iter()
                    .map(|f| f.resolve(lookup))
                    .collect::<Result<_>>()?,
            )),
            Compound::Enum(e) => Ok(Compound::Enum(e.resolve(lookup)?)),
            Compound::Named(name) => {
                // TODO(mbm): named type model cycle detection
                let ty = lookup(name)?;
                ty.resolve(lookup)
            }
        }
    }
}

impl std::fmt::Display for Compound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Compound::Primitive(ty) => ty.fmt(f),
            Compound::Struct(fields) => write!(
                f,
                "{{{fields}}}",
                fields = fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name.0, f.ty))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Compound::Enum(e) => {
                write!(f, "enum({name})", name = e.name.0,)
            }
            Compound::Named(name) => write!(f, "{}", name.0),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Const {
    Bool(bool),
    Int(i128),
    BitVector(usize, BigUint),
    Unspecified,
}

impl Const {
    pub fn ty(&self) -> Type {
        match self {
            Const::Bool(_) => Type::Bool,
            Const::Int(_) => Type::Int,
            Const::BitVector(w, _) => Type::BitVector(Width::Bits(*w)),
            Const::Unspecified => Type::Unspecified,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Const::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i128> {
        match self {
            Const::Int(v) => Some(*v),
            _ => None,
        }
    }
}

impl std::fmt::Display for Const {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Const::Bool(b) => write!(f, "{b}"),
            Const::Int(v) => write!(f, "{v}"),
            Const::BitVector(bits, v) => {
                if bits % 4 == 0 {
                    write!(f, "#x{v:0>nibbles$x}", nibbles = bits / 4)
                } else {
                    write!(f, "#b{v:0>bits$b}")
                }
            }
            Const::Unspecified => write!(f, "\u{2a33}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assert_partial_order_properties;

    #[test]
    fn test_width_partial_order_less_than() {
        assert!(Width::Unknown < Width::Bits(64));
    }

    #[test]
    fn test_width_partial_order_properties() {
        assert_partial_order_properties(&[Width::Unknown, Width::Bits(32), Width::Bits(64)]);
    }

    #[test]
    fn test_type_partial_order_less_than() {
        assert!(Type::Unknown < Type::BitVector(Width::Unknown));
        assert!(Type::BitVector(Width::Unknown) < Type::BitVector(Width::Bits(64)));
        assert!(Type::Unknown < Type::Int);
        assert!(Type::Unknown < Type::Bool);
    }

    #[test]
    fn test_type_partial_order_properties() {
        assert_partial_order_properties(&[
            Type::Unspecified,
            Type::Unknown,
            Type::BitVector(Width::Unknown),
            Type::BitVector(Width::Bits(32)),
            Type::BitVector(Width::Bits(64)),
            Type::Int,
            Type::Bool,
            Type::Unit,
        ]);
    }
}

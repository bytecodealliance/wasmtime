//! This module defines the `Type` type, representing the dynamic form of a component interface type.

use crate::component::values::{self, Val};
use anyhow::{anyhow, Result};
use std::fmt;
use std::mem;
use std::ops::Deref;
use std::sync::Arc;
use wasmtime_environ::component::{
    CanonicalAbiInfo, ComponentTypes, InterfaceType, TypeEnumIndex, TypeExpectedIndex,
    TypeFlagsIndex, TypeInterfaceIndex, TypeOptionIndex, TypeRecordIndex, TypeTupleIndex,
    TypeUnionIndex, TypeVariantIndex, VariantInfo,
};

#[derive(Clone)]
struct Handle<T> {
    index: T,
    types: Arc<ComponentTypes>,
}

impl<T: fmt::Debug> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle")
            .field("index", &self.index)
            .finish()
    }
}

impl<T: PartialEq> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        // FIXME: This is an overly-restrictive definition of equality in that it doesn't consider types to be
        // equal unless they refer to the same declaration in the same component.  It's a good shortcut for the
        // common case, but we should also do a recursive structural equality test if the shortcut test fails.
        self.index == other.index && Arc::ptr_eq(&self.types, &other.types)
    }
}

impl<T: Eq> Eq for Handle<T> {}

/// A `list` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct List(Handle<TypeInterfaceIndex>);

impl List {
    /// Instantiate this type with the specified `values`.
    pub fn new_val(&self, values: Box<[Val]>) -> Result<Val> {
        Ok(Val::List(values::List::new(self, values)?))
    }

    /// Retreive the element type of this `list`.
    pub fn ty(&self) -> Type {
        Type::from(&self.0.types[self.0.index], &self.0.types)
    }
}

/// A field declaration belonging to a `record`
pub struct Field<'a> {
    /// The name of the field
    pub name: &'a str,
    /// The type of the field
    pub ty: Type,
}

/// A `record` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Record(Handle<TypeRecordIndex>);

impl Record {
    /// Instantiate this type with the specified `values`.
    pub fn new_val<'a>(&self, values: impl IntoIterator<Item = (&'a str, Val)>) -> Result<Val> {
        Ok(Val::Record(values::Record::new(self, values)?))
    }

    /// Retrieve the fields of this `record` in declaration order.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = Field<'_>> {
        self.0.types[self.0.index].fields.iter().map(|field| Field {
            name: &field.name,
            ty: Type::from(&field.ty, &self.0.types),
        })
    }
}

/// A `tuple` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tuple(Handle<TypeTupleIndex>);

impl Tuple {
    /// Instantiate this type ith the specified `values`.
    pub fn new_val(&self, values: Box<[Val]>) -> Result<Val> {
        Ok(Val::Tuple(values::Tuple::new(self, values)?))
    }

    /// Retrieve the types of the fields of this `tuple` in declaration order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.0.types[self.0.index]
            .types
            .iter()
            .map(|ty| Type::from(ty, &self.0.types))
    }
}

/// A case declaration belonging to a `variant`
pub struct Case<'a> {
    /// The name of the case
    pub name: &'a str,
    /// The type of the case
    pub ty: Type,
}

/// A `variant` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Variant(Handle<TypeVariantIndex>);

impl Variant {
    /// Instantiate this type with the specified case `name` and `value`.
    pub fn new_val(&self, name: &str, value: Val) -> Result<Val> {
        Ok(Val::Variant(values::Variant::new(self, name, value)?))
    }

    /// Retrieve the cases of this `variant` in declaration order.
    pub fn cases(&self) -> impl ExactSizeIterator<Item = Case> {
        self.0.types[self.0.index].cases.iter().map(|case| Case {
            name: &case.name,
            ty: Type::from(&case.ty, &self.0.types),
        })
    }

    pub(crate) fn variant_info(&self) -> &VariantInfo {
        &self.0.types[self.0.index].info
    }
}

/// An `enum` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Enum(Handle<TypeEnumIndex>);

impl Enum {
    /// Instantiate this type with the specified case `name`.
    pub fn new_val(&self, name: &str) -> Result<Val> {
        Ok(Val::Enum(values::Enum::new(self, name)?))
    }

    /// Retrieve the names of the cases of this `enum` in declaration order.
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.types[self.0.index]
            .names
            .iter()
            .map(|name| name.deref())
    }

    pub(crate) fn variant_info(&self) -> &VariantInfo {
        &self.0.types[self.0.index].info
    }
}

/// A `union` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Union(Handle<TypeUnionIndex>);

impl Union {
    /// Instantiate this type with the specified `discriminant` and `value`.
    pub fn new_val(&self, discriminant: u32, value: Val) -> Result<Val> {
        Ok(Val::Union(values::Union::new(self, discriminant, value)?))
    }

    /// Retrieve the types of the cases of this `union` in declaration order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.0.types[self.0.index]
            .types
            .iter()
            .map(|ty| Type::from(ty, &self.0.types))
    }

    pub(crate) fn variant_info(&self) -> &VariantInfo {
        &self.0.types[self.0.index].info
    }
}

/// An `option` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Option(Handle<TypeOptionIndex>);

impl Option {
    /// Instantiate this type with the specified `value`.
    pub fn new_val(&self, value: std::option::Option<Val>) -> Result<Val> {
        Ok(Val::Option(values::Option::new(self, value)?))
    }

    /// Retrieve the type parameter for this `option`.
    pub fn ty(&self) -> Type {
        Type::from(&self.0.types[self.0.index].ty, &self.0.types)
    }

    pub(crate) fn variant_info(&self) -> &VariantInfo {
        &self.0.types[self.0.index].info
    }
}

/// An `expected` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Expected(Handle<TypeExpectedIndex>);

impl Expected {
    /// Instantiate this type with the specified `value`.
    pub fn new_val(&self, value: Result<Val, Val>) -> Result<Val> {
        Ok(Val::Expected(values::Expected::new(self, value)?))
    }

    /// Retrieve the `ok` type parameter for this `option`.
    pub fn ok(&self) -> Type {
        Type::from(&self.0.types[self.0.index].ok, &self.0.types)
    }

    /// Retrieve the `err` type parameter for this `option`.
    pub fn err(&self) -> Type {
        Type::from(&self.0.types[self.0.index].err, &self.0.types)
    }

    pub(crate) fn variant_info(&self) -> &VariantInfo {
        &self.0.types[self.0.index].info
    }
}

/// A `flags` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Flags(Handle<TypeFlagsIndex>);

impl Flags {
    /// Instantiate this type with the specified flag `names`.
    pub fn new_val(&self, names: &[&str]) -> Result<Val> {
        Ok(Val::Flags(values::Flags::new(self, names)?))
    }

    /// Retrieve the names of the flags of this `flags` type in declaration order.
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.types[self.0.index]
            .names
            .iter()
            .map(|name| name.deref())
    }
}

/// Represents a component model interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    /// Unit
    Unit,
    /// Boolean
    Bool,
    /// Signed 8-bit integer
    S8,
    /// Unsigned 8-bit integer
    U8,
    /// Signed 16-bit integer
    S16,
    /// Unsigned 16-bit integer
    U16,
    /// Signed 32-bit integer
    S32,
    /// Unsigned 32-bit integer
    U32,
    /// Signed 64-bit integer
    S64,
    /// Unsigned 64-bit integer
    U64,
    /// 64-bit floating point value
    Float32,
    /// 64-bit floating point value
    Float64,
    /// 32-bit character
    Char,
    /// Character string
    String,
    /// List of values
    List(List),
    /// Record
    Record(Record),
    /// Tuple
    Tuple(Tuple),
    /// Variant
    Variant(Variant),
    /// Enum
    Enum(Enum),
    /// Union
    Union(Union),
    /// Option
    Option(Option),
    /// Expected
    Expected(Expected),
    /// Bit flags
    Flags(Flags),
}

impl Type {
    /// Retrieve the inner [`List`] of a [`Type::List`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::List`].
    pub fn unwrap_list(&self) -> &List {
        if let Type::List(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a list", self.desc())
        }
    }

    /// Retrieve the inner [`Record`] of a [`Type::Record`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Record`].
    pub fn unwrap_record(&self) -> &Record {
        if let Type::Record(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a record", self.desc())
        }
    }

    /// Retrieve the inner [`Tuple`] of a [`Type::Tuple`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Tuple`].
    pub fn unwrap_tuple(&self) -> &Tuple {
        if let Type::Tuple(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a tuple", self.desc())
        }
    }

    /// Retrieve the inner [`Variant`] of a [`Type::Variant`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Variant`].
    pub fn unwrap_variant(&self) -> &Variant {
        if let Type::Variant(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a variant", self.desc())
        }
    }

    /// Retrieve the inner [`Enum`] of a [`Type::Enum`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Enum`].
    pub fn unwrap_enum(&self) -> &Enum {
        if let Type::Enum(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a enum", self.desc())
        }
    }

    /// Retrieve the inner [`Union`] of a [`Type::Union`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Union`].
    pub fn unwrap_union(&self) -> &Union {
        if let Type::Union(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a union", self.desc())
        }
    }

    /// Retrieve the inner [`Option`] of a [`Type::Option`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Option`].
    pub fn unwrap_option(&self) -> &Option {
        if let Type::Option(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a option", self.desc())
        }
    }

    /// Retrieve the inner [`Expected`] of a [`Type::Expected`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Expected`].
    pub fn unwrap_expected(&self) -> &Expected {
        if let Type::Expected(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a expected", self.desc())
        }
    }

    /// Retrieve the inner [`Flags`] of a [`Type::Flags`].
    ///
    /// # Panics
    ///
    /// This will panic if `self` is not a [`Type::Flags`].
    pub fn unwrap_flags(&self) -> &Flags {
        if let Type::Flags(handle) = self {
            &handle
        } else {
            panic!("attempted to unwrap a {} as a flags", self.desc())
        }
    }

    pub(crate) fn check(&self, value: &Val) -> Result<()> {
        let other = &value.ty();
        if self == other {
            Ok(())
        } else if mem::discriminant(self) != mem::discriminant(other) {
            Err(anyhow!(
                "type mismatch: expected {}, got {}",
                self.desc(),
                other.desc()
            ))
        } else {
            Err(anyhow!(
                "type mismatch for {}, possibly due to mixing distinct composite types",
                self.desc()
            ))
        }
    }

    /// Convert the specified `InterfaceType` to a `Type`.
    pub(crate) fn from(ty: &InterfaceType, types: &Arc<ComponentTypes>) -> Self {
        match ty {
            InterfaceType::Unit => Type::Unit,
            InterfaceType::Bool => Type::Bool,
            InterfaceType::S8 => Type::S8,
            InterfaceType::U8 => Type::U8,
            InterfaceType::S16 => Type::S16,
            InterfaceType::U16 => Type::U16,
            InterfaceType::S32 => Type::S32,
            InterfaceType::U32 => Type::U32,
            InterfaceType::S64 => Type::S64,
            InterfaceType::U64 => Type::U64,
            InterfaceType::Float32 => Type::Float32,
            InterfaceType::Float64 => Type::Float64,
            InterfaceType::Char => Type::Char,
            InterfaceType::String => Type::String,
            InterfaceType::List(index) => Type::List(List(Handle {
                index: *index,
                types: types.clone(),
            })),
            InterfaceType::Record(index) => Type::Record(Record(Handle {
                index: *index,
                types: types.clone(),
            })),
            InterfaceType::Tuple(index) => Type::Tuple(Tuple(Handle {
                index: *index,
                types: types.clone(),
            })),
            InterfaceType::Variant(index) => Type::Variant(Variant(Handle {
                index: *index,
                types: types.clone(),
            })),
            InterfaceType::Enum(index) => Type::Enum(Enum(Handle {
                index: *index,
                types: types.clone(),
            })),
            InterfaceType::Union(index) => Type::Union(Union(Handle {
                index: *index,
                types: types.clone(),
            })),
            InterfaceType::Option(index) => Type::Option(Option(Handle {
                index: *index,
                types: types.clone(),
            })),
            InterfaceType::Expected(index) => Type::Expected(Expected(Handle {
                index: *index,
                types: types.clone(),
            })),
            InterfaceType::Flags(index) => Type::Flags(Flags(Handle {
                index: *index,
                types: types.clone(),
            })),
        }
    }

    /// Return the number of stack slots needed to store values of this type in lowered form.
    pub(crate) fn flatten_count(&self) -> usize {
        match self {
            Type::Unit => 0,

            Type::Bool
            | Type::S8
            | Type::U8
            | Type::S16
            | Type::U16
            | Type::S32
            | Type::U32
            | Type::S64
            | Type::U64
            | Type::Float32
            | Type::Float64
            | Type::Char
            | Type::Enum(_) => 1,

            Type::String | Type::List(_) => 2,

            Type::Record(handle) => handle.fields().map(|field| field.ty.flatten_count()).sum(),

            Type::Tuple(handle) => handle.types().map(|ty| ty.flatten_count()).sum(),

            Type::Variant(handle) => {
                1 + handle
                    .cases()
                    .map(|case| case.ty.flatten_count())
                    .max()
                    .unwrap_or(0)
            }

            Type::Union(handle) => {
                1 + handle
                    .types()
                    .map(|ty| ty.flatten_count())
                    .max()
                    .unwrap_or(0)
            }

            Type::Option(handle) => 1 + handle.ty().flatten_count(),

            Type::Expected(handle) => {
                1 + handle
                    .ok()
                    .flatten_count()
                    .max(handle.err().flatten_count())
            }

            Type::Flags(handle) => values::u32_count_for_flag_count(handle.names().len()),
        }
    }

    fn desc(&self) -> &'static str {
        match self {
            Type::Unit => "unit",
            Type::Bool => "bool",
            Type::S8 => "s8",
            Type::U8 => "u8",
            Type::S16 => "s16",
            Type::U16 => "u16",
            Type::S32 => "s32",
            Type::U32 => "u32",
            Type::S64 => "s64",
            Type::U64 => "u64",
            Type::Float32 => "float32",
            Type::Float64 => "float64",
            Type::Char => "char",
            Type::String => "string",
            Type::List(_) => "list",
            Type::Record(_) => "record",
            Type::Tuple(_) => "tuple",
            Type::Variant(_) => "variant",
            Type::Enum(_) => "enum",
            Type::Union(_) => "union",
            Type::Option(_) => "option",
            Type::Expected(_) => "expected",
            Type::Flags(_) => "flags",
        }
    }

    /// Calculate the size and alignment requirements for the specified type.
    pub(crate) fn canonical_abi(&self) -> &CanonicalAbiInfo {
        match self {
            Type::Unit => &CanonicalAbiInfo::ZERO,
            Type::Bool | Type::S8 | Type::U8 => &CanonicalAbiInfo::SCALAR1,
            Type::S16 | Type::U16 => &CanonicalAbiInfo::SCALAR2,
            Type::S32 | Type::U32 | Type::Char | Type::Float32 => &CanonicalAbiInfo::SCALAR4,
            Type::S64 | Type::U64 | Type::Float64 => &CanonicalAbiInfo::SCALAR8,
            Type::String | Type::List(_) => &CanonicalAbiInfo::POINTER_PAIR,
            Type::Record(handle) => &handle.0.types[handle.0.index].abi,
            Type::Tuple(handle) => &handle.0.types[handle.0.index].abi,
            Type::Variant(handle) => &handle.0.types[handle.0.index].abi,
            Type::Enum(handle) => &handle.0.types[handle.0.index].abi,
            Type::Union(handle) => &handle.0.types[handle.0.index].abi,
            Type::Option(handle) => &handle.0.types[handle.0.index].abi,
            Type::Expected(handle) => &handle.0.types[handle.0.index].abi,
            Type::Flags(handle) => &handle.0.types[handle.0.index].abi,
        }
    }
}

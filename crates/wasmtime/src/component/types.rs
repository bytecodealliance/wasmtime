//! This module defines the `Type` type, representing the dynamic form of a component interface type.

use crate::component::func::{self, Lift, Memory, Options};
use crate::component::values::{self, Val};
use crate::store::StoreOpaque;
use crate::ValRaw;
use anyhow::{anyhow, bail, Context, Error, Result};
use std::collections::HashMap;
use std::fmt;
use std::iter;
use std::mem;
use std::ops::Deref;
use std::sync::Arc;
use wasmtime_component_util::{DiscriminantSize, FlagsSize};
use wasmtime_environ::component::{
    ComponentTypes, InterfaceType, TypeEnumIndex, TypeExpectedIndex, TypeFlagsIndex,
    TypeInterfaceIndex, TypeRecordIndex, TypeTupleIndex, TypeUnionIndex, TypeVariantIndex,
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
        let ty = self.ty();
        for (index, value) in values.iter().enumerate() {
            ty.check(value)
                .with_context(|| format!("type mismatch for element {index} of list"))?;
        }

        Ok(Val::List(values::List {
            ty: self.clone(),
            values,
        }))
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
    pub fn new_val<
        'a,
        I: ExactSizeIterator<Item = (&'a str, Val)>,
        II: IntoIterator<Item = (&'a str, Val), IntoIter = I>,
    >(
        &self,
        values: II,
    ) -> Result<Val> {
        let values = values.into_iter();

        if values.len() != self.fields().len() {
            bail!(
                "expected {} value(s); got {}",
                self.fields().len(),
                values.len()
            );
        }

        Ok(Val::Record(values::Record {
            ty: self.clone(),
            values: values
                .zip(self.fields())
                .map(|((name, value), field)| {
                    if name == field.name {
                        field
                            .ty
                            .check(&value)
                            .with_context(|| format!("type mismatch for field {name} of record"))?;

                        Ok(value)
                    } else {
                        Err(anyhow!(
                            "field name mismatch: expected {}; got {name}",
                            field.name
                        ))
                    }
                })
                .collect::<Result<_>>()?,
        }))
    }

    /// Retrieve the fields of this `record` in declaration order.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = Field> {
        self.0.types[self.0.index].fields.iter().map(|field| Field {
            name: &field.name,
            ty: Type::from(&field.ty, &self.0.types),
        })
    }
}

/// A `tuple` interface type
#[derive(Clone, Debug)]
pub struct Tuple(Handle<TypeTupleIndex>);

impl Tuple {
    /// Instantiate this type ith the specified `values`.
    pub fn new_val(&self, values: Box<[Val]>) -> Result<Val> {
        if values.len() != self.types().len() {
            bail!(
                "expected {} value(s); got {}",
                self.types().len(),
                values.len()
            );
        }

        for (index, (value, ty)) in values.iter().zip(self.types()).enumerate() {
            ty.check(value)
                .with_context(|| format!("type mismatch for field {index} of tuple"))?;
        }

        Ok(Val::Tuple(values::Tuple {
            ty: self.clone(),
            values,
        }))
    }

    /// Retrieve the types of the fields of this `tuple` in declaration order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.0.types[self.0.index]
            .types
            .iter()
            .map(|ty| Type::from(ty, &self.0.types))
    }
}

impl PartialEq for Tuple {
    fn eq(&self, other: &Self) -> bool {
        if self.0 == other.0 {
            return true;
        }

        let self_types = self.types();
        let other_types = other.types();
        if self_types.len() == other_types.len() {
            self_types
                .zip(other_types)
                .all(|(self_type, other_type)| self_type == other_type)
        } else {
            false
        }
    }
}

impl Eq for Tuple {}

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
        let (discriminant, ty) = self
            .cases()
            .enumerate()
            .find_map(|(index, case)| {
                if case.name == name {
                    Some((index, case.ty))
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow!("unknown variant case: {name}"))?;

        ty.check(&value)
            .with_context(|| format!("type mismatch for case {name} of variant"))?;

        Ok(Val::Variant(values::Variant {
            ty: self.clone(),
            discriminant: u32::try_from(discriminant)?,
            value: Box::new(value),
        }))
    }

    /// Retrieve the cases of this `variant` in declaration order.
    pub fn cases(&self) -> impl ExactSizeIterator<Item = Case> {
        self.0.types[self.0.index].cases.iter().map(|case| Case {
            name: &case.name,
            ty: Type::from(&case.ty, &self.0.types),
        })
    }
}

/// An `enum` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Enum(Handle<TypeEnumIndex>);

impl Enum {
    /// Instantiate this type with the specified case `name`.
    pub fn new_val(&self, name: &str) -> Result<Val> {
        let discriminant = u32::try_from(
            self.names()
                .position(|n| n == name)
                .ok_or_else(|| anyhow!("unknown enum case: {name}"))?,
        )?;

        Ok(Val::Enum(values::Enum {
            ty: self.clone(),
            discriminant,
        }))
    }

    /// Retrieve the names of the cases of this `enum` in declaration order.
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.types[self.0.index]
            .names
            .iter()
            .map(|name| name.deref())
    }
}

/// A `union` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Union(Handle<TypeUnionIndex>);

impl Union {
    /// Instantiate this type with the specified `discriminant` and `value`.
    pub fn new_val(&self, discriminant: u32, value: Val) -> Result<Val> {
        if let Some(ty) = self.types().nth(usize::try_from(discriminant)?) {
            ty.check(&value)
                .with_context(|| format!("type mismatch for case {discriminant} of union"))?;

            Ok(Val::Union(values::Union {
                ty: self.clone(),
                discriminant,
                value: Box::new(value),
            }))
        } else {
            Err(anyhow!(
                "discriminant {discriminant} out of range: [0,{})",
                self.types().len()
            ))
        }
    }

    /// Retrieve the types of the cases of this `union` in declaration order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.0.types[self.0.index]
            .types
            .iter()
            .map(|ty| Type::from(ty, &self.0.types))
    }
}

/// An `option` interface type
#[derive(Clone, Debug)]
pub struct Option(Handle<TypeInterfaceIndex>);

impl Option {
    /// Instantiate this type with the specified `value`.
    pub fn new_val(&self, value: std::option::Option<Val>) -> Result<Val> {
        let value = value
            .map(|value| {
                self.ty()
                    .check(&value)
                    .context("type mismatch for option")?;

                Ok::<_, Error>(value)
            })
            .transpose()?;

        Ok(Val::Option(values::Option {
            ty: self.clone(),
            discriminant: if value.is_none() { 0 } else { 1 },
            value: Box::new(value.unwrap_or(Val::Unit)),
        }))
    }

    /// Retrieve the type parameter for this `option`.
    pub fn ty(&self) -> Type {
        Type::from(&self.0.types[self.0.index], &self.0.types)
    }
}

impl PartialEq for Option {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || self.ty() == other.ty()
    }
}

impl Eq for Option {}

/// An `expected` interface type
#[derive(Clone, Debug)]
pub struct Expected(Handle<TypeExpectedIndex>);

impl Expected {
    /// Instantiate this type with the specified `value`.
    pub fn new_val(&self, value: Result<Val, Val>) -> Result<Val> {
        Ok(Val::Expected(values::Expected {
            ty: self.clone(),
            discriminant: if value.is_ok() { 0 } else { 1 },
            value: Box::new(match value {
                Ok(value) => {
                    self.ok()
                        .check(&value)
                        .context("type mismatch for ok case of expected")?;
                    value
                }
                Err(value) => {
                    self.err()
                        .check(&value)
                        .context("type mismatch for err case of expected")?;
                    value
                }
            }),
        }))
    }

    /// Retrieve the `ok` type parameter for this `option`.
    pub fn ok(&self) -> Type {
        Type::from(&self.0.types[self.0.index].ok, &self.0.types)
    }

    /// Retrieve the `err` type parameter for this `option`.
    pub fn err(&self) -> Type {
        Type::from(&self.0.types[self.0.index].err, &self.0.types)
    }
}

impl PartialEq for Expected {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || (self.ok() == other.ok() && self.err() == other.err())
    }
}

impl Eq for Expected {}

/// A `flags` interface type
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Flags(Handle<TypeFlagsIndex>);

impl Flags {
    /// Instantiate this type with the specified flag `names`.
    pub fn new_val(&self, names: &[&str]) -> Result<Val> {
        let map = self
            .names()
            .enumerate()
            .map(|(index, name)| (name, index))
            .collect::<HashMap<_, _>>();

        let mut values = vec![0_u32; values::u32_count_for_flag_count(self.names().len())];

        for name in names {
            let index = map
                .get(name)
                .ok_or_else(|| anyhow!("unknown flag: {name}"))?;
            values[index / 32] |= 1 << (index % 32);
        }

        Ok(Val::Flags(values::Flags {
            ty: self.clone(),
            count: u32::try_from(map.len())?,
            value: values.into(),
        }))
    }

    /// Retrieve the names of the flags of this `flags` type in declaration order.
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.types[self.0.index]
            .names
            .iter()
            .map(|name| name.deref())
    }
}

/// Represents the size and alignment requirements of the heap-serialized form of a type
pub(crate) struct SizeAndAlignment {
    pub(crate) size: usize,
    pub(crate) alignment: u32,
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

    /// Retrieve any types nested within this type.
    ///
    /// For example, this will return the field types in order of declaration for a record type.  It will return
    /// the element type for a list type and nothing for non-composite types like bool, u8, and string.
    pub fn nested(&self) -> Box<[Type]> {
        match self {
            Type::Unit
            | Type::Bool
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
            | Type::String
            | Type::Enum(_)
            | Type::Flags(_) => Box::new([]),

            Type::List(handle) => Box::new([handle.ty()]),

            Type::Record(handle) => handle.fields().map(|field| field.ty).collect(),

            Type::Tuple(handle) => handle.types().collect(),

            Type::Variant(handle) => handle.cases().map(|case| case.ty).collect(),

            Type::Union(handle) => handle.types().collect(),

            Type::Option(handle) => Box::new([handle.ty()]),

            Type::Expected(handle) => Box::new([handle.ok(), handle.err()]),
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

    /// Deserialize a value of this type from core Wasm stack values.
    pub(crate) fn lift<'a>(
        &self,
        store: &StoreOpaque,
        options: &Options,
        src: &mut std::slice::Iter<'_, ValRaw>,
    ) -> Result<Val> {
        Ok(match self {
            Type::Unit => Val::Unit,
            Type::Bool => Val::Bool(bool::lift(store, options, next(src))?),
            Type::S8 => Val::S8(i8::lift(store, options, next(src))?),
            Type::U8 => Val::U8(u8::lift(store, options, next(src))?),
            Type::S16 => Val::S16(i16::lift(store, options, next(src))?),
            Type::U16 => Val::U16(u16::lift(store, options, next(src))?),
            Type::S32 => Val::S32(i32::lift(store, options, next(src))?),
            Type::U32 => Val::U32(u32::lift(store, options, next(src))?),
            Type::S64 => Val::S64(i64::lift(store, options, next(src))?),
            Type::U64 => Val::U64(u64::lift(store, options, next(src))?),
            Type::Float32 => Val::Float32(u32::lift(store, options, next(src))?),
            Type::Float64 => Val::Float64(u64::lift(store, options, next(src))?),
            Type::Char => Val::Char(char::lift(store, options, next(src))?),
            Type::String => {
                Val::String(Box::<str>::lift(store, options, &[*next(src), *next(src)])?)
            }
            Type::List(handle) => {
                // FIXME: needs memory64 treatment
                let ptr = u32::lift(store, options, next(src))? as usize;
                let len = u32::lift(store, options, next(src))? as usize;
                load_list(handle, &Memory::new(store, options), ptr, len)?
            }
            Type::Record(handle) => Val::Record(values::Record {
                ty: handle.clone(),
                values: handle
                    .fields()
                    .map(|field| field.ty.lift(store, options, src))
                    .collect::<Result<_>>()?,
            }),
            Type::Tuple(handle) => Val::Tuple(values::Tuple {
                ty: handle.clone(),
                values: handle
                    .types()
                    .map(|ty| ty.lift(store, options, src))
                    .collect::<Result<_>>()?,
            }),
            Type::Variant(handle) => {
                let (discriminant, value) = lift_variant(
                    self.flatten_count(),
                    handle.cases().map(|case| case.ty),
                    store,
                    options,
                    src,
                )?;

                Val::Variant(values::Variant {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Enum(handle) => {
                let (discriminant, _) = lift_variant(
                    self.flatten_count(),
                    handle.names().map(|_| Type::Unit),
                    store,
                    options,
                    src,
                )?;

                Val::Enum(values::Enum {
                    ty: handle.clone(),
                    discriminant,
                })
            }
            Type::Union(handle) => {
                let (discriminant, value) =
                    lift_variant(self.flatten_count(), handle.types(), store, options, src)?;

                Val::Union(values::Union {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Option(handle) => {
                let (discriminant, value) = lift_variant(
                    self.flatten_count(),
                    [Type::Unit, handle.ty()].into_iter(),
                    store,
                    options,
                    src,
                )?;

                Val::Option(values::Option {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Expected(handle) => {
                let (discriminant, value) = lift_variant(
                    self.flatten_count(),
                    [handle.ok(), handle.err()].into_iter(),
                    store,
                    options,
                    src,
                )?;

                Val::Expected(values::Expected {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Flags(handle) => {
                let count = u32::try_from(handle.names().len()).unwrap();
                assert!(count <= 32);
                let value = iter::once(u32::lift(store, options, next(src))?).collect();

                Val::Flags(values::Flags {
                    ty: handle.clone(),
                    count,
                    value,
                })
            }
        })
    }

    /// Deserialize a value of this type from the heap.
    pub(crate) fn load(&self, mem: &Memory, bytes: &[u8]) -> Result<Val> {
        Ok(match self {
            Type::Unit => Val::Unit,
            Type::Bool => Val::Bool(bool::load(mem, bytes)?),
            Type::S8 => Val::S8(i8::load(mem, bytes)?),
            Type::U8 => Val::U8(u8::load(mem, bytes)?),
            Type::S16 => Val::S16(i16::load(mem, bytes)?),
            Type::U16 => Val::U16(u16::load(mem, bytes)?),
            Type::S32 => Val::S32(i32::load(mem, bytes)?),
            Type::U32 => Val::U32(u32::load(mem, bytes)?),
            Type::S64 => Val::S64(i64::load(mem, bytes)?),
            Type::U64 => Val::U64(u64::load(mem, bytes)?),
            Type::Float32 => Val::Float32(u32::load(mem, bytes)?),
            Type::Float64 => Val::Float64(u64::load(mem, bytes)?),
            Type::Char => Val::Char(char::load(mem, bytes)?),
            Type::String => Val::String(Box::<str>::load(mem, bytes)?),
            Type::List(handle) => {
                // FIXME: needs memory64 treatment
                let ptr = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
                let len = u32::from_le_bytes(bytes[4..].try_into().unwrap()) as usize;
                load_list(handle, mem, ptr, len)?
            }
            Type::Record(handle) => Val::Record(values::Record {
                ty: handle.clone(),
                values: load_record(handle.fields().map(|field| field.ty), mem, bytes)?,
            }),
            Type::Tuple(handle) => Val::Tuple(values::Tuple {
                ty: handle.clone(),
                values: load_record(handle.types(), mem, bytes)?,
            }),
            Type::Variant(handle) => {
                let (discriminant, value) =
                    self.load_variant(handle.cases().map(|case| case.ty), mem, bytes)?;

                Val::Variant(values::Variant {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Enum(handle) => {
                let (discriminant, _) =
                    self.load_variant(handle.names().map(|_| Type::Unit), mem, bytes)?;

                Val::Enum(values::Enum {
                    ty: handle.clone(),
                    discriminant,
                })
            }
            Type::Union(handle) => {
                let (discriminant, value) = self.load_variant(handle.types(), mem, bytes)?;

                Val::Union(values::Union {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Option(handle) => {
                let (discriminant, value) =
                    self.load_variant([Type::Unit, handle.ty()].into_iter(), mem, bytes)?;

                Val::Option(values::Option {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Expected(handle) => {
                let (discriminant, value) =
                    self.load_variant([handle.ok(), handle.err()].into_iter(), mem, bytes)?;

                Val::Expected(values::Expected {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Flags(handle) => Val::Flags(values::Flags {
                ty: handle.clone(),
                count: u32::try_from(handle.names().len())?,
                value: match FlagsSize::from_count(handle.names().len()) {
                    FlagsSize::Size1 => iter::once(u8::load(mem, bytes)? as u32).collect(),
                    FlagsSize::Size2 => iter::once(u16::load(mem, bytes)? as u32).collect(),
                    FlagsSize::Size4Plus(n) => (0..n)
                        .map(|index| u32::load(mem, &bytes[index * 4..][..4]))
                        .collect::<Result<_>>()?,
                },
            }),
        })
    }

    fn load_variant(
        &self,
        mut types: impl ExactSizeIterator<Item = Type>,
        mem: &Memory,
        bytes: &[u8],
    ) -> Result<(u32, Val)> {
        let discriminant_size = DiscriminantSize::from_count(types.len()).unwrap();
        let discriminant = match discriminant_size {
            DiscriminantSize::Size1 => u8::load(mem, &bytes[..1])? as u32,
            DiscriminantSize::Size2 => u16::load(mem, &bytes[..2])? as u32,
            DiscriminantSize::Size4 => u32::load(mem, &bytes[..4])?,
        };
        let ty = types.nth(discriminant as usize).ok_or_else(|| {
            anyhow!(
                "discriminant {} out of range [0..{})",
                discriminant,
                types.len()
            )
        })?;
        let value = ty.load(
            mem,
            &bytes[func::align_to(
                usize::from(discriminant_size),
                self.size_and_alignment().alignment,
            )..][..ty.size_and_alignment().size],
        )?;
        Ok((discriminant, value))
    }

    /// Calculate the size and alignment requirements for the specified type.
    pub(crate) fn size_and_alignment(&self) -> SizeAndAlignment {
        match self {
            Type::Unit => SizeAndAlignment {
                size: 0,
                alignment: 1,
            },

            Type::Bool | Type::S8 | Type::U8 => SizeAndAlignment {
                size: 1,
                alignment: 1,
            },

            Type::S16 | Type::U16 => SizeAndAlignment {
                size: 2,
                alignment: 2,
            },

            Type::S32 | Type::U32 | Type::Char | Type::Float32 => SizeAndAlignment {
                size: 4,
                alignment: 4,
            },

            Type::S64 | Type::U64 | Type::Float64 => SizeAndAlignment {
                size: 8,
                alignment: 8,
            },

            Type::String | Type::List(_) => SizeAndAlignment {
                size: 8,
                alignment: 4,
            },

            Type::Record(handle) => {
                record_size_and_alignment(handle.fields().map(|field| field.ty))
            }

            Type::Tuple(handle) => record_size_and_alignment(handle.types()),

            Type::Variant(handle) => variant_size_and_alignment(handle.cases().map(|case| case.ty)),

            Type::Enum(handle) => variant_size_and_alignment(handle.names().map(|_| Type::Unit)),

            Type::Union(handle) => variant_size_and_alignment(handle.types()),

            Type::Option(handle) => {
                variant_size_and_alignment([Type::Unit, handle.ty()].into_iter())
            }

            Type::Expected(handle) => {
                variant_size_and_alignment([handle.ok(), handle.err()].into_iter())
            }

            Type::Flags(handle) => match FlagsSize::from_count(handle.names().len()) {
                FlagsSize::Size1 => SizeAndAlignment {
                    size: 1,
                    alignment: 1,
                },
                FlagsSize::Size2 => SizeAndAlignment {
                    size: 2,
                    alignment: 2,
                },
                FlagsSize::Size4Plus(n) => SizeAndAlignment {
                    size: n * 4,
                    alignment: 4,
                },
            },
        }
    }

    /// Calculate the aligned offset of a field of this type, updating `offset` to point to just after that field.
    pub(crate) fn next_field(&self, offset: &mut usize) -> usize {
        let SizeAndAlignment { size, alignment } = self.size_and_alignment();
        *offset = func::align_to(*offset, alignment);
        let result = *offset;
        *offset += size;
        result
    }
}

fn load_list(handle: &List, mem: &Memory, ptr: usize, len: usize) -> Result<Val> {
    let element_type = handle.ty();
    let SizeAndAlignment {
        size: element_size,
        alignment: element_alignment,
    } = element_type.size_and_alignment();

    match len
        .checked_mul(element_size)
        .and_then(|len| ptr.checked_add(len))
    {
        Some(n) if n <= mem.as_slice().len() => {}
        _ => bail!("list pointer/length out of bounds of memory"),
    }
    if ptr % usize::try_from(element_alignment)? != 0 {
        bail!("list pointer is not aligned")
    }

    Ok(Val::List(values::List {
        ty: handle.clone(),
        values: (0..len)
            .map(|index| {
                element_type.load(
                    mem,
                    &mem.as_slice()[ptr + (index * element_size)..][..element_size],
                )
            })
            .collect::<Result<_>>()?,
    }))
}

fn load_record(
    types: impl Iterator<Item = Type>,
    mem: &Memory,
    bytes: &[u8],
) -> Result<Box<[Val]>> {
    let mut offset = 0;
    types
        .map(|ty| {
            ty.load(
                mem,
                &bytes[ty.next_field(&mut offset)..][..ty.size_and_alignment().size],
            )
        })
        .collect()
}

fn lift_variant<'a>(
    flatten_count: usize,
    mut types: impl ExactSizeIterator<Item = Type>,
    store: &StoreOpaque,
    options: &Options,
    src: &mut std::slice::Iter<'_, ValRaw>,
) -> Result<(u32, Val)> {
    let len = types.len();
    let discriminant = next(src).get_u32();
    let ty = types
        .nth(discriminant as usize)
        .ok_or_else(|| anyhow!("discriminant {} out of range [0..{})", discriminant, len))?;
    let value = ty.lift(store, options, src)?;
    for _ in (1 + ty.flatten_count())..flatten_count {
        next(src);
    }
    Ok((discriminant, value))
}

fn record_size_and_alignment(types: impl Iterator<Item = Type>) -> SizeAndAlignment {
    let mut offset = 0;
    let mut align = 1;
    for ty in types {
        let SizeAndAlignment { size, alignment } = ty.size_and_alignment();
        offset = func::align_to(offset, alignment) + size;
        align = align.max(alignment);
    }

    SizeAndAlignment {
        size: func::align_to(offset, align),
        alignment: align,
    }
}

fn variant_size_and_alignment(types: impl ExactSizeIterator<Item = Type>) -> SizeAndAlignment {
    let discriminant_size = DiscriminantSize::from_count(types.len()).unwrap();
    let mut alignment = u32::from(discriminant_size);
    let mut size = 0;
    for ty in types {
        let size_and_alignment = ty.size_and_alignment();
        alignment = alignment.max(size_and_alignment.alignment);
        size = size.max(size_and_alignment.size);
    }

    SizeAndAlignment {
        size: func::align_to(usize::from(discriminant_size), alignment) + size,
        alignment,
    }
}

fn next<'a>(src: &mut std::slice::Iter<'a, ValRaw>) -> &'a ValRaw {
    src.next().unwrap()
}

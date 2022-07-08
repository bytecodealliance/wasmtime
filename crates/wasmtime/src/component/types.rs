use crate::component::func::{self, Lift, Memory, Options, WasmStr};
use crate::component::values::{
    self, Enum, Expected, Flags, List, Option, Record, Tuple, Union, Val, Variant,
};
use crate::store::StoreOpaque;
use crate::ValRaw;
use anyhow::{anyhow, bail, Context, Error, Result};
use std::collections::HashMap;
use std::fmt;
use std::iter;
use std::ops::Deref;
use std::sync::Arc;
use wasmtime_component_util::{DiscriminantSize, FlagsSize};
use wasmtime_environ::component::{
    ComponentTypes, InterfaceType, TypeEnumIndex, TypeExpectedIndex, TypeFlagsIndex,
    TypeInterfaceIndex, TypeRecordIndex, TypeTupleIndex, TypeUnionIndex, TypeVariantIndex,
};

#[derive(Clone)]
pub struct Handle<T> {
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TypeIndex(TypeInterfaceIndex);

impl Handle<TypeIndex> {
    pub fn ty(&self) -> Type {
        Type::from(&self.types[self.index.0], &self.types)
    }
}

pub struct Field<'a> {
    pub name: &'a str,
    pub ty: Type,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RecordIndex(TypeRecordIndex);

impl Handle<RecordIndex> {
    pub fn fields(&self) -> impl ExactSizeIterator<Item = Field> {
        self.types[self.index.0].fields.iter().map(|field| Field {
            name: &field.name,
            ty: Type::from(&field.ty, &self.types),
        })
    }
}

pub struct Case<'a> {
    pub name: &'a str,
    pub ty: Type,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct VariantIndex(TypeVariantIndex);

impl Handle<VariantIndex> {
    pub fn cases(&self) -> impl ExactSizeIterator<Item = Case> {
        self.types[self.index.0].cases.iter().map(|case| Case {
            name: &case.name,
            ty: Type::from(&case.ty, &self.types),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FlagsIndex(TypeFlagsIndex);

impl Handle<FlagsIndex> {
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.types[self.index.0]
            .names
            .iter()
            .map(|name| name.deref())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TupleIndex(TypeTupleIndex);

impl Handle<TupleIndex> {
    pub fn types(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.types[self.index.0]
            .types
            .iter()
            .map(|ty| Type::from(ty, &self.types))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct EnumIndex(TypeEnumIndex);

impl Handle<EnumIndex> {
    pub fn names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.types[self.index.0]
            .names
            .iter()
            .map(|name| name.deref())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct UnionIndex(TypeUnionIndex);

impl Handle<UnionIndex> {
    pub fn types(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.types[self.index.0]
            .types
            .iter()
            .map(|ty| Type::from(ty, &self.types))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ExpectedIndex(TypeExpectedIndex);

impl Handle<ExpectedIndex> {
    pub fn ok(&self) -> Type {
        Type::from(&self.types[self.index.0].ok, &self.types)
    }

    pub fn err(&self) -> Type {
        Type::from(&self.types[self.index.0].err, &self.types)
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
    List(Handle<TypeIndex>),
    /// Record
    Record(Handle<RecordIndex>),
    /// Variant
    Variant(Handle<VariantIndex>),
    /// Bit flags
    Flags(Handle<FlagsIndex>),
    /// Tuple
    Tuple(Handle<TupleIndex>),
    /// Enum
    Enum(Handle<EnumIndex>),
    /// Union
    Union(Handle<UnionIndex>),
    /// Option
    Option(Handle<TypeIndex>),
    /// Expected
    Expected(Handle<ExpectedIndex>),
}

impl Type {
    /// Instantiate this type (which must be a `Type::List`) with the specified `values`.
    pub fn new_list(&self, values: Box<[Val]>) -> Result<Val> {
        if let Type::List(handle) = self {
            let ty = handle.ty();
            for (index, value) in values.iter().enumerate() {
                ty.check(value)
                    .with_context(|| format!("type mismatch for element {index} of list"))?;
            }

            Ok(Val::List(List {
                ty: handle.clone(),
                values,
            }))
        } else {
            Err(anyhow!("cannot make list from {} type", self.desc()))
        }
    }

    /// Instantiate this type (which must be a `Type::Record`) with the specified `values`.
    pub fn new_record<
        'a,
        I: ExactSizeIterator<Item = (&'a str, Val)>,
        II: IntoIterator<Item = (&'a str, Val), IntoIter = I>,
    >(
        &self,
        values: II,
    ) -> Result<Val> {
        if let Type::Record(handle) = self {
            let values = values.into_iter();

            if values.len() != handle.fields().len() {
                bail!(
                    "expected {} value(s); got {}",
                    handle.fields().len(),
                    values.len()
                );
            }

            Ok(Val::Record(Record {
                ty: handle.clone(),
                values: values
                    .zip(handle.fields())
                    .map(|((name, value), field)| {
                        if name == field.name {
                            field.ty.check(&value).with_context(|| {
                                format!("type mismatch for field {name} of record")
                            })?;

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
        } else {
            Err(anyhow!("cannot make record from {} type", self.desc()))
        }
    }

    /// Instantiate this type (which must be a `Type::Tuple`) with the specified `values`.
    pub fn new_tuple(&self, values: Box<[Val]>) -> Result<Val> {
        if let Type::Tuple(handle) = self {
            if values.len() != handle.types().len() {
                bail!(
                    "expected {} value(s); got {}",
                    handle.types().len(),
                    values.len()
                );
            }

            for (index, (value, ty)) in values.iter().zip(handle.types()).enumerate() {
                ty.check(value)
                    .with_context(|| format!("type mismatch for field {index} of tuple"))?;
            }

            Ok(Val::Tuple(Tuple {
                ty: handle.clone(),
                values,
            }))
        } else {
            Err(anyhow!("cannot make tuple from {} type", self.desc()))
        }
    }

    /// Instantiate this type (which must be a `Type::Variant`) with the specified case `name` and `value`.
    pub fn new_variant(&self, name: &str, value: Val) -> Result<Val> {
        if let Type::Variant(handle) = self {
            let (discriminant, ty) = handle
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

            Ok(Val::Variant(Variant {
                ty: handle.clone(),
                discriminant: u32::try_from(discriminant)?,
                value: Box::new(value),
            }))
        } else {
            Err(anyhow!("cannot make variant from {} type", self.desc()))
        }
    }

    /// Instantiate this type (which must be a `Type::Enum`) with the specified case `name`.
    pub fn new_enum(&self, name: &str) -> Result<Val> {
        if let Type::Enum(handle) = self {
            let discriminant = u32::try_from(
                handle
                    .names()
                    .position(|n| n == name)
                    .ok_or_else(|| anyhow!("unknown enum case: {name}"))?,
            )?;

            Ok(Val::Enum(Enum {
                ty: handle.clone(),
                discriminant,
            }))
        } else {
            Err(anyhow!("cannot make enum from {} type", self.desc()))
        }
    }

    /// Instantiate this type (which must be a `Type::Union`) with the specified `discriminant` and `value`.
    pub fn new_union(&self, discriminant: u32, value: Val) -> Result<Val> {
        if let Type::Union(handle) = self {
            if let Some(ty) = handle.types().nth(usize::try_from(discriminant)?) {
                ty.check(&value)
                    .with_context(|| format!("type mismatch for case {discriminant} of union"))?;

                Ok(Val::Union(Union {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                }))
            } else {
                Err(anyhow!(
                    "discriminant {discriminant} out of range: [0,{})",
                    handle.types().len()
                ))
            }
        } else {
            Err(anyhow!("cannot make union from {} type", self.desc()))
        }
    }

    /// Instantiate this type (which must be a `Type::Option`) with the specified `value`.
    pub fn new_option(&self, value: std::option::Option<Val>) -> Result<Val> {
        if let Type::Option(handle) = self {
            let value = value
                .map(|value| {
                    handle
                        .ty()
                        .check(&value)
                        .context("type mismatch for option")?;

                    Ok::<_, Error>(value)
                })
                .transpose()?;

            Ok(Val::Option(Option {
                ty: handle.clone(),
                discriminant: if value.is_none() { 0 } else { 1 },
                value: Box::new(value.unwrap_or(Val::Unit)),
            }))
        } else {
            Err(anyhow!("cannot make option from {} type", self.desc()))
        }
    }

    /// Instantiate this type (which must be a `Type::Expected`) with the specified `value`.
    pub fn new_expected(&self, value: Result<Val, Val>) -> Result<Val> {
        if let Type::Expected(handle) = self {
            Ok(Val::Expected(Expected {
                ty: handle.clone(),
                discriminant: if value.is_ok() { 0 } else { 1 },
                value: Box::new(match value {
                    Ok(value) => {
                        handle
                            .ok()
                            .check(&value)
                            .context("type mismatch for ok case of expected")?;
                        value
                    }
                    Err(value) => {
                        handle
                            .err()
                            .check(&value)
                            .context("type mismatch for err case of expected")?;
                        value
                    }
                }),
            }))
        } else {
            Err(anyhow!("cannot make expected from {} type", self.desc()))
        }
    }

    /// Instantiate this type (which must be a `Type::Flags`) with the specified flag `names`.
    pub fn new_flags(&self, names: &[&str]) -> Result<Val> {
        if let Type::Flags(handle) = self {
            let map = handle
                .names()
                .enumerate()
                .map(|(index, name)| (name, index))
                .collect::<HashMap<_, _>>();

            let mut values = vec![0_u32; values::u32_count_for_flag_count(handle.names().len())];

            for name in names {
                let index = map
                    .get(name)
                    .ok_or_else(|| anyhow!("unknown flag: {name}"))?;
                values[index / 32] |= 1 << (index % 32);
            }

            Ok(Val::Flags(Flags {
                ty: handle.clone(),
                count: u32::try_from(map.len())?,
                value: values.into(),
            }))
        } else {
            Err(anyhow!("cannot make flags from {} type", self.desc()))
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
        if self == &value.ty() {
            Ok(())
        } else {
            Err(anyhow!(
                "type mismatch: expected {}, got {}",
                self.desc(),
                value.ty().desc()
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
            InterfaceType::List(index) => Type::List(Handle {
                index: TypeIndex(*index),
                types: types.clone(),
            }),
            InterfaceType::Record(index) => Type::Record(Handle {
                index: RecordIndex(*index),
                types: types.clone(),
            }),
            InterfaceType::Variant(index) => Type::Variant(Handle {
                index: VariantIndex(*index),
                types: types.clone(),
            }),
            InterfaceType::Flags(index) => Type::Flags(Handle {
                index: FlagsIndex(*index),
                types: types.clone(),
            }),
            InterfaceType::Tuple(index) => Type::Tuple(Handle {
                index: TupleIndex(*index),
                types: types.clone(),
            }),
            InterfaceType::Enum(index) => Type::Enum(Handle {
                index: EnumIndex(*index),
                types: types.clone(),
            }),
            InterfaceType::Union(index) => Type::Union(Handle {
                index: UnionIndex(*index),
                types: types.clone(),
            }),
            InterfaceType::Option(index) => Type::Option(Handle {
                index: TypeIndex(*index),
                types: types.clone(),
            }),
            InterfaceType::Expected(index) => Type::Expected(Handle {
                index: ExpectedIndex(*index),
                types: types.clone(),
            }),
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
            Type::Variant(_) => "variant",
            Type::Flags(_) => "flags",
            Type::Tuple(_) => "tuple",
            Type::Enum(_) => "enum",
            Type::Union(_) => "union",
            Type::Option(_) => "option",
            Type::Expected(_) => "expected",
        }
    }

    /// Deserialize a value of this type from core Wasm stack values.
    pub(crate) fn lift<'a>(
        &self,
        store: &StoreOpaque,
        options: &Options,
        src: &mut impl Iterator<Item = &'a ValRaw>,
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
            Type::String | Type::List(_) => {
                // These won't fit in func::MAX_STACK_RESULTS as of this writing, so presumably we should never
                // reach here
                unreachable!()
            }
            Type::Record(handle) => Val::Record(Record {
                ty: handle.clone(),
                values: handle
                    .fields()
                    .map(|field| field.ty.lift(store, options, src))
                    .collect::<Result<_>>()?,
            }),
            Type::Tuple(handle) => Val::Tuple(Tuple {
                ty: handle.clone(),
                values: handle
                    .types()
                    .map(|ty| ty.lift(store, options, src))
                    .collect::<Result<_>>()?,
            }),
            Type::Variant(handle) => {
                let (discriminant, value) =
                    lift_variant(handle.cases().map(|case| case.ty), store, options, src)?;

                Val::Variant(Variant {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Enum(handle) => {
                let (discriminant, _) =
                    lift_variant(handle.names().map(|_| Type::Unit), store, options, src)?;

                Val::Enum(Enum {
                    ty: handle.clone(),
                    discriminant,
                })
            }
            Type::Union(handle) => {
                let (discriminant, value) = lift_variant(handle.types(), store, options, src)?;

                Val::Union(Union {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Option(handle) => {
                let (discriminant, value) =
                    lift_variant([Type::Unit, handle.ty()].into_iter(), store, options, src)?;

                Val::Option(Option {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Expected(handle) => {
                let (discriminant, value) =
                    lift_variant([handle.ok(), handle.err()].into_iter(), store, options, src)?;

                Val::Expected(Expected {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Flags(handle) => {
                let count = u32::try_from(handle.names().len()).unwrap();
                assert!(count <= 32);
                let value = iter::once(u32::lift(store, options, next(src))?).collect();

                Val::Flags(Flags {
                    ty: handle.clone(),
                    count,
                    value,
                })
            }
        })
    }

    /// Deserialize a value of this type from the heap.
    pub(crate) fn load(&self, store: &StoreOpaque, mem: &Memory, bytes: &[u8]) -> Result<Val> {
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
            Type::String => Val::String(Box::from(
                WasmStr::load(mem, bytes)?._to_str(store)?.deref(),
            )),
            Type::List(handle) => {
                // FIXME: needs memory64 treatment
                let ptr = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
                let len = u32::from_le_bytes(bytes[4..].try_into().unwrap()) as usize;
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

                Val::List(List {
                    ty: handle.clone(),
                    values: (0..len)
                        .map(|index| {
                            element_type.load(
                                store,
                                mem,
                                &mem.as_slice()[ptr + (index * element_size)..][..element_size],
                            )
                        })
                        .collect::<Result<_>>()?,
                })
            }
            Type::Record(handle) => Val::Record(Record {
                ty: handle.clone(),
                values: load_record(handle.fields().map(|field| field.ty), store, mem, bytes)?,
            }),
            Type::Tuple(handle) => Val::Tuple(Tuple {
                ty: handle.clone(),
                values: load_record(handle.types(), store, mem, bytes)?,
            }),
            Type::Variant(handle) => {
                let (discriminant, value) =
                    self.load_variant(handle.cases().map(|case| case.ty), store, mem, bytes)?;

                Val::Variant(Variant {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Enum(handle) => {
                let (discriminant, _) =
                    self.load_variant(handle.names().map(|_| Type::Unit), store, mem, bytes)?;

                Val::Enum(Enum {
                    ty: handle.clone(),
                    discriminant,
                })
            }
            Type::Union(handle) => {
                let (discriminant, value) = self.load_variant(handle.types(), store, mem, bytes)?;

                Val::Union(Union {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Option(handle) => {
                let (discriminant, value) =
                    self.load_variant([Type::Unit, handle.ty()].into_iter(), store, mem, bytes)?;

                Val::Option(Option {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Expected(handle) => {
                let (discriminant, value) =
                    self.load_variant([handle.ok(), handle.err()].into_iter(), store, mem, bytes)?;

                Val::Expected(Expected {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Flags(handle) => Val::Flags(Flags {
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
        store: &StoreOpaque,
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
            store,
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

fn load_record(
    types: impl Iterator<Item = Type>,
    store: &StoreOpaque,
    mem: &Memory,
    bytes: &[u8],
) -> Result<Box<[Val]>> {
    let mut offset = 0;
    types
        .map(|ty| {
            ty.load(
                store,
                mem,
                &bytes[ty.next_field(&mut offset)..][..ty.size_and_alignment().size],
            )
        })
        .collect()
}

fn lift_variant<'a>(
    mut types: impl ExactSizeIterator<Item = Type>,
    store: &StoreOpaque,
    options: &Options,
    src: &mut impl Iterator<Item = &'a ValRaw>,
) -> Result<(u32, Val)> {
    let len = types.len();
    let discriminant = next(src).get_u32();
    let ty = types
        .nth(discriminant as usize)
        .ok_or_else(|| anyhow!("discriminant {} out of range [0..{})", discriminant, len))?;
    let value = ty.lift(store, options, src)?;
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
    let mut alignment = 1;
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

fn next<'a>(src: &mut impl Iterator<Item = &'a ValRaw>) -> &'a ValRaw {
    src.next().unwrap()
}

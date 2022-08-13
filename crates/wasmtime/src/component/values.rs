use crate::component::func::{Lift, Lower, Memory, MemoryMut, Options};
use crate::component::types::{self, Type};
use crate::store::StoreOpaque;
use crate::{AsContextMut, StoreContextMut, ValRaw};
use anyhow::{anyhow, bail, Context, Error, Result};
use std::collections::HashMap;
use std::fmt;
use std::iter;
use std::mem::MaybeUninit;
use std::ops::Deref;
use wasmtime_component_util::{DiscriminantSize, FlagsSize};
use wasmtime_environ::component::VariantInfo;

#[derive(PartialEq, Eq, Clone)]
pub struct List {
    ty: types::List,
    values: Box<[Val]>,
}

impl List {
    /// Instantiate the specified type with the specified `values`.
    pub fn new(ty: &types::List, values: Box<[Val]>) -> Result<Self> {
        let element_type = ty.ty();
        for (index, value) in values.iter().enumerate() {
            element_type
                .check(value)
                .with_context(|| format!("type mismatch for element {index} of list"))?;
        }

        Ok(Self {
            ty: ty.clone(),
            values,
        })
    }

    /// Returns the corresponding type of this list
    pub fn ty(&self) -> &types::List {
        &self.ty
    }
}

impl Deref for List {
    type Target = [Val];

    fn deref(&self) -> &[Val] {
        &self.values
    }
}

impl fmt::Debug for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_list();
        for val in self.iter() {
            f.entry(val);
        }
        f.finish()
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Record {
    ty: types::Record,
    values: Box<[Val]>,
}

impl Record {
    /// Instantiate the specified type with the specified `values`.
    pub fn new<'a>(
        ty: &types::Record,
        values: impl IntoIterator<Item = (&'a str, Val)>,
    ) -> Result<Self> {
        let mut fields = ty.fields();
        let expected_len = fields.len();
        let mut iter = values.into_iter();
        let mut values = Vec::with_capacity(expected_len);
        loop {
            match (fields.next(), iter.next()) {
                (Some(field), Some((name, value))) => {
                    if name == field.name {
                        field
                            .ty
                            .check(&value)
                            .with_context(|| format!("type mismatch for field {name} of record"))?;

                        values.push(value);
                    } else {
                        bail!("field name mismatch: expected {}; got {name}", field.name)
                    }
                }
                (None, Some((_, value))) => values.push(value),
                _ => break,
            }
        }

        if values.len() != expected_len {
            bail!("expected {} value(s); got {}", expected_len, values.len());
        }

        Ok(Self {
            ty: ty.clone(),
            values: values.into(),
        })
    }

    /// Returns the corresponding type of this record.
    pub fn ty(&self) -> &types::Record {
        &self.ty
    }

    /// Gets the value of the specified field `name` from this record.
    pub fn fields(&self) -> impl Iterator<Item = (&str, &Val)> {
        assert_eq!(self.values.len(), self.ty.fields().len());
        self.ty
            .fields()
            .zip(self.values.iter())
            .map(|(ty, val)| (ty.name, val))
    }
}

impl fmt::Debug for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Record");
        for (name, val) in self.fields() {
            f.field(name, val);
        }
        f.finish()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tuple {
    ty: types::Tuple,
    values: Box<[Val]>,
}

impl Tuple {
    /// Instantiate the specified type ith the specified `values`.
    pub fn new(ty: &types::Tuple, values: Box<[Val]>) -> Result<Self> {
        if values.len() != ty.types().len() {
            bail!(
                "expected {} value(s); got {}",
                ty.types().len(),
                values.len()
            );
        }

        for (index, (value, ty)) in values.iter().zip(ty.types()).enumerate() {
            ty.check(value)
                .with_context(|| format!("type mismatch for field {index} of tuple"))?;
        }

        Ok(Self {
            ty: ty.clone(),
            values,
        })
    }

    /// Returns the type of this tuple.
    pub fn ty(&self) -> &types::Tuple {
        &self.ty
    }

    /// Returns the list of values that this tuple contains.
    pub fn values(&self) -> &[Val] {
        &self.values
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Variant {
    ty: types::Variant,
    discriminant: u32,
    value: Box<Val>,
}

impl Variant {
    /// Instantiate the specified type with the specified case `name` and `value`.
    pub fn new(ty: &types::Variant, name: &str, value: Val) -> Result<Self> {
        let (discriminant, case_type) = ty
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

        case_type
            .check(&value)
            .with_context(|| format!("type mismatch for case {name} of variant"))?;

        Ok(Self {
            ty: ty.clone(),
            discriminant: u32::try_from(discriminant)?,
            value: Box::new(value),
        })
    }

    /// Returns the type of this variant.
    pub fn ty(&self) -> &types::Variant {
        &self.ty
    }

    /// Returns name of the discriminant of this value within the variant type.
    pub fn discriminant(&self) -> &str {
        self.ty
            .cases()
            .nth(self.discriminant as usize)
            .unwrap()
            .name
    }

    /// Returns the payload value for this variant.
    pub fn payload(&self) -> &Val {
        &self.value
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(self.discriminant())
            .field(self.payload())
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Enum {
    ty: types::Enum,
    discriminant: u32,
}

impl Enum {
    /// Instantiate the specified type with the specified case `name`.
    pub fn new(ty: &types::Enum, name: &str) -> Result<Self> {
        let discriminant = u32::try_from(
            ty.names()
                .position(|n| n == name)
                .ok_or_else(|| anyhow!("unknown enum case: {name}"))?,
        )?;

        Ok(Self {
            ty: ty.clone(),
            discriminant,
        })
    }

    /// Returns the type of this value.
    pub fn ty(&self) -> &types::Enum {
        &self.ty
    }

    /// Returns name of this enum value.
    pub fn discriminant(&self) -> &str {
        self.ty.names().nth(self.discriminant as usize).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Union {
    ty: types::Union,
    discriminant: u32,
    value: Box<Val>,
}

impl Union {
    /// Instantiate the specified type with the specified `discriminant` and `value`.
    pub fn new(ty: &types::Union, discriminant: u32, value: Val) -> Result<Self> {
        if let Some(case_ty) = ty.types().nth(usize::try_from(discriminant)?) {
            case_ty
                .check(&value)
                .with_context(|| format!("type mismatch for case {discriminant} of union"))?;

            Ok(Self {
                ty: ty.clone(),
                discriminant,
                value: Box::new(value),
            })
        } else {
            Err(anyhow!(
                "discriminant {discriminant} out of range: [0,{})",
                ty.types().len()
            ))
        }
    }

    /// Returns the type of this value.
    pub fn ty(&self) -> &types::Union {
        &self.ty
    }

    /// Returns name of the discriminant of this value within the union type.
    pub fn discriminant(&self) -> u32 {
        self.discriminant
    }

    /// Returns the payload value for this union.
    pub fn payload(&self) -> &Val {
        &self.value
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Option {
    ty: types::Option,
    discriminant: u32,
    value: Box<Val>,
}

impl Option {
    /// Instantiate the specified type with the specified `value`.
    pub fn new(ty: &types::Option, value: std::option::Option<Val>) -> Result<Self> {
        let value = value
            .map(|value| {
                ty.ty().check(&value).context("type mismatch for option")?;

                Ok::<_, Error>(value)
            })
            .transpose()?;

        Ok(Self {
            ty: ty.clone(),
            discriminant: if value.is_none() { 0 } else { 1 },
            value: Box::new(value.unwrap_or(Val::Unit)),
        })
    }

    /// Returns the type of this value.
    pub fn ty(&self) -> &types::Option {
        &self.ty
    }

    /// Returns the optional value contained within.
    pub fn value(&self) -> std::option::Option<&Val> {
        if self.discriminant == 0 {
            None
        } else {
            Some(&self.value)
        }
    }
}

impl fmt::Debug for Option {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value().fmt(f)
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Expected {
    ty: types::Expected,
    discriminant: u32,
    value: Box<Val>,
}

impl Expected {
    /// Instantiate the specified type with the specified `value`.
    pub fn new(ty: &types::Expected, value: Result<Val, Val>) -> Result<Self> {
        Ok(Self {
            ty: ty.clone(),
            discriminant: if value.is_ok() { 0 } else { 1 },
            value: Box::new(match value {
                Ok(value) => {
                    ty.ok()
                        .check(&value)
                        .context("type mismatch for ok case of expected")?;
                    value
                }
                Err(value) => {
                    ty.err()
                        .check(&value)
                        .context("type mismatch for err case of expected")?;
                    value
                }
            }),
        })
    }

    /// Returns the type of this value.
    pub fn ty(&self) -> &types::Expected {
        &self.ty
    }

    /// Returns the result value contained within.
    pub fn value(&self) -> Result<&Val, &Val> {
        if self.discriminant == 0 {
            Ok(&self.value)
        } else {
            Err(&self.value)
        }
    }
}

impl fmt::Debug for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value().fmt(f)
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Flags {
    ty: types::Flags,
    count: u32,
    value: Box<[u32]>,
}

impl Flags {
    /// Instantiate the specified type with the specified flag `names`.
    pub fn new(ty: &types::Flags, names: &[&str]) -> Result<Self> {
        let map = ty
            .names()
            .enumerate()
            .map(|(index, name)| (name, index))
            .collect::<HashMap<_, _>>();

        let mut values = vec![0_u32; u32_count_for_flag_count(ty.names().len())];

        for name in names {
            let index = map
                .get(name)
                .ok_or_else(|| anyhow!("unknown flag: {name}"))?;
            values[index / 32] |= 1 << (index % 32);
        }

        Ok(Self {
            ty: ty.clone(),
            count: u32::try_from(map.len())?,
            value: values.into(),
        })
    }

    /// Returns the type of this value.
    pub fn ty(&self) -> &types::Flags {
        &self.ty
    }

    /// Returns an iterator over the set of names that this flags set contains.
    pub fn flags(&self) -> impl Iterator<Item = &str> {
        (0..self.count).filter_map(|i| {
            let (idx, bit) = ((i / 32) as usize, i % 32);
            if self.value[idx] & (1 << bit) != 0 {
                Some(self.ty.names().nth(i as usize).unwrap())
            } else {
                None
            }
        })
    }
}

impl fmt::Debug for Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut set = f.debug_set();
        for flag in self.flags() {
            set.entry(&flag);
        }
        set.finish()
    }
}

/// Represents possible runtime values which a component function can either consume or produce
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Val {
    /// Unit
    Unit,
    /// Boolean
    Bool(bool),
    /// Signed 8-bit integer
    S8(i8),
    /// Unsigned 8-bit integer
    U8(u8),
    /// Signed 16-bit integer
    S16(i16),
    /// Unsigned 16-bit integer
    U16(u16),
    /// Signed 32-bit integer
    S32(i32),
    /// Unsigned 32-bit integer
    U32(u32),
    /// Signed 64-bit integer
    S64(i64),
    /// Unsigned 64-bit integer
    U64(u64),
    /// 32-bit floating point value
    Float32(u32),
    /// 64-bit floating point value
    Float64(u64),
    /// 32-bit character
    Char(char),
    /// Character string
    String(Box<str>),
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

impl Val {
    /// Retrieve the [`Type`] of this value.
    pub fn ty(&self) -> Type {
        match self {
            Val::Unit => Type::Unit,
            Val::Bool(_) => Type::Bool,
            Val::S8(_) => Type::S8,
            Val::U8(_) => Type::U8,
            Val::S16(_) => Type::S16,
            Val::U16(_) => Type::U16,
            Val::S32(_) => Type::S32,
            Val::U32(_) => Type::U32,
            Val::S64(_) => Type::S64,
            Val::U64(_) => Type::U64,
            Val::Float32(_) => Type::Float32,
            Val::Float64(_) => Type::Float64,
            Val::Char(_) => Type::Char,
            Val::String(_) => Type::String,
            Val::List(List { ty, .. }) => Type::List(ty.clone()),
            Val::Record(Record { ty, .. }) => Type::Record(ty.clone()),
            Val::Tuple(Tuple { ty, .. }) => Type::Tuple(ty.clone()),
            Val::Variant(Variant { ty, .. }) => Type::Variant(ty.clone()),
            Val::Enum(Enum { ty, .. }) => Type::Enum(ty.clone()),
            Val::Union(Union { ty, .. }) => Type::Union(ty.clone()),
            Val::Option(Option { ty, .. }) => Type::Option(ty.clone()),
            Val::Expected(Expected { ty, .. }) => Type::Expected(ty.clone()),
            Val::Flags(Flags { ty, .. }) => Type::Flags(ty.clone()),
        }
    }

    /// Deserialize a value of this type from core Wasm stack values.
    pub(crate) fn lift<'a>(
        ty: &Type,
        store: &StoreOpaque,
        options: &Options,
        src: &mut std::slice::Iter<'_, ValRaw>,
    ) -> Result<Val> {
        Ok(match ty {
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
            Type::Record(handle) => Val::Record(Record {
                ty: handle.clone(),
                values: handle
                    .fields()
                    .map(|field| Self::lift(&field.ty, store, options, src))
                    .collect::<Result<_>>()?,
            }),
            Type::Tuple(handle) => Val::Tuple(Tuple {
                ty: handle.clone(),
                values: handle
                    .types()
                    .map(|ty| Self::lift(&ty, store, options, src))
                    .collect::<Result<_>>()?,
            }),
            Type::Variant(handle) => {
                let (discriminant, value) = lift_variant(
                    ty.flatten_count(),
                    handle.cases().map(|case| case.ty),
                    store,
                    options,
                    src,
                )?;

                Val::Variant(Variant {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Enum(handle) => {
                let (discriminant, _) = lift_variant(
                    ty.flatten_count(),
                    handle.names().map(|_| Type::Unit),
                    store,
                    options,
                    src,
                )?;

                Val::Enum(Enum {
                    ty: handle.clone(),
                    discriminant,
                })
            }
            Type::Union(handle) => {
                let (discriminant, value) =
                    lift_variant(ty.flatten_count(), handle.types(), store, options, src)?;

                Val::Union(Union {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Option(handle) => {
                let (discriminant, value) = lift_variant(
                    ty.flatten_count(),
                    [Type::Unit, handle.ty()].into_iter(),
                    store,
                    options,
                    src,
                )?;

                Val::Option(Option {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Expected(handle) => {
                let (discriminant, value) = lift_variant(
                    ty.flatten_count(),
                    [handle.ok(), handle.err()].into_iter(),
                    store,
                    options,
                    src,
                )?;

                Val::Expected(Expected {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Flags(handle) => {
                let count = u32::try_from(handle.names().len()).unwrap();
                let value = iter::repeat_with(|| u32::lift(store, options, next(src)))
                    .take(u32_count_for_flag_count(count.try_into()?))
                    .collect::<Result<_>>()?;

                Val::Flags(Flags {
                    ty: handle.clone(),
                    count,
                    value,
                })
            }
        })
    }

    /// Deserialize a value of this type from the heap.
    pub(crate) fn load(ty: &Type, mem: &Memory, bytes: &[u8]) -> Result<Val> {
        Ok(match ty {
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
            Type::Record(handle) => Val::Record(Record {
                ty: handle.clone(),
                values: load_record(handle.fields().map(|field| field.ty), mem, bytes)?,
            }),
            Type::Tuple(handle) => Val::Tuple(Tuple {
                ty: handle.clone(),
                values: load_record(handle.types(), mem, bytes)?,
            }),
            Type::Variant(handle) => {
                let (discriminant, value) = load_variant(
                    handle.variant_info(),
                    handle.cases().map(|case| case.ty),
                    mem,
                    bytes,
                )?;

                Val::Variant(Variant {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Enum(handle) => {
                let (discriminant, _) = load_variant(
                    handle.variant_info(),
                    handle.names().map(|_| Type::Unit),
                    mem,
                    bytes,
                )?;

                Val::Enum(Enum {
                    ty: handle.clone(),
                    discriminant,
                })
            }
            Type::Union(handle) => {
                let (discriminant, value) =
                    load_variant(handle.variant_info(), handle.types(), mem, bytes)?;

                Val::Union(Union {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Option(handle) => {
                let (discriminant, value) = load_variant(
                    handle.variant_info(),
                    [Type::Unit, handle.ty()].into_iter(),
                    mem,
                    bytes,
                )?;

                Val::Option(Option {
                    ty: handle.clone(),
                    discriminant,
                    value: Box::new(value),
                })
            }
            Type::Expected(handle) => {
                let (discriminant, value) = load_variant(
                    handle.variant_info(),
                    [handle.ok(), handle.err()].into_iter(),
                    mem,
                    bytes,
                )?;

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
                    FlagsSize::Size0 => Box::new([]),
                    FlagsSize::Size1 => iter::once(u8::load(mem, bytes)? as u32).collect(),
                    FlagsSize::Size2 => iter::once(u16::load(mem, bytes)? as u32).collect(),
                    FlagsSize::Size4Plus(n) => (0..n)
                        .map(|index| u32::load(mem, &bytes[index * 4..][..4]))
                        .collect::<Result<_>>()?,
                },
            }),
        })
    }

    /// Serialize this value as core Wasm stack values.
    pub(crate) fn lower<T>(
        &self,
        store: &mut StoreContextMut<T>,
        options: &Options,
        dst: &mut std::slice::IterMut<'_, MaybeUninit<ValRaw>>,
    ) -> Result<()> {
        match self {
            Val::Unit => (),
            Val::Bool(value) => value.lower(store, options, next_mut(dst))?,
            Val::S8(value) => value.lower(store, options, next_mut(dst))?,
            Val::U8(value) => value.lower(store, options, next_mut(dst))?,
            Val::S16(value) => value.lower(store, options, next_mut(dst))?,
            Val::U16(value) => value.lower(store, options, next_mut(dst))?,
            Val::S32(value) => value.lower(store, options, next_mut(dst))?,
            Val::U32(value) => value.lower(store, options, next_mut(dst))?,
            Val::S64(value) => value.lower(store, options, next_mut(dst))?,
            Val::U64(value) => value.lower(store, options, next_mut(dst))?,
            Val::Float32(value) => value.lower(store, options, next_mut(dst))?,
            Val::Float64(value) => value.lower(store, options, next_mut(dst))?,
            Val::Char(value) => value.lower(store, options, next_mut(dst))?,
            Val::String(value) => {
                let my_dst = &mut MaybeUninit::<[ValRaw; 2]>::uninit();
                value.lower(store, options, my_dst)?;
                let my_dst = unsafe { my_dst.assume_init() };
                next_mut(dst).write(my_dst[0]);
                next_mut(dst).write(my_dst[1]);
            }
            Val::List(List { values, ty }) => {
                let (ptr, len) = lower_list(
                    &ty.ty(),
                    &mut MemoryMut::new(store.as_context_mut(), options),
                    values,
                )?;
                next_mut(dst).write(ValRaw::i64(ptr as i64));
                next_mut(dst).write(ValRaw::i64(len as i64));
            }
            Val::Record(Record { values, .. }) | Val::Tuple(Tuple { values, .. }) => {
                for value in values.deref() {
                    value.lower(store, options, dst)?;
                }
            }
            Val::Variant(Variant {
                discriminant,
                value,
                ..
            })
            | Val::Union(Union {
                discriminant,
                value,
                ..
            })
            | Val::Option(Option {
                discriminant,
                value,
                ..
            })
            | Val::Expected(Expected {
                discriminant,
                value,
                ..
            }) => {
                next_mut(dst).write(ValRaw::u32(*discriminant));
                value.lower(store, options, dst)?;
                for _ in (1 + value.ty().flatten_count())..self.ty().flatten_count() {
                    next_mut(dst).write(ValRaw::u32(0));
                }
            }
            Val::Enum(Enum { discriminant, .. }) => {
                next_mut(dst).write(ValRaw::u32(*discriminant));
            }
            Val::Flags(Flags { value, .. }) => {
                for value in value.deref() {
                    next_mut(dst).write(ValRaw::u32(*value));
                }
            }
        }

        Ok(())
    }

    /// Serialize this value to the heap at the specified memory location.
    pub(crate) fn store<T>(&self, mem: &mut MemoryMut<'_, T>, offset: usize) -> Result<()> {
        debug_assert!(offset % usize::try_from(self.ty().canonical_abi().align32)? == 0);

        match self {
            Val::Unit => (),
            Val::Bool(value) => value.store(mem, offset)?,
            Val::S8(value) => value.store(mem, offset)?,
            Val::U8(value) => value.store(mem, offset)?,
            Val::S16(value) => value.store(mem, offset)?,
            Val::U16(value) => value.store(mem, offset)?,
            Val::S32(value) => value.store(mem, offset)?,
            Val::U32(value) => value.store(mem, offset)?,
            Val::S64(value) => value.store(mem, offset)?,
            Val::U64(value) => value.store(mem, offset)?,
            Val::Float32(value) => value.store(mem, offset)?,
            Val::Float64(value) => value.store(mem, offset)?,
            Val::Char(value) => value.store(mem, offset)?,
            Val::String(value) => value.store(mem, offset)?,
            Val::List(List { values, ty }) => {
                let (ptr, len) = lower_list(&ty.ty(), mem, values)?;
                // FIXME: needs memory64 handling
                *mem.get(offset + 0) = (ptr as i32).to_le_bytes();
                *mem.get(offset + 4) = (len as i32).to_le_bytes();
            }
            Val::Record(Record { values, .. }) | Val::Tuple(Tuple { values, .. }) => {
                let mut offset = offset;
                for value in values.deref() {
                    value.store(
                        mem,
                        value.ty().canonical_abi().next_field32_size(&mut offset),
                    )?;
                }
            }
            Val::Variant(Variant {
                discriminant,
                value,
                ty,
            }) => self.store_variant(*discriminant, value, ty.variant_info(), mem, offset)?,

            Val::Enum(Enum { discriminant, ty }) => {
                self.store_variant(*discriminant, &Val::Unit, ty.variant_info(), mem, offset)?
            }

            Val::Union(Union {
                discriminant,
                value,
                ty,
            }) => self.store_variant(*discriminant, value, ty.variant_info(), mem, offset)?,

            Val::Option(Option {
                discriminant,
                value,
                ty,
            }) => self.store_variant(*discriminant, value, ty.variant_info(), mem, offset)?,

            Val::Expected(Expected {
                discriminant,
                value,
                ty,
            }) => self.store_variant(*discriminant, value, ty.variant_info(), mem, offset)?,

            Val::Flags(Flags { count, value, .. }) => {
                match FlagsSize::from_count(*count as usize) {
                    FlagsSize::Size0 => {}
                    FlagsSize::Size1 => u8::try_from(value[0]).unwrap().store(mem, offset)?,
                    FlagsSize::Size2 => u16::try_from(value[0]).unwrap().store(mem, offset)?,
                    FlagsSize::Size4Plus(_) => {
                        let mut offset = offset;
                        for value in value.deref() {
                            value.store(mem, offset)?;
                            offset += 4;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn store_variant<T>(
        &self,
        discriminant: u32,
        value: &Val,
        info: &VariantInfo,
        mem: &mut MemoryMut<'_, T>,
        offset: usize,
    ) -> Result<()> {
        match info.size {
            DiscriminantSize::Size1 => u8::try_from(discriminant).unwrap().store(mem, offset)?,
            DiscriminantSize::Size2 => u16::try_from(discriminant).unwrap().store(mem, offset)?,
            DiscriminantSize::Size4 => discriminant.store(mem, offset)?,
        }

        let offset = offset + usize::try_from(info.payload_offset32).unwrap();
        value.store(mem, offset)
    }
}

fn load_list(handle: &types::List, mem: &Memory, ptr: usize, len: usize) -> Result<Val> {
    let element_type = handle.ty();
    let abi = element_type.canonical_abi();
    let element_size = usize::try_from(abi.size32).unwrap();
    let element_alignment = abi.align32;

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

    Ok(Val::List(List {
        ty: handle.clone(),
        values: (0..len)
            .map(|index| {
                Val::load(
                    &element_type,
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
            let abi = ty.canonical_abi();
            let offset = abi.next_field32(&mut offset);
            let offset = usize::try_from(offset).unwrap();
            let size = usize::try_from(abi.size32).unwrap();
            Val::load(&ty, mem, &bytes[offset..][..size])
        })
        .collect()
}

fn load_variant(
    info: &VariantInfo,
    mut types: impl ExactSizeIterator<Item = Type>,
    mem: &Memory,
    bytes: &[u8],
) -> Result<(u32, Val)> {
    let discriminant = match info.size {
        DiscriminantSize::Size1 => u32::from(u8::load(mem, &bytes[..1])?),
        DiscriminantSize::Size2 => u32::from(u16::load(mem, &bytes[..2])?),
        DiscriminantSize::Size4 => u32::load(mem, &bytes[..4])?,
    };
    let case_ty = types.nth(discriminant as usize).ok_or_else(|| {
        anyhow!(
            "discriminant {} out of range [0..{})",
            discriminant,
            types.len()
        )
    })?;
    let payload_offset = usize::try_from(info.payload_offset32).unwrap();
    let case_size = usize::try_from(case_ty.canonical_abi().size32).unwrap();
    let value = Val::load(&case_ty, mem, &bytes[payload_offset..][..case_size])?;
    Ok((discriminant, value))
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
    let value = Val::lift(&ty, store, options, src)?;
    for _ in (1 + ty.flatten_count())..flatten_count {
        next(src);
    }
    Ok((discriminant, value))
}

/// Lower a list with the specified element type and values.
fn lower_list<T>(
    element_type: &Type,
    mem: &mut MemoryMut<'_, T>,
    items: &[Val],
) -> Result<(usize, usize)> {
    let abi = element_type.canonical_abi();
    let elt_size = usize::try_from(abi.size32)?;
    let elt_align = abi.align32;
    let size = items
        .len()
        .checked_mul(elt_size)
        .ok_or_else(|| anyhow::anyhow!("size overflow copying a list"))?;
    let ptr = mem.realloc(0, 0, elt_align, size)?;
    let mut element_ptr = ptr;
    for item in items {
        item.store(mem, element_ptr)?;
        element_ptr += elt_size;
    }
    Ok((ptr, items.len()))
}

/// Calculate the size of a u32 array needed to represent the specified number of bit flags.
///
/// Note that this will always return at least 1, even if the `count` parameter is zero.
pub(crate) fn u32_count_for_flag_count(count: usize) -> usize {
    match FlagsSize::from_count(count) {
        FlagsSize::Size0 => 0,
        FlagsSize::Size1 | FlagsSize::Size2 => 1,
        FlagsSize::Size4Plus(n) => n,
    }
}

fn next<'a>(src: &mut std::slice::Iter<'a, ValRaw>) -> &'a ValRaw {
    src.next().unwrap()
}

fn next_mut<'a>(
    dst: &mut std::slice::IterMut<'a, MaybeUninit<ValRaw>>,
) -> &'a mut MaybeUninit<ValRaw> {
    dst.next().unwrap()
}

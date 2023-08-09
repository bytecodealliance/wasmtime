use crate::component::func::{bad_type_info, Lift, LiftContext, Lower, LowerContext};
use crate::component::types::{self, Type};
use crate::component::ResourceAny;
use crate::ValRaw;
use anyhow::{anyhow, bail, Context, Error, Result};
use std::collections::HashMap;
use std::fmt;
use std::iter;
use std::mem::MaybeUninit;
use std::ops::Deref;
use wasmtime_component_util::{DiscriminantSize, FlagsSize};
use wasmtime_environ::component::{
    CanonicalAbiInfo, ComponentTypes, InterfaceType, TypeListIndex, VariantInfo,
};

/// Represents runtime list values
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

/// Represents runtime record values
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

/// Represents runtime tuple values
#[derive(PartialEq, Eq, Clone)]
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

impl fmt::Debug for Tuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tuple = f.debug_tuple("");
        for val in self.values() {
            tuple.field(val);
        }
        tuple.finish()
    }
}

/// Represents runtime variant values
#[derive(PartialEq, Eq, Clone)]
pub struct Variant {
    ty: types::Variant,
    discriminant: u32,
    value: Option<Box<Val>>,
}

impl Variant {
    /// Instantiate the specified type with the specified case `name` and `value`.
    pub fn new(ty: &types::Variant, name: &str, value: Option<Val>) -> Result<Self> {
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

        typecheck_payload(name, case_type.as_ref(), value.as_ref())?;

        Ok(Self {
            ty: ty.clone(),
            discriminant: u32::try_from(discriminant)?,
            value: value.map(Box::new),
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
    pub fn payload(&self) -> Option<&Val> {
        self.value.as_deref()
    }

    fn as_generic<'a>(
        &'a self,
        types: &'a ComponentTypes,
        ty: InterfaceType,
    ) -> GenericVariant<'a> {
        let ty = match ty {
            InterfaceType::Variant(i) => &types[i],
            _ => bad_type_info(),
        };
        GenericVariant {
            discriminant: self.discriminant,
            abi: &ty.abi,
            info: &ty.info,
            payload: self
                .value
                .as_deref()
                .zip(ty.cases[self.discriminant as usize].ty),
        }
    }
}

fn typecheck_payload(name: &str, case_type: Option<&Type>, value: Option<&Val>) -> Result<()> {
    match (case_type, value) {
        (Some(expected), Some(actual)) => expected
            .check(&actual)
            .with_context(|| format!("type mismatch for case {name} of variant")),
        (None, None) => Ok(()),
        (Some(_), None) => bail!("expected a payload for case `{name}`"),
        (None, Some(_)) => bail!("did not expect payload for case `{name}`"),
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(self.discriminant())
            .field(&self.payload())
            .finish()
    }
}

/// Represents runtime enum values
#[derive(PartialEq, Eq, Clone)]
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

    fn as_generic<'a>(
        &'a self,
        types: &'a ComponentTypes,
        ty: InterfaceType,
    ) -> GenericVariant<'a> {
        let ty = match ty {
            InterfaceType::Enum(i) => &types[i],
            _ => bad_type_info(),
        };
        GenericVariant {
            discriminant: self.discriminant,
            abi: &ty.abi,
            info: &ty.info,
            payload: None,
        }
    }
}

impl fmt::Debug for Enum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.discriminant(), f)
    }
}

/// Represents runtime union values
#[derive(PartialEq, Eq, Clone)]
pub struct Union {
    ty: types::Union,
    discriminant: u32,
    value: Option<Box<Val>>,
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
                value: Some(Box::new(value)),
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
        self.value.as_ref().unwrap()
    }

    fn as_generic<'a>(
        &'a self,
        types: &'a ComponentTypes,
        ty: InterfaceType,
    ) -> GenericVariant<'a> {
        let ty = match ty {
            InterfaceType::Union(i) => &types[i],
            _ => bad_type_info(),
        };
        GenericVariant {
            discriminant: self.discriminant,
            abi: &ty.abi,
            info: &ty.info,
            payload: self
                .value
                .as_deref()
                .zip(Some(ty.types[self.discriminant as usize])),
        }
    }
}

impl fmt::Debug for Union {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&format!("U{}", self.discriminant()))
            .field(self.payload())
            .finish()
    }
}

/// Represents runtime option values
#[derive(PartialEq, Eq, Clone)]
pub struct OptionVal {
    ty: types::OptionType,
    discriminant: u32,
    value: Option<Box<Val>>,
}

impl OptionVal {
    /// Instantiate the specified type with the specified `value`.
    pub fn new(ty: &types::OptionType, value: Option<Val>) -> Result<Self> {
        let value = value
            .map(|value| {
                ty.ty().check(&value).context("type mismatch for option")?;

                Ok::<_, Error>(value)
            })
            .transpose()?;

        Ok(Self {
            ty: ty.clone(),
            discriminant: if value.is_none() { 0 } else { 1 },
            value: value.map(Box::new),
        })
    }

    /// Returns the type of this value.
    pub fn ty(&self) -> &types::OptionType {
        &self.ty
    }

    /// Returns the optional value contained within.
    pub fn value(&self) -> Option<&Val> {
        self.value.as_deref()
    }

    fn as_generic<'a>(
        &'a self,
        types: &'a ComponentTypes,
        ty: InterfaceType,
    ) -> GenericVariant<'a> {
        let ty = match ty {
            InterfaceType::Option(i) => &types[i],
            _ => bad_type_info(),
        };
        GenericVariant {
            discriminant: self.discriminant,
            abi: &ty.abi,
            info: &ty.info,
            payload: self.value.as_deref().zip(Some(ty.ty)),
        }
    }
}

impl fmt::Debug for OptionVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value().fmt(f)
    }
}

/// Represents runtime result values
#[derive(PartialEq, Eq, Clone)]
pub struct ResultVal {
    ty: types::ResultType,
    discriminant: u32,
    value: Option<Box<Val>>,
}

impl ResultVal {
    /// Instantiate the specified type with the specified `value`.
    pub fn new(ty: &types::ResultType, value: Result<Option<Val>, Option<Val>>) -> Result<Self> {
        Ok(Self {
            ty: ty.clone(),
            discriminant: if value.is_ok() { 0 } else { 1 },
            value: match value {
                Ok(value) => {
                    typecheck_payload("ok", ty.ok().as_ref(), value.as_ref())?;
                    value.map(Box::new)
                }
                Err(value) => {
                    typecheck_payload("err", ty.err().as_ref(), value.as_ref())?;
                    value.map(Box::new)
                }
            },
        })
    }

    /// Returns the type of this value.
    pub fn ty(&self) -> &types::ResultType {
        &self.ty
    }

    /// Returns the result value contained within.
    pub fn value(&self) -> Result<Option<&Val>, Option<&Val>> {
        if self.discriminant == 0 {
            Ok(self.value.as_deref())
        } else {
            Err(self.value.as_deref())
        }
    }

    fn as_generic<'a>(
        &'a self,
        types: &'a ComponentTypes,
        ty: InterfaceType,
    ) -> GenericVariant<'a> {
        let ty = match ty {
            InterfaceType::Result(i) => &types[i],
            _ => bad_type_info(),
        };
        GenericVariant {
            discriminant: self.discriminant,
            abi: &ty.abi,
            info: &ty.info,
            payload: self.value.as_deref().zip(if self.discriminant == 0 {
                ty.ok
            } else {
                ty.err
            }),
        }
    }
}

impl fmt::Debug for ResultVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value().fmt(f)
    }
}

/// Represents runtime flag values
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

        let count = usize::from(ty.canonical_abi().flat_count.unwrap());
        let mut values = vec![0_u32; count];

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

/// Represents possible runtime values which a component function can either
/// consume or produce
///
/// This is a dynamic representation of possible values in the component model.
/// Note that this is not an efficient representation but is instead intended to
/// be a flexible and somewhat convenient representation. The most efficient
/// representation of component model types is to use the `bindgen!` macro to
/// generate native Rust types with specialized liftings and lowerings.
///
/// This type is used in conjunction with [`Func::call`] for example if the
/// signature of a component is not statically known ahead of time.
///
/// # Notes on Equality
///
/// This type implements both the Rust `PartialEq` and `Eq` traits. This type
/// additionally contains values which are not necessarily easily equated,
/// however, such as floats (`Float32` and `Float64`) and resources. Equality
/// does require that two values have the same type, and then these cases are
/// handled as:
///
/// * Floats are tested if they are "semantically the same" meaning all NaN
///   values are equal to all other NaN values. Additionally zero values must be
///   exactly the same, so positive zero is not equal to negative zero. The
///   primary use case at this time is fuzzing-related equality which this is
///   sufficient for.
///
/// * Resources are tested if their types and indices into the host table are
///   equal. This does not compare the underlying representation so borrows of
///   the same guest resource are not considered equal. This additionally
///   doesn't go further and test for equality in the guest itself (for example
///   two different heap allocations of `Box<u32>` can be equal in normal Rust
///   if they contain the same value, but will never be considered equal when
///   compared as `Val::Resource`s).
///
/// In general if a strict guarantee about equality is required here it's
/// recommended to "build your own" as this equality intended for fuzzing
/// Wasmtime may not be suitable for you.
///
/// [`Func::call`]: crate::component::Func::call
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Val {
    Bool(bool),
    S8(i8),
    U8(u8),
    S16(i16),
    U16(u16),
    S32(i32),
    U32(u32),
    S64(i64),
    U64(u64),
    Float32(f32),
    Float64(f64),
    Char(char),
    String(Box<str>),
    List(List),
    Record(Record),
    Tuple(Tuple),
    Variant(Variant),
    Enum(Enum),
    Union(Union),
    Option(OptionVal),
    Result(ResultVal),
    Flags(Flags),
    Resource(ResourceAny),
}

impl Val {
    /// Retrieve the [`Type`] of this value.
    pub fn ty(&self) -> Type {
        match self {
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
            Val::Option(OptionVal { ty, .. }) => Type::Option(ty.clone()),
            Val::Result(ResultVal { ty, .. }) => Type::Result(ty.clone()),
            Val::Flags(Flags { ty, .. }) => Type::Flags(ty.clone()),
            Val::Resource(r) => {
                if r.owned() {
                    Type::Own(r.ty())
                } else {
                    Type::Borrow(r.ty())
                }
            }
        }
    }

    /// Deserialize a value of this type from core Wasm stack values.
    pub(crate) fn lift(
        cx: &mut LiftContext<'_>,
        ty: InterfaceType,
        src: &mut std::slice::Iter<'_, ValRaw>,
    ) -> Result<Val> {
        Ok(match ty {
            InterfaceType::Bool => Val::Bool(bool::lift(cx, ty, next(src))?),
            InterfaceType::S8 => Val::S8(i8::lift(cx, ty, next(src))?),
            InterfaceType::U8 => Val::U8(u8::lift(cx, ty, next(src))?),
            InterfaceType::S16 => Val::S16(i16::lift(cx, ty, next(src))?),
            InterfaceType::U16 => Val::U16(u16::lift(cx, ty, next(src))?),
            InterfaceType::S32 => Val::S32(i32::lift(cx, ty, next(src))?),
            InterfaceType::U32 => Val::U32(u32::lift(cx, ty, next(src))?),
            InterfaceType::S64 => Val::S64(i64::lift(cx, ty, next(src))?),
            InterfaceType::U64 => Val::U64(u64::lift(cx, ty, next(src))?),
            InterfaceType::Float32 => Val::Float32(f32::lift(cx, ty, next(src))?),
            InterfaceType::Float64 => Val::Float64(f64::lift(cx, ty, next(src))?),
            InterfaceType::Char => Val::Char(char::lift(cx, ty, next(src))?),
            InterfaceType::Own(_) | InterfaceType::Borrow(_) => {
                Val::Resource(ResourceAny::lift(cx, ty, next(src))?)
            }
            InterfaceType::String => {
                Val::String(Box::<str>::lift(cx, ty, &[*next(src), *next(src)])?)
            }
            InterfaceType::List(i) => {
                // FIXME: needs memory64 treatment
                let ptr = u32::lift(cx, InterfaceType::U32, next(src))? as usize;
                let len = u32::lift(cx, InterfaceType::U32, next(src))? as usize;
                load_list(cx, i, ptr, len)?
            }
            InterfaceType::Record(i) => Val::Record(Record {
                ty: types::Record::from(i, &cx.instance_type()),
                values: cx.types[i]
                    .fields
                    .iter()
                    .map(|field| Self::lift(cx, field.ty, src))
                    .collect::<Result<_>>()?,
            }),
            InterfaceType::Tuple(i) => Val::Tuple(Tuple {
                ty: types::Tuple::from(i, &cx.instance_type()),
                values: cx.types[i]
                    .types
                    .iter()
                    .map(|ty| Self::lift(cx, *ty, src))
                    .collect::<Result<_>>()?,
            }),
            InterfaceType::Variant(i) => {
                let (discriminant, value) = lift_variant(
                    cx,
                    cx.types.canonical_abi(&ty).flat_count(usize::MAX).unwrap(),
                    cx.types[i].cases.iter().map(|case| case.ty),
                    src,
                )?;

                Val::Variant(Variant {
                    ty: types::Variant::from(i, &cx.instance_type()),
                    discriminant,
                    value,
                })
            }
            InterfaceType::Enum(i) => {
                let (discriminant, _) = lift_variant(
                    cx,
                    cx.types.canonical_abi(&ty).flat_count(usize::MAX).unwrap(),
                    cx.types[i].names.iter().map(|_| None),
                    src,
                )?;

                Val::Enum(Enum {
                    ty: types::Enum::from(i, &cx.instance_type()),
                    discriminant,
                })
            }
            InterfaceType::Union(i) => {
                let (discriminant, value) = lift_variant(
                    cx,
                    cx.types.canonical_abi(&ty).flat_count(usize::MAX).unwrap(),
                    cx.types[i].types.iter().copied().map(Some),
                    src,
                )?;

                Val::Union(Union {
                    ty: types::Union::from(i, &cx.instance_type()),
                    discriminant,
                    value,
                })
            }
            InterfaceType::Option(i) => {
                let (discriminant, value) = lift_variant(
                    cx,
                    cx.types.canonical_abi(&ty).flat_count(usize::MAX).unwrap(),
                    [None, Some(cx.types[i].ty)].into_iter(),
                    src,
                )?;

                Val::Option(OptionVal {
                    ty: types::OptionType::from(i, &cx.instance_type()),
                    discriminant,
                    value,
                })
            }
            InterfaceType::Result(i) => {
                let result_ty = &cx.types[i];
                let (discriminant, value) = lift_variant(
                    cx,
                    cx.types.canonical_abi(&ty).flat_count(usize::MAX).unwrap(),
                    [result_ty.ok, result_ty.err].into_iter(),
                    src,
                )?;

                Val::Result(ResultVal {
                    ty: types::ResultType::from(i, &cx.instance_type()),
                    discriminant,
                    value,
                })
            }
            InterfaceType::Flags(i) => {
                let count = u32::try_from(cx.types[i].names.len()).unwrap();
                let u32_count = cx.types.canonical_abi(&ty).flat_count(usize::MAX).unwrap();
                let value = iter::repeat_with(|| u32::lift(cx, InterfaceType::U32, next(src)))
                    .take(u32_count)
                    .collect::<Result<_>>()?;

                Val::Flags(Flags {
                    ty: types::Flags::from(i, &cx.instance_type()),
                    count,
                    value,
                })
            }
        })
    }

    /// Deserialize a value of this type from the heap.
    pub(crate) fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Val> {
        Ok(match ty {
            InterfaceType::Bool => Val::Bool(bool::load(cx, ty, bytes)?),
            InterfaceType::S8 => Val::S8(i8::load(cx, ty, bytes)?),
            InterfaceType::U8 => Val::U8(u8::load(cx, ty, bytes)?),
            InterfaceType::S16 => Val::S16(i16::load(cx, ty, bytes)?),
            InterfaceType::U16 => Val::U16(u16::load(cx, ty, bytes)?),
            InterfaceType::S32 => Val::S32(i32::load(cx, ty, bytes)?),
            InterfaceType::U32 => Val::U32(u32::load(cx, ty, bytes)?),
            InterfaceType::S64 => Val::S64(i64::load(cx, ty, bytes)?),
            InterfaceType::U64 => Val::U64(u64::load(cx, ty, bytes)?),
            InterfaceType::Float32 => Val::Float32(f32::load(cx, ty, bytes)?),
            InterfaceType::Float64 => Val::Float64(f64::load(cx, ty, bytes)?),
            InterfaceType::Char => Val::Char(char::load(cx, ty, bytes)?),
            InterfaceType::String => Val::String(<Box<str>>::load(cx, ty, bytes)?),
            InterfaceType::Own(_) | InterfaceType::Borrow(_) => {
                Val::Resource(ResourceAny::load(cx, ty, bytes)?)
            }
            InterfaceType::List(i) => {
                // FIXME: needs memory64 treatment
                let ptr = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
                let len = u32::from_le_bytes(bytes[4..].try_into().unwrap()) as usize;
                load_list(cx, i, ptr, len)?
            }

            InterfaceType::Record(i) => Val::Record(Record {
                ty: types::Record::from(i, &cx.instance_type()),
                values: load_record(cx, cx.types[i].fields.iter().map(|field| field.ty), bytes)?,
            }),
            InterfaceType::Tuple(i) => Val::Tuple(Tuple {
                ty: types::Tuple::from(i, &cx.instance_type()),
                values: load_record(cx, cx.types[i].types.iter().copied(), bytes)?,
            }),
            InterfaceType::Variant(i) => {
                let ty = &cx.types[i];
                let (discriminant, value) =
                    load_variant(cx, &ty.info, ty.cases.iter().map(|case| case.ty), bytes)?;

                Val::Variant(Variant {
                    ty: types::Variant::from(i, &cx.instance_type()),
                    discriminant,
                    value,
                })
            }
            InterfaceType::Enum(i) => {
                let ty = &cx.types[i];
                let (discriminant, _) =
                    load_variant(cx, &ty.info, ty.names.iter().map(|_| None), bytes)?;

                Val::Enum(Enum {
                    ty: types::Enum::from(i, &cx.instance_type()),
                    discriminant,
                })
            }
            InterfaceType::Union(i) => {
                let ty = &cx.types[i];
                let (discriminant, value) =
                    load_variant(cx, &ty.info, ty.types.iter().copied().map(Some), bytes)?;

                Val::Union(Union {
                    ty: types::Union::from(i, &cx.instance_type()),
                    discriminant,
                    value,
                })
            }
            InterfaceType::Option(i) => {
                let ty = &cx.types[i];
                let (discriminant, value) =
                    load_variant(cx, &ty.info, [None, Some(ty.ty)].into_iter(), bytes)?;

                Val::Option(OptionVal {
                    ty: types::OptionType::from(i, &cx.instance_type()),
                    discriminant,
                    value,
                })
            }
            InterfaceType::Result(i) => {
                let ty = &cx.types[i];
                let (discriminant, value) =
                    load_variant(cx, &ty.info, [ty.ok, ty.err].into_iter(), bytes)?;

                Val::Result(ResultVal {
                    ty: types::ResultType::from(i, &cx.instance_type()),
                    discriminant,
                    value,
                })
            }
            InterfaceType::Flags(i) => Val::Flags(Flags {
                ty: types::Flags::from(i, &cx.instance_type()),
                count: u32::try_from(cx.types[i].names.len())?,
                value: match FlagsSize::from_count(cx.types[i].names.len()) {
                    FlagsSize::Size0 => Box::new([]),
                    FlagsSize::Size1 => {
                        iter::once(u8::load(cx, InterfaceType::U8, bytes)?.into()).collect()
                    }
                    FlagsSize::Size2 => {
                        iter::once(u16::load(cx, InterfaceType::U16, bytes)?.into()).collect()
                    }
                    FlagsSize::Size4Plus(n) => (0..n)
                        .map(|index| {
                            u32::load(
                                cx,
                                InterfaceType::U32,
                                &bytes[usize::from(index) * 4..][..4],
                            )
                        })
                        .collect::<Result<_>>()?,
                },
            }),
        })
    }

    /// Serialize this value as core Wasm stack values.
    pub(crate) fn lower<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        dst: &mut std::slice::IterMut<'_, MaybeUninit<ValRaw>>,
    ) -> Result<()> {
        match self {
            Val::Bool(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::S8(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::U8(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::S16(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::U16(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::S32(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::U32(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::S64(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::U64(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::Float32(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::Float64(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::Char(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::Resource(value) => value.lower(cx, ty, next_mut(dst))?,
            Val::String(value) => {
                let my_dst = &mut MaybeUninit::<[ValRaw; 2]>::uninit();
                value.lower(cx, ty, my_dst)?;
                let my_dst = unsafe { my_dst.assume_init() };
                next_mut(dst).write(my_dst[0]);
                next_mut(dst).write(my_dst[1]);
            }
            Val::List(List { values, .. }) => {
                let ty = match ty {
                    InterfaceType::List(i) => &cx.types[i],
                    _ => bad_type_info(),
                };
                let (ptr, len) = lower_list(cx, ty.element, values)?;
                next_mut(dst).write(ValRaw::i64(ptr as i64));
                next_mut(dst).write(ValRaw::i64(len as i64));
            }
            Val::Record(Record { values, .. }) => {
                let ty = match ty {
                    InterfaceType::Record(i) => &cx.types[i],
                    _ => bad_type_info(),
                };
                for (value, field) in values.iter().zip(ty.fields.iter()) {
                    value.lower(cx, field.ty, dst)?;
                }
            }
            Val::Tuple(Tuple { values, .. }) => {
                let ty = match ty {
                    InterfaceType::Tuple(i) => &cx.types[i],
                    _ => bad_type_info(),
                };
                for (value, ty) in values.iter().zip(ty.types.iter()) {
                    value.lower(cx, *ty, dst)?;
                }
            }
            Val::Variant(v) => v.as_generic(cx.types, ty).lower(cx, dst)?,
            Val::Union(v) => v.as_generic(cx.types, ty).lower(cx, dst)?,
            Val::Option(v) => v.as_generic(cx.types, ty).lower(cx, dst)?,
            Val::Result(v) => v.as_generic(cx.types, ty).lower(cx, dst)?,
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
    pub(crate) fn store<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        ty: InterfaceType,
        offset: usize,
    ) -> Result<()> {
        debug_assert!(offset % usize::try_from(cx.types.canonical_abi(&ty).align32)? == 0);

        match self {
            Val::Bool(value) => value.store(cx, ty, offset)?,
            Val::U8(value) => value.store(cx, ty, offset)?,
            Val::S8(value) => value.store(cx, ty, offset)?,
            Val::U16(value) => value.store(cx, ty, offset)?,
            Val::S16(value) => value.store(cx, ty, offset)?,
            Val::U32(value) => value.store(cx, ty, offset)?,
            Val::S32(value) => value.store(cx, ty, offset)?,
            Val::U64(value) => value.store(cx, ty, offset)?,
            Val::S64(value) => value.store(cx, ty, offset)?,
            Val::Float32(value) => value.store(cx, ty, offset)?,
            Val::Float64(value) => value.store(cx, ty, offset)?,
            Val::Char(value) => value.store(cx, ty, offset)?,
            Val::String(value) => value.store(cx, ty, offset)?,
            Val::Resource(value) => value.store(cx, ty, offset)?,
            Val::List(List { values, .. }) => {
                let ty = match ty {
                    InterfaceType::List(i) => &cx.types[i],
                    _ => bad_type_info(),
                };
                let (ptr, len) = lower_list(cx, ty.element, values)?;
                // FIXME: needs memory64 handling
                *cx.get(offset + 0) = (ptr as i32).to_le_bytes();
                *cx.get(offset + 4) = (len as i32).to_le_bytes();
            }
            Val::Record(Record { values, .. }) => {
                let ty = match ty {
                    InterfaceType::Record(i) => &cx.types[i],
                    _ => bad_type_info(),
                };
                let mut offset = offset;
                for (value, field) in values.iter().zip(ty.fields.iter()) {
                    value.store(
                        cx,
                        field.ty,
                        cx.types
                            .canonical_abi(&field.ty)
                            .next_field32_size(&mut offset),
                    )?;
                }
            }
            Val::Tuple(Tuple { values, .. }) => {
                let ty = match ty {
                    InterfaceType::Tuple(i) => &cx.types[i],
                    _ => bad_type_info(),
                };
                let mut offset = offset;
                for (value, ty) in values.iter().zip(ty.types.iter()) {
                    value.store(
                        cx,
                        *ty,
                        cx.types.canonical_abi(ty).next_field32_size(&mut offset),
                    )?;
                }
            }

            Val::Variant(v) => v.as_generic(cx.types, ty).store(cx, offset)?,
            Val::Enum(v) => v.as_generic(cx.types, ty).store(cx, offset)?,
            Val::Union(v) => v.as_generic(cx.types, ty).store(cx, offset)?,
            Val::Option(v) => v.as_generic(cx.types, ty).store(cx, offset)?,
            Val::Result(v) => v.as_generic(cx.types, ty).store(cx, offset)?,

            Val::Flags(Flags { count, value, .. }) => {
                match FlagsSize::from_count(*count as usize) {
                    FlagsSize::Size0 => {}
                    FlagsSize::Size1 => {
                        u8::try_from(value[0])
                            .unwrap()
                            .store(cx, InterfaceType::U8, offset)?
                    }
                    FlagsSize::Size2 => {
                        u16::try_from(value[0])
                            .unwrap()
                            .store(cx, InterfaceType::U16, offset)?
                    }
                    FlagsSize::Size4Plus(_) => {
                        let mut offset = offset;
                        for value in value.deref() {
                            value.store(cx, InterfaceType::U32, offset)?;
                            offset += 4;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl PartialEq for Val {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // IEEE 754 equality considers NaN inequal to NaN and negative zero
            // equal to positive zero, however we do the opposite here, because
            // this logic is used by testing and fuzzing, which want to know
            // whether two values are semantically the same, rather than
            // numerically equal.
            (Self::Float32(l), Self::Float32(r)) => {
                (*l != 0.0 && l == r)
                    || (*l == 0.0 && l.to_bits() == r.to_bits())
                    || (l.is_nan() && r.is_nan())
            }
            (Self::Float32(_), _) => false,
            (Self::Float64(l), Self::Float64(r)) => {
                (*l != 0.0 && l == r)
                    || (*l == 0.0 && l.to_bits() == r.to_bits())
                    || (l.is_nan() && r.is_nan())
            }
            (Self::Float64(_), _) => false,

            (Self::Bool(l), Self::Bool(r)) => l == r,
            (Self::Bool(_), _) => false,
            (Self::S8(l), Self::S8(r)) => l == r,
            (Self::S8(_), _) => false,
            (Self::U8(l), Self::U8(r)) => l == r,
            (Self::U8(_), _) => false,
            (Self::S16(l), Self::S16(r)) => l == r,
            (Self::S16(_), _) => false,
            (Self::U16(l), Self::U16(r)) => l == r,
            (Self::U16(_), _) => false,
            (Self::S32(l), Self::S32(r)) => l == r,
            (Self::S32(_), _) => false,
            (Self::U32(l), Self::U32(r)) => l == r,
            (Self::U32(_), _) => false,
            (Self::S64(l), Self::S64(r)) => l == r,
            (Self::S64(_), _) => false,
            (Self::U64(l), Self::U64(r)) => l == r,
            (Self::U64(_), _) => false,
            (Self::Char(l), Self::Char(r)) => l == r,
            (Self::Char(_), _) => false,
            (Self::String(l), Self::String(r)) => l == r,
            (Self::String(_), _) => false,
            (Self::List(l), Self::List(r)) => l == r,
            (Self::List(_), _) => false,
            (Self::Record(l), Self::Record(r)) => l == r,
            (Self::Record(_), _) => false,
            (Self::Tuple(l), Self::Tuple(r)) => l == r,
            (Self::Tuple(_), _) => false,
            (Self::Variant(l), Self::Variant(r)) => l == r,
            (Self::Variant(_), _) => false,
            (Self::Enum(l), Self::Enum(r)) => l == r,
            (Self::Enum(_), _) => false,
            (Self::Union(l), Self::Union(r)) => l == r,
            (Self::Union(_), _) => false,
            (Self::Option(l), Self::Option(r)) => l == r,
            (Self::Option(_), _) => false,
            (Self::Result(l), Self::Result(r)) => l == r,
            (Self::Result(_), _) => false,
            (Self::Flags(l), Self::Flags(r)) => l == r,
            (Self::Flags(_), _) => false,
            (Self::Resource(l), Self::Resource(r)) => l == r,
            (Self::Resource(_), _) => false,
        }
    }
}

impl Eq for Val {}

struct GenericVariant<'a> {
    discriminant: u32,
    payload: Option<(&'a Val, InterfaceType)>,
    abi: &'a CanonicalAbiInfo,
    info: &'a VariantInfo,
}

impl GenericVariant<'_> {
    fn lower<T>(
        &self,
        cx: &mut LowerContext<'_, T>,
        dst: &mut std::slice::IterMut<'_, MaybeUninit<ValRaw>>,
    ) -> Result<()> {
        next_mut(dst).write(ValRaw::u32(self.discriminant));

        // For the remaining lowered representation of this variant that
        // the payload didn't write we write out zeros here to ensure
        // the entire variant is written.
        let value_flat = match self.payload {
            Some((value, ty)) => {
                value.lower(cx, ty, dst)?;
                cx.types.canonical_abi(&ty).flat_count(usize::MAX).unwrap()
            }
            None => 0,
        };
        let variant_flat = self.abi.flat_count(usize::MAX).unwrap();
        for _ in (1 + value_flat)..variant_flat {
            next_mut(dst).write(ValRaw::u64(0));
        }
        Ok(())
    }

    fn store<T>(&self, cx: &mut LowerContext<'_, T>, offset: usize) -> Result<()> {
        match self.info.size {
            DiscriminantSize::Size1 => {
                u8::try_from(self.discriminant)
                    .unwrap()
                    .store(cx, InterfaceType::U8, offset)?
            }
            DiscriminantSize::Size2 => {
                u16::try_from(self.discriminant)
                    .unwrap()
                    .store(cx, InterfaceType::U16, offset)?
            }
            DiscriminantSize::Size4 => self.discriminant.store(cx, InterfaceType::U32, offset)?,
        }

        if let Some((value, ty)) = self.payload {
            let offset = offset + usize::try_from(self.info.payload_offset32).unwrap();
            value.store(cx, ty, offset)?;
        }

        Ok(())
    }
}

fn load_list(cx: &mut LiftContext<'_>, ty: TypeListIndex, ptr: usize, len: usize) -> Result<Val> {
    let elem = cx.types[ty].element;
    let abi = cx.types.canonical_abi(&elem);
    let element_size = usize::try_from(abi.size32).unwrap();
    let element_alignment = abi.align32;

    match len
        .checked_mul(element_size)
        .and_then(|len| ptr.checked_add(len))
    {
        Some(n) if n <= cx.memory().len() => {}
        _ => bail!("list pointer/length out of bounds of memory"),
    }
    if ptr % usize::try_from(element_alignment)? != 0 {
        bail!("list pointer is not aligned")
    }

    Ok(Val::List(List {
        ty: types::List::from(ty, &cx.instance_type()),
        values: (0..len)
            .map(|index| {
                Val::load(
                    cx,
                    elem,
                    &cx.memory()[ptr + (index * element_size)..][..element_size],
                )
            })
            .collect::<Result<_>>()?,
    }))
}

fn load_record(
    cx: &mut LiftContext<'_>,
    types: impl Iterator<Item = InterfaceType>,
    bytes: &[u8],
) -> Result<Box<[Val]>> {
    let mut offset = 0;
    types
        .map(|ty| {
            let abi = cx.types.canonical_abi(&ty);
            let offset = abi.next_field32(&mut offset);
            let offset = usize::try_from(offset).unwrap();
            let size = usize::try_from(abi.size32).unwrap();
            Val::load(cx, ty, &bytes[offset..][..size])
        })
        .collect()
}

fn load_variant(
    cx: &mut LiftContext<'_>,
    info: &VariantInfo,
    mut types: impl ExactSizeIterator<Item = Option<InterfaceType>>,
    bytes: &[u8],
) -> Result<(u32, Option<Box<Val>>)> {
    let discriminant = match info.size {
        DiscriminantSize::Size1 => u32::from(u8::load(cx, InterfaceType::U8, &bytes[..1])?),
        DiscriminantSize::Size2 => u32::from(u16::load(cx, InterfaceType::U16, &bytes[..2])?),
        DiscriminantSize::Size4 => u32::load(cx, InterfaceType::U32, &bytes[..4])?,
    };
    let case_ty = types.nth(discriminant as usize).ok_or_else(|| {
        anyhow!(
            "discriminant {} out of range [0..{})",
            discriminant,
            types.len()
        )
    })?;
    let value = match case_ty {
        Some(case_ty) => {
            let payload_offset = usize::try_from(info.payload_offset32).unwrap();
            let case_abi = cx.types.canonical_abi(&case_ty);
            let case_size = usize::try_from(case_abi.size32).unwrap();
            Some(Box::new(Val::load(
                cx,
                case_ty,
                &bytes[payload_offset..][..case_size],
            )?))
        }
        None => None,
    };
    Ok((discriminant, value))
}

fn lift_variant(
    cx: &mut LiftContext<'_>,
    flatten_count: usize,
    mut types: impl ExactSizeIterator<Item = Option<InterfaceType>>,
    src: &mut std::slice::Iter<'_, ValRaw>,
) -> Result<(u32, Option<Box<Val>>)> {
    let len = types.len();
    let discriminant = next(src).get_u32();
    let ty = types
        .nth(discriminant as usize)
        .ok_or_else(|| anyhow!("discriminant {} out of range [0..{})", discriminant, len))?;
    let (value, value_flat) = match ty {
        Some(ty) => (
            Some(Box::new(Val::lift(cx, ty, src)?)),
            cx.types.canonical_abi(&ty).flat_count(usize::MAX).unwrap(),
        ),
        None => (None, 0),
    };
    for _ in (1 + value_flat)..flatten_count {
        next(src);
    }
    Ok((discriminant, value))
}

/// Lower a list with the specified element type and values.
fn lower_list<T>(
    cx: &mut LowerContext<'_, T>,
    element_type: InterfaceType,
    items: &[Val],
) -> Result<(usize, usize)> {
    let abi = cx.types.canonical_abi(&element_type);
    let elt_size = usize::try_from(abi.size32)?;
    let elt_align = abi.align32;
    let size = items
        .len()
        .checked_mul(elt_size)
        .ok_or_else(|| anyhow::anyhow!("size overflow copying a list"))?;
    let ptr = cx.realloc(0, 0, elt_align, size)?;
    let mut element_ptr = ptr;
    for item in items {
        item.store(cx, element_type, element_ptr)?;
        element_ptr += elt_size;
    }
    Ok((ptr, items.len()))
}

fn next<'a>(src: &mut std::slice::Iter<'a, ValRaw>) -> &'a ValRaw {
    src.next().unwrap()
}

fn next_mut<'a>(
    dst: &mut std::slice::IterMut<'a, MaybeUninit<ValRaw>>,
) -> &'a mut MaybeUninit<ValRaw> {
    dst.next().unwrap()
}

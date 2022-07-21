use crate::component::func::{self, Lower, MemoryMut, Options};
use crate::component::types::{self, SizeAndAlignment, Type};
use crate::{AsContextMut, StoreContextMut, ValRaw};
use anyhow::Result;
use std::mem::MaybeUninit;
use std::ops::Deref;
use wasmtime_component_util::{DiscriminantSize, FlagsSize};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct List {
    pub(crate) ty: types::List,
    pub(crate) values: Box<[Val]>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Record {
    pub(crate) ty: types::Record,
    pub(crate) values: Box<[Val]>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Tuple {
    pub(crate) ty: types::Tuple,
    pub(crate) values: Box<[Val]>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Variant {
    pub(crate) ty: types::Variant,
    pub(crate) discriminant: u32,
    pub(crate) value: Box<Val>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Enum {
    pub(crate) ty: types::Enum,
    pub(crate) discriminant: u32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Union {
    pub(crate) ty: types::Union,
    pub(crate) discriminant: u32,
    pub(crate) value: Box<Val>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Option {
    pub(crate) ty: types::Option,
    pub(crate) discriminant: u32,
    pub(crate) value: Box<Val>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Expected {
    pub(crate) ty: types::Expected,
    pub(crate) discriminant: u32,
    pub(crate) value: Box<Val>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Flags {
    pub(crate) ty: types::Flags,
    pub(crate) count: u32,
    pub(crate) value: Box<[u32]>,
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

    /// Serialize this value as core Wasm stack values.
    pub(crate) fn lower<T>(
        &self,
        store: &mut StoreContextMut<T>,
        options: &Options,
        dst: &mut std::slice::IterMut<'_, MaybeUninit<ValRaw>>,
    ) -> Result<()> {
        match self {
            Val::Unit => (),
            Val::Bool(value) => value.lower(store, options, next(dst))?,
            Val::S8(value) => value.lower(store, options, next(dst))?,
            Val::U8(value) => value.lower(store, options, next(dst))?,
            Val::S16(value) => value.lower(store, options, next(dst))?,
            Val::U16(value) => value.lower(store, options, next(dst))?,
            Val::S32(value) => value.lower(store, options, next(dst))?,
            Val::U32(value) => value.lower(store, options, next(dst))?,
            Val::S64(value) => value.lower(store, options, next(dst))?,
            Val::U64(value) => value.lower(store, options, next(dst))?,
            Val::Float32(value) => value.lower(store, options, next(dst))?,
            Val::Float64(value) => value.lower(store, options, next(dst))?,
            Val::Char(value) => value.lower(store, options, next(dst))?,
            Val::String(value) => {
                let my_dst = &mut MaybeUninit::<[ValRaw; 2]>::uninit();
                value.lower(store, options, my_dst)?;
                let my_dst = unsafe { my_dst.assume_init() };
                next(dst).write(my_dst[0]);
                next(dst).write(my_dst[1]);
            }
            Val::List(List { values, ty }) => {
                let (ptr, len) = lower_list(
                    &ty.ty(),
                    &mut MemoryMut::new(store.as_context_mut(), options),
                    values,
                )?;
                next(dst).write(ValRaw::i64(ptr as i64));
                next(dst).write(ValRaw::i64(len as i64));
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
                next(dst).write(ValRaw::u32(*discriminant));
                value.lower(store, options, dst)?;
                for _ in (1 + value.ty().flatten_count())..self.ty().flatten_count() {
                    next(dst).write(ValRaw::u32(0));
                }
            }
            Val::Enum(Enum { discriminant, .. }) => {
                next(dst).write(ValRaw::u32(*discriminant));
            }
            Val::Flags(Flags { value, .. }) => {
                for value in value.deref() {
                    next(dst).write(ValRaw::u32(*value));
                }
            }
        }

        Ok(())
    }

    /// Serialize this value to the heap at the specified memory location.
    pub(crate) fn store<T>(&self, mem: &mut MemoryMut<'_, T>, offset: usize) -> Result<()> {
        debug_assert!(offset % usize::try_from(self.ty().size_and_alignment().alignment)? == 0);

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
                    value.store(mem, value.ty().next_field(&mut offset))?;
                }
            }
            Val::Variant(Variant {
                discriminant,
                value,
                ty,
            }) => self.store_variant(*discriminant, value, ty.cases().len(), mem, offset)?,

            Val::Enum(Enum { discriminant, ty }) => {
                self.store_variant(*discriminant, &Val::Unit, ty.names().len(), mem, offset)?
            }

            Val::Union(Union {
                discriminant,
                value,
                ty,
            }) => self.store_variant(*discriminant, value, ty.types().len(), mem, offset)?,

            Val::Option(Option {
                discriminant,
                value,
                ..
            })
            | Val::Expected(Expected {
                discriminant,
                value,
                ..
            }) => self.store_variant(*discriminant, value, 2, mem, offset)?,

            Val::Flags(Flags { count, value, .. }) => {
                match FlagsSize::from_count(*count as usize) {
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
        case_count: usize,
        mem: &mut MemoryMut<'_, T>,
        offset: usize,
    ) -> Result<()> {
        let discriminant_size = DiscriminantSize::from_count(case_count).unwrap();
        match discriminant_size {
            DiscriminantSize::Size1 => u8::try_from(discriminant).unwrap().store(mem, offset)?,
            DiscriminantSize::Size2 => u16::try_from(discriminant).unwrap().store(mem, offset)?,
            DiscriminantSize::Size4 => (discriminant).store(mem, offset)?,
        }

        value.store(
            mem,
            offset
                + func::align_to(
                    discriminant_size.into(),
                    self.ty().size_and_alignment().alignment,
                ),
        )
    }
}

/// Lower a list with the specified element type and values.
fn lower_list<T>(
    element_type: &Type,
    mem: &mut MemoryMut<'_, T>,
    items: &[Val],
) -> Result<(usize, usize)> {
    let SizeAndAlignment {
        size: element_size,
        alignment: element_alignment,
    } = element_type.size_and_alignment();
    let size = items
        .len()
        .checked_mul(element_size)
        .ok_or_else(|| anyhow::anyhow!("size overflow copying a list"))?;
    let ptr = mem.realloc(0, 0, element_alignment, size)?;
    let mut element_ptr = ptr;
    for item in items {
        item.store(mem, element_ptr)?;
        element_ptr += element_size;
    }
    Ok((ptr, items.len()))
}

/// Calculate the size of a u32 array needed to represent the specified number of bit flags.
///
/// Note that this will always return at least 1, even if the `count` parameter is zero.
pub(crate) fn u32_count_for_flag_count(count: usize) -> usize {
    match FlagsSize::from_count(count) {
        FlagsSize::Size1 | FlagsSize::Size2 => 1,
        FlagsSize::Size4Plus(n) => n,
    }
}

fn next<'a>(dst: &mut std::slice::IterMut<'a, MaybeUninit<ValRaw>>) -> &'a mut MaybeUninit<ValRaw> {
    dst.next().unwrap()
}

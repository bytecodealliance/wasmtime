use anyhow::{bail, Result};
use std::fmt::{self, Display};
use wasmtime_environ::{
    EngineOrModuleTypeIndex, EntityType, Global, Memory, ModuleTypes, Table, TypeTrace,
    VMSharedTypeIndex, WasmArrayType, WasmCompositeType, WasmFieldType, WasmFuncType, WasmHeapType,
    WasmRefType, WasmStorageType, WasmSubType, WasmValType,
};

use crate::{type_registry::RegisteredType, Engine};

pub(crate) mod matching;

// Type Representations

// Type attributes

/// Indicator of whether a global is mutable or not
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Mutability {
    /// The global is constant and its value does not change
    Const,
    /// The value of the global can change over time
    Var,
}

// Value Types

/// A list of all possible value types in WebAssembly.
///
/// # Subtyping and Equality
///
/// `ValType` does not implement `Eq`, because reference types have a subtyping
/// relationship, and so 99.99% of the time you actually want to check whether
/// one type matches (i.e. is a subtype of) another type. You can use the
/// [`ValType::matches`] and [`Val::matches_ty`][crate::Val::matches_ty] methods
/// to perform these types of checks. If, however, you are in that 0.01%
/// scenario where you need to check precise equality between types, you can use
/// the [`ValType::eq`] method.
#[derive(Clone, Hash)]
pub enum ValType {
    // NB: the ordering of variants here is intended to match the ordering in
    // `wasmtime_types::WasmType` to help improve codegen when converting.
    //
    /// Signed 32 bit integer.
    I32,
    /// Signed 64 bit integer.
    I64,
    /// Floating point 32 bit integer.
    F32,
    /// Floating point 64 bit integer.
    F64,
    /// A 128 bit number.
    V128,
    /// An opaque reference to some type on the heap.
    Ref(RefType),
}

impl fmt::Debug for ValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Display for ValType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValType::I32 => write!(f, "i32"),
            ValType::I64 => write!(f, "i64"),
            ValType::F32 => write!(f, "f32"),
            ValType::F64 => write!(f, "f64"),
            ValType::V128 => write!(f, "v128"),
            ValType::Ref(r) => Display::fmt(r, f),
        }
    }
}

impl From<RefType> for ValType {
    #[inline]
    fn from(r: RefType) -> Self {
        ValType::Ref(r)
    }
}

impl ValType {
    /// The `externref` type, aka `(ref null extern)`.
    pub const EXTERNREF: Self = ValType::Ref(RefType::EXTERNREF);

    /// The `funcref` type, aka `(ref null func)`.
    pub const FUNCREF: Self = ValType::Ref(RefType::FUNCREF);

    /// The `nullfuncref` type, aka `(ref null nofunc)`.
    pub const NULLFUNCREF: Self = ValType::Ref(RefType::NULLFUNCREF);

    /// The `anyref` type, aka `(ref null any)`.
    pub const ANYREF: Self = ValType::Ref(RefType::ANYREF);

    /// The `i31ref` type, aka `(ref null i31)`.
    pub const I31REF: Self = ValType::Ref(RefType::I31REF);

    /// The `nullref` type, aka `(ref null none)`.
    pub const NULLREF: Self = ValType::Ref(RefType::NULLREF);

    /// Returns true if `ValType` matches any of the numeric types. (e.g. `I32`,
    /// `I64`, `F32`, `F64`).
    #[inline]
    pub fn is_num(&self) -> bool {
        match self {
            ValType::I32 | ValType::I64 | ValType::F32 | ValType::F64 => true,
            _ => false,
        }
    }

    /// Is this the `i32` type?
    #[inline]
    pub fn is_i32(&self) -> bool {
        matches!(self, ValType::I32)
    }

    /// Is this the `i64` type?
    #[inline]
    pub fn is_i64(&self) -> bool {
        matches!(self, ValType::I64)
    }

    /// Is this the `f32` type?
    #[inline]
    pub fn is_f32(&self) -> bool {
        matches!(self, ValType::F32)
    }

    /// Is this the `f64` type?
    #[inline]
    pub fn is_f64(&self) -> bool {
        matches!(self, ValType::F64)
    }

    /// Is this the `v128` type?
    #[inline]
    pub fn is_v128(&self) -> bool {
        matches!(self, ValType::V128)
    }

    /// Returns true if `ValType` is any kind of reference type.
    #[inline]
    pub fn is_ref(&self) -> bool {
        matches!(self, ValType::Ref(_))
    }

    /// Is this the `funcref` (aka `(ref null func)`) type?
    #[inline]
    pub fn is_funcref(&self) -> bool {
        matches!(
            self,
            ValType::Ref(RefType {
                is_nullable: true,
                heap_type: HeapType::Func
            })
        )
    }

    /// Is this the `externref` (aka `(ref null extern)`) type?
    #[inline]
    pub fn is_externref(&self) -> bool {
        matches!(
            self,
            ValType::Ref(RefType {
                is_nullable: true,
                heap_type: HeapType::Extern
            })
        )
    }

    /// Is this the `anyref` (aka `(ref null any)`) type?
    #[inline]
    pub fn is_anyref(&self) -> bool {
        matches!(
            self,
            ValType::Ref(RefType {
                is_nullable: true,
                heap_type: HeapType::Any
            })
        )
    }

    /// Get the underlying reference type, if this value type is a reference
    /// type.
    #[inline]
    pub fn as_ref(&self) -> Option<&RefType> {
        match self {
            ValType::Ref(r) => Some(r),
            _ => None,
        }
    }

    /// Get the underlying reference type, panicking if this value type is not a
    /// reference type.
    #[inline]
    pub fn unwrap_ref(&self) -> &RefType {
        self.as_ref()
            .expect("ValType::unwrap_ref on a non-reference type")
    }

    /// Does this value type match the other type?
    ///
    /// That is, is this value type a subtype of the other?
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn matches(&self, other: &ValType) -> bool {
        match (self, other) {
            (Self::I32, Self::I32) => true,
            (Self::I64, Self::I64) => true,
            (Self::F32, Self::F32) => true,
            (Self::F64, Self::F64) => true,
            (Self::V128, Self::V128) => true,
            (Self::Ref(a), Self::Ref(b)) => a.matches(b),
            (Self::I32, _)
            | (Self::I64, _)
            | (Self::F32, _)
            | (Self::F64, _)
            | (Self::V128, _)
            | (Self::Ref(_), _) => false,
        }
    }

    /// Is value type `a` precisely equal to value type `b`?
    ///
    /// Returns `false` even if `a` is a subtype of `b` or vice versa, if they
    /// are not exactly the same value type.
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine.
    pub fn eq(a: &Self, b: &Self) -> bool {
        a.matches(b) && b.matches(a)
    }

    pub(crate) fn ensure_matches(&self, engine: &Engine, other: &ValType) -> Result<()> {
        if !self.comes_from_same_engine(engine) || !other.comes_from_same_engine(engine) {
            bail!("type used with wrong engine");
        }
        if self.matches(other) {
            Ok(())
        } else {
            bail!("type mismatch: expected {other}, found {self}")
        }
    }

    pub(crate) fn comes_from_same_engine(&self, engine: &Engine) -> bool {
        match self {
            Self::I32 | Self::I64 | Self::F32 | Self::F64 | Self::V128 => true,
            Self::Ref(r) => r.comes_from_same_engine(engine),
        }
    }

    pub(crate) fn to_wasm_type(&self) -> WasmValType {
        match self {
            Self::I32 => WasmValType::I32,
            Self::I64 => WasmValType::I64,
            Self::F32 => WasmValType::F32,
            Self::F64 => WasmValType::F64,
            Self::V128 => WasmValType::V128,
            Self::Ref(r) => WasmValType::Ref(r.to_wasm_type()),
        }
    }

    #[inline]
    pub(crate) fn from_wasm_type(engine: &Engine, ty: &WasmValType) -> Self {
        match ty {
            WasmValType::I32 => Self::I32,
            WasmValType::I64 => Self::I64,
            WasmValType::F32 => Self::F32,
            WasmValType::F64 => Self::F64,
            WasmValType::V128 => Self::V128,
            WasmValType::Ref(r) => Self::Ref(RefType::from_wasm_type(engine, r)),
        }
    }
}

/// Opaque references to data in the Wasm heap or to host data.
///
/// # Subtyping and Equality
///
/// `RefType` does not implement `Eq`, because reference types have a subtyping
/// relationship, and so 99.99% of the time you actually want to check whether
/// one type matches (i.e. is a subtype of) another type. You can use the
/// [`RefType::matches`] and [`Ref::matches_ty`][crate::Ref::matches_ty] methods
/// to perform these types of checks. If, however, you are in that 0.01%
/// scenario where you need to check precise equality between types, you can use
/// the [`RefType::eq`] method.
#[derive(Clone, Hash)]
pub struct RefType {
    is_nullable: bool,
    heap_type: HeapType,
}

impl fmt::Debug for RefType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl fmt::Display for RefType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(ref ")?;
        if self.is_nullable() {
            write!(f, "null ")?;
        }
        write!(f, "{})", self.heap_type())
    }
}

impl RefType {
    /// The `externref` type, aka `(ref null extern)`.
    pub const EXTERNREF: Self = RefType {
        is_nullable: true,
        heap_type: HeapType::Extern,
    };

    /// The `funcref` type, aka `(ref null func)`.
    pub const FUNCREF: Self = RefType {
        is_nullable: true,
        heap_type: HeapType::Func,
    };

    /// The `nullfuncref` type, aka `(ref null nofunc)`.
    pub const NULLFUNCREF: Self = RefType {
        is_nullable: true,
        heap_type: HeapType::NoFunc,
    };

    /// The `anyref` type, aka `(ref null any)`.
    pub const ANYREF: Self = RefType {
        is_nullable: true,
        heap_type: HeapType::Any,
    };

    /// The `i31ref` type, aka `(ref null i31)`.
    pub const I31REF: Self = RefType {
        is_nullable: true,
        heap_type: HeapType::I31,
    };

    /// The `nullref` type, aka `(ref null none)`.
    pub const NULLREF: Self = RefType {
        is_nullable: true,
        heap_type: HeapType::None,
    };

    /// Construct a new reference type.
    pub fn new(is_nullable: bool, heap_type: HeapType) -> RefType {
        RefType {
            is_nullable,
            heap_type,
        }
    }

    /// Can this type of reference be null?
    pub fn is_nullable(&self) -> bool {
        self.is_nullable
    }

    /// The heap type that this is a reference to.
    #[inline]
    pub fn heap_type(&self) -> &HeapType {
        &self.heap_type
    }

    /// Does this reference type match the other?
    ///
    /// That is, is this reference type a subtype of the other?
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn matches(&self, other: &RefType) -> bool {
        if self.is_nullable() && !other.is_nullable() {
            return false;
        }
        self.heap_type().matches(other.heap_type())
    }

    /// Is reference type `a` precisely equal to reference type `b`?
    ///
    /// Returns `false` even if `a` is a subtype of `b` or vice versa, if they
    /// are not exactly the same reference type.
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine.
    pub fn eq(a: &RefType, b: &RefType) -> bool {
        a.matches(b) && b.matches(a)
    }

    pub(crate) fn ensure_matches(&self, engine: &Engine, other: &RefType) -> Result<()> {
        if !self.comes_from_same_engine(engine) || !other.comes_from_same_engine(engine) {
            bail!("type used with wrong engine");
        }
        if self.matches(other) {
            Ok(())
        } else {
            bail!("type mismatch: expected {other}, found {self}")
        }
    }

    pub(crate) fn comes_from_same_engine(&self, engine: &Engine) -> bool {
        self.heap_type().comes_from_same_engine(engine)
    }

    pub(crate) fn to_wasm_type(&self) -> WasmRefType {
        WasmRefType {
            nullable: self.is_nullable(),
            heap_type: self.heap_type().to_wasm_type(),
        }
    }

    pub(crate) fn from_wasm_type(engine: &Engine, ty: &WasmRefType) -> RefType {
        RefType {
            is_nullable: ty.nullable,
            heap_type: HeapType::from_wasm_type(engine, &ty.heap_type),
        }
    }

    pub(crate) fn is_gc_heap_type(&self) -> bool {
        self.heap_type().is_vmgcref_type_and_points_to_object()
    }
}

/// The heap types that can Wasm can have references to.
///
/// # Subtyping and Equality
///
/// `HeapType` does not implement `Eq`, because heap types have a subtyping
/// relationship, and so 99.99% of the time you actually want to check whether
/// one type matches (i.e. is a subtype of) another type. You can use the
/// [`HeapType::matches`] method to perform these types of checks. If, however,
/// you are in that 0.01% scenario where you need to check precise equality
/// between types, you can use the [`HeapType::eq`] method.
#[derive(Debug, Clone, Hash)]
pub enum HeapType {
    /// The abstract `extern` heap type represents external host data.
    Extern,

    /// The abstract `func` heap type represents a reference to any kind of
    /// function.
    ///
    /// This is the top type for the function references type hierarchy, and is
    /// therefore a supertype of every function reference.
    Func,

    /// A reference to a function of a specific, concrete type.
    ///
    /// These are subtypes of `func` and supertypes of `nofunc`.
    ConcreteFunc(FuncType),

    /// The abstract `nofunc` heap type represents the null function reference.
    ///
    /// This is the bottom type for the function references type hierarchy, and
    /// therefore `nofunc` is a subtype of all function reference types.
    NoFunc,

    /// The abstract `any` heap type represents all internal Wasm data.
    ///
    /// This is the top type of the internal type hierarchy, and is therefore a
    /// supertype of all internal types (such as `i31`, `struct`s, and
    /// `array`s).
    Any,

    /// The `i31` heap type represents unboxed 31-bit integers.
    ///
    /// This is a subtype of `any` and a supertype of `none`.
    I31,

    /// The abstract `array` heap type represents a reference to any kind of array.
    ///
    /// This is a subtype of `any` and a supertype of all concrete array types,
    /// as well as a supertype of the abstract `none` heap type.
    //
    // TODO: add docs for subtype of `eq` once we add that heap type
    Array,

    /// A reference to an array of a specific, concrete type.
    ///
    /// These are subtypes of the `array` heap type (therefore also a subtype of
    /// `any`) and supertypes of the `none` heap type.
    //
    // TODO: add docs for subtype of `eq` once we add that heap type
    ConcreteArray(ArrayType),

    /// The abstract `none` heap type represents the null internal reference.
    ///
    /// This is the bottom type for the internal type hierarchy, and therefore
    /// `none` is a subtype of internal types.
    None,
}

impl Display for HeapType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeapType::Extern => write!(f, "extern"),
            HeapType::Func => write!(f, "func"),
            HeapType::NoFunc => write!(f, "nofunc"),
            HeapType::Any => write!(f, "any"),
            HeapType::I31 => write!(f, "i31"),
            HeapType::Array => write!(f, "array"),
            HeapType::None => write!(f, "none"),
            HeapType::ConcreteFunc(ty) => write!(f, "(concrete func {:?})", ty.type_index()),
            HeapType::ConcreteArray(ty) => write!(f, "(concrete array {:?})", ty.type_index()),
        }
    }
}

impl From<FuncType> for HeapType {
    #[inline]
    fn from(f: FuncType) -> Self {
        HeapType::ConcreteFunc(f)
    }
}

impl From<ArrayType> for HeapType {
    #[inline]
    fn from(a: ArrayType) -> Self {
        HeapType::ConcreteArray(a)
    }
}

impl HeapType {
    /// Is this the abstract `extern` heap type?
    pub fn is_extern(&self) -> bool {
        matches!(self, HeapType::Extern)
    }

    /// Is this the abstract `func` heap type?
    pub fn is_func(&self) -> bool {
        matches!(self, HeapType::Func)
    }

    /// Is this the abstract `nofunc` heap type?
    pub fn is_no_func(&self) -> bool {
        matches!(self, HeapType::NoFunc)
    }

    /// Is this the abstract `any` heap type?
    pub fn is_any(&self) -> bool {
        matches!(self, HeapType::Any)
    }

    /// Is this the abstract `i31` heap type?
    pub fn is_i31(&self) -> bool {
        matches!(self, HeapType::I31)
    }

    /// Is this the abstract `none` heap type?
    pub fn is_none(&self) -> bool {
        matches!(self, HeapType::None)
    }

    /// Is this an abstract type?
    ///
    /// Types that are not abstract are concrete, user-defined types.
    pub fn is_abstract(&self) -> bool {
        !self.is_concrete()
    }

    /// Is this a concrete, user-defined heap type?
    ///
    /// Types that are not concrete, user-defined types are abstract types.
    #[inline]
    pub fn is_concrete(&self) -> bool {
        matches!(self, HeapType::ConcreteFunc(_) | HeapType::ConcreteArray(_))
    }

    /// Is this a concrete, user-defined function type?
    pub fn is_concrete_func(&self) -> bool {
        matches!(self, HeapType::ConcreteFunc(_))
    }

    /// Get the underlying concrete, user-defined function type, if any.
    ///
    /// Returns `None` if this is not a concrete function type.
    pub fn as_concrete_func(&self) -> Option<&FuncType> {
        match self {
            HeapType::ConcreteFunc(f) => Some(f),
            _ => None,
        }
    }

    /// Get the underlying concrete, user-defined type, panicking if this is not
    /// a concrete function type.
    pub fn unwrap_concrete_func(&self) -> &FuncType {
        self.as_concrete_func().unwrap()
    }

    /// Is this a concrete, user-defined array type?
    pub fn is_concrete_array(&self) -> bool {
        matches!(self, HeapType::ConcreteArray(_))
    }

    /// Get the underlying concrete, user-defined array type, if any.
    ///
    /// Returns `None` for if this is not a concrete array type.
    pub fn as_concrete_array(&self) -> Option<&ArrayType> {
        match self {
            HeapType::ConcreteArray(f) => Some(f),
            _ => None,
        }
    }

    /// Get the underlying concrete, user-defined type, panicking if this is not
    /// a concrete array type.
    pub fn unwrap_concrete_array(&self) -> &ArrayType {
        self.as_concrete_array().unwrap()
    }

    /// Get the top type of this heap type's type hierarchy.
    ///
    /// The returned heap type is a supertype of all types in this heap type's
    /// type hierarchy.
    #[inline]
    pub fn top(&self) -> HeapType {
        match self {
            HeapType::Func | HeapType::ConcreteFunc(_) | HeapType::NoFunc => HeapType::Func,

            HeapType::Extern => HeapType::Extern,

            HeapType::Any
            | HeapType::I31
            | HeapType::Array
            | HeapType::ConcreteArray(_)
            | HeapType::None => HeapType::Any,
        }
    }

    /// Is this the top type within its type hierarchy?
    pub fn is_top(&self) -> bool {
        match self {
            HeapType::Any | HeapType::Extern | HeapType::Func => true,
            _ => false,
        }
    }

    /// Does this heap type match the other heap type?
    ///
    /// That is, is this heap type a subtype of the other?
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn matches(&self, other: &HeapType) -> bool {
        match (self, other) {
            (HeapType::Extern, HeapType::Extern) => true,
            (HeapType::Extern, _) => false,

            (HeapType::NoFunc, HeapType::NoFunc | HeapType::ConcreteFunc(_) | HeapType::Func) => {
                true
            }
            (HeapType::NoFunc, _) => false,

            (HeapType::ConcreteFunc(_), HeapType::Func) => true,
            (HeapType::ConcreteFunc(a), HeapType::ConcreteFunc(b)) => a.matches(b),
            (HeapType::ConcreteFunc(_), _) => false,

            (HeapType::Func, HeapType::Func) => true,
            (HeapType::Func, _) => false,

            (
                HeapType::None,
                HeapType::None
                | HeapType::ConcreteArray(_)
                | HeapType::Array
                | HeapType::I31
                | HeapType::Any,
            ) => true,
            (HeapType::None, _) => false,

            (HeapType::ConcreteArray(_), HeapType::Array | HeapType::Any) => true,
            (HeapType::ConcreteArray(a), HeapType::ConcreteArray(b)) => a.matches(b),
            (HeapType::ConcreteArray(_), _) => false,

            (HeapType::Array, HeapType::Array | HeapType::Any) => true,
            (HeapType::Array, _) => false,

            (HeapType::I31, HeapType::I31 | HeapType::Any) => true,
            (HeapType::I31, _) => false,

            (HeapType::Any, HeapType::Any) => true,
            (HeapType::Any, _) => false,
        }
    }

    /// Is heap type `a` precisely equal to heap type `b`?
    ///
    /// Returns `false` even if `a` is a subtype of `b` or vice versa, if they
    /// are not exactly the same heap type.
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn eq(a: &HeapType, b: &HeapType) -> bool {
        a.matches(b) && b.matches(a)
    }

    pub(crate) fn ensure_matches(&self, engine: &Engine, other: &HeapType) -> Result<()> {
        if !self.comes_from_same_engine(engine) || !other.comes_from_same_engine(engine) {
            bail!("type used with wrong engine");
        }
        if self.matches(other) {
            Ok(())
        } else {
            bail!("type mismatch: expected {other}, found {self}");
        }
    }

    pub(crate) fn comes_from_same_engine(&self, engine: &Engine) -> bool {
        match self {
            HeapType::Extern
            | HeapType::Func
            | HeapType::NoFunc
            | HeapType::Any
            | HeapType::I31
            | HeapType::Array
            | HeapType::None => true,
            HeapType::ConcreteFunc(ty) => ty.comes_from_same_engine(engine),
            HeapType::ConcreteArray(ty) => ty.comes_from_same_engine(engine),
        }
    }

    pub(crate) fn to_wasm_type(&self) -> WasmHeapType {
        match self {
            HeapType::Extern => WasmHeapType::Extern,
            HeapType::Func => WasmHeapType::Func,
            HeapType::NoFunc => WasmHeapType::NoFunc,
            HeapType::Any => WasmHeapType::Any,
            HeapType::I31 => WasmHeapType::I31,
            HeapType::Array => WasmHeapType::Array,
            HeapType::None => WasmHeapType::None,
            HeapType::ConcreteFunc(f) => {
                WasmHeapType::ConcreteFunc(EngineOrModuleTypeIndex::Engine(f.type_index()))
            }
            HeapType::ConcreteArray(a) => {
                WasmHeapType::ConcreteArray(EngineOrModuleTypeIndex::Engine(a.type_index()))
            }
        }
    }

    pub(crate) fn from_wasm_type(engine: &Engine, ty: &WasmHeapType) -> HeapType {
        match ty {
            WasmHeapType::Extern => HeapType::Extern,
            WasmHeapType::Func => HeapType::Func,
            WasmHeapType::NoFunc => HeapType::NoFunc,
            WasmHeapType::Any => HeapType::Any,
            WasmHeapType::I31 => HeapType::I31,
            WasmHeapType::Array => HeapType::Array,
            WasmHeapType::None => HeapType::None,
            WasmHeapType::ConcreteFunc(EngineOrModuleTypeIndex::Engine(idx)) => {
                HeapType::ConcreteFunc(FuncType::from_shared_type_index(engine, *idx))
            }
            WasmHeapType::ConcreteArray(EngineOrModuleTypeIndex::Engine(idx)) => {
                HeapType::ConcreteArray(ArrayType::from_shared_type_index(engine, *idx))
            }

            WasmHeapType::ConcreteFunc(EngineOrModuleTypeIndex::Module(_))
            | WasmHeapType::ConcreteFunc(EngineOrModuleTypeIndex::RecGroup(_))
            | WasmHeapType::ConcreteArray(EngineOrModuleTypeIndex::Module(_))
            | WasmHeapType::ConcreteArray(EngineOrModuleTypeIndex::RecGroup(_)) => {
                panic!("HeapType::from_wasm_type on non-canonicalized-for-runtime-usage heap type")
            }
        }
    }

    pub(crate) fn as_registered_type(&self) -> Option<&RegisteredType> {
        match self {
            HeapType::ConcreteFunc(f) => Some(&f.registered_type),
            HeapType::ConcreteArray(a) => Some(&a.registered_type),
            HeapType::Extern
            | HeapType::Func
            | HeapType::NoFunc
            | HeapType::Any
            | HeapType::I31
            | HeapType::Array
            | HeapType::None => None,
        }
    }

    #[inline]
    pub(crate) fn is_vmgcref_type(&self) -> bool {
        match self.top() {
            Self::Any | Self::Extern => true,
            Self::Func => false,
            ty => unreachable!("not a top type: {ty:?}"),
        }
    }

    /// Is this a `VMGcRef` type that is not i31 and is not an uninhabited
    /// bottom type?
    #[inline]
    pub(crate) fn is_vmgcref_type_and_points_to_object(&self) -> bool {
        self.is_vmgcref_type() && !matches!(self, HeapType::I31 | HeapType::NoFunc | HeapType::None)
    }
}

// External Types

/// A list of all possible types which can be externally referenced from a
/// WebAssembly module.
///
/// This list can be found in [`ImportType`] or [`ExportType`], so these types
/// can either be imported or exported.
#[derive(Debug, Clone)]
pub enum ExternType {
    /// This external type is the type of a WebAssembly function.
    Func(FuncType),
    /// This external type is the type of a WebAssembly global.
    Global(GlobalType),
    /// This external type is the type of a WebAssembly table.
    Table(TableType),
    /// This external type is the type of a WebAssembly memory.
    Memory(MemoryType),
}

macro_rules! extern_type_accessors {
    ($(($variant:ident($ty:ty) $get:ident $unwrap:ident))*) => ($(
        /// Attempt to return the underlying type of this external type,
        /// returning `None` if it is a different type.
        pub fn $get(&self) -> Option<&$ty> {
            if let ExternType::$variant(e) = self {
                Some(e)
            } else {
                None
            }
        }

        /// Returns the underlying descriptor of this [`ExternType`], panicking
        /// if it is a different type.
        ///
        /// # Panics
        ///
        /// Panics if `self` is not of the right type.
        pub fn $unwrap(&self) -> &$ty {
            self.$get().expect(concat!("expected ", stringify!($ty)))
        }
    )*)
}

impl ExternType {
    extern_type_accessors! {
        (Func(FuncType) func unwrap_func)
        (Global(GlobalType) global unwrap_global)
        (Table(TableType) table unwrap_table)
        (Memory(MemoryType) memory unwrap_memory)
    }

    pub(crate) fn from_wasmtime(
        engine: &Engine,
        types: &ModuleTypes,
        ty: &EntityType,
    ) -> ExternType {
        match ty {
            EntityType::Function(idx) => match idx {
                EngineOrModuleTypeIndex::Engine(e) => {
                    FuncType::from_shared_type_index(engine, *e).into()
                }
                EngineOrModuleTypeIndex::Module(m) => {
                    FuncType::from_wasm_func_type(engine, types[*m].unwrap_func().clone()).into()
                }
                EngineOrModuleTypeIndex::RecGroup(_) => unreachable!(),
            },
            EntityType::Global(ty) => GlobalType::from_wasmtime_global(engine, ty).into(),
            EntityType::Memory(ty) => MemoryType::from_wasmtime_memory(ty).into(),
            EntityType::Table(ty) => TableType::from_wasmtime_table(engine, ty).into(),
            EntityType::Tag(_) => unimplemented!("wasm tag support"),
        }
    }
}

impl From<FuncType> for ExternType {
    fn from(ty: FuncType) -> ExternType {
        ExternType::Func(ty)
    }
}

impl From<GlobalType> for ExternType {
    fn from(ty: GlobalType) -> ExternType {
        ExternType::Global(ty)
    }
}

impl From<MemoryType> for ExternType {
    fn from(ty: MemoryType) -> ExternType {
        ExternType::Memory(ty)
    }
}

impl From<TableType> for ExternType {
    fn from(ty: TableType) -> ExternType {
        ExternType::Table(ty)
    }
}

/// The storage type of a `struct` field or `array` element.
///
/// This is either a packed 8- or -16 bit integer, or else it is some unpacked
/// Wasm value type.
#[derive(Clone, Hash)]
pub enum StorageType {
    /// `i8`, an 8-bit integer.
    I8,
    /// `i16`, a 16-bit integer.
    I16,
    /// A value type.
    ValType(ValType),
}

impl From<ValType> for StorageType {
    #[inline]
    fn from(v: ValType) -> Self {
        StorageType::ValType(v)
    }
}

impl StorageType {
    /// Is this an `i8`?
    #[inline]
    pub fn is_i8(&self) -> bool {
        matches!(self, Self::I8)
    }

    /// Is this an `i16`?
    #[inline]
    pub fn is_i16(&self) -> bool {
        matches!(self, Self::I16)
    }

    /// Is this a Wasm value type?
    #[inline]
    pub fn is_val_type(&self) -> bool {
        matches!(self, Self::I16)
    }

    /// Get this storage type's underlying value type, if any.
    ///
    /// Returns `None` if this storage type is not a value type.
    #[inline]
    pub fn as_val_type(&self) -> Option<&ValType> {
        match self {
            Self::ValType(v) => Some(v),
            _ => None,
        }
    }

    /// Get this storage type's underlying value type, panicking if it is not a
    /// value type.
    pub fn unwrap_val_type(&self) -> &ValType {
        self.as_val_type().unwrap()
    }

    /// Does this field type match the other field type?
    ///
    /// That is, is this field type a subtype of the other field type?
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (StorageType::I8, StorageType::I8) => true,
            (StorageType::I8, _) => false,
            (StorageType::I16, StorageType::I16) => true,
            (StorageType::I16, _) => false,
            (StorageType::ValType(a), StorageType::ValType(b)) => a.matches(b),
            (StorageType::ValType(_), _) => false,
        }
    }

    /// Is field type `a` precisely equal to field type `b`?
    ///
    /// Returns `false` even if `a` is a subtype of `b` or vice versa, if they
    /// are not exactly the same field type.
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn eq(a: &Self, b: &Self) -> bool {
        a.matches(b) && b.matches(a)
    }

    pub(crate) fn from_wasm_storage_type(engine: &Engine, ty: &WasmStorageType) -> Self {
        match ty {
            WasmStorageType::I8 => Self::I8,
            WasmStorageType::I16 => Self::I16,
            WasmStorageType::Val(v) => ValType::from_wasm_type(engine, &v).into(),
        }
    }

    pub(crate) fn to_wasm_storage_type(&self) -> WasmStorageType {
        match self {
            Self::I8 => WasmStorageType::I8,
            Self::I16 => WasmStorageType::I16,
            Self::ValType(v) => WasmStorageType::Val(v.to_wasm_type()),
        }
    }
}

/// The type of a `struct` field or an `array`'s elements.
///
/// This is a pair of both the field's storage type and its mutability
/// (i.e. whether the field can be updated or not).
#[derive(Clone, Hash)]
pub struct FieldType {
    mutability: Mutability,
    element_type: StorageType,
}

impl FieldType {
    /// Construct a new field type from the given parts.
    #[inline]
    pub fn new(mutability: Mutability, element_type: StorageType) -> Self {
        Self {
            mutability,
            element_type,
        }
    }

    /// Get whether or not this field type is mutable.
    #[inline]
    pub fn mutability(&self) -> Mutability {
        self.mutability
    }

    /// Get this field type's storage type.
    #[inline]
    pub fn element_type(&self) -> &StorageType {
        &self.element_type
    }

    /// Does this field type match the other field type?
    ///
    /// That is, is this field type a subtype of the other field type?
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn matches(&self, other: &Self) -> bool {
        (other.mutability == Mutability::Var || self.mutability == Mutability::Const)
            && self.element_type.matches(&other.element_type)
    }

    /// Is field type `a` precisely equal to field type `b`?
    ///
    /// Returns `false` even if `a` is a subtype of `b` or vice versa, if they
    /// are not exactly the same field type.
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn eq(a: &Self, b: &Self) -> bool {
        a.matches(b) && b.matches(a)
    }

    pub(crate) fn from_wasm_field_type(engine: &Engine, ty: &WasmFieldType) -> Self {
        Self {
            mutability: if ty.mutable {
                Mutability::Var
            } else {
                Mutability::Const
            },
            element_type: StorageType::from_wasm_storage_type(engine, &ty.element_type),
        }
    }

    pub(crate) fn to_wasm_field_type(&self) -> WasmFieldType {
        WasmFieldType {
            element_type: self.element_type.to_wasm_storage_type(),
            mutable: matches!(self.mutability, Mutability::Var),
        }
    }
}

/// The type of a WebAssembly array.
///
/// WebAssembly arrays are dynamically-sized, but not resizable. They contain
/// either unpacked [`Val`][crate::Val]s or packed 8-/16-bit integers.
///
/// # Subtyping and Equality
///
/// `ArrayType` does not implement `Eq`, because reference types have a
/// subtyping relationship, and so 99.99% of the time you actually want to check
/// whether one type matches (i.e. is a subtype of) another type. You can use
/// the [`ArrayType::matches`] method to perform these types of checks. If,
/// however, you are in that 0.01% scenario where you need to check precise
/// equality between types, you can use the [`ArrayType::eq`] method.
//
// TODO: Once we have array values, update above docs with a reference to the
// future `Array::matches_ty` method
#[derive(Debug, Clone, Hash)]
pub struct ArrayType {
    registered_type: RegisteredType,
}

impl ArrayType {
    /// Construct a new `ArrayType` with the given field type's mutability and
    /// storage type.
    ///
    /// The result will be associated with the given engine, and attempts to use
    /// it with other engines will panic (for example, checking whether it is a
    /// subtype of another array type that is associated with a different
    /// engine).
    pub fn new(engine: &Engine, field_type: FieldType) -> Self {
        // Same as in `FuncType::new`: we must prevent any `RegisteredType` in
        // `field_type` from being reclaimed while constructing this array type.
        let _registration = field_type
            .element_type
            .as_val_type()
            .and_then(|v| v.as_ref())
            .and_then(|r| r.heap_type().as_registered_type());

        let wasm_ty = WasmArrayType(field_type.to_wasm_field_type());
        Self::from_wasm_array_type(engine, wasm_ty)
    }

    /// Get the engine that this array type is associated with.
    pub fn engine(&self) -> &Engine {
        self.registered_type.engine()
    }

    /// Get this array's underlying field type.
    ///
    /// The field type contains information about both this array type's
    /// mutability and the storage type used for its elements.
    pub fn field_type(&self) -> FieldType {
        FieldType::from_wasm_field_type(self.engine(), &self.as_wasm_array_type().0)
    }

    /// Get this array type's mutability and whether its instances' elements can
    /// be updated or not.
    ///
    /// This is a convenience method providing a short-hand for
    /// `my_array_type.field_type().mutability()`.
    pub fn mutability(&self) -> Mutability {
        if self.as_wasm_array_type().0.mutable {
            Mutability::Var
        } else {
            Mutability::Const
        }
    }

    /// Get the storage type used for this array type's elements.
    ///
    /// This is a convenience method providing a short-hand for
    /// `my_array_type.field_type().element_type()`.
    pub fn element_type(&self) -> StorageType {
        StorageType::from_wasm_storage_type(
            self.engine(),
            &self.registered_type.unwrap_array().0.element_type,
        )
    }

    /// Does this array type match the other array type?
    ///
    /// That is, is this function type a subtype of the other array type?
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn matches(&self, other: &ArrayType) -> bool {
        assert!(self.comes_from_same_engine(other.engine()));

        // Avoid matching on structure for subtyping checks when we have
        // precisely the same type.
        if self.type_index() == other.type_index() {
            return true;
        }

        self.field_type().matches(&other.field_type())
    }

    /// Is array type `a` precisely equal to array type `b`?
    ///
    /// Returns `false` even if `a` is a subtype of `b` or vice versa, if they
    /// are not exactly the same array type.
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn eq(a: &ArrayType, b: &ArrayType) -> bool {
        assert!(a.comes_from_same_engine(b.engine()));
        a.type_index() == b.type_index()
    }

    pub(crate) fn comes_from_same_engine(&self, engine: &Engine) -> bool {
        Engine::same(self.registered_type.engine(), engine)
    }

    pub(crate) fn type_index(&self) -> VMSharedTypeIndex {
        self.registered_type.index()
    }

    pub(crate) fn as_wasm_array_type(&self) -> &WasmArrayType {
        self.registered_type.unwrap_array()
    }

    /// Construct a `ArrayType` from a `WasmArrayType`.
    ///
    /// This method should only be used when something has already registered --
    /// and is *keeping registered* -- any other concrete Wasm types referenced
    /// by the given `WasmArrayType`.
    ///
    /// For example, this method may be called to convert an array type from
    /// within a Wasm module's `ModuleTypes` since the Wasm module itself is
    /// holding a strong reference to all of its types, including any `(ref null
    /// <index>)` types used as the element type for this array type.
    pub(crate) fn from_wasm_array_type(engine: &Engine, ty: WasmArrayType) -> ArrayType {
        let ty = RegisteredType::new(
            engine,
            WasmSubType {
                // TODO:
                //
                // is_final: true,
                // supertype: None,
                composite_type: WasmCompositeType::Array(ty),
            },
        );
        Self {
            registered_type: ty,
        }
    }

    pub(crate) fn from_shared_type_index(engine: &Engine, index: VMSharedTypeIndex) -> ArrayType {
        let ty = RegisteredType::root(engine, index).expect(
            "VMSharedTypeIndex is not registered in the Engine! Wrong \
             engine? Didn't root the index somewhere?",
        );
        assert!(ty.is_array());
        Self {
            registered_type: ty,
        }
    }
}

/// The type of a WebAssembly function.
///
/// WebAssembly functions can have 0 or more parameters and results.
///
/// # Subtyping and Equality
///
/// `FuncType` does not implement `Eq`, because reference types have a subtyping
/// relationship, and so 99.99% of the time you actually want to check whether
/// one type matches (i.e. is a subtype of) another type. You can use the
/// [`FuncType::matches`] and [`Func::matches_ty`][crate::Func::matches_ty]
/// methods to perform these types of checks. If, however, you are in that 0.01%
/// scenario where you need to check precise equality between types, you can use
/// the [`FuncType::eq`] method.
#[derive(Debug, Clone, Hash)]
pub struct FuncType {
    registered_type: RegisteredType,
}

impl Display for FuncType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(type (func")?;
        if self.params().len() > 0 {
            write!(f, " (param")?;
            for p in self.params() {
                write!(f, " {p}")?;
            }
            write!(f, ")")?;
        }
        if self.results().len() > 0 {
            write!(f, " (result")?;
            for r in self.results() {
                write!(f, " {r}")?;
            }
            write!(f, ")")?;
        }
        write!(f, "))")
    }
}

impl FuncType {
    /// Creates a new function descriptor from the given parameters and results.
    ///
    /// The function descriptor returned will represent a function which takes
    /// `params` as arguments and returns `results` when it is finished.
    pub fn new(
        engine: &Engine,
        params: impl IntoIterator<Item = ValType>,
        results: impl IntoIterator<Item = ValType>,
    ) -> FuncType {
        // Keep any of our parameters' and results' `RegisteredType`s alive
        // across `Self::from_wasm_func_type`. If one of our given `ValType`s is
        // the only thing keeping a type in the registry, we don't want to
        // unregister it when we convert the `ValType` into a `WasmValType` just
        // before we register our new `WasmFuncType` that will reference it.
        let mut registrations = smallvec::SmallVec::<[_; 4]>::new();

        let mut to_wasm_type = |ty: ValType| {
            if let Some(r) = ty.as_ref() {
                if let Some(r) = r.heap_type().as_registered_type() {
                    registrations.push(r.clone());
                }
            }
            ty.to_wasm_type()
        };

        Self::from_wasm_func_type(
            engine,
            WasmFuncType::new(
                params.into_iter().map(&mut to_wasm_type).collect(),
                results.into_iter().map(&mut to_wasm_type).collect(),
            ),
        )
    }

    /// Get the engine that this function type is associated with.
    pub fn engine(&self) -> &Engine {
        self.registered_type.engine()
    }

    /// Get the `i`th parameter type.
    ///
    /// Returns `None` if `i` is out of bounds.
    pub fn param(&self, i: usize) -> Option<ValType> {
        let engine = self.engine();
        self.registered_type
            .unwrap_func()
            .params()
            .get(i)
            .map(|ty| ValType::from_wasm_type(engine, ty))
    }

    /// Returns the list of parameter types for this function.
    #[inline]
    pub fn params(&self) -> impl ExactSizeIterator<Item = ValType> + '_ {
        let engine = self.engine();
        self.registered_type
            .unwrap_func()
            .params()
            .iter()
            .map(|ty| ValType::from_wasm_type(engine, ty))
    }

    /// Get the `i`th result type.
    ///
    /// Returns `None` if `i` is out of bounds.
    pub fn result(&self, i: usize) -> Option<ValType> {
        let engine = self.engine();
        self.registered_type
            .unwrap_func()
            .returns()
            .get(i)
            .map(|ty| ValType::from_wasm_type(engine, ty))
    }

    /// Returns the list of result types for this function.
    #[inline]
    pub fn results(&self) -> impl ExactSizeIterator<Item = ValType> + '_ {
        let engine = self.engine();
        self.registered_type
            .unwrap_func()
            .returns()
            .iter()
            .map(|ty| ValType::from_wasm_type(engine, ty))
    }

    /// Does this function type match the other function type?
    ///
    /// That is, is this function type a subtype of the other function type?
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn matches(&self, other: &FuncType) -> bool {
        assert!(self.comes_from_same_engine(other.engine()));

        // Avoid matching on structure for subtyping checks when we have
        // precisely the same type.
        if self.type_index() == other.type_index() {
            return true;
        }

        self.params().len() == other.params().len()
            && self.results().len() == other.results().len()
            // Params are contravariant and results are covariant. For more
            // details and a refresher on variance, read
            // https://github.com/bytecodealliance/wasm-tools/blob/f1d89a4/crates/wasmparser/src/readers/core/types/matches.rs#L137-L174
            && self
                .params()
                .zip(other.params())
                .all(|(a, b)| b.matches(&a))
            && self
                .results()
                .zip(other.results())
                .all(|(a, b)| a.matches(&b))
    }

    /// Is function type `a` precisely equal to function type `b`?
    ///
    /// Returns `false` even if `a` is a subtype of `b` or vice versa, if they
    /// are not exactly the same function type.
    ///
    /// # Panics
    ///
    /// Panics if either type is associated with a different engine from the
    /// other.
    pub fn eq(a: &FuncType, b: &FuncType) -> bool {
        assert!(a.comes_from_same_engine(b.engine()));
        a.type_index() == b.type_index()
    }

    pub(crate) fn comes_from_same_engine(&self, engine: &Engine) -> bool {
        Engine::same(self.registered_type.engine(), engine)
    }

    pub(crate) fn type_index(&self) -> VMSharedTypeIndex {
        self.registered_type.index()
    }

    pub(crate) fn as_wasm_func_type(&self) -> &WasmFuncType {
        self.registered_type.unwrap_func()
    }

    pub(crate) fn into_registered_type(self) -> RegisteredType {
        self.registered_type
    }

    /// Construct a `FuncType` from a `WasmFuncType`.
    ///
    /// This method should only be used when something has already registered --
    /// and is *keeping registered* -- any other concrete Wasm types referenced
    /// by the given `WasmFuncType`.
    ///
    /// For example, this method may be called to convert a function type from
    /// within a Wasm module's `ModuleTypes` since the Wasm module itself is
    /// holding a strong reference to all of its types, including any `(ref null
    /// <index>)` types used in the function's parameters and results.
    pub(crate) fn from_wasm_func_type(engine: &Engine, ty: WasmFuncType) -> FuncType {
        let ty = RegisteredType::new(
            engine,
            WasmSubType {
                // TODO:
                //
                // is_final: true,
                // supertype: None,
                composite_type: WasmCompositeType::Func(ty),
            },
        );
        Self {
            registered_type: ty,
        }
    }

    pub(crate) fn from_shared_type_index(engine: &Engine, index: VMSharedTypeIndex) -> FuncType {
        let ty = RegisteredType::root(engine, index).expect(
            "VMSharedTypeIndex is not registered in the Engine! Wrong \
             engine? Didn't root the index somewhere?",
        );
        assert!(ty.is_func());
        Self {
            registered_type: ty,
        }
    }
}

// Global Types

/// A WebAssembly global descriptor.
///
/// This type describes an instance of a global in a WebAssembly module. Globals
/// are local to an [`Instance`](crate::Instance) and are either immutable or
/// mutable.
#[derive(Debug, Clone, Hash)]
pub struct GlobalType {
    content: ValType,
    mutability: Mutability,
}

impl GlobalType {
    /// Creates a new global descriptor of the specified `content` type and
    /// whether or not it's mutable.
    pub fn new(content: ValType, mutability: Mutability) -> GlobalType {
        GlobalType {
            content,
            mutability,
        }
    }

    /// Returns the value type of this global descriptor.
    pub fn content(&self) -> &ValType {
        &self.content
    }

    /// Returns whether or not this global is mutable.
    pub fn mutability(&self) -> Mutability {
        self.mutability
    }

    pub(crate) fn to_wasm_type(&self) -> Global {
        let wasm_ty = self.content().to_wasm_type();
        let mutability = matches!(self.mutability(), Mutability::Var);
        Global {
            wasm_ty,
            mutability,
        }
    }

    /// Returns `None` if the wasmtime global has a type that we can't
    /// represent, but that should only very rarely happen and indicate a bug.
    pub(crate) fn from_wasmtime_global(engine: &Engine, global: &Global) -> GlobalType {
        let ty = ValType::from_wasm_type(engine, &global.wasm_ty);
        let mutability = if global.mutability {
            Mutability::Var
        } else {
            Mutability::Const
        };
        GlobalType::new(ty, mutability)
    }
}

// Table Types

/// A descriptor for a table in a WebAssembly module.
///
/// Tables are contiguous chunks of a specific element, typically a `funcref` or
/// an `externref`. The most common use for tables is a function table through
/// which `call_indirect` can invoke other functions.
#[derive(Debug, Clone, Hash)]
pub struct TableType {
    // Keep a `wasmtime::RefType` so that `TableType::element` doesn't need to
    // take an `&Engine`.
    element: RefType,
    ty: Table,
}

impl TableType {
    /// Creates a new table descriptor which will contain the specified
    /// `element` and have the `limits` applied to its length.
    pub fn new(element: RefType, min: u32, max: Option<u32>) -> TableType {
        let wasm_ty = element.to_wasm_type();

        debug_assert!(
            wasm_ty.is_canonicalized_for_runtime_usage(),
            "should be canonicalized for runtime usage: {wasm_ty:?}"
        );

        TableType {
            element,
            ty: Table {
                wasm_ty,
                minimum: min,
                maximum: max,
            },
        }
    }

    /// Returns the element value type of this table.
    pub fn element(&self) -> &RefType {
        &self.element
    }

    /// Returns minimum number of elements this table must have
    pub fn minimum(&self) -> u32 {
        self.ty.minimum
    }

    /// Returns the optionally-specified maximum number of elements this table
    /// can have.
    ///
    /// If this returns `None` then the table is not limited in size.
    pub fn maximum(&self) -> Option<u32> {
        self.ty.maximum
    }

    pub(crate) fn from_wasmtime_table(engine: &Engine, table: &Table) -> TableType {
        let element = RefType::from_wasm_type(engine, &table.wasm_ty);
        TableType {
            element,
            ty: table.clone(),
        }
    }

    pub(crate) fn wasmtime_table(&self) -> &Table {
        &self.ty
    }
}

// Memory Types

/// A descriptor for a WebAssembly memory type.
///
/// Memories are described in units of pages (64KB) and represent contiguous
/// chunks of addressable memory.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MemoryType {
    ty: Memory,
}

impl MemoryType {
    /// Creates a new descriptor for a 32-bit WebAssembly memory given the
    /// specified limits of the memory.
    ///
    /// The `minimum` and `maximum`  values here are specified in units of
    /// WebAssembly pages, which are 64k.
    pub fn new(minimum: u32, maximum: Option<u32>) -> MemoryType {
        MemoryType {
            ty: Memory {
                memory64: false,
                shared: false,
                minimum: minimum.into(),
                maximum: maximum.map(|i| i.into()),
            },
        }
    }

    /// Creates a new descriptor for a 64-bit WebAssembly memory given the
    /// specified limits of the memory.
    ///
    /// The `minimum` and `maximum`  values here are specified in units of
    /// WebAssembly pages, which are 64k.
    ///
    /// Note that 64-bit memories are part of the memory64 proposal for
    /// WebAssembly which is not standardized yet.
    pub fn new64(minimum: u64, maximum: Option<u64>) -> MemoryType {
        MemoryType {
            ty: Memory {
                memory64: true,
                shared: false,
                minimum,
                maximum,
            },
        }
    }

    /// Creates a new descriptor for shared WebAssembly memory given the
    /// specified limits of the memory.
    ///
    /// The `minimum` and `maximum`  values here are specified in units of
    /// WebAssembly pages, which are 64k.
    ///
    /// Note that shared memories are part of the threads proposal for
    /// WebAssembly which is not standardized yet.
    pub fn shared(minimum: u32, maximum: u32) -> MemoryType {
        MemoryType {
            ty: Memory {
                memory64: false,
                shared: true,
                minimum: minimum.into(),
                maximum: Some(maximum.into()),
            },
        }
    }

    /// Returns whether this is a 64-bit memory or not.
    ///
    /// Note that 64-bit memories are part of the memory64 proposal for
    /// WebAssembly which is not standardized yet.
    pub fn is_64(&self) -> bool {
        self.ty.memory64
    }

    /// Returns whether this is a shared memory or not.
    ///
    /// Note that shared memories are part of the threads proposal for
    /// WebAssembly which is not standardized yet.
    pub fn is_shared(&self) -> bool {
        self.ty.shared
    }

    /// Returns minimum number of WebAssembly pages this memory must have.
    ///
    /// Note that the return value, while a `u64`, will always fit into a `u32`
    /// for 32-bit memories.
    pub fn minimum(&self) -> u64 {
        self.ty.minimum
    }

    /// Returns the optionally-specified maximum number of pages this memory
    /// can have.
    ///
    /// If this returns `None` then the memory is not limited in size.
    ///
    /// Note that the return value, while a `u64`, will always fit into a `u32`
    /// for 32-bit memories.
    pub fn maximum(&self) -> Option<u64> {
        self.ty.maximum
    }

    pub(crate) fn from_wasmtime_memory(memory: &Memory) -> MemoryType {
        MemoryType { ty: memory.clone() }
    }

    pub(crate) fn wasmtime_memory(&self) -> &Memory {
        &self.ty
    }
}

// Import Types

/// A descriptor for an imported value into a wasm module.
///
/// This type is primarily accessed from the
/// [`Module::imports`](crate::Module::imports) API. Each [`ImportType`]
/// describes an import into the wasm module with the module/name that it's
/// imported from as well as the type of item that's being imported.
#[derive(Clone)]
pub struct ImportType<'module> {
    /// The module of the import.
    module: &'module str,

    /// The field of the import.
    name: &'module str,

    /// The type of the import.
    ty: EntityType,
    types: &'module ModuleTypes,
    engine: &'module Engine,
}

impl<'module> ImportType<'module> {
    /// Creates a new import descriptor which comes from `module` and `name` and
    /// is of type `ty`.
    pub(crate) fn new(
        module: &'module str,
        name: &'module str,
        ty: EntityType,
        types: &'module ModuleTypes,
        engine: &'module Engine,
    ) -> ImportType<'module> {
        ImportType {
            module,
            name,
            ty,
            types,
            engine,
        }
    }

    /// Returns the module name that this import is expected to come from.
    pub fn module(&self) -> &'module str {
        self.module
    }

    /// Returns the field name of the module that this import is expected to
    /// come from.
    pub fn name(&self) -> &'module str {
        self.name
    }

    /// Returns the expected type of this import.
    pub fn ty(&self) -> ExternType {
        ExternType::from_wasmtime(self.engine, self.types, &self.ty)
    }
}

impl<'module> fmt::Debug for ImportType<'module> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportType")
            .field("module", &self.module())
            .field("name", &self.name())
            .field("ty", &self.ty())
            .finish()
    }
}

// Export Types

/// A descriptor for an exported WebAssembly value.
///
/// This type is primarily accessed from the
/// [`Module::exports`](crate::Module::exports) accessor and describes what
/// names are exported from a wasm module and the type of the item that is
/// exported.
#[derive(Clone)]
pub struct ExportType<'module> {
    /// The name of the export.
    name: &'module str,

    /// The type of the export.
    ty: EntityType,
    types: &'module ModuleTypes,
    engine: &'module Engine,
}

impl<'module> ExportType<'module> {
    /// Creates a new export which is exported with the given `name` and has the
    /// given `ty`.
    pub(crate) fn new(
        name: &'module str,
        ty: EntityType,
        types: &'module ModuleTypes,
        engine: &'module Engine,
    ) -> ExportType<'module> {
        ExportType {
            name,
            ty,
            types,
            engine,
        }
    }

    /// Returns the name by which this export is known.
    pub fn name(&self) -> &'module str {
        self.name
    }

    /// Returns the type of this export.
    pub fn ty(&self) -> ExternType {
        ExternType::from_wasmtime(self.engine, self.types, &self.ty)
    }
}

impl<'module> fmt::Debug for ExportType<'module> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExportType")
            .field("name", &self.name().to_owned())
            .field("ty", &self.ty())
            .finish()
    }
}

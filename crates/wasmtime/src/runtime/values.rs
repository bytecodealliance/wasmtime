use crate::runtime::vm::VMGcRef;
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::{
    AnyRef, ArrayRef, AsContext, AsContextMut, ExnRef, ExternRef, Frozen, Func, HeapTopType,
    HeapType, RefType, Rooted, StructRef, V128, ValType, prelude::*,
};
use core::ptr;

pub use crate::runtime::vm::ValRaw;

/// A stub implementation for continuation references.
///
/// This is a placeholder until continuation objects are fully integrated
/// with the GC system (see #10248).
#[derive(Debug, Clone, Copy)]
pub struct ContRef;

/// Possible runtime values that a WebAssembly module can either consume or
/// produce.
///
/// Note that we inline the `enum Ref { ... }` variants into `enum Val { ... }`
/// here as a size optimization.
#[derive(Debug, Clone, Copy)]
pub enum Val {
    // NB: the ordering here is intended to match the ordering in
    // `ValType` to improve codegen when learning the type of a value.
    //
    /// A 32-bit integer.
    I32(i32),

    /// A 64-bit integer.
    I64(i64),

    /// A 32-bit float.
    ///
    /// Note that the raw bits of the float are stored here, and you can use
    /// `f32::from_bits` to create an `f32` value.
    F32(u32),

    /// A 64-bit float.
    ///
    /// Note that the raw bits of the float are stored here, and you can use
    /// `f64::from_bits` to create an `f64` value.
    F64(u64),

    /// A 128-bit number.
    V128(V128),

    /// A function reference.
    FuncRef(Option<Func>),

    /// An external reference.
    ExternRef(Option<Rooted<ExternRef>>),

    /// An internal reference.
    AnyRef(Option<Rooted<AnyRef>>),

    /// An exception reference.
    ExnRef(Option<Rooted<ExnRef>>),

    /// A continuation reference.
    ///
    /// Note: This is currently a stub implementation as continuation objects
    /// are not yet fully integrated with the GC system. See #10248.
    ContRef(Option<ContRef>),

    /// A non-null GC reference, as a [`Frozen`] reference.
    ///
    /// Accessors that read GC references out of the store, such as
    /// [`Global::get`] and [`StructRef::field`], return these (and null
    /// references as usual). They are valid until the store's GC heap or
    /// execution changes; see [`Frozen`]. Other APIs accept them while they
    /// are valid, as they do rooted references, and [`Val::to_rooted`] roots
    /// them to keep them longer.
    ///
    /// [`Global::get`]: crate::Global::get
    /// [`StructRef::field`]: crate::StructRef::field
    Frozen(FrozenRef),
}

/// A non-null [`Frozen`] reference in one of Wasm's GC type hierarchies; see
/// [`Val::Frozen`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrozenRef {
    /// An external reference.
    Extern(Frozen<ExternRef>),
    /// An internal reference.
    Any(Frozen<AnyRef>),
    /// An exception reference.
    Exn(Frozen<ExnRef>),
}

impl FrozenRef {
    /// Is this reference still valid?
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn is_valid(&self, store: impl AsContext) -> bool {
        self.gc_ref(store.as_context().0).is_ok()
    }

    /// The internal reference, if this is one.
    pub fn as_any(&self) -> Option<Frozen<AnyRef>> {
        match self {
            FrozenRef::Any(a) => Some(*a),
            _ => None,
        }
    }

    /// The exception reference, if this is one.
    pub fn as_exn(&self) -> Option<Frozen<ExnRef>> {
        match self {
            FrozenRef::Exn(e) => Some(*e),
            _ => None,
        }
    }

    /// The external reference, if this is one.
    pub fn as_extern(&self) -> Option<Frozen<ExternRef>> {
        match self {
            FrozenRef::Extern(e) => Some(*e),
            _ => None,
        }
    }

    /// Roots the referenced object in the current LIFO rooting scope; see
    /// [`Frozen::to_rooted`].
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn to_rooted(&self, mut store: impl AsContextMut) -> Result<Ref> {
        let store = store.as_context_mut();
        Ok(match self {
            FrozenRef::Extern(e) => Ref::Extern(Some(e.to_rooted(store)?)),
            FrozenRef::Any(a) => Ref::Any(Some(a.to_rooted(store)?)),
            FrozenRef::Exn(e) => Ref::Exn(Some(e.to_rooted(store)?)),
        })
    }

    /// The referenced object, if this reference is still valid.
    pub(crate) fn gc_ref(&self, store: &StoreOpaque) -> Result<VMGcRef> {
        match self {
            FrozenRef::Extern(e) => e.gc_ref(store),
            FrozenRef::Any(a) => a.gc_ref(store),
            FrozenRef::Exn(e) => e.gc_ref(store),
        }
    }

    /// The referenced object, cloned for storing it somewhere, if this
    /// reference is still valid.
    pub(crate) fn clone_gc_ref(&self, store: &mut AutoAssertNoGc<'_>) -> Result<VMGcRef> {
        let gc_ref = self.gc_ref(store)?;
        Ok(store.clone_gc_ref(&gc_ref))
    }

    fn heap_type(&self, store: &StoreOpaque) -> Result<HeapType> {
        Ok(match self {
            FrozenRef::Extern(e) => {
                e.gc_ref(store)?;
                HeapType::Extern
            }
            FrozenRef::Any(a) => a._ty(store)?,
            FrozenRef::Exn(e) => {
                e.gc_ref(store)?;
                HeapType::Exn
            }
        })
    }

    /// A value of type `top` for the GC reference `gc_ref`, frozen.
    pub(crate) fn val(store: &StoreOpaque, top: HeapTopType, gc_ref: Option<&VMGcRef>) -> Val {
        let Some(gc_ref) = gc_ref else {
            return Val::null_ref(&top.into());
        };
        Val::Frozen(match top {
            HeapTopType::Extern => FrozenRef::Extern(Frozen::new(store, gc_ref)),
            HeapTopType::Any => FrozenRef::Any(Frozen::new(store, gc_ref)),
            HeapTopType::Exn => FrozenRef::Exn(Frozen::new(store, gc_ref)),
            HeapTopType::Func | HeapTopType::Cont => unreachable!("not GC references"),
        })
    }

    /// Like `FrozenRef::val`, for a GC reference's raw bits.
    pub(crate) fn val_from_raw(store: &StoreOpaque, top: HeapTopType, raw: u32) -> Val {
        Self::val(store, top, VMGcRef::from_raw_u32(raw).as_ref())
    }

    fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match self {
            FrozenRef::Extern(e) => e.comes_from_same_store(store),
            FrozenRef::Any(a) => a.comes_from_same_store(store),
            FrozenRef::Exn(e) => e.comes_from_same_store(store),
        }
    }
}

macro_rules! accessors {
    ($bind:ident $(($variant:ident($ty:ty) $get:ident $unwrap:ident $cvt:expr))*) => ($(
        /// Attempt to access the underlying value of this `Val`, returning
        /// `None` if it is not the correct type.
        #[inline]
        pub fn $get(&self) -> Option<$ty> {
            if let Val::$variant($bind) = self {
                Some($cvt)
            } else {
                None
            }
        }

        /// Returns the underlying value of this `Val`, panicking if it's the
        /// wrong type.
        ///
        /// # Panics
        ///
        /// Panics if `self` is not of the right type.
        #[inline]
        pub fn $unwrap(&self) -> $ty {
            self.$get().expect(concat!("expected ", stringify!($ty)))
        }
    )*)
}

impl Val {
    /// Returns the null reference for the given heap type.
    #[inline]
    pub fn null_ref(heap_type: &HeapType) -> Val {
        Ref::null(&heap_type).into()
    }

    /// Returns the null function reference value.
    ///
    /// The return value has type `(ref null nofunc)` aka `nullfuncref` and is a
    /// subtype of all function references.
    #[inline]
    pub const fn null_func_ref() -> Val {
        Val::FuncRef(None)
    }

    /// Returns the null external reference value.
    ///
    /// The return value has type `(ref null noextern)` aka `nullexternref` and
    /// is a subtype of all external references.
    #[inline]
    pub const fn null_extern_ref() -> Val {
        Val::ExternRef(None)
    }

    /// Returns the null internal reference value.
    ///
    /// The return value has type `(ref null none)` aka `nullref` and is a
    /// subtype of all internal references.
    #[inline]
    pub const fn null_any_ref() -> Val {
        Val::AnyRef(None)
    }

    /// Returns the default value for the given type, if any exists.
    ///
    /// Returns `None` if there is no default value for the given type (for
    /// example, non-nullable reference types do not have a default value).
    pub fn default_for_ty(ty: &ValType) -> Option<Val> {
        match ty {
            ValType::I32 => Some(Val::I32(0)),
            ValType::I64 => Some(Val::I64(0)),
            ValType::F32 => Some(Val::F32(0)),
            ValType::F64 => Some(Val::F64(0)),
            ValType::V128 => Some(Val::V128(V128::from(0))),
            ValType::Ref(ref_ty) => {
                if ref_ty.is_nullable() {
                    Some(Val::null_ref(ref_ty.heap_type()))
                } else {
                    None
                }
            }
        }
    }

    /// Returns the corresponding [`ValType`] for this `Val`.
    ///
    /// # Errors
    ///
    /// Returns an error if this value is a GC reference that has since been
    /// unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this value is associated with a different store.
    #[inline]
    pub fn ty(&self, store: impl AsContext) -> Result<ValType> {
        self.load_ty(&store.as_context().0)
    }

    #[inline]
    pub(crate) fn load_ty(&self, store: &StoreOpaque) -> Result<ValType> {
        Ok(match self {
            Val::I32(_) => ValType::I32,
            Val::I64(_) => ValType::I64,
            Val::F32(_) => ValType::F32,
            Val::F64(_) => ValType::F64,
            Val::V128(_) => ValType::V128,
            Val::ExternRef(Some(e)) => {
                e.try_gc_ref(store)?;
                ValType::Ref(RefType::new(false, HeapType::Extern))
            }
            Val::ExternRef(None) => ValType::NULLEXTERNREF,
            Val::FuncRef(None) => ValType::NULLFUNCREF,
            Val::FuncRef(Some(f)) => ValType::Ref(RefType::new(
                false,
                HeapType::ConcreteFunc(f.load_ty(store)),
            )),
            Val::AnyRef(None) => ValType::NULLREF,
            Val::AnyRef(Some(a)) => ValType::Ref(RefType::new(false, a._ty(store)?)),
            Val::ExnRef(None) => ValType::NULLEXNREF,
            Val::ExnRef(Some(e)) => {
                e.try_gc_ref(store)?;
                ValType::Ref(RefType::new(false, HeapType::Exn))
            }
            Val::Frozen(f) => ValType::Ref(RefType::new(false, f.heap_type(store)?)),
            Val::ContRef(_) => {
                // TODO(#10248): Return proper continuation reference type when available
                return Err(crate::format_err!(
                    "continuation references not yet supported in embedder API"
                ));
            }
        })
    }

    /// Does this value match the given type?
    ///
    /// Returns an error is an underlying `Rooted` has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this value is not associated with the given store.
    pub fn matches_ty(&self, store: impl AsContext, ty: &ValType) -> Result<bool> {
        self._matches_ty(&store.as_context().0, ty)
    }

    pub(crate) fn _matches_ty(&self, store: &StoreOpaque, ty: &ValType) -> Result<bool> {
        assert!(self.comes_from_same_store(store));
        assert!(ty.comes_from_same_engine(store.engine()));
        Ok(match (self, ty) {
            (Val::I32(_), ValType::I32)
            | (Val::I64(_), ValType::I64)
            | (Val::F32(_), ValType::F32)
            | (Val::F64(_), ValType::F64)
            | (Val::V128(_), ValType::V128) => true,

            (Val::FuncRef(f), ValType::Ref(ref_ty)) => Ref::from(*f)._matches_ty(store, ref_ty)?,
            (Val::ExternRef(e), ValType::Ref(ref_ty)) => {
                Ref::from(*e)._matches_ty(store, ref_ty)?
            }
            (Val::AnyRef(a), ValType::Ref(ref_ty)) => Ref::from(*a)._matches_ty(store, ref_ty)?,
            (Val::ExnRef(e), ValType::Ref(ref_ty)) => Ref::from(*e)._matches_ty(store, ref_ty)?,
            (Val::Frozen(f), ValType::Ref(ref_ty)) => Ref::Frozen(*f)._matches_ty(store, ref_ty)?,

            (Val::I32(_), _)
            | (Val::I64(_), _)
            | (Val::F32(_), _)
            | (Val::F64(_), _)
            | (Val::V128(_), _)
            | (Val::FuncRef(_), _)
            | (Val::ExternRef(_), _)
            | (Val::AnyRef(_), _)
            | (Val::ExnRef(_), _)
            | (Val::Frozen(_), _)
            | (Val::ContRef(_), _) => false,
        })
    }

    pub(crate) fn ensure_matches_ty(&self, store: &StoreOpaque, ty: &ValType) -> Result<()> {
        if !self.comes_from_same_store(store) {
            bail!("value used with wrong store")
        }
        if !ty.comes_from_same_engine(store.engine()) {
            bail!("type used with wrong engine")
        }
        if self._matches_ty(store, ty)? {
            Ok(())
        } else {
            let actual_ty = self.load_ty(store)?;
            bail!("type mismatch: expected {ty}, found {actual_ty}")
        }
    }

    /// Convenience method to convert this [`Val`] into a [`ValRaw`].
    ///
    /// Returns an error if this value is a GC reference and the GC reference
    /// has been unrooted.
    ///
    /// # Safety
    ///
    /// The returned [`ValRaw`] does not carry type information and is only safe
    /// to use within the context of this store itself. For more information see
    /// [`ExternRef::to_raw`] and [`Func::to_raw`].
    pub fn to_raw(&self, mut store: impl AsContextMut) -> Result<ValRaw> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        self.to_raw_(&mut store)
    }

    pub(crate) fn to_raw_(&self, store: &mut AutoAssertNoGc) -> Result<ValRaw> {
        match self {
            Val::I32(i) => Ok(ValRaw::i32(*i)),
            Val::I64(i) => Ok(ValRaw::i64(*i)),
            Val::F32(u) => Ok(ValRaw::f32(*u)),
            Val::F64(u) => Ok(ValRaw::f64(*u)),
            Val::V128(b) => Ok(ValRaw::v128(b.as_u128())),
            Val::ExternRef(e) => Ok(ValRaw::externref(match e {
                None => 0,
                Some(e) => e._to_raw(store)?,
            })),
            Val::AnyRef(e) => Ok(ValRaw::anyref(match e {
                None => 0,
                Some(e) => e._to_raw(store)?,
            })),
            Val::ExnRef(e) => Ok(ValRaw::exnref(match e {
                None => 0,
                Some(e) => e._to_raw(store)?,
            })),
            Val::FuncRef(f) => Ok(ValRaw::funcref(match f {
                Some(f) => f.to_raw_(store),
                None => ptr::null_mut(),
            })),
            Val::Frozen(f) => {
                let gc_ref = f.clone_gc_ref(store)?;
                let raw = store
                    .unwrap_gc_store_mut()
                    .expose_gc_ref_to_wasm(gc_ref)?
                    .get();
                Ok(match f {
                    FrozenRef::Extern(_) => ValRaw::externref(raw),
                    FrozenRef::Any(_) => ValRaw::anyref(raw),
                    FrozenRef::Exn(_) => ValRaw::exnref(raw),
                })
            }
            Val::ContRef(_) => {
                // TODO(#10248): Implement proper continuation reference to_raw conversion
                Err(crate::format_err!(
                    "continuation references not yet supported in to_raw conversion"
                ))
            }
        }
    }

    /// Convenience method to convert a [`ValRaw`] into a [`Val`].
    ///
    /// # Unsafety
    ///
    /// This method is unsafe for the reasons that [`ExternRef::from_raw`] and
    /// [`Func::from_raw`] are unsafe. Additionally there's no guarantee
    /// otherwise that `raw` should have the type `ty` specified.
    pub unsafe fn from_raw(mut store: impl AsContextMut, raw: ValRaw, ty: ValType) -> Val {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        // SAFETY: `_from_raw` has the same contract as this function.
        unsafe { Self::_from_raw(&mut store, raw, &ty) }
    }

    /// Same as [`Self::from_raw`], but with a monomorphic store.
    pub(crate) unsafe fn _from_raw(
        store: &mut AutoAssertNoGc<'_>,
        raw: ValRaw,
        ty: &ValType,
    ) -> Val {
        match ty {
            ValType::I32 => Val::I32(raw.get_i32()),
            ValType::I64 => Val::I64(raw.get_i64()),
            ValType::F32 => Val::F32(raw.get_f32()),
            ValType::F64 => Val::F64(raw.get_f64()),
            ValType::V128 => Val::V128(raw.get_v128().into()),
            ValType::Ref(ref_ty) => {
                let ref_ = match ref_ty.heap_type() {
                    // SAFETY: it's a safety contract of this function that the
                    // funcref is valid and owned by the provided store.
                    HeapType::Func | HeapType::ConcreteFunc(_) => unsafe {
                        Func::_from_raw(store, raw.get_funcref()).into()
                    },

                    HeapType::NoFunc => Ref::Func(None),

                    HeapType::NoCont | HeapType::ConcreteCont(_) | HeapType::Cont => {
                        // TODO(#10248): Required to support stack switching in the embedder API.
                        unimplemented!()
                    }

                    HeapType::Extern => ExternRef::_from_raw(store, raw.get_externref()).into(),

                    HeapType::NoExtern => Ref::Extern(None),

                    HeapType::Any
                    | HeapType::Eq
                    | HeapType::I31
                    | HeapType::Array
                    | HeapType::ConcreteArray(_)
                    | HeapType::Struct
                    | HeapType::ConcreteStruct(_) => {
                        AnyRef::_from_raw(store, raw.get_anyref()).into()
                    }

                    HeapType::Exn => ExnRef::_from_raw(store, raw.get_exnref()).into(),
                    HeapType::NoExn => Ref::Exn(None),

                    HeapType::None => Ref::Any(None),
                };
                assert!(
                    ref_ty.is_nullable() || !ref_.is_null(),
                    "if the type is not nullable, we shouldn't get null; got \
                     type = {ref_ty}, ref = {ref_:?}"
                );
                ref_.into()
            }
        }
    }

    accessors! {
        e
        (I32(i32) i32 unwrap_i32 *e)
        (I64(i64) i64 unwrap_i64 *e)
        (F32(f32) f32 unwrap_f32 f32::from_bits(*e))
        (F64(f64) f64 unwrap_f64 f64::from_bits(*e))
        (FuncRef(Option<&Func>) func_ref unwrap_func_ref e.as_ref())
        (ExternRef(Option<&Rooted<ExternRef>>) extern_ref unwrap_extern_ref e.as_ref())
        (AnyRef(Option<&Rooted<AnyRef>>) any_ref unwrap_any_ref e.as_ref())
        (V128(V128) v128 unwrap_v128 *e)
    }

    /// Get this value's underlying reference, if any.
    #[inline]
    pub fn ref_(self) -> Option<Ref> {
        match self {
            Val::FuncRef(f) => Some(Ref::Func(f)),
            Val::ExternRef(e) => Some(Ref::Extern(e)),
            Val::AnyRef(a) => Some(Ref::Any(a)),
            Val::ExnRef(e) => Some(Ref::Exn(e)),
            Val::Frozen(f) => Some(Ref::Frozen(f)),
            Val::I32(_) | Val::I64(_) | Val::F32(_) | Val::F64(_) | Val::V128(_) => None,
            Val::ContRef(_) => None, // TODO(#10248): Return proper Ref::Cont when available
        }
    }

    /// Attempt to access the underlying `externref` value of this `Val`.
    ///
    /// If this is not an `externref`, then `None` is returned.
    ///
    /// If this is a null `externref`, then `Some(None)` is returned.
    ///
    /// If this is a non-null `externref`, then `Some(Some(..))` is returned.
    #[inline]
    pub fn externref(&self) -> Option<Option<&Rooted<ExternRef>>> {
        match self {
            Val::ExternRef(None) => Some(None),
            Val::ExternRef(Some(e)) => Some(Some(e)),
            _ => None,
        }
    }

    /// Returns the underlying `externref` value of this `Val`, panicking if it's the
    /// wrong type.
    ///
    /// If this is a null `externref`, then `None` is returned.
    ///
    /// If this is a non-null `externref`, then `Some(..)` is returned.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not a (nullable) `externref`.
    #[inline]
    pub fn unwrap_externref(&self) -> Option<&Rooted<ExternRef>> {
        self.externref().expect("expected externref")
    }

    /// Attempt to access the underlying `anyref` value of this `Val`.
    ///
    /// If this is not an `anyref`, then `None` is returned.
    ///
    /// If this is a null `anyref`, then `Some(None)` is returned.
    ///
    /// If this is a non-null `anyref`, then `Some(Some(..))` is returned.
    #[inline]
    pub fn anyref(&self) -> Option<Option<&Rooted<AnyRef>>> {
        match self {
            Val::AnyRef(None) => Some(None),
            Val::AnyRef(Some(e)) => Some(Some(e)),
            _ => None,
        }
    }

    /// Returns the underlying `anyref` value of this `Val`, panicking if it's the
    /// wrong type.
    ///
    /// If this is a null `anyref`, then `None` is returned.
    ///
    /// If this is a non-null `anyref`, then `Some(..)` is returned.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not a (nullable) `anyref`.
    #[inline]
    pub fn unwrap_anyref(&self) -> Option<&Rooted<AnyRef>> {
        self.anyref().expect("expected anyref")
    }

    /// Attempt to access the underlying `exnref` value of this `Val`.
    ///
    /// If this is not an `exnref`, then `None` is returned.
    ///
    /// If this is a null `exnref`, then `Some(None)` is returned.
    ///
    /// If this is a non-null `exnref`, then `Some(Some(..))` is returned.
    #[inline]
    pub fn exnref(&self) -> Option<Option<&Rooted<ExnRef>>> {
        match self {
            Val::ExnRef(None) => Some(None),
            Val::ExnRef(Some(e)) => Some(Some(e)),
            _ => None,
        }
    }

    /// Returns the underlying `exnref` value of this `Val`, panicking if it's the
    /// wrong type.
    ///
    /// If this is a null `exnref`, then `None` is returned.
    ///
    /// If this is a non-null `exnref`, then `Some(..)` is returned.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not a (nullable) `exnref`.
    #[inline]
    pub fn unwrap_exnref(&self) -> Option<&Rooted<ExnRef>> {
        self.exnref().expect("expected exnref")
    }

    /// Attempt to access the underlying `funcref` value of this `Val`.
    ///
    /// If this is not an `funcref`, then `None` is returned.
    ///
    /// If this is a null `funcref`, then `Some(None)` is returned.
    ///
    /// If this is a non-null `funcref`, then `Some(Some(..))` is returned.
    #[inline]
    pub fn funcref(&self) -> Option<Option<&Func>> {
        match self {
            Val::FuncRef(None) => Some(None),
            Val::FuncRef(Some(f)) => Some(Some(f)),
            _ => None,
        }
    }

    /// Returns the underlying `funcref` value of this `Val`, panicking if it's the
    /// wrong type.
    ///
    /// If this is a null `funcref`, then `None` is returned.
    ///
    /// If this is a non-null `funcref`, then `Some(..)` is returned.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not a (nullable) `funcref`.
    #[inline]
    pub fn unwrap_funcref(&self) -> Option<&Func> {
        self.funcref().expect("expected funcref")
    }

    /// Attempt to access the underlying frozen `anyref` value of this `Val`,
    /// as returned by an accessor such as [`Global::get`](crate::Global::get).
    ///
    /// If this is not a null `anyref` or a frozen one, then `None` is
    /// returned.
    ///
    /// If this is a null `anyref`, then `Some(None)` is returned.
    ///
    /// If this is a frozen `anyref`, then `Some(Some(..))` is returned.
    #[inline]
    pub fn frozen_anyref(&self) -> Option<Option<Frozen<AnyRef>>> {
        match self {
            Val::AnyRef(None) => Some(None),
            Val::Frozen(FrozenRef::Any(a)) => Some(Some(*a)),
            _ => None,
        }
    }

    /// Returns the underlying frozen `anyref` value of this `Val`; see
    /// [`Val::frozen_anyref`].
    ///
    /// # Panics
    ///
    /// Panics if `self` is not a null or frozen `anyref`.
    #[inline]
    pub fn unwrap_frozen_anyref(&self) -> Option<Frozen<AnyRef>> {
        self.frozen_anyref().expect("expected frozen anyref")
    }

    /// Attempt to access the underlying frozen `exnref` value of this `Val`;
    /// see [`Val::frozen_anyref`].
    #[inline]
    pub fn frozen_exnref(&self) -> Option<Option<Frozen<ExnRef>>> {
        match self {
            Val::ExnRef(None) => Some(None),
            Val::Frozen(FrozenRef::Exn(e)) => Some(Some(*e)),
            _ => None,
        }
    }

    /// Returns the underlying frozen `exnref` value of this `Val`; see
    /// [`Val::frozen_anyref`].
    ///
    /// # Panics
    ///
    /// Panics if `self` is not a null or frozen `exnref`.
    #[inline]
    pub fn unwrap_frozen_exnref(&self) -> Option<Frozen<ExnRef>> {
        self.frozen_exnref().expect("expected frozen exnref")
    }

    /// Attempt to access the underlying frozen `externref` value of this
    /// `Val`; see [`Val::frozen_anyref`].
    #[inline]
    pub fn frozen_externref(&self) -> Option<Option<Frozen<ExternRef>>> {
        match self {
            Val::ExternRef(None) => Some(None),
            Val::Frozen(FrozenRef::Extern(e)) => Some(Some(*e)),
            _ => None,
        }
    }

    /// Returns the underlying frozen `externref` value of this `Val`; see
    /// [`Val::frozen_anyref`].
    ///
    /// # Panics
    ///
    /// Panics if `self` is not a null or frozen `externref`.
    #[inline]
    pub fn unwrap_frozen_externref(&self) -> Option<Frozen<ExternRef>> {
        self.frozen_externref().expect("expected frozen externref")
    }

    /// Roots this value's GC reference in the current LIFO rooting scope if
    /// it is [frozen](Val::Frozen), to keep it alive and usable after the
    /// frozen reference becomes invalid. Other values are returned as they
    /// are.
    ///
    /// # Errors
    ///
    /// Returns an error if this value is a frozen reference that is no longer
    /// valid.
    ///
    /// # Panics
    ///
    /// Panics if this value is associated with a different store.
    pub fn to_rooted(&self, store: impl AsContextMut) -> Result<Val> {
        match self {
            Val::Frozen(f) => Ok(f.to_rooted(store)?.into()),
            v => Ok(*v),
        }
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match self {
            Val::FuncRef(Some(f)) => f.comes_from_same_store(store),
            Val::FuncRef(None) => true,

            Val::ExternRef(Some(x)) => x.comes_from_same_store(store),
            Val::ExternRef(None) => true,

            Val::AnyRef(Some(a)) => a.comes_from_same_store(store),
            Val::AnyRef(None) => true,

            Val::ExnRef(Some(e)) => e.comes_from_same_store(store),
            Val::ExnRef(None) => true,

            Val::Frozen(f) => f.comes_from_same_store(store),

            // Integers, floats, and vectors have no association with any
            // particular store, so they're always considered as "yes I came
            // from that store",
            Val::I32(_) | Val::I64(_) | Val::F32(_) | Val::F64(_) | Val::V128(_) => true,

            // Continuation references are not yet associated with stores
            Val::ContRef(_) => true, // TODO(#10248): Proper store association when implemented
        }
    }
}

impl From<i32> for Val {
    #[inline]
    fn from(val: i32) -> Val {
        Val::I32(val)
    }
}

impl From<i64> for Val {
    #[inline]
    fn from(val: i64) -> Val {
        Val::I64(val)
    }
}

impl From<f32> for Val {
    #[inline]
    fn from(val: f32) -> Val {
        Val::F32(val.to_bits())
    }
}

impl From<f64> for Val {
    #[inline]
    fn from(val: f64) -> Val {
        Val::F64(val.to_bits())
    }
}

impl From<Ref> for Val {
    #[inline]
    fn from(val: Ref) -> Val {
        match val {
            Ref::Extern(e) => Val::ExternRef(e),
            Ref::Func(f) => Val::FuncRef(f),
            Ref::Any(a) => Val::AnyRef(a),
            Ref::Exn(e) => Val::ExnRef(e),
            Ref::Frozen(f) => Val::Frozen(f),
        }
    }
}

impl From<Rooted<ExternRef>> for Val {
    #[inline]
    fn from(val: Rooted<ExternRef>) -> Val {
        Val::ExternRef(Some(val))
    }
}

impl From<Option<Rooted<ExternRef>>> for Val {
    #[inline]
    fn from(val: Option<Rooted<ExternRef>>) -> Val {
        Val::ExternRef(val)
    }
}

impl From<Rooted<AnyRef>> for Val {
    #[inline]
    fn from(val: Rooted<AnyRef>) -> Val {
        Val::AnyRef(Some(val))
    }
}

impl From<Option<Rooted<AnyRef>>> for Val {
    #[inline]
    fn from(val: Option<Rooted<AnyRef>>) -> Val {
        Val::AnyRef(val)
    }
}

impl From<Rooted<StructRef>> for Val {
    #[inline]
    fn from(val: Rooted<StructRef>) -> Val {
        Val::AnyRef(Some(val.into()))
    }
}

impl From<Option<Rooted<StructRef>>> for Val {
    #[inline]
    fn from(val: Option<Rooted<StructRef>>) -> Val {
        Val::AnyRef(val.map(Into::into))
    }
}

impl From<Rooted<ArrayRef>> for Val {
    #[inline]
    fn from(val: Rooted<ArrayRef>) -> Val {
        Val::AnyRef(Some(val.into()))
    }
}

impl From<Option<Rooted<ArrayRef>>> for Val {
    #[inline]
    fn from(val: Option<Rooted<ArrayRef>>) -> Val {
        Val::AnyRef(val.map(Into::into))
    }
}

impl From<Rooted<ExnRef>> for Val {
    #[inline]
    fn from(val: Rooted<ExnRef>) -> Val {
        Val::ExnRef(Some(val))
    }
}

impl From<Option<Rooted<ExnRef>>> for Val {
    #[inline]
    fn from(val: Option<Rooted<ExnRef>>) -> Val {
        Val::ExnRef(val)
    }
}

impl From<Func> for Val {
    #[inline]
    fn from(val: Func) -> Val {
        Val::FuncRef(Some(val))
    }
}

impl From<Option<Func>> for Val {
    #[inline]
    fn from(val: Option<Func>) -> Val {
        Val::FuncRef(val)
    }
}

impl From<u128> for Val {
    #[inline]
    fn from(val: u128) -> Val {
        Val::V128(val.into())
    }
}

impl From<V128> for Val {
    #[inline]
    fn from(val: V128) -> Val {
        Val::V128(val)
    }
}

/// A reference.
///
/// References come in three broad flavors:
///
/// 1. Function references. These are references to a function that can be
///    invoked.
///
/// 2. External references. These are references to data that is external
///    and opaque to the Wasm guest, provided by the host.
///
/// 3. Internal references. These are references to allocations inside the
///    Wasm's heap, such as structs and arrays. These are part of the GC
///    proposal, and not yet implemented in Wasmtime.
///
/// At the Wasm level, there are nullable and non-nullable variants of each type
/// of reference. Both variants are represented with `Ref` at the Wasmtime API
/// level. For example, values of both `(ref extern)` and `(ref null extern)`
/// types will be represented as `Ref::Extern(Option<ExternRef>)` in the
/// Wasmtime API. Nullable references are represented as `Option<Ref>` where
/// null references are represented as `None`. Wasm can construct null
/// references via the `ref.null <heap-type>` instruction.
///
/// References are non-forgable: Wasm cannot create invalid references, for
/// example, by claiming that the integer `0xbad1bad2` is actually a reference.
#[derive(Debug, Clone)]
pub enum Ref {
    // NB: We have a variant for each of the type hierarchies defined in Wasm,
    // and push the `Option` that provides nullability into each variant. This
    // allows us to get the most-precise type of any reference value, whether it
    // is null or not, without any additional metadata.
    //
    // Consider if we instead had the nullability inside `Val::Ref` and each of
    // the `Ref` variants did not have an `Option`:
    //
    //     enum Val {
    //         Ref(Option<Ref>),
    //         // Etc...
    //     }
    //     enum Ref {
    //         Func(Func),
    //         External(ExternRef),
    //         // Etc...
    //     }
    //
    // In this scenario, what type would we return from `Val::ty` for
    // `Val::Ref(None)`? Because Wasm has multiple separate type hierarchies,
    // there is no single common bottom type for all the different kinds of
    // references. So in this scenario, `Val::Ref(None)` doesn't have enough
    // information to reconstruct the value's type. That's a problem for us
    // because we need to get a value's type at various times all over the code
    // base.
    //
    /// A first-class reference to a WebAssembly function.
    ///
    /// The host, or the Wasm guest, can invoke this function.
    ///
    /// The host can create function references via [`Func::new`] or
    /// [`Func::wrap`].
    ///
    /// The Wasm guest can create non-null function references via the
    /// `ref.func` instruction, or null references via the `ref.null func`
    /// instruction.
    Func(Option<Func>),

    /// A reference to an value outside of the Wasm heap.
    ///
    /// These references are opaque to the Wasm itself. Wasm can't create
    /// non-null external references, nor do anything with them accept pass them
    /// around as function arguments and returns and place them into globals and
    /// tables.
    ///
    /// Wasm can create null external references via the `ref.null extern`
    /// instruction.
    Extern(Option<Rooted<ExternRef>>),

    /// An internal reference.
    ///
    /// The `AnyRef` type represents WebAssembly `anyref` values. These can be
    /// references to `struct`s and `array`s or inline/unboxed 31-bit
    /// integers.
    ///
    /// Unlike `externref`, Wasm guests can directly allocate `anyref`s, and
    /// does not need to rely on the host to do that.
    Any(Option<Rooted<AnyRef>>),

    /// An exception-object reference.
    ///
    /// The `ExnRef` type represents WebAssembly `exnref`
    /// values. These are references to exception objects as caught by
    /// `catch_ref` clauses on `try_table` instructions, or as
    /// allocated via the host API.
    Exn(Option<Rooted<ExnRef>>),

    /// A non-null GC reference, as a [`Frozen`] reference; see
    /// [`Val::Frozen`].
    Frozen(FrozenRef),
}

impl From<FrozenRef> for Ref {
    #[inline]
    fn from(f: FrozenRef) -> Ref {
        Ref::Frozen(f)
    }
}

impl From<FrozenRef> for Val {
    #[inline]
    fn from(f: FrozenRef) -> Val {
        Val::Frozen(f)
    }
}

impl From<Func> for Ref {
    #[inline]
    fn from(f: Func) -> Ref {
        Ref::Func(Some(f))
    }
}

impl From<Option<Func>> for Ref {
    #[inline]
    fn from(f: Option<Func>) -> Ref {
        Ref::Func(f)
    }
}

impl From<Rooted<ExternRef>> for Ref {
    #[inline]
    fn from(e: Rooted<ExternRef>) -> Ref {
        Ref::Extern(Some(e))
    }
}

impl From<Option<Rooted<ExternRef>>> for Ref {
    #[inline]
    fn from(e: Option<Rooted<ExternRef>>) -> Ref {
        Ref::Extern(e)
    }
}

impl From<Rooted<AnyRef>> for Ref {
    #[inline]
    fn from(e: Rooted<AnyRef>) -> Ref {
        Ref::Any(Some(e))
    }
}

impl From<Option<Rooted<AnyRef>>> for Ref {
    #[inline]
    fn from(e: Option<Rooted<AnyRef>>) -> Ref {
        Ref::Any(e)
    }
}

impl From<Rooted<StructRef>> for Ref {
    #[inline]
    fn from(e: Rooted<StructRef>) -> Ref {
        Ref::Any(Some(e.into()))
    }
}

impl From<Option<Rooted<StructRef>>> for Ref {
    #[inline]
    fn from(e: Option<Rooted<StructRef>>) -> Ref {
        Ref::Any(e.map(Into::into))
    }
}

impl From<Rooted<ArrayRef>> for Ref {
    #[inline]
    fn from(e: Rooted<ArrayRef>) -> Ref {
        Ref::Any(Some(e.into()))
    }
}

impl From<Option<Rooted<ArrayRef>>> for Ref {
    #[inline]
    fn from(e: Option<Rooted<ArrayRef>>) -> Ref {
        Ref::Any(e.map(Into::into))
    }
}

impl From<Rooted<ExnRef>> for Ref {
    #[inline]
    fn from(e: Rooted<ExnRef>) -> Ref {
        Ref::Exn(Some(e))
    }
}

impl From<Option<Rooted<ExnRef>>> for Ref {
    #[inline]
    fn from(e: Option<Rooted<ExnRef>>) -> Ref {
        Ref::Exn(e)
    }
}

impl Ref {
    /// Create a null reference to the given heap type.
    #[inline]
    pub fn null(heap_type: &HeapType) -> Self {
        match heap_type.top() {
            HeapTopType::Any => Ref::Any(None),
            HeapTopType::Extern => Ref::Extern(None),
            HeapTopType::Func => Ref::Func(None),
            HeapTopType::Exn => Ref::Exn(None),
            HeapTopType::Cont => unimplemented!("embedding API for `(ref cont)`"),
        }
    }

    /// Is this a null reference?
    #[inline]
    pub fn is_null(&self) -> bool {
        match self {
            Ref::Any(None) | Ref::Extern(None) | Ref::Func(None) | Ref::Exn(None) => true,
            Ref::Any(Some(_))
            | Ref::Extern(Some(_))
            | Ref::Func(Some(_))
            | Ref::Exn(Some(_))
            | Ref::Frozen(_) => false,
        }
    }

    /// Is this a non-null reference?
    #[inline]
    pub fn is_non_null(&self) -> bool {
        !self.is_null()
    }

    /// Is this an `extern` reference?
    #[inline]
    pub fn is_extern(&self) -> bool {
        matches!(self, Ref::Extern(_))
    }

    /// Get the underlying `extern` reference, if any.
    ///
    /// Returns `None` if this `Ref` is not an `extern` reference, eg it is a
    /// `func` reference.
    ///
    /// Returns `Some(None)` if this `Ref` is a null `extern` reference.
    ///
    /// Returns `Some(Some(_))` if this `Ref` is a non-null `extern` reference.
    #[inline]
    pub fn as_extern(&self) -> Option<Option<&Rooted<ExternRef>>> {
        match self {
            Ref::Extern(e) => Some(e.as_ref()),
            _ => None,
        }
    }

    /// Get the underlying `extern` reference, panicking if this is a different
    /// kind of reference.
    ///
    /// Returns `None` if this `Ref` is a null `extern` reference.
    ///
    /// Returns `Some(_)` if this `Ref` is a non-null `extern` reference.
    #[inline]
    pub fn unwrap_extern(&self) -> Option<&Rooted<ExternRef>> {
        self.as_extern()
            .expect("Ref::unwrap_extern on non-extern reference")
    }

    /// Is this an `any` reference?
    #[inline]
    pub fn is_any(&self) -> bool {
        matches!(self, Ref::Any(_))
    }

    /// Get the underlying `any` reference, if any.
    ///
    /// Returns `None` if this `Ref` is not an `any` reference, eg it is a
    /// `func` reference.
    ///
    /// Returns `Some(None)` if this `Ref` is a null `any` reference.
    ///
    /// Returns `Some(Some(_))` if this `Ref` is a non-null `any` reference.
    #[inline]
    pub fn as_any(&self) -> Option<Option<&Rooted<AnyRef>>> {
        match self {
            Ref::Any(e) => Some(e.as_ref()),
            _ => None,
        }
    }

    /// Get the underlying `any` reference, panicking if this is a different
    /// kind of reference.
    ///
    /// Returns `None` if this `Ref` is a null `any` reference.
    ///
    /// Returns `Some(_)` if this `Ref` is a non-null `any` reference.
    #[inline]
    pub fn unwrap_any(&self) -> Option<&Rooted<AnyRef>> {
        self.as_any().expect("Ref::unwrap_any on non-any reference")
    }

    /// Is this an `exn` reference?
    #[inline]
    pub fn is_exn(&self) -> bool {
        matches!(self, Ref::Exn(_))
    }

    /// Get the underlying `exn` reference, if any.
    ///
    /// Returns `None` if this `Ref` is not an `exn` reference, eg it is a
    /// `func` reference.
    ///
    /// Returns `Some(None)` if this `Ref` is a null `exn` reference.
    ///
    /// Returns `Some(Some(_))` if this `Ref` is a non-null `exn` reference.
    #[inline]
    pub fn as_exn(&self) -> Option<Option<&Rooted<ExnRef>>> {
        match self {
            Ref::Exn(e) => Some(e.as_ref()),
            _ => None,
        }
    }

    /// Get the underlying `exn` reference, panicking if this is a different
    /// kind of reference.
    ///
    /// Returns `None` if this `Ref` is a null `exn` reference.
    ///
    /// Returns `Some(_)` if this `Ref` is a non-null `exn` reference.
    #[inline]
    pub fn unwrap_exn(&self) -> Option<&Rooted<ExnRef>> {
        self.as_exn().expect("Ref::unwrap_exn on non-exn reference")
    }

    /// Is this a `func` reference?
    #[inline]
    pub fn is_func(&self) -> bool {
        matches!(self, Ref::Func(_))
    }

    /// Get the underlying `func` reference, if any.
    ///
    /// Returns `None` if this `Ref` is not an `func` reference, eg it is an
    /// `extern` reference.
    ///
    /// Returns `Some(None)` if this `Ref` is a null `func` reference.
    ///
    /// Returns `Some(Some(_))` if this `Ref` is a non-null `func` reference.
    #[inline]
    pub fn as_func(&self) -> Option<Option<&Func>> {
        match self {
            Ref::Func(f) => Some(f.as_ref()),
            _ => None,
        }
    }

    /// Get the underlying `func` reference, panicking if this is a different
    /// kind of reference.
    ///
    /// Returns `None` if this `Ref` is a null `func` reference.
    ///
    /// Returns `Some(_)` if this `Ref` is a non-null `func` reference.
    #[inline]
    pub fn unwrap_func(&self) -> Option<&Func> {
        self.as_func()
            .expect("Ref::unwrap_func on non-func reference")
    }

    /// Get the type of this reference.
    ///
    /// # Errors
    ///
    /// Return an error if this reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn ty(&self, store: impl AsContext) -> Result<RefType> {
        self.load_ty(&store.as_context().0)
    }

    pub(crate) fn load_ty(&self, store: &StoreOpaque) -> Result<RefType> {
        assert!(self.comes_from_same_store(store));
        Ok(RefType::new(
            self.is_null(),
            // NB: We choose the most-specific heap type we can here and let
            // subtyping do its thing if callers are matching against a
            // `HeapType::Func`.
            match self {
                Ref::Extern(None) => HeapType::NoExtern,
                Ref::Extern(Some(e)) => {
                    e.try_gc_ref(store)?;
                    HeapType::Extern
                }

                Ref::Func(None) => HeapType::NoFunc,
                Ref::Func(Some(f)) => HeapType::ConcreteFunc(f.load_ty(store)),

                Ref::Any(None) => HeapType::None,
                Ref::Any(Some(a)) => a._ty(store)?,

                Ref::Exn(None) => HeapType::NoExn,
                Ref::Exn(Some(e)) => {
                    e.try_gc_ref(store)?;
                    HeapType::Exn
                }

                Ref::Frozen(f) => f.heap_type(store)?,
            },
        ))
    }

    /// Does this reference value match the given type?
    ///
    /// Returns an error if the underlying `Rooted` has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is not associated with the given store.
    pub fn matches_ty(&self, store: impl AsContext, ty: &RefType) -> Result<bool> {
        self._matches_ty(&store.as_context().0, ty)
    }

    pub(crate) fn _matches_ty(&self, store: &StoreOpaque, ty: &RefType) -> Result<bool> {
        assert!(self.comes_from_same_store(store));
        assert!(ty.comes_from_same_engine(store.engine()));
        if self.is_null() && !ty.is_nullable() {
            return Ok(false);
        }
        Ok(match (self, ty.heap_type()) {
            (Ref::Extern(Some(e)), HeapType::Extern) => {
                e.try_gc_ref(store)?;
                true
            }
            (Ref::Extern(None), HeapType::Extern) => true,
            (Ref::Extern(None), HeapType::NoExtern) => true,
            (Ref::Extern(_), _) => false,

            (Ref::Func(_), HeapType::Func) => true,
            (Ref::Func(None), HeapType::NoFunc | HeapType::ConcreteFunc(_)) => true,
            (Ref::Func(Some(f)), HeapType::ConcreteFunc(func_ty)) => f._matches_ty(store, func_ty),
            (Ref::Func(_), _) => false,

            (Ref::Any(Some(a)), HeapType::Any) => {
                a.try_gc_ref(store)?;
                true
            }
            (Ref::Any(None), HeapType::Any) => true,
            (Ref::Any(Some(a)), HeapType::I31) => a._is_i31(store)?,
            (Ref::Any(Some(a)), HeapType::Struct) => a._is_struct(store)?,
            (Ref::Any(Some(a)), HeapType::ConcreteStruct(_ty)) => match a._as_struct(store)? {
                None => false,
                Some(s) => s._matches_ty(store, _ty)?,
            },
            (Ref::Any(Some(a)), HeapType::Eq) => a._is_eqref(store)?,
            (Ref::Any(Some(a)), HeapType::Array) => a._is_array(store)?,
            (Ref::Any(Some(a)), HeapType::ConcreteArray(_ty)) => match a._as_array(store)? {
                None => false,
                Some(a) => a._matches_ty(store, _ty)?,
            },
            (
                Ref::Any(None),
                HeapType::None
                | HeapType::I31
                | HeapType::ConcreteStruct(_)
                | HeapType::Struct
                | HeapType::ConcreteArray(_)
                | HeapType::Array
                | HeapType::Eq,
            ) => true,
            (Ref::Any(_), _) => false,

            (Ref::Exn(Some(e)), HeapType::Exn) => {
                e.try_gc_ref(store)?;
                true
            }
            (Ref::Exn(None), HeapType::Exn) => true,
            (Ref::Exn(None), HeapType::NoExn) => true,
            (Ref::Exn(_), _) => false,

            (Ref::Frozen(f), heap_ty) => f.heap_type(store)?.matches(heap_ty),
        })
    }

    pub(crate) fn ensure_matches_ty(&self, store: &StoreOpaque, ty: &RefType) -> Result<()> {
        if !self.comes_from_same_store(store) {
            bail!("reference used with wrong store")
        }
        if !ty.comes_from_same_engine(store.engine()) {
            bail!("type used with wrong engine")
        }
        if self._matches_ty(store, ty)? {
            Ok(())
        } else {
            let actual_ty = self.load_ty(store)?;
            bail!("type mismatch: expected {ty}, found {actual_ty}")
        }
    }

    /// Roots this reference in the current LIFO rooting scope if it is
    /// [frozen](Ref::Frozen); see [`Val::to_rooted`].
    ///
    /// # Errors
    ///
    /// Returns an error if this is a frozen reference that is no longer
    /// valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn to_rooted(&self, store: impl AsContextMut) -> Result<Ref> {
        match self {
            Ref::Frozen(f) => f.to_rooted(store),
            r => Ok(r.clone()),
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match self {
            Ref::Func(Some(f)) => f.comes_from_same_store(store),
            Ref::Func(None) => true,
            Ref::Extern(Some(x)) => x.comes_from_same_store(store),
            Ref::Extern(None) => true,
            Ref::Any(Some(a)) => a.comes_from_same_store(store),
            Ref::Any(None) => true,
            Ref::Exn(Some(e)) => e.comes_from_same_store(store),
            Ref::Exn(None) => true,
            Ref::Frozen(f) => f.comes_from_same_store(store),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn size_of_val() {
        // Try to keep tabs on the size of `Val` and make sure we don't grow its
        // size.
        //
        // The largest variant is `Frozen`: a `Frozen` reference holds a 64-bit
        // store ID, a 64-bit GC epoch, and a 32-bit raw reference, and
        // `FrozenRef` adds a tag. On x86, `u64` is only 4-byte aligned, so
        // this packs into 24 bytes; elsewhere padding makes it 32.
        let expected = if cfg!(target_arch = "x86_64")
            || cfg!(target_arch = "aarch64")
            || cfg!(target_arch = "s390x")
            || cfg!(target_arch = "riscv64")
            || cfg!(target_arch = "arm")
        {
            32
        } else if cfg!(target_arch = "x86") {
            24
        } else {
            panic!("unsupported architecture")
        };
        assert_eq!(std::mem::size_of::<Val>(), expected);
    }

    #[test]
    fn size_of_ref() {
        // Try to keep tabs on the size of `Ref` and make sure we don't grow its
        // size.
        //
        // The largest variant is `Frozen`: a `Frozen` reference holds a 64-bit
        // store ID, a 64-bit GC epoch, and a 32-bit raw reference, and
        // `FrozenRef` adds a tag. On x86, `u64` is only 4-byte aligned, so
        // this packs into 24 bytes; elsewhere padding makes it 32.
        let expected = if cfg!(target_arch = "x86_64")
            || cfg!(target_arch = "aarch64")
            || cfg!(target_arch = "s390x")
            || cfg!(target_arch = "riscv64")
            || cfg!(target_arch = "arm")
        {
            32
        } else if cfg!(target_arch = "x86") {
            24
        } else {
            panic!("unsupported architecture")
        };
        assert_eq!(std::mem::size_of::<Ref>(), expected);
    }

    #[test]
    #[should_panic]
    fn val_matches_ty_wrong_engine() {
        let e1 = Engine::default();
        let e2 = Engine::default();

        let t1 = FuncType::new(&e1, None, None);
        let t2 = FuncType::new(&e2, None, None);

        let mut s1 = Store::new(&e1, ());
        let f = Func::new(&mut s1, t1.clone(), |_caller, _args, _results| Ok(()));

        // Should panic.
        let _ = Val::FuncRef(Some(f)).matches_ty(
            &s1,
            &ValType::Ref(RefType::new(true, HeapType::ConcreteFunc(t2))),
        );
    }

    #[test]
    #[should_panic]
    fn ref_matches_ty_wrong_engine() {
        let e1 = Engine::default();
        let e2 = Engine::default();

        let t1 = FuncType::new(&e1, None, None);
        let t2 = FuncType::new(&e2, None, None);

        let mut s1 = Store::new(&e1, ());
        let f = Func::new(&mut s1, t1.clone(), |_caller, _args, _results| Ok(()));

        // Should panic.
        let _ = Ref::Func(Some(f)).matches_ty(&s1, &RefType::new(true, HeapType::ConcreteFunc(t2)));
    }
}

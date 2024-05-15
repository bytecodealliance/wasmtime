#![cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    allow(unused_imports)
)]

use crate::runtime::vm::V128Abi;
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::{Result, ValRaw, ValType, WasmTy};
use core::cmp::Ordering;
use core::fmt;
use core::mem::MaybeUninit;

/// Representation of a 128-bit vector type, `v128`, for WebAssembly.
///
/// This type corresponds to the `v128` type in WebAssembly and can be used with
/// the [`TypedFunc`] API for example. This is additionally
/// the payload of [`Val::V128`](crate::Val).
///
/// # Platform specifics
///
/// This type can currently only be used on x86_64 and AArch64 with the
/// [`TypedFunc`] API. Rust does not have stable support on other platforms for
/// this type so invoking functions with `v128` parameters requires the
/// [`Func::call`](crate::Func::call) API (or perhaps
/// [`Func::call_unchecked`](crate::Func::call_unchecked).
///
/// [`TypedFunc`]: crate::TypedFunc
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct V128(V128Abi);

union Reinterpret {
    abi: V128Abi,
    u128: u128,
}

impl V128 {
    /// Returns the representation of this `v128` as a 128-bit integer in Rust.
    pub fn as_u128(&self) -> u128 {
        unsafe { Reinterpret { abi: self.0 }.u128 }
    }
}

/// Primary constructor of a `V128` type.
impl From<u128> for V128 {
    fn from(val: u128) -> V128 {
        unsafe { V128(Reinterpret { u128: val }.abi) }
    }
}

impl From<V128> for u128 {
    fn from(val: V128) -> u128 {
        val.as_u128()
    }
}

impl fmt::Debug for V128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_u128().fmt(f)
    }
}

impl PartialEq for V128 {
    fn eq(&self, other: &V128) -> bool {
        self.as_u128() == other.as_u128()
    }
}

impl Eq for V128 {}

impl PartialOrd for V128 {
    fn partial_cmp(&self, other: &V128) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for V128 {
    fn cmp(&self, other: &V128) -> Ordering {
        self.as_u128().cmp(&other.as_u128())
    }
}

// Note that this trait is conditionally implemented which is intentional. See
// the documentation above in the `cfg_if!` for why this is conditional.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
unsafe impl WasmTy for V128 {
    #[inline]
    fn valtype() -> ValType {
        ValType::V128
    }

    #[inline]
    fn compatible_with_store(&self, _: &StoreOpaque) -> bool {
        true
    }

    fn dynamic_concrete_type_check(
        &self,
        _: &StoreOpaque,
        _: bool,
        _: &crate::HeapType,
    ) -> anyhow::Result<()> {
        unreachable!()
    }

    #[inline]
    fn store(self, _store: &mut AutoAssertNoGc<'_>, ptr: &mut MaybeUninit<ValRaw>) -> Result<()> {
        ptr.write(ValRaw::v128(self.as_u128()));
        Ok(())
    }

    #[inline]
    unsafe fn load(_store: &mut AutoAssertNoGc<'_>, ptr: &ValRaw) -> Self {
        V128::from(ptr.get_v128())
    }
}

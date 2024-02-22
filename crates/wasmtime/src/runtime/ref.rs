#[cfg(feature = "gc")]
mod gc_ref;
#[cfg(feature = "gc")]
pub use gc_ref::*;

#[cfg(not(feature = "gc"))]
mod no_gc_ref;
#[cfg(not(feature = "gc"))]
pub use no_gc_ref::*;

use std::ops::Deref;

/// A common trait implemented by all garbage-collected reference types.
///
/// This is a sealed trait, and may not be implemented for any types outside of
/// the `wasmtime` crate.
pub trait GcRef: GcRefImpl {}

impl<T> GcRef for T where T: GcRefImpl {}

/// A trait implemented for GC references that are guaranteed to be rooted:
///
/// * [`Rooted<T>`][crate::Rooted]
/// * [`ManuallyRooted<T>`][crate::ManuallyRooted]
///
/// You can use this to abstract over the different kinds of rooted GC
/// references. Note that `Deref<Target = T>` is a supertrait for
/// `RootedGcRef<T>`, so all rooted GC references deref to their underlying `T`,
/// allowing you to call its methods.
///
/// This is a sealed trait, and may not be implemented for any types outside of
/// the `wasmtime` crate.
pub trait RootedGcRef<T>: RootedGcRefImpl<T> + Deref<Target = T>
where
    T: GcRef,
{
}

impl<T, U> RootedGcRef<T> for U
where
    T: GcRef,
    U: RootedGcRefImpl<T> + Deref<Target = T>,
{
}

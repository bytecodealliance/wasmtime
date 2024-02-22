#[cfg(feature = "gc")]
mod gc_ref;
#[cfg(feature = "gc")]
pub use gc_ref::*;

#[cfg(not(feature = "gc"))]
mod no_gc_ref;
#[cfg(not(feature = "gc"))]
pub use no_gc_ref::*;

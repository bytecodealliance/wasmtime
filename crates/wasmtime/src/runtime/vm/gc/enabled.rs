//! Implementation of garbage collection and GC types in Wasmtime.

mod arrayref;
mod exnref;
mod externref;
#[cfg(feature = "gc-drc")]
mod free_list;
mod structref;
#[cfg(any(feature = "gc-drc", feature = "gc-copying"))]
mod trace_infos;

pub use arrayref::*;
pub use exnref::*;
pub use externref::*;
pub use structref::*;

#[cfg(feature = "gc-drc")]
mod drc;
#[cfg(feature = "gc-drc")]
pub use drc::*;

#[cfg(feature = "gc-null")]
mod null;
#[cfg(feature = "gc-null")]
pub use null::*;

#[cfg(feature = "gc-copying")]
mod copying;
#[cfg(feature = "gc-copying")]
pub use copying::*;

/// A hasher that doesn't hash, for use in the trace-info hash map, where we are
/// just using scalar keys and aren't overly concerned with collision-based DoS.
#[derive(Default)]
pub struct NopHasher(u64);

impl core::hash::BuildHasher for NopHasher {
    type Hasher = Self;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        NopHasher::default()
    }
}

impl core::hash::Hasher for NopHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.0.to_ne_bytes();
        let n = hash.len().min(bytes.len());
        hash[..n].copy_from_slice(bytes);
        self.0 = u64::from_ne_bytes(hash);
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.write_u64(i.into());
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write_u64(i.into())
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write_u64(i.into())
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i.try_into().unwrap());
    }
}

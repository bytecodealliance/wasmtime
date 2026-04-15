//! Shared tracing information for GC collectors.
//!
//! Both the DRC and copying collectors need to know how to trace GC objects to
//! find outgoing GC-reference edges. This module provides a shared `TraceInfos`
//! type that both collectors use.

use crate::hash_map::HashMap;
use crate::{Engine, EngineWeak};
use alloc::boxed::Box;
use core::hash::BuildHasher;
use wasmtime_environ::{GcLayout, VMSharedTypeIndex};

/// How to trace a GC object.
pub(super) enum TraceInfo {
    /// How to trace an array.
    Array {
        /// Whether this array type's elements are GC references, and need
        /// tracing.
        gc_ref_elems: bool,
    },

    /// How to trace a struct.
    Struct {
        /// The offsets of each GC reference field that needs tracing in
        /// instances of this struct type.
        gc_ref_offsets: Box<[u32]>,
    },
}

/// A hasher that doesn't hash, for use in the trace-info hash map, where we are
/// just using scalar keys and aren't overly concerned with collision-based DoS.
#[derive(Default)]
struct NopHasher(u64);

impl BuildHasher for NopHasher {
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

/// A map from GC type indices to their tracing information.
#[derive(Default)]
pub(super) struct TraceInfos {
    engine: EngineWeak,
    map: HashMap<VMSharedTypeIndex, TraceInfo, NopHasher>,
    gc_ref_array_elems_offset: u32,
}

impl TraceInfos {
    /// Create a new `TraceInfos` with the given engine and expected array
    /// element offset for GC-ref arrays.
    pub fn new(engine: &Engine, gc_ref_array_elems_offset: u32) -> Self {
        let mut map = HashMap::default();
        map.reserve(1);
        Self {
            engine: engine.weak(),
            map,
            gc_ref_array_elems_offset,
        }
    }

    fn engine(&self) -> Engine {
        self.engine.upgrade().unwrap()
    }

    /// Get the trace info for the given type, if we have it.
    #[allow(dead_code)]
    pub fn get(&self, ty: &VMSharedTypeIndex) -> Option<&TraceInfo> {
        self.map.get(ty)
    }

    /// Index into the trace infos, panicking if the type is not present.
    pub fn trace_info(&self, ty: &VMSharedTypeIndex) -> &TraceInfo {
        &self.map[ty]
    }

    /// Ensure that we have tracing information for the given type.
    pub fn ensure(&mut self, ty: VMSharedTypeIndex) {
        if self.map.contains_key(&ty) {
            return;
        }
        self.insert_new(ty);
    }

    fn insert_new(&mut self, ty: VMSharedTypeIndex) {
        debug_assert!(!self.map.contains_key(&ty));

        let engine = self.engine();
        let gc_layout = engine
            .signatures()
            .layout(ty)
            .unwrap_or_else(|| panic!("should have a GC layout for {ty:?}"));

        let info = match gc_layout {
            GcLayout::Array(l) => {
                if l.elems_are_gc_refs {
                    debug_assert_eq!(l.elem_offset(0), self.gc_ref_array_elems_offset);
                }
                TraceInfo::Array {
                    gc_ref_elems: l.elems_are_gc_refs,
                }
            }
            GcLayout::Struct(l) => TraceInfo::Struct {
                gc_ref_offsets: l
                    .fields
                    .iter()
                    .filter_map(|f| if f.is_gc_ref { Some(f.offset) } else { None })
                    .collect(),
            },
        };

        let old_entry = self.map.insert(ty, info);
        debug_assert!(old_entry.is_none());
    }
}

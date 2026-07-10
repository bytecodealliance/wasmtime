//! Shared tracing information for GC collectors.
//!
//! Both the DRC and copying collectors need to know how to trace GC objects to
//! find outgoing GC-reference edges. This module provides a shared `TraceInfos`
//! type that both collectors use.

use crate::hash_map::{Entry, HashMap};
use crate::module::RegisteredModuleId;
use crate::vm::{GcStoreTraceState, NopHasher, TraceInfo};
use wasmtime_environ::{ModuleInternedTypeIndex, VMSharedTypeIndex};

#[derive(Clone, Copy)]
enum TraceInfoLoc {
    HostType(VMSharedTypeIndex),
    Module(RegisteredModuleId, ModuleInternedTypeIndex),
}

/// A map from GC type indices to where their tracing information can be found.
#[derive(Default)]
pub(super) struct TraceInfos {
    map: HashMap<VMSharedTypeIndex, TraceInfoLoc, NopHasher>,
}

impl TraceInfos {
    /// Create a new `TraceInfos` with the given engine and expected array
    /// element offset for GC-ref arrays.
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),
        }
    }

    /// Remove all trace info from this collection.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Lookup trace information for `ty`, panicking if it can't be found.
    pub fn trace_info<'a>(
        &mut self,
        ty: &VMSharedTypeIndex,
        state: &'a GcStoreTraceState<'_>,
    ) -> &'a TraceInfo {
        self.trace_info_(ty, state)
            .unwrap_or_else(|| panic!("failed to find trace information for {ty:?}"))
    }

    fn trace_info_<'a>(
        &mut self,
        ty: &VMSharedTypeIndex,
        state: &'a GcStoreTraceState<'_>,
    ) -> Option<&'a TraceInfo> {
        // Determine where the trace information for `ty` is stored. This lookup
        // is cached within `self.map` to avoid hitting `find_trace_info` too
        // often.
        let loc = match self.map.entry(*ty) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(find_trace_info(ty, state)?),
        };

        // Use `loc` to, relatively quickly, go to the trace information as
        // stored within `state`.
        Some(match loc {
            TraceInfoLoc::HostType(ty) => state.gc_host_alloc_types[ty].1.as_ref()?,
            TraceInfoLoc::Module(module_id, ty) => state
                .modules
                .module_by_id(*module_id)?
                .signatures()
                .trace_info(*ty)?,
        })
    }
}

/// Locates the trace information for `ty` within `state`.
///
/// This is a one-time operation per-store which is used to determine where
/// exactly trace information can be found. This is structured to notably be
/// relatively cheap to compute but additionally require zero up-front compute
/// in terms of instantiation or when a store is created.
fn find_trace_info(ty: &VMSharedTypeIndex, state: &GcStoreTraceState<'_>) -> Option<TraceInfoLoc> {
    if state.gc_host_alloc_types.contains_key(ty) {
        return Some(TraceInfoLoc::HostType(*ty));
    }

    // It's expected that most stores have a small number of modules, hence the
    // linear iteration here.
    for (id, module) in state.modules.all_modules() {
        if let Some(module_ty) = module.signatures().shared_type_with_trace_info(*ty) {
            return Some(TraceInfoLoc::Module(id, module_ty));
        }
    }

    None
}

//! This module provides a set of primitives that allow implementing an incremental cache on top of
//! Cranelift, making it possible to reuse previous compiled artifacts for functions that have been
//! compiled previously.
//!
//! This set of operation is experimental and can be enabled using the Cargo feature
//! `incremental-cache`.
//!
//! This can bring speedups in different cases: change-code-and-immediately-recompile iterations
//! get faster, modules sharing lots of code can reuse each other's artifacts, etc.
//!
//! The three main primitives are the following:
//! - `compute_cache_key` is used to compute the cache key associated to a `Function`. This is
//! basically the content of the function, modulo a few things the caching system is resilient to.
//! - `serialize_compiled` is used to serialize the result of a compilation, so it can be reused
//! later on by...
//! - `try_finish_recompile`, which reads binary blobs serialized with `serialize_compiled`,
//! re-creating the compilation artifact from those.
//!
//! The `CacheStore` trait and `Context::compile_with_cache` method are provided as
//! high-level, easy-to-use facilities to make use of that cache, and show an example of how to use
//! the above three primitives to form a full incremental caching system.

use crate::alloc::string::String;
use crate::alloc::vec::Vec;
use crate::binemit::CodeOffset;
use crate::ir::dfg::{BlockData, ValueDataPacked};
use crate::ir::function::VersionMarker;
use crate::ir::instructions::InstructionData;
use crate::ir::{
    self, Block, Constant, ConstantData, ConstantPool, DataFlowGraph, DynamicStackSlot,
    DynamicStackSlots, DynamicTypes, ExtFuncData, ExternalName, FuncRef, Function, Immediate, Inst,
    JumpTables, Layout, LibCall, SigRef, Signature, SourceLoc, StackSlot, StackSlots, Value,
    ValueLabel, ValueLabelAssignments, ValueLabelStart, ValueList, TESTCASE_NAME_LENGTH,
};
use crate::machinst::isle::UnwindInst;
use crate::machinst::{CompiledCode, MachBufferFinalized};
use crate::result::CompileResult;
use crate::{isa::TargetIsa, timing};
use crate::{CompileError, Context, HashMap};
use crate::{MachCallSite, MachReloc, MachSrcLoc, MachStackMap, MachTrap, ValueLabelsRanges};
use alloc::borrow::{Cow, ToOwned as _};
use alloc::collections::BTreeMap;
use alloc::string::ToString as _;
use cranelift_entity::{EntityRef as _, SecondaryMap};
use cranelift_entity::{ListPool, PrimaryMap};
use smallvec::SmallVec;

impl Context {
    /// Compile the function, as in `compile`, but tries to reuse compiled artifacts from former
    /// compilations using the provided cache store.
    pub fn compile_with_cache(
        &mut self,
        isa: &dyn TargetIsa,
        cache_store: &mut dyn CacheKvStore,
    ) -> CompileResult<(&CompiledCode, bool)> {
        let (cache_key, cache_key_hash) = {
            let _tt = timing::try_incremental_cache();

            let (cache_key, cache_key_hash) = compute_cache_key(isa, &self.func);

            if let Some(blob) = cache_store.get(&cache_key_hash.0) {
                if let Ok(compiled_code) = try_finish_recompile(&cache_key, &self.func, &blob) {
                    let info = compiled_code.code_info();

                    if isa.flags().enable_incremental_compilation_cache_checks() {
                        let actual_result = self.compile(isa)?;
                        assert_eq!(*actual_result, compiled_code);
                        assert_eq!(actual_result.code_info(), info);
                        // no need to set `compiled_code` here, it's set by `compile()`.
                        return Ok((actual_result, true));
                    }

                    let compiled_code = self.compiled_code.insert(compiled_code);
                    return Ok((compiled_code, true));
                }
            }

            (cache_key, cache_key_hash)
        };

        self.compile_inner(isa).map_err(|err| CompileError {
            inner: err,
            func: &self.func,
        })?;
        let compiled_code = self.compiled_code.as_ref().unwrap();

        let _tt = timing::store_incremental_cache();
        if let Ok(blob) = serialize_compiled(cache_key, &self.func, compiled_code) {
            cache_store.insert(&cache_key_hash.0, blob);
        }

        Ok((compiled_code, false))
    }
}

/// Backing storage for an incremental compilation cache, when enabled.
pub trait CacheKvStore {
    /// Given a cache key hash, retrieves the associated opaque serialized data.
    fn get(&self, key: &[u8]) -> Option<Cow<[u8]>>;

    /// Given a new cache key and a serialized blob obtained from `serialize_compiled`, stores it
    /// in the cache store.
    fn insert(&mut self, key: &[u8], val: Vec<u8>);
}

/// Hashed `CachedKey`, to use as an identifier when looking up whether a function has already been
/// compiled or not.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct CacheKeyHash([u8; 8]);

impl std::fmt::Display for CacheKeyHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        i64::from_le_bytes(self.0).fmt(f)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedFunc {
    cache_key: CacheKey,
    compile_result: CachedCompiledCode,
}

/// Same as `ValueLabelStart`, but we relocate the `from` source location.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Hash)]
struct CachedValueLabelStart {
    from: RelSourceLoc,
    label: ValueLabel,
}

impl CachedValueLabelStart {
    #[allow(dead_code)]
    /// Not intended for use; see comment on top of `CacheKey`.
    fn check_from_src(self) -> ValueLabelStart {
        ValueLabelStart {
            from: self.from.expand(SourceLoc::new(0)),
            label: self.label,
        }
    }
}

/// Same as `ValueLabelAssignments`, but we relocate the `from` source location.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Hash)]
enum CachedValueLabelAssignments {
    Starts(alloc::vec::Vec<CachedValueLabelStart>),
    Alias { from: RelSourceLoc, value: Value },
}

impl CachedValueLabelAssignments {
    #[allow(dead_code)]
    /// Not intended for use; see comment on top of `CacheKey`.
    fn check_from_src(self) -> ValueLabelAssignments {
        match self {
            CachedValueLabelAssignments::Starts(vec) => {
                ValueLabelAssignments::Starts(vec.into_iter().map(|v| v.check_from_src()).collect())
            }
            CachedValueLabelAssignments::Alias { from, value } => ValueLabelAssignments::Alias {
                from: from.expand(SourceLoc::new(0)),
                value,
            },
        }
    }
}

#[derive(Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
struct CachedConstantPool {
    handles_to_values: BTreeMap<Constant, ConstantData>,
    values_to_handles: BTreeMap<ConstantData, Constant>,
}

impl CachedConstantPool {
    #[allow(dead_code)]
    /// Not intended for use; see comment on top of `CacheKey`.
    fn check_from_src(self) -> ConstantPool {
        ConstantPool {
            handles_to_values: self.handles_to_values.into_iter().collect(),
            values_to_handles: self.values_to_handles.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
enum CachedExternalName {
    User,
    TestCase {
        length: u8,
        ascii: [u8; TESTCASE_NAME_LENGTH],
    },
    LibCall(LibCall),
}

impl CachedExternalName {
    #[allow(dead_code)]
    /// Not intended for use; see comment on top of `CacheKey`.
    fn check_from_src(self) -> ExternalName {
        match self {
            CachedExternalName::User => ExternalName::User {
                namespace: 0, // caching is resilient with respect to namespace/index
                index: 0,
            },
            CachedExternalName::TestCase { length, ascii } => {
                ExternalName::TestCase { length, ascii }
            }
            CachedExternalName::LibCall(libcall) => ExternalName::LibCall(libcall),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
struct CachedExtFuncData {
    name: CachedExternalName,
    signature: SigRef,
    colocated: bool,
}

impl CachedExtFuncData {
    #[allow(dead_code)]
    /// Not intended for use; see comment on top of `CacheKey`.
    fn check_from_src(self) -> ExtFuncData {
        ExtFuncData {
            name: self.name.check_from_src(),
            signature: self.signature,
            colocated: self.colocated,
        }
    }
}

#[derive(Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
struct CachedDataFlowGraph {
    // --
    // Those fields are the same as in DataFlowGraph
    // --
    insts: PrimaryMap<Inst, InstructionData>,
    results: SecondaryMap<Inst, ValueList>,
    blocks: PrimaryMap<Block, BlockData>,
    dynamic_types: DynamicTypes,
    value_lists: Vec<Value>,
    values: PrimaryMap<Value, ValueDataPacked>,
    signatures: PrimaryMap<SigRef, Signature>,
    old_signatures: SecondaryMap<SigRef, Option<Signature>>,
    immediates: PrimaryMap<Immediate, ConstantData>,

    // --
    // Fields that we tweaked for caching
    // --
    constants: CachedConstantPool,
    values_labels: Option<BTreeMap<Value, CachedValueLabelAssignments>>,
    /// Same as `DataFlowGraph::ext_funcs`, but we remove the identifying fields of the
    /// `ExternalName` so two calls to external user functions appear the same and end up in the
    /// same cache bucket.
    ext_funcs: PrimaryMap<FuncRef, CachedExtFuncData>,
}

impl CachedDataFlowGraph {
    #[allow(dead_code)]
    /// Not intended for use; see comment on top of `CacheKey`.
    fn check_from_src(self) -> DataFlowGraph {
        DataFlowGraph {
            insts: self.insts,
            results: self.results,
            blocks: self.blocks,
            dynamic_types: self.dynamic_types,
            value_lists: ListPool::new(), // properly converted
            values: self.values,
            signatures: self.signatures,
            old_signatures: self.old_signatures,
            ext_funcs: self
                .ext_funcs
                .into_iter()
                .map(|(_k, v)| v.check_from_src())
                .collect(),
            values_labels: self.values_labels.map(|btree| {
                btree
                    .into_iter()
                    .map(|(k, v)| (k, v.check_from_src()))
                    .collect()
            }),
            constants: self.constants.check_from_src(),
            immediates: self.immediates,
        }
    }
}

/// Key for caching a single function's compilation.
///
/// If two functions get the same `CacheKey`, then we can reuse the compiled artifacts, modulo some
/// relocation fixups.
///
/// Everything in a `Function` that uniquely identifies a function must be included in this data
/// structure. For that matter, there's a method `check_from_func`.
#[derive(Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CacheKey {
    version_marker: VersionMarker,
    signature: Signature,
    sized_stack_slots: StackSlots,
    dynamic_stack_slots: DynamicStackSlots,
    global_values: PrimaryMap<ir::GlobalValue, ir::GlobalValueData>,
    heaps: PrimaryMap<ir::Heap, ir::HeapData>,
    tables: PrimaryMap<ir::Table, ir::TableData>,
    jump_tables: JumpTables,
    dfg: CachedDataFlowGraph,
    layout: Layout,
    stack_limit: Option<ir::GlobalValue>,

    // Extra fields
    parameters: CompileParameters,
}

#[derive(Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
struct CompileParameters {
    isa: String,
    triple: String,
    flags: String,
    isa_flags: Vec<String>,
}

impl CompileParameters {
    fn from_isa(isa: &dyn TargetIsa) -> Self {
        Self {
            isa: isa.name().to_owned(),
            triple: isa.triple().to_string(),
            flags: isa.flags().to_string(),
            isa_flags: isa
                .isa_flags()
                .into_iter()
                .map(|v| v.value_string())
                .collect(),
        }
    }
}

impl CacheKey {
    #[allow(dead_code)]
    /// This function is not intended to be used, but serves as a static compile check that every
    /// field in `Function` is also present (or handled somehow) in `CacheKey`.
    ///
    /// See comment on top of `CacheKey`.
    fn check_from_src(self) -> Function {
        Function {
            version_marker: self.version_marker,
            name: Default::default(), // caching is resilient to source locs
            signature: self.signature,
            sized_stack_slots: self.sized_stack_slots,
            dynamic_stack_slots: self.dynamic_stack_slots,
            global_values: self.global_values,
            heaps: self.heaps,
            tables: self.tables,
            jump_tables: self.jump_tables,
            dfg: self.dfg.check_from_src(),
            layout: self.layout,
            srclocs: Default::default(), // caching is resilient to source locs
            stack_limit: self.stack_limit,
        }
    }

    /// Creates a new cache store key for a function.
    ///
    /// This is a bit expensive to compute, so it should be cached and reused as much as possible.
    fn new(isa: &dyn TargetIsa, f: &Function, offset: SourceLoc) -> Self {
        let constants = CachedConstantPool {
            handles_to_values: f.dfg.constants.handles_to_values.clone(),
            values_to_handles: f
                .dfg
                .constants
                .values_to_handles
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };

        let values_labels = f.dfg.values_labels.clone().map(|vl| {
            vl.into_iter()
                .map(|(k, v)| {
                    let v = match v {
                        ValueLabelAssignments::Starts(vec) => CachedValueLabelAssignments::Starts(
                            vec.into_iter()
                                .map(|vls| CachedValueLabelStart {
                                    from: RelSourceLoc::new(vls.from, offset),
                                    label: vls.label,
                                })
                                .collect(),
                        ),
                        ValueLabelAssignments::Alias { from, value } => {
                            CachedValueLabelAssignments::Alias {
                                from: RelSourceLoc::new(from, offset),
                                value,
                            }
                        }
                    };
                    (k, v)
                })
                .collect()
        });

        let ext_funcs = f
            .dfg
            .ext_funcs
            .values()
            .map(|ext_data| {
                let name = match ext_data.name {
                    ExternalName::User { .. } => {
                        // Remove the identifying properties of the call to that external function,
                        // as we want two functions with calls to different functions to reuse each
                        // other's cached artifacts.
                        CachedExternalName::User
                    }
                    ExternalName::TestCase { length, ascii } => {
                        CachedExternalName::TestCase { length, ascii }
                    }
                    ExternalName::LibCall(libcall) => CachedExternalName::LibCall(libcall),
                };
                let data = CachedExtFuncData {
                    name,
                    signature: ext_data.signature.clone(),
                    colocated: ext_data.colocated,
                };
                // Note: we rely on the iteration order being the same as the insertion order in
                // PrimaryMap, here.
                data
            })
            .collect();

        let value_lists = f.dfg.value_lists.internal_data().to_vec();

        let dfg = CachedDataFlowGraph {
            insts: f.dfg.insts.clone(),
            results: f.dfg.results.clone(),
            dynamic_types: f.dfg.dynamic_types.clone(),
            value_lists,
            blocks: f.dfg.blocks.clone(),
            values: f.dfg.values.clone(),
            signatures: f.dfg.signatures.clone(),
            old_signatures: f.dfg.old_signatures.clone(),
            immediates: f.dfg.immediates.clone(),

            constants,
            values_labels,
            ext_funcs,
        };

        let mut layout = f.layout.clone();
        // Make sure the blocks and instructions are sequenced the same way as we might
        // have serialized them earlier. This is the symmetric of what's done in
        // `try_load`.
        layout.full_renumber();

        CacheKey {
            version_marker: f.version_marker,
            signature: f.signature.clone(),
            sized_stack_slots: f.sized_stack_slots.clone(),
            dynamic_stack_slots: f.dynamic_stack_slots.clone(),
            global_values: f.global_values.clone(),
            heaps: f.heaps.clone(),
            tables: f.tables.clone(),
            jump_tables: f.jump_tables.clone(),
            dfg,
            layout,
            stack_limit: f.stack_limit.clone(),
            parameters: CompileParameters::from_isa(isa),
        }
    }
}

/// Relative source location.
///
/// This can be used to recompute source locations independently of the other functions in the
/// project.
#[derive(Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
struct RelSourceLoc(u32);

impl RelSourceLoc {
    fn new(loc: SourceLoc, offset: SourceLoc) -> Self {
        if loc.is_default() || offset.is_default() {
            Self(SourceLoc::default().bits())
        } else {
            Self(loc.bits() - offset.bits())
        }
    }

    fn expand(&self, offset: SourceLoc) -> SourceLoc {
        if SourceLoc::new(self.0).is_default() || offset.is_default() {
            SourceLoc::default()
        } else {
            SourceLoc::new(self.0 + offset.bits())
        }
    }
}

// --
// Copies of data structures that use `RelSourceLoc` instead of `SourceLoc`.

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedMachSrcLoc {
    start: CodeOffset,
    end: CodeOffset,
    loc: RelSourceLoc,
}

impl CachedMachSrcLoc {
    fn new(loc: &MachSrcLoc, offset: SourceLoc) -> Self {
        Self {
            start: loc.start,
            end: loc.end,
            loc: RelSourceLoc::new(loc.loc, offset),
        }
    }

    fn expand(self, offset: SourceLoc) -> MachSrcLoc {
        MachSrcLoc {
            start: self.start,
            end: self.end,
            loc: self.loc.expand(offset),
        }
    }
}

// --
// Our final data structure.

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedMachBufferFinalized {
    data: SmallVec<[u8; 1024]>,
    relocs: SmallVec<[MachReloc; 16]>,
    traps: SmallVec<[MachTrap; 16]>,
    call_sites: SmallVec<[MachCallSite; 16]>,
    srclocs: SmallVec<[CachedMachSrcLoc; 64]>,
    stack_maps: SmallVec<[MachStackMap; 8]>,
    unwind_info: SmallVec<[(CodeOffset, UnwindInst); 8]>,
}

impl CachedMachBufferFinalized {
    fn new(mbf: &MachBufferFinalized, offset: SourceLoc) -> Self {
        Self {
            data: mbf.data.clone(),
            relocs: mbf.relocs().into_iter().cloned().collect(),
            traps: mbf.traps().into_iter().cloned().collect(),
            call_sites: mbf.call_sites().into_iter().cloned().collect(),
            srclocs: mbf
                .get_srclocs_sorted()
                .iter()
                .map(|loc| CachedMachSrcLoc::new(loc, offset))
                .collect(),
            stack_maps: mbf.stack_maps.clone(),
            unwind_info: mbf.unwind_info.clone(),
        }
    }

    fn expand(self, offset: SourceLoc) -> MachBufferFinalized {
        MachBufferFinalized {
            data: self.data,
            relocs: self.relocs,
            traps: self.traps,
            call_sites: self.call_sites,
            srclocs: self
                .srclocs
                .into_iter()
                .map(|loc| loc.expand(offset))
                .collect(),
            stack_maps: self.stack_maps,
            unwind_info: self.unwind_info,
        }
    }
}

#[derive(Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct UserExternalName {
    namespace: u32,
    index: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedCompiledCode {
    // These are fields from the `CompiledCode`.
    // ---
    buffer: CachedMachBufferFinalized,
    frame_size: u32,
    disasm: Option<String>,
    value_labels_ranges: ValueLabelsRanges,
    sized_stackslot_offsets: PrimaryMap<StackSlot, u32>,
    dynamic_stackslot_offsets: PrimaryMap<DynamicStackSlot, u32>,
    bb_starts: Vec<CodeOffset>,
    bb_edges: Vec<(CodeOffset, CodeOffset)>,

    // These are extra fields useful for patching purposes.
    // ---
    /// Inverted mapping of user external name to `FuncRef`, constructed once to allow patching.
    func_refs: HashMap<UserExternalName, FuncRef>,
}

impl CachedCompiledCode {
    fn new(func: &Function, mcr: &CompiledCode) -> Self {
        let offset = func
            .srclocs
            .get(Inst::new(0))
            .cloned()
            .unwrap_or(SourceLoc::new(0));

        let func_refs = func
            .dfg
            .ext_funcs
            .iter()
            .filter_map(|(func_ref, ext_data)| {
                if let ExternalName::User { namespace, index } = &ext_data.name {
                    Some((
                        UserExternalName {
                            namespace: *namespace,
                            index: *index,
                        },
                        func_ref,
                    ))
                } else {
                    None
                }
            })
            .collect();

        Self {
            buffer: CachedMachBufferFinalized::new(&mcr.buffer, offset),
            frame_size: mcr.frame_size,
            disasm: mcr.disasm.clone(),
            value_labels_ranges: mcr.value_labels_ranges.clone(),
            sized_stackslot_offsets: mcr.sized_stackslot_offsets.clone(),
            dynamic_stackslot_offsets: mcr.dynamic_stackslot_offsets.clone(),
            bb_starts: mcr.bb_starts.clone(),
            bb_edges: mcr.bb_edges.clone(),
            func_refs,
        }
    }

    fn expand(self, after: &Function, offset: SourceLoc) -> CompiledCode {
        let mut buffer = self.buffer.expand(offset);

        // Adjust external names in relocations.
        for reloc in buffer.relocs.iter_mut() {
            if let ExternalName::User {
                ref mut namespace,
                ref mut index,
            } = reloc.name
            {
                if let Some(func_ref) = self.func_refs.get(&UserExternalName {
                    namespace: *namespace,
                    index: *index,
                }) {
                    if let ExternalName::User {
                        namespace: new_namespace,
                        index: new_index,
                    } = &after.dfg.ext_funcs[*func_ref].name
                    {
                        *namespace = *new_namespace;
                        *index = *new_index;
                    } else {
                        panic!("unexpected kind of relocation??");
                    }
                } else {
                    panic!("didn't find previous mention of {};{}", namespace, index);
                }
            }
        }

        CompiledCode {
            buffer,
            frame_size: self.frame_size,
            disasm: self.disasm,
            value_labels_ranges: self.value_labels_ranges,
            sized_stackslot_offsets: self.sized_stackslot_offsets,
            dynamic_stackslot_offsets: self.dynamic_stackslot_offsets,
            bb_starts: self.bb_starts,
            bb_edges: self.bb_edges,
        }
    }
}

/// Compute a cache key, and hash it on your behalf.
///
/// Since computing the `CacheKey` is a bit expensive, it should be done as least as possible.
#[inline(never)]
pub fn compute_cache_key(isa: &dyn TargetIsa, func: &Function) -> (CacheKey, CacheKeyHash) {
    let srcloc_offset = func
        .srclocs
        .get(Inst::new(0))
        .cloned()
        .unwrap_or(SourceLoc::new(0));

    let cache_key = CacheKey::new(isa, func, srcloc_offset);

    let hash = {
        use core::hash::{Hash as _, Hasher as _};
        let mut hasher = crate::fx::FxHasher::default();
        cache_key.hash(&mut hasher);
        hasher.finish()
    };

    (cache_key, CacheKeyHash(hash.to_le_bytes()))
}

/// Given a function that's been successfully compiled, serialize it to a blob that the caller may
/// store somewhere for future use by `try_finish_recompile`.
#[inline(never)]
pub fn serialize_compiled(
    cache_key: CacheKey,
    func: &Function,
    result: &CompiledCode,
) -> Result<Vec<u8>, bincode::Error> {
    let cached = CachedFunc {
        cache_key,
        compile_result: CachedCompiledCode::new(func, result),
    };
    bincode::serialize(&cached)
}

/// Given a function that's been precompiled and its entry in the caching storage, try to shortcut
/// compilation of the given function.
#[inline(never)]
pub fn try_finish_recompile(
    cache_key: &CacheKey,
    func: &Function,
    bytes: &[u8],
) -> Result<CompiledCode, ()> {
    let srcloc_offset = func
        .srclocs
        .get(Inst::new(0))
        .cloned()
        .unwrap_or(SourceLoc::new(0));

    // try to deserialize, if not failure, return final recompiled code
    match bincode::deserialize::<CachedFunc>(bytes) {
        Ok(mut result) => {
            // Make sure the blocks and instructions are sequenced the same way as we might
            // have serialized them earlier. This is the symmetric of what's done in
            // `CacheKey`'s ctor.
            result.cache_key.layout.full_renumber();

            if *cache_key == result.cache_key {
                let compiled_code = result.compile_result.expand(func, srcloc_offset);
                return Ok(compiled_code);
            } else {
                eprintln!("{} not read from cache: source mismatch", func.name);

                //if cache_key.version_marker != result.cache_key.version_marker {
                //eprintln!("     because of version marker")
                //}
                //if cache_key.signature != result.cache_key.signature {
                //eprintln!("     because of signature")
                //}
                //if cache_key.stack_slots != result.cache_key.stack_slots {
                //eprintln!("     because of stack slots")
                //}
                //if cache_key.global_values != result.cache_key.global_values {
                //eprintln!("     because of global values")
                //}
                //if cache_key.heaps != result.cache_key.heaps {
                //eprintln!("     because of heaps")
                //}
                //if cache_key.tables != result.cache_key.tables {
                //eprintln!("     because of tables")
                //}
                //if cache_key.jump_tables != result.cache_key.jump_tables {
                //eprintln!("     because of jump tables")
                //}
                //if cache_key.dfg != result.cache_key.dfg {
                //eprintln!("     because of dfg")
                //}
                //if cache_key.layout != result.cache_key.layout {
                //if func.layout.blocks().count() < 8 {
                //eprintln!(
                //"     because of layout:\n{:?}\n{:?}",
                //cache_key.layout, result.cache_key.layout
                //);
                //} else {
                //eprintln!("     because of layout",);
                //}
                //}
                //if cache_key.stack_limit != result.cache_key.stack_limit {
                //eprintln!("     because of stack limit")
                //}
            }
        }

        Err(err) => {
            eprintln!("Couldn't deserialize cache entry: {err}");
            //log::debug!("Couldn't deserialize cache entry with key {hash:x}: {err}");
        }
    }

    Err(())
}

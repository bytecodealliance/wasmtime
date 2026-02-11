//! Definitions of runtime structures and metadata which are serialized into ELF
//! with `bincode` as part of a module's compilation process.

use crate::prelude::*;
use crate::{FilePos, FuncIndex, FuncKey, FuncKeyIndex, FuncKeyKind, FuncKeyNamespace, Module};
use core::ops::Range;
use core::{fmt, u32};
use core::{iter, str};
use cranelift_entity::{EntityRef, PrimaryMap};
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Description of where a function is located in the text section of a
/// compiled image.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionLoc {
    /// The byte offset from the start of the text section where this
    /// function starts.
    pub start: u32,
    /// The byte length of this function's function body.
    pub length: u32,
}

impl FunctionLoc {
    /// Is this an empty function location?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

/// The checksum of a Wasm binary.
///
/// Allows for features requiring the exact same Wasm Module (e.g. deterministic replay)
/// to verify that the binary used matches the one originally compiled.
#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub struct WasmChecksum([u8; 32]);

impl WasmChecksum {
    /// Construct a [`WasmChecksum`] from the given wasm binary.
    pub fn from_binary(bin: &[u8]) -> WasmChecksum {
        WasmChecksum(Sha256::digest(bin).into())
    }
}

impl core::ops::Deref for WasmChecksum {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A builder for a `CompiledFunctionsTable`.
pub struct CompiledFunctionsTableBuilder {
    inner: CompiledFunctionsTable,
}

impl CompiledFunctionsTableBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            inner: CompiledFunctionsTable {
                namespaces: PrimaryMap::new(),
                func_loc_starts: PrimaryMap::new(),
                sparse_starts: PrimaryMap::new(),
                src_loc_starts: PrimaryMap::new(),
                sparse_indices: PrimaryMap::new(),
                func_locs: PrimaryMap::new(),
                src_locs: PrimaryMap::new(),
            },
        }
    }

    fn last_namespace(&self) -> Option<FuncKeyNamespace> {
        let (_, &ns) = self.inner.namespaces.last()?;
        Some(ns)
    }

    fn last_key_index(&self) -> Option<FuncKeyIndex> {
        let (ns_idx, ns) = self.inner.namespaces.last()?;
        let start = self.inner.func_loc_starts[ns_idx];
        if CompiledFunctionsTable::is_dense(ns.kind()) {
            let len = self.inner.func_locs.len();
            let len = u32::try_from(len).unwrap();
            let key_index = len - start.as_u32();
            let key_index = FuncKeyIndex::from_raw(key_index);
            Some(key_index)
        } else {
            let sparse_start = self.inner.sparse_starts[ns_idx];
            if self.inner.sparse_indices.len() > sparse_start.index() {
                let (_, &key_index) = self.inner.sparse_indices.last().unwrap();
                Some(key_index)
            } else {
                None
            }
        }
    }

    fn last_func_loc(&self) -> Option<FunctionLoc> {
        let (_, &loc) = self.inner.func_locs.last()?;
        Some(loc)
    }

    /// Push a new entry into this builder.
    ///
    /// Panics if the key or function location is out of order.
    pub fn push_func(
        &mut self,
        key: FuncKey,
        func_loc: FunctionLoc,
        src_loc: FilePos,
    ) -> &mut Self {
        let (key_ns, key_index) = key.into_parts();

        assert!(
            self.last_namespace().is_none_or(|ns| ns <= key_ns),
            "`FuncKey`s pushed out of order"
        );
        assert!(
            self.last_key_index().is_none_or(
                |i| i <= key_index || self.last_namespace().is_some_and(|ns| ns != key_ns)
            ),
            "`FuncKey`s pushed out of order"
        );
        assert!(
            self.last_func_loc()
                .is_none_or(|l| l.start + l.length <= func_loc.start),
            "`FunctionLoc`s pushed out of order"
        );

        // Make sure that there is a `kind` entry for this key's kind.
        let kind_start_index = self
            .inner
            .namespaces
            .last()
            .and_then(|(ns_idx, ns)| {
                if *ns == key_ns {
                    Some(self.inner.func_loc_starts[ns_idx])
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                let start = self.inner.func_locs.next_key();
                let ns_idx = self.inner.namespaces.push(key_ns);
                let ns_idx2 = self.inner.func_loc_starts.push(start);
                let ns_idx3 = self
                    .inner
                    .sparse_starts
                    .push(self.inner.sparse_indices.next_key());
                let ns_idx4 = self
                    .inner
                    .src_loc_starts
                    .push(self.inner.src_locs.next_key());
                debug_assert_eq!(ns_idx, ns_idx2);
                debug_assert_eq!(ns_idx, ns_idx3);
                debug_assert_eq!(ns_idx, ns_idx4);
                start
            });

        if CompiledFunctionsTable::is_dense(key.kind()) {
            // Figure out the index within `func_locs` for this key's entry.
            let index = kind_start_index.as_u32() + key_index.into_raw();
            let index = FuncLocIndex::from_u32(index);
            debug_assert!(self.inner.func_locs.get(index).is_none());

            // Fill in null entries for any key indices that have been omitted.
            //
            // Note that we need a null `FunctionLoc`, but we also need
            // `func_locs` to be sorted so that we support reverse
            // lookups. Therefore, we take care to create an empty function
            // location that starts at the text offset that the previous one (if
            // any) ends at, and use that as our null entry.
            let null_func_loc = FunctionLoc {
                start: self
                    .last_func_loc()
                    .map(|l| l.start + l.length)
                    .unwrap_or_default(),
                length: 0,
            };
            let gap = index.index() - self.inner.func_locs.len();
            self.inner
                .func_locs
                .extend(iter::repeat(null_func_loc).take(gap));
            debug_assert_eq!(index, self.inner.func_locs.next_key());

            if CompiledFunctionsTable::has_src_locs(key_ns.kind()) {
                self.inner
                    .src_locs
                    .extend(iter::repeat(FilePos::none()).take(gap));
            }
        } else {
            debug_assert!(
                src_loc.is_none(),
                "sparse keys do not have source locations"
            );
            self.inner.sparse_indices.push(key_index);
        }

        // And finally, we push this entry.
        self.inner.func_locs.push(func_loc);
        if CompiledFunctionsTable::has_src_locs(key_ns.kind()) {
            self.inner.src_locs.push(src_loc);
        } else {
            debug_assert!(src_loc.is_none());
        }

        self
    }

    /// Finish construction of the `CompiledFunctionsTable`.
    pub fn finish(self) -> CompiledFunctionsTable {
        self.inner
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
struct NamespaceIndex(u32);
cranelift_entity::entity_impl!(NamespaceIndex);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
struct FuncLocIndex(u32);
cranelift_entity::entity_impl!(FuncLocIndex);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
struct SparseIndex(u32);
cranelift_entity::entity_impl!(SparseIndex);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
struct SrcLocIndex(u32);
cranelift_entity::entity_impl!(SrcLocIndex);

/// A table describing the set of functions compiled into an artifact, their
/// locations within the text section, and etc...
///
/// Logically, this type is a map from a `FuncKey` to the associated function's
///
/// * location within the associated text section, and
/// * optional source location.
///
/// How this map is *actually* implemented is with a series of lookup and binary
/// search tables, split out in a data-oriented, struct-of-arrays style. We
/// organize the data in this way is service of three goals:
///
/// 1. Provide fast look ups: We need to look up the metadata for a function by
///    its key at runtime. During instantiation, for example, we need to create
///    `VMFuncRef`s for escaping functions and this requires looking up the
///    locations of those Wasm functions and their associated array-to-Wasm
///    trampolines.
///
/// 2. Keep memory overheads low and code size small: This type is serialized
///    into all of our ELF artifacts and deserialized into all `Module`s and
///    `Component`s at runtime.
///
/// 3. Be generic over any kind of function (whether defined Wasm function,
///    trampoline, or etc...) that we compile: Adding a new kind of trampoline,
///    for example, should not require updating this structure to add a new
///    table of the function locations for just trampolines of that new kind. We
///    should be able to store and query all kinds of functions uniformly.
//
// TODO: This structure could be directly encoded as raw ELF sections, instead
// of a `struct` containing a bunch of `PrimaryMap`s, which would allow us to
// avoid the serialize/deserialize runtime costs.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompiledFunctionsTable {
    /// A binary-search index for this table, mapping raw `FuncKeyNamespace`s to
    /// their associated `NamespaceIndex`. That `NamespaceIndex` can then be
    /// used to find the range of other entity indices that are specific to that
    /// namespace.
    namespaces: PrimaryMap<NamespaceIndex, FuncKeyNamespace>,

    /// `self.func_loc_starts[i]..self.func_loc_starts[i+1]` describes the range
    /// within `self.func_locs` whose entries are associated with the namespace
    /// `self.index[i]`.
    ///
    /// When `self.func_loc_starts[i+1]` is out of bounds, then the range is to
    /// the end of `self.func_locs`.
    func_loc_starts: PrimaryMap<NamespaceIndex, FuncLocIndex>,

    /// `self.sparse_starts[i]..self.sparse_starts[i+1]` describes the range
    /// within `self.sparse_indices` whose entries are associated with the
    /// namespace `self.index[i]`.
    ///
    /// When `self.sparse_starts[i+1]` is out of bounds, then the range is to
    /// the end of `self.sparse_indices`.
    ///
    /// Entries are only valid for sparse, non-dense namespaces.
    sparse_starts: PrimaryMap<NamespaceIndex, SparseIndex>,

    /// `self.src_loc_starts[i]..self.src_loc_starts[i+1]` describes the range
    /// within `self.src_loc_indices` whose entries are associated with the
    /// namespace `self.index[i]`.
    ///
    /// When `self.src_loc_starts[i+1]` is out of bounds, then the range is to
    /// the end of `self.src_locs`.
    ///
    /// Entries are only valid for namespaces whose functions have source
    /// locations.
    src_loc_starts: PrimaryMap<NamespaceIndex, SrcLocIndex>,

    /// `self.sparse_indices[i]` contains the index part of
    /// `FuncKey::from_parts(ns, index)` where `ns` is determined by
    /// `self.sparse_starts` and is a sparse, non-dense key kind. (Note that for
    /// dense keys, this information is implicitly encoded in their offset from
    /// the namespace's start index.)
    ///
    /// This is sorted to allow for binary searches.
    sparse_indices: PrimaryMap<SparseIndex, FuncKeyIndex>,

    /// `self.func_locs[i]` contains the location within the text section of
    /// `FuncKey::from_parts(self.namespaces[ns], i - start)`'s function, where
    /// `ns` and `start` are determined by `self.func_loc_starts`.
    ///
    /// Values are sorted by function location to support reverse queries from
    /// function location back to `FuncKey`.
    ///
    /// The absence of a function location (for gaps in dense namespaces) is
    /// represented with `FunctionLoc::none()`.
    func_locs: PrimaryMap<FuncLocIndex, FunctionLoc>,

    /// `self.src_locs[i]` contains the initial source location of
    /// `FuncKey::from_parts(self.namespaces[ns], i - start)`'s function, where
    /// `ns` and `start` are determined by `self.src_loc_starts`.
    ///
    /// The absence of a source location is represented by `FilePos::none()`.
    src_locs: PrimaryMap<SrcLocIndex, FilePos>,
}

impl CompiledFunctionsTable {
    #[inline]
    fn namespace_index(&self, namespace: FuncKeyNamespace) -> Option<NamespaceIndex> {
        const LINEAR_SEARCH_LIMIT: usize = 32;
        if self.namespaces.len() <= LINEAR_SEARCH_LIMIT {
            self.namespaces
                .iter()
                .find_map(|(idx, ns)| if *ns == namespace { Some(idx) } else { None })
        } else {
            self.namespaces
                .binary_search_values_by_key(&namespace, |ns| *ns)
                .ok()
        }
    }

    #[inline]
    fn func_loc_range(&self, ns_idx: NamespaceIndex) -> Range<FuncLocIndex> {
        let start = self.func_loc_starts[ns_idx];
        let next_ns_idx = NamespaceIndex::from_u32(ns_idx.as_u32() + 1);
        let end = self
            .func_loc_starts
            .get(next_ns_idx)
            .copied()
            .unwrap_or_else(|| self.func_locs.next_key());
        start..end
    }

    fn sparse_range(&self, ns_idx: NamespaceIndex) -> Range<SparseIndex> {
        debug_assert!(!Self::is_dense(self.namespaces[ns_idx].kind()));
        let start = self.sparse_starts[ns_idx];
        let next_ns_idx = NamespaceIndex::from_u32(ns_idx.as_u32() + 1);
        let end = self
            .sparse_starts
            .get(next_ns_idx)
            .copied()
            .unwrap_or_else(|| self.sparse_indices.next_key());
        start..end
    }

    fn src_loc_range(&self, ns_idx: NamespaceIndex) -> Range<SrcLocIndex> {
        debug_assert!(Self::has_src_locs(self.namespaces[ns_idx].kind()));
        let start = self.src_loc_starts[ns_idx];
        let next_ns_idx = NamespaceIndex::from_u32(ns_idx.as_u32() + 1);
        let end = self
            .src_loc_starts
            .get(next_ns_idx)
            .copied()
            .unwrap_or_else(|| self.src_locs.next_key());
        start..end
    }

    /// Get the index within `self.{func_locs,src_locs}` that is associated with
    /// the given `key`, if any.
    #[inline]
    fn func_loc_index(&self, key: FuncKey) -> Option<FuncLocIndex> {
        let (key_ns, key_index) = key.into_parts();
        let ns_idx = self.namespace_index(key_ns)?;
        let Range { start, end } = self.func_loc_range(ns_idx);

        let index = if Self::is_dense(key.kind()) {
            let index = start.as_u32().checked_add(key_index.into_raw())?;
            FuncLocIndex::from_u32(index)
        } else {
            let sparse_range = self.sparse_range(ns_idx);
            let sparse_subslice = self.sparse_indices.get_range(sparse_range).unwrap();
            match sparse_subslice.binary_search(&key_index) {
                Ok(i) => FuncLocIndex::new(start.index() + i),
                Err(_) => return None,
            }
        };

        if index < end { Some(index) } else { None }
    }

    /// Get the location of the function associated with the given `key` inside
    /// the text section, if any.
    #[inline]
    pub fn func_loc(&self, key: FuncKey) -> Option<&FunctionLoc> {
        let index = self.func_loc_index(key)?;
        let loc = &self.func_locs[index];
        if loc.is_empty() { None } else { Some(loc) }
    }

    fn src_loc_index(&self, key: FuncKey) -> Option<SrcLocIndex> {
        let (key_ns, key_index) = key.into_parts();
        if !Self::has_src_locs(key_ns.kind()) {
            return None;
        }

        let ns_idx = self.namespace_index(key_ns)?;
        let Range { start, end } = self.src_loc_range(ns_idx);

        debug_assert!(Self::is_dense(key_ns.kind()));
        let index = start.as_u32().checked_add(key_index.into_raw())?;
        let index = SrcLocIndex::from_u32(index);
        if index >= end {
            return None;
        }

        Some(index)
    }

    /// Get the initial source location of the function associated with the
    /// given `key`, if any.
    pub fn src_loc(&self, key: FuncKey) -> Option<FilePos> {
        let index = self.src_loc_index(key)?;
        let loc = self.src_locs[index];
        if loc.is_none() { None } else { Some(loc) }
    }

    /// Given an offset into the text section, get the key for its associated
    /// function and its offset within that function.
    pub fn func_by_text_offset(&self, text_offset: u32) -> Option<FuncKey> {
        let index = match self.func_locs.as_values_slice().binary_search_by(|loc| {
            if loc.is_empty() {
                loc.start
                    .cmp(&text_offset)
                    .then_with(|| core::cmp::Ordering::Less)
            } else {
                if loc.start > text_offset {
                    core::cmp::Ordering::Greater
                } else if loc.start + loc.length <= text_offset {
                    core::cmp::Ordering::Less
                } else {
                    debug_assert!(loc.start <= text_offset);
                    debug_assert!(text_offset < loc.start + loc.length);
                    core::cmp::Ordering::Equal
                }
            }
        }) {
            // Exact match, the offset is at the end of this function.
            Ok(k) => k,
            // Not an exact match: `k` is where the offset would be
            // "inserted". Since we key based on the end, function `k` might
            // contain the offset, so we'll validate on the range check
            // below.
            Err(k) => k,
        };
        let index = FuncLocIndex::new(index);

        // Make sure that the text offset is actually within this function.
        // Non-exact binary search results can either be because we have a text
        // offset within a function but not exactly at its inclusive end, or
        // because the text offset is not within any of our functions. We filter
        // that latter case out with this check.
        let loc = self.func_locs.get(index)?;
        let start = loc.start;
        let end = start + loc.length;
        if text_offset < start || end < text_offset {
            return None;
        }

        let ns_idx = match self
            .func_loc_starts
            .binary_search_values_by_key(&index, |s| *s)
        {
            // Exact match: `i` is the entry's index.
            Ok(i) => i,
            // Not an exact match: the index, if it were the start of a
            // namespace's range, would be at `i`. Therefore, our namespace
            // entry is actually at index `i - 1`.
            Err(i) => {
                let i = i.as_u32();
                assert_ne!(i, 0);
                NamespaceIndex::from_u32(i - 1)
            }
        };
        let key_ns = self.namespaces[ns_idx];
        let start = self.func_loc_starts[ns_idx];

        let key_index = if Self::is_dense(key_ns.kind()) {
            let key_index = index.as_u32() - start.as_u32();
            FuncKeyIndex::from_raw(key_index)
        } else {
            let sparse_offset = index.as_u32() - start.as_u32();
            let sparse_start = self.sparse_starts[ns_idx];
            let sparse_index = SparseIndex::from_u32(sparse_start.as_u32() + sparse_offset);
            debug_assert!(
                {
                    let range = self.sparse_range(ns_idx);
                    range.start <= sparse_index && sparse_index < range.end
                },
                "{sparse_index:?} is not within {:?}",
                self.sparse_range(ns_idx)
            );
            self.sparse_indices[sparse_index]
        };
        let key = FuncKey::from_parts(key_ns, key_index);

        Some(key)
    }

    /// Whether the given kind's index space is (generally) densely populated
    /// and therefore we should densely pack them in the table for `O(1)`
    /// lookups; otherwise, we should avoid code size bloat by using the sparse
    /// table indirection and `O(log n)` binary search lookups.
    fn is_dense(kind: FuncKeyKind) -> bool {
        match kind {
            FuncKeyKind::DefinedWasmFunction
            | FuncKeyKind::WasmToArrayTrampoline
            | FuncKeyKind::PulleyHostCall => true,

            FuncKeyKind::ArrayToWasmTrampoline
            | FuncKeyKind::WasmToBuiltinTrampoline
            | FuncKeyKind::PatchableToBuiltinTrampoline => false,

            #[cfg(feature = "component-model")]
            FuncKeyKind::ComponentTrampoline
            | FuncKeyKind::ResourceDropTrampoline
            | FuncKeyKind::UnsafeIntrinsic => true,
        }
    }

    /// Whether the given function kind has source locations or not.
    fn has_src_locs(kind: FuncKeyKind) -> bool {
        match kind {
            FuncKeyKind::DefinedWasmFunction => true,
            FuncKeyKind::ArrayToWasmTrampoline
            | FuncKeyKind::WasmToArrayTrampoline
            | FuncKeyKind::WasmToBuiltinTrampoline
            | FuncKeyKind::PatchableToBuiltinTrampoline
            | FuncKeyKind::PulleyHostCall => false,
            #[cfg(feature = "component-model")]
            FuncKeyKind::ComponentTrampoline
            | FuncKeyKind::ResourceDropTrampoline
            | FuncKeyKind::UnsafeIntrinsic => false,
        }
    }
}

/// Secondary in-memory results of module compilation.
///
/// This opaque structure can be optionally passed back to
/// `CompiledModule::from_artifacts` to avoid decoding extra information there.
#[derive(Serialize, Deserialize)]
pub struct CompiledModuleInfo {
    /// Type information about the compiled WebAssembly module.
    pub module: Module,

    /// General compilation metadata.
    pub meta: Metadata,

    /// Sorted list, by function index, of names we have for this module.
    pub func_names: Vec<FunctionName>,

    /// Checksum of the source Wasm binary from which this module was compiled.
    pub checksum: WasmChecksum,
}

/// The name of a function stored in the
/// [`ELF_NAME_DATA`](crate::obj::ELF_NAME_DATA) section.
#[derive(Serialize, Deserialize)]
pub struct FunctionName {
    /// The Wasm function index of this function.
    pub idx: FuncIndex,
    /// The offset of the name in the
    /// [`ELF_NAME_DATA`](crate::obj::ELF_NAME_DATA) section.
    pub offset: u32,
    /// The length of the name in bytes.
    pub len: u32,
}

/// Metadata associated with a compiled ELF artifact.
#[derive(Serialize, Deserialize)]
pub struct Metadata {
    /// Whether or not the original wasm module contained debug information that
    /// we skipped and did not parse.
    pub has_unparsed_debuginfo: bool,

    /// Offset in the original wasm file to the code section.
    pub code_section_offset: u64,

    /// Whether or not custom wasm-specific dwarf sections were inserted into
    /// the ELF image.
    ///
    /// Note that even if this flag is `true` sections may be missing if they
    /// weren't found in the original wasm module itself.
    pub has_wasm_debuginfo: bool,

    /// Dwarf sections and the offsets at which they're stored in the
    /// ELF_WASMTIME_DWARF
    pub dwarf: Vec<(u8, Range<u64>)>,
}

/// Value of a configured setting for a [`Compiler`](crate::Compiler)
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug)]
pub enum FlagValue<'a> {
    /// Name of the value that has been configured for this setting.
    Enum(&'a str),
    /// The numerical value of the configured settings.
    Num(u8),
    /// Whether the setting is on or off.
    Bool(bool),
}

impl fmt::Display for FlagValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Enum(v) => v.fmt(f),
            Self::Num(v) => v.fmt(f),
            Self::Bool(v) => v.fmt(f),
        }
    }
}

/// Types of objects that can be created by `Compiler::object`
pub enum ObjectKind {
    /// A core wasm compilation artifact
    Module,
    /// A component compilation artifact
    Component,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DefinedFuncIndex, StaticModuleIndex};

    fn func_loc(range: Range<u32>) -> FunctionLoc {
        FunctionLoc {
            start: range.start,
            length: range.end - range.start,
        }
    }

    fn def_func_key(m: u32, f: u32) -> FuncKey {
        FuncKey::DefinedWasmFunction(
            StaticModuleIndex::from_u32(m),
            DefinedFuncIndex::from_u32(f),
        )
    }

    fn array_to_wasm_tramp_key(m: u32, f: u32) -> FuncKey {
        FuncKey::ArrayToWasmTrampoline(
            StaticModuleIndex::from_u32(m),
            DefinedFuncIndex::from_u32(f),
        )
    }

    fn make_test_table() -> CompiledFunctionsTable {
        let mut builder = CompiledFunctionsTableBuilder::new();

        builder
            // ========= Dense =========
            .push_func(def_func_key(0, 0), func_loc(0..10), FilePos::new(111))
            .push_func(def_func_key(0, 1), func_loc(10..20), FilePos::new(222))
            .push_func(def_func_key(0, 2), func_loc(20..30), FilePos::none())
            // Gap in dense keys!
            .push_func(def_func_key(0, 5), func_loc(30..40), FilePos::new(333))
            // ========= Sparse =========
            .push_func(
                array_to_wasm_tramp_key(0, 1),
                func_loc(100..110),
                FilePos::none(),
            )
            .push_func(
                array_to_wasm_tramp_key(0, 2),
                func_loc(110..120),
                FilePos::none(),
            )
            .push_func(
                array_to_wasm_tramp_key(0, 5),
                func_loc(120..130),
                FilePos::none(),
            );

        builder.finish()
    }

    #[test]
    fn src_locs() {
        let table = make_test_table();

        for (key, expected) in [
            (def_func_key(0, 0), Some(FilePos::new(111))),
            (def_func_key(0, 1), Some(FilePos::new(222))),
            (def_func_key(0, 2), None),
            (def_func_key(0, 3), None),
            (def_func_key(0, 4), None),
            (def_func_key(0, 5), Some(FilePos::new(333))),
            (array_to_wasm_tramp_key(0, 0), None),
            (array_to_wasm_tramp_key(0, 1), None),
            (array_to_wasm_tramp_key(0, 2), None),
            (array_to_wasm_tramp_key(0, 3), None),
            (array_to_wasm_tramp_key(0, 4), None),
            (array_to_wasm_tramp_key(0, 5), None),
        ] {
            eprintln!("Checking key {key:?}");
            let actual = table.src_loc(key);
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn func_locs() {
        let table = make_test_table();

        for (key, expected) in [
            (def_func_key(0, 0), Some(0)),
            (def_func_key(0, 1), Some(10)),
            (def_func_key(0, 2), Some(20)),
            (def_func_key(0, 3), None),
            (def_func_key(0, 4), None),
            (def_func_key(0, 5), Some(30)),
            (array_to_wasm_tramp_key(0, 0), None),
            (array_to_wasm_tramp_key(0, 1), Some(100)),
            (array_to_wasm_tramp_key(0, 2), Some(110)),
            (array_to_wasm_tramp_key(0, 3), None),
            (array_to_wasm_tramp_key(0, 4), None),
            (array_to_wasm_tramp_key(0, 5), Some(120)),
        ] {
            let actual = table.func_loc(key);
            match (expected, actual) {
                (None, None) => {}
                (Some(expected), Some(actual)) => assert_eq!(expected, actual.start),
                (None, Some(actual)) => {
                    panic!("expected no function location for {key:?}, got {actual:?}")
                }
                (Some(_), None) => {
                    panic!("expected a function location for {key:?}, but got nothing")
                }
            }
        }
    }

    #[test]
    fn reverse_func_locs() {
        let table = make_test_table();

        for (range, expected) in [
            (0..10, Some(def_func_key(0, 0))),
            (10..20, Some(def_func_key(0, 1))),
            (20..30, Some(def_func_key(0, 2))),
            (30..40, Some(def_func_key(0, 5))),
            (40..100, None),
            (100..110, Some(array_to_wasm_tramp_key(0, 1))),
            (110..120, Some(array_to_wasm_tramp_key(0, 2))),
            (120..130, Some(array_to_wasm_tramp_key(0, 5))),
            (140..150, None),
        ] {
            for i in range {
                eprintln!("Checking offset {i}");
                let actual = table.func_by_text_offset(i);
                assert_eq!(expected, actual);
            }
        }
    }

    #[test]
    fn reverse_lookups() {
        use arbitrary::{Result, Unstructured};

        arbtest::arbtest(|u| run(u)).budget_ms(1_000);

        fn run(u: &mut Unstructured<'_>) -> Result<()> {
            let mut funcs = Vec::new();

            // Build up a random set of functions with random indices.
            for _ in 0..u.int_in_range(1..=200)? {
                let key = match u.int_in_range(0..=6)? {
                    0 => FuncKey::DefinedWasmFunction(idx(u, 10)?, idx(u, 200)?),
                    1 => FuncKey::ArrayToWasmTrampoline(idx(u, 10)?, idx(u, 200)?),
                    2 => FuncKey::WasmToArrayTrampoline(idx(u, 100)?),
                    3 => FuncKey::WasmToBuiltinTrampoline(u.arbitrary()?),
                    4 => FuncKey::PulleyHostCall(u.arbitrary()?),
                    5 => FuncKey::ComponentTrampoline(u.arbitrary()?, idx(u, 50)?),
                    6 => FuncKey::ResourceDropTrampoline,
                    _ => unreachable!(),
                };
                funcs.push(key);
            }

            // Sort/dedup our list of `funcs` to satisfy the requirement of
            // `CompiledFunctionsTableBuilder::push_func`.
            funcs.sort();
            funcs.dedup();

            let mut builder = CompiledFunctionsTableBuilder::new();
            let mut size = 0;
            let mut expected = Vec::new();
            for key in funcs {
                let length = u.int_in_range(1..=10)?;
                for _ in 0..length {
                    expected.push(key);
                }
                // println!("push {key:?} - {length}");
                builder.push_func(
                    key,
                    FunctionLoc {
                        start: size,
                        length,
                    },
                    FilePos::none(),
                );
                size += length;
            }
            let index = builder.finish();

            let mut expected = expected.iter();
            for i in 0..size {
                // println!("lookup {i}");
                let actual = index.func_by_text_offset(i).unwrap();
                assert_eq!(Some(&actual), expected.next());
            }

            Ok(())
        }

        fn idx<T>(u: &mut Unstructured<'_>, max: usize) -> Result<T>
        where
            T: EntityRef,
        {
            Ok(T::new(u.int_in_range(0..=max - 1)?))
        }
    }
}

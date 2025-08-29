//! Definitions of runtime structures and metadata which are serialized into ELF
//! with `bincode` as part of a module's compilation process.

use crate::{FilePos, FuncIndex, Module};
use crate::{FuncKey, prelude::*};
use core::fmt;
use core::ops::Range;
use core::str;
use serde_derive::{Deserialize, Serialize};

/// Description of where a function is located in the text section of a
/// compiled image.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct FunctionLoc {
    /// The byte offset from the start of the text section where this
    /// function starts.
    pub start: u32,
    /// The byte length of this function's function body.
    pub length: u32,
}

/// A builder for a `CompiledFunctionsIndex`.
pub struct CompiledFunctionsIndexBuilder {
    inner: CompiledFunctionsIndex,
    last_kind: Option<u32>,
    last_index: Option<u32>,
    last_loc: Option<FunctionLoc>,
}

impl CompiledFunctionsIndexBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            inner: CompiledFunctionsIndex {
                func_key_kinds: vec![],
                func_key_indices: vec![],
                func_locs: vec![],
                src_locs: vec![],
            },
            last_kind: None,
            last_index: None,
            last_loc: None,
        }
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
        // Internal integrity checks.
        debug_assert_eq!(
            self.inner.func_key_indices.len(),
            self.inner.func_locs.len()
        );
        debug_assert_eq!(self.inner.func_key_indices.len(), self.inner.src_locs.len());

        let (raw_kind, raw_index) = key.into_raw_parts();

        // Function contract checks.
        assert!(
            self.last_kind.is_none_or(|k| k <= raw_kind),
            "`FuncKey`s pushed out of order"
        );
        assert!(
            self.last_index
                .is_none_or(|i| i <= raw_index || self.last_kind.is_some_and(|k| k != raw_kind)),
            "`FuncKey`s pushed out of order"
        );
        assert!(
            self.last_loc
                .is_none_or(|l| l.start + l.length <= func_loc.start),
            "`FunctionLoc`s pushed out of order"
        );

        // Okay, actually push the entry.
        let index = self.inner.func_key_indices.len();
        if self.last_kind.is_none_or(|k| k != raw_kind) {
            let index = u32::try_from(index).unwrap();
            self.inner.func_key_kinds.push((raw_kind, index));
            self.last_kind = Some(raw_kind);
        }

        self.inner.func_key_indices.push(raw_index);
        self.last_index = Some(raw_index);

        self.inner.func_locs.push(func_loc);
        self.last_loc = Some(func_loc);

        self.inner.src_locs.push(src_loc);
        self
    }

    /// Finish construction of the `CompiledFunctionsIndex`.
    pub fn finish(self) -> CompiledFunctionsIndex {
        self.inner
    }
}

/// An index describing the set of functions compiled into an artifact, their
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
// of a `struct` containing a bunch of `Vec`s, which would allow us to avoid the
// serialize/deserialize runtime costs.
#[derive(Serialize, Deserialize)]
pub struct CompiledFunctionsIndex {
    /// A binary search table mapping raw `FuncKey` kinds to ranges within
    /// `func_key_indices`. Each entry contains a `(kind, index)` pair, and
    /// entries are sorted by both `kind` and index.
    ///
    /// `self.func_key_kinds[kind].index..self.func_key_kinds[kind+1].index`
    /// describes the range within `self.func_key_indices` whose entries are
    /// associated with `kind`. When `self.func_key_kinds[kind+1]` is out of
    /// bounds, then the range is to the end of `self.func_key_indices`.
    func_key_kinds: Vec<(u32, u32)>,

    /// An entry containing a value `v` represents the inclusion of
    /// `FuncKey::from_raw_parts(kind, v)` in this table, where `kind` is
    /// determined by `self.func_key_kinds` (see comment above).
    ///
    /// To get the metadata associated with an entry at
    /// `self.func_key_indices[i]`, look up the `i`th entry in
    /// `self.{func_locs,src_locs}`.
    func_key_indices: Vec<u32>,

    /// `self.func_locs[i]` contains the location of
    /// `self.func_key_indices[i]`'s function in the text section.
    ///
    /// Sorted by function location to support reverse queries from function
    /// location to `FuncKey`.
    func_locs: Vec<FunctionLoc>,

    /// `self.src_locs[i]` contains the location of `self.func_key_indices[i]`'s
    /// source location, if any. The absence of a source location is represented
    /// by `FilePos::none()`.
    src_locs: Vec<FilePos>,
}

impl CompiledFunctionsIndex {
    /// Get the range of indices within
    /// `self.{func_key_indices,func_locs,src_locs}` that are associated with
    /// the given `kind`, if any.
    fn kind_range(&self, kind: u32) -> Option<Range<usize>> {
        let index = self
            .func_key_kinds
            .binary_search_by_key(&kind, |(k, _)| *k)
            .ok()?;

        let (_, start) = self.func_key_kinds[index];
        let start = usize::try_from(start).unwrap();

        let end = self
            .func_key_kinds
            .get(index + 1)
            .map(|(_, end)| usize::try_from(*end).unwrap())
            .unwrap_or(self.func_key_indices.len());

        Some(start..end)
    }

    /// Get the index within `self.{func_locs,src_locs}` that is associated with
    /// the given `key`, if any.
    fn index(&self, key: FuncKey) -> Option<usize> {
        let (raw_kind, raw_index) = key.into_raw_parts();
        let Range { start, end } = self.kind_range(raw_kind)?;
        let subslice = &self.func_key_indices[start..end];
        let subslice_index = subslice.binary_search(&raw_index).ok()?;
        let index = start + subslice_index;
        Some(usize::try_from(index).unwrap())
    }

    /// Get the location of the function associated with the given `key` inside
    /// the text section, if any.
    pub fn func_loc(&self, key: FuncKey) -> Option<&FunctionLoc> {
        let index = self.index(key)?;
        let ret = &self.func_locs[index];
        Some(ret)
    }

    /// Get the initial source location of the function associated with the
    /// given `key`, if any.
    pub fn src_loc(&self, key: FuncKey) -> Option<FilePos> {
        let index = self.index(key)?;
        Some(self.src_locs[index])
    }

    /// Given an offset into the text section, get the key for its associated
    /// function and its offset within that function.
    pub fn func_by_text_offset(&self, text_offset: u32) -> Option<(FuncKey, u32)> {
        let index = match self.func_locs.binary_search_by_key(&text_offset, |loc| {
            debug_assert!(loc.length > 0);
            // Return the inclusive "end" of the function
            loc.start + loc.length - 1
        }) {
            // Exact match, the offset is at the end of this function.
            Ok(k) => k,
            // Not an exact match: `k` is where the offset would be
            // "inserted". Since we key based on the end, function `k` might
            // contain the offset, so we'll validate on the range check
            // below.
            Err(k) => k,
        };

        let loc = self.func_locs.get(index)?;
        let start = loc.start;
        let end = loc.start + loc.length;

        if text_offset < start || end < text_offset {
            return None;
        }

        let func_offset = text_offset - start;
        let key_index = self.func_key_indices[index];
        let index = u32::try_from(index).unwrap();

        let kind_index = match self
            .func_key_kinds
            .binary_search_by_key(&index, |(_kind, start_index)| *start_index)
        {
            // Exact match: `i` is the kind index.
            Ok(i) => i,
            // Not an exact match: the index, if it were the start of a kind's
            // range, would be at `i`. Therefore, our kind is actually at index
            // `i - 1`.
            Err(i) => {
                assert_ne!(i, 0);
                i - 1
            }
        };

        let (kind, _) = self.func_key_kinds[kind_index];
        let key = FuncKey::from_raw_parts(kind, key_index);

        Some((key, func_offset))
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

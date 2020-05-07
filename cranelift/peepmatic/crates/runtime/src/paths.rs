//! Representing paths through the dataflow graph.
//!
//! Paths are relative from a *root* instruction, which is the instruction we
//! are determining which, if any, optimizations apply.
//!
//! Paths are series of indices through each instruction's children as we
//! traverse down the graph from the root. Children are immediates followed by
//! arguments: `[imm0, imm1, ..., immN, arg0, arg1, ..., argN]`.
//!
//! ## Examples
//!
//! * `[0]` is the path to the root.
//! * `[0, 0]` is the path to the root's first child.
//! * `[0, 1]` is the path to the root's second child.
//! * `[0, 1, 0]` is the path to the root's second child's first child.
//!
//! ## Interning
//!
//! To avoid extra allocations, de-duplicate paths, and reference them via a
//! fixed-length value, we intern paths inside a `PathInterner` and then
//! reference them via `PathId`.

// TODO: Make `[]` the path to the root, and get rid of this redundant leading
// zero that is currently in every single path.

use serde::de::{Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A path through the data-flow graph from the root instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Path<'a>(pub &'a [u8]);

impl Path<'_> {
    /// Construct a new path through the data-flow graph from the root
    /// instruction.
    pub fn new(path: &impl AsRef<[u8]>) -> Path {
        Path(path.as_ref())
    }
}

/// An identifier for an interned path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathId(u16);

/// An interner and de-duplicator for `Path`s.
///
/// Can be serialized and deserialized while maintaining the same id to interned
/// path mapping.
#[derive(Debug, Default)]
pub struct PathInterner {
    // A map from a path (whose owned data is inside `arena`) to the canonical
    // `PathId` we assigned it when interning it.
    map: HashMap<UnsafePath, PathId>,

    // A map from a `PathId` index to an unsafe, self-borrowed path pointing
    // into `arena`. It is safe to given these out as safe `Path`s, as long as
    // the lifetime is not longer than this `PathInterner`'s lifetime.
    paths: Vec<UnsafePath>,

    // Bump allocation arena for path data. The bump arena ensures that these
    // allocations never move, and are therefore safe for self-references.
    arena: bumpalo::Bump,
}

impl PathInterner {
    /// Construct a new, empty `PathInterner`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a path into this `PathInterner`, returning its canonical
    /// `PathId`.
    ///
    /// If we've already interned this path before, then the existing id we
    /// already assigned to it is returned. If we've never seen this path
    /// before, then it is copied into this `PathInterner` and a new id is
    /// assigned to it.
    #[inline]
    pub fn intern<'a>(&mut self, path: Path<'a>) -> PathId {
        let unsafe_path = unsafe { UnsafePath::from_path(&path) };
        if let Some(id) = self.map.get(&unsafe_path) {
            return *id;
        }
        self.intern_new(path)
    }

    #[inline(never)]
    fn intern_new<'a>(&mut self, path: Path<'a>) -> PathId {
        let id: u16 = self
            .paths
            .len()
            .try_into()
            .expect("too many paths interned");
        let id = PathId(id);

        let our_path = self.arena.alloc_slice_copy(&path.0);
        let unsafe_path = unsafe { UnsafePath::from_slice(&our_path) };

        self.paths.push(unsafe_path.clone());
        let old = self.map.insert(unsafe_path, id);

        debug_assert!(old.is_none());
        debug_assert_eq!(self.lookup(id), path);
        debug_assert_eq!(self.intern(path), id);

        id
    }

    /// Lookup a previously interned path by id.
    #[inline]
    pub fn lookup<'a>(&'a self, id: PathId) -> Path<'a> {
        let unsafe_path = self
            .paths
            .get(id.0 as usize)
            .unwrap_or_else(|| Self::lookup_failure());
        unsafe { unsafe_path.as_path() }
    }

    #[inline(never)]
    fn lookup_failure() -> ! {
        panic!(
            "no path for the given id; this can only happen when mixing `PathId`s with different \
             `PathInterner`s"
        )
    }
}

impl Serialize for PathInterner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.paths.len()))?;
        for p in &self.paths {
            let p = unsafe { p.as_path() };
            seq.serialize_element(&p)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for PathInterner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(PathInternerVisitor {
            marker: PhantomData,
        })
    }
}

struct PathInternerVisitor {
    marker: PhantomData<fn() -> PathInterner>,
}

impl<'de> Visitor<'de> for PathInternerVisitor {
    type Value = PathInterner;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a `peepmatic_runtime::paths::PathInterner`")
    }

    fn visit_seq<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: SeqAccess<'de>,
    {
        const DEFAULT_CAPACITY: usize = 16;
        let capacity = access.size_hint().unwrap_or(DEFAULT_CAPACITY);

        let mut interner = PathInterner {
            map: HashMap::with_capacity(capacity),
            paths: Vec::with_capacity(capacity),
            arena: bumpalo::Bump::new(),
        };

        while let Some(path) = access.next_element::<Path>()? {
            interner.intern(path);
        }

        Ok(interner)
    }
}

/// An unsafe, unchecked borrow of a path. Not for use outside of
/// `PathInterner`!
#[derive(Clone, Debug)]
struct UnsafePath {
    ptr: *const u8,
    len: usize,
}

impl PartialEq for UnsafePath {
    fn eq(&self, rhs: &UnsafePath) -> bool {
        unsafe { self.as_slice() == rhs.as_slice() }
    }
}

impl Eq for UnsafePath {}

impl Hash for UnsafePath {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        unsafe { self.as_slice().hash(hasher) }
    }
}

/// Safety: callers must ensure that the constructed values won't have unsafe
/// usages of `PartialEq`, `Eq`, or `Hash`.
impl UnsafePath {
    unsafe fn from_path(p: &Path) -> Self {
        Self::from_slice(&p.0)
    }

    unsafe fn from_slice(s: &[u8]) -> Self {
        UnsafePath {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }
}

/// Safety: callers must ensure that `'a` does not outlive the lifetime of the
/// underlying data.
impl UnsafePath {
    unsafe fn as_slice<'a>(&self) -> &'a [u8] {
        std::slice::from_raw_parts(self.ptr, self.len)
    }

    unsafe fn as_path<'a>(&self) -> Path<'a> {
        Path(self.as_slice())
    }
}

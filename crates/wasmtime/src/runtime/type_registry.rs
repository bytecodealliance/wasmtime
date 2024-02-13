//! Implement a registry of types: function, struct, and array definitions.
//!
//! Helps implement fast indirect call signature checking, reference type
//! casting, and etc.

use std::fmt::Debug;
use std::{collections::HashMap, sync::RwLock};
use std::{convert::TryFrom, sync::Arc};
use wasmtime_environ::{ModuleInternedTypeIndex, ModuleTypes, PrimaryMap, WasmFuncType};
use wasmtime_runtime::VMSharedTypeIndex;

use crate::Engine;

/// Represents a collection of shared types.
///
/// This is used to register shared types with a shared type registry.
///
/// The collection will unregister any contained types with the registry
/// when dropped.
pub struct TypeCollection {
    engine: Engine,
    types: PrimaryMap<ModuleInternedTypeIndex, VMSharedTypeIndex>,
    reverse_types: HashMap<VMSharedTypeIndex, ModuleInternedTypeIndex>,
}

impl Debug for TypeCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let TypeCollection {
            engine: _,
            types,
            reverse_types: _,
        } = self;
        f.debug_struct("TypeCollection")
            .field("types", types)
            .finish_non_exhaustive()
    }
}

impl TypeCollection {
    /// Creates a type collection for a module given the module's types.
    pub fn new_for_module(engine: &Engine, types: &ModuleTypes) -> Self {
        let engine = engine.clone();
        let registry = engine.signatures();
        let types = registry.0.write().unwrap().register_for_module(types);
        let reverse_types = types.iter().map(|(k, v)| (*v, k)).collect();

        Self {
            engine,
            types,
            reverse_types,
        }
    }

    /// Treats the type collection as a map from a module type index to
    /// registered shared type indexes.
    ///
    /// This is used for looking up module shared type indexes during module
    /// instantiation.
    pub fn as_module_map(&self) -> &PrimaryMap<ModuleInternedTypeIndex, VMSharedTypeIndex> {
        &self.types
    }

    /// Gets the shared type index given a module type index.
    #[inline]
    pub fn shared_type(&self, index: ModuleInternedTypeIndex) -> Option<VMSharedTypeIndex> {
        self.types.get(index).copied()
    }

    /// Get the module-local type index for the given shared type index.
    pub fn module_local_type(&self, index: VMSharedTypeIndex) -> Option<ModuleInternedTypeIndex> {
        self.reverse_types.get(&index).copied()
    }
}

impl Drop for TypeCollection {
    fn drop(&mut self) {
        if !self.types.is_empty() {
            self.engine
                .signatures()
                .0
                .write()
                .unwrap()
                .unregister_types(self);
        }
    }
}

/// A Wasm type that has been registered in the engine's `TypeRegistry`.
///
/// Prevents its associated type from being unregistered while it is alive.
///
/// Automatically unregisters the type on drop. (Unless other `RegisteredTypes`
/// are keeping the type registered).
///
/// Dereferences to its underlying `WasmFuncType`.
pub struct RegisteredType {
    engine: Engine,
    index: VMSharedTypeIndex,

    // This field is not *strictly* necessary to have in this type, since we
    // could always grab the registry's lock and look it up by index, but
    // holding this reference should make accessing the actual type that much
    // cheaper.
    ty: Arc<WasmFuncType>,
}

impl Debug for RegisteredType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let RegisteredType {
            engine: _,
            index,
            ty,
        } = self;
        f.debug_struct("RegisteredType")
            .field("index", index)
            .field("ty", ty)
            .finish_non_exhaustive()
    }
}

impl Clone for RegisteredType {
    fn clone(&self) -> Self {
        {
            let i = usize::try_from(self.index.bits()).unwrap();
            let mut registry = self.engine.signatures().0.write().unwrap();
            let entry = registry.entries[i].unwrap_occupied_mut();
            entry.references += 1;
            log::trace!(
                "cloned registered type {:?} (references -> {})",
                self.index,
                entry.references
            );
        }

        RegisteredType {
            engine: self.engine.clone(),
            index: self.index,
            ty: Arc::clone(&self.ty),
        }
    }
}

impl Drop for RegisteredType {
    fn drop(&mut self) {
        self.engine
            .signatures()
            .0
            .write()
            .unwrap()
            .unregister_entry(self.index, 1);
    }
}

impl std::ops::Deref for RegisteredType {
    type Target = Arc<WasmFuncType>;

    fn deref(&self) -> &Self::Target {
        &self.ty
    }
}

impl PartialEq for RegisteredType {
    fn eq(&self, other: &Self) -> bool {
        let eq = Arc::ptr_eq(&self.ty, &other.ty);

        if cfg!(debug_assertions) {
            if eq {
                assert_eq!(self.index, other.index);
                assert!(Engine::same(&self.engine, &other.engine));
            } else {
                assert!(self.index != other.index || !Engine::same(&self.engine, &other.engine));
            }
        }

        eq
    }
}

impl Eq for RegisteredType {}

impl std::hash::Hash for RegisteredType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.ty);
        ptr.hash(state);
    }
}

impl RegisteredType {
    /// Constructs a new `RegisteredType`, registering the given type with the
    /// engine's `TypeRegistry`.
    pub fn new(engine: &Engine, ty: &WasmFuncType) -> RegisteredType {
        let (index, ty) = engine.signatures().0.write().unwrap().register_raw(ty);
        RegisteredType::from_parts(engine.clone(), index, ty)
    }

    /// Create an owning handle to the given index's associated type.
    ///
    /// This will prevent the associated type from being unregistered as long as
    /// the returned `RegisteredType` is kept alive.
    ///
    /// Returns `None` if `index` is not registered in the given engine's
    /// registry.
    pub fn root(engine: &Engine, index: VMSharedTypeIndex) -> Option<RegisteredType> {
        let i = usize::try_from(index.bits()).unwrap();
        let ty = {
            let mut inner = engine.signatures().0.write().unwrap();
            let e = inner.entries.get_mut(i)?.as_occupied_mut()?;
            e.references += 1;
            log::trace!("rooting {index:?} (references -> {})", e.references);
            Arc::clone(&e.ty)
        };
        Some(RegisteredType::from_parts(engine.clone(), index, ty))
    }

    /// Construct a new `RegisteredType`.
    ///
    /// It is the caller's responsibility to ensure that the entry's reference
    /// count has already been incremented.
    fn from_parts(engine: Engine, index: VMSharedTypeIndex, ty: Arc<WasmFuncType>) -> Self {
        debug_assert!({
            let registry = engine.signatures().0.read().unwrap();
            let i = usize::try_from(index.bits()).unwrap();
            let e = registry.entries[i].as_occupied().unwrap();
            e.references > 0
        });
        RegisteredType { engine, index, ty }
    }

    /// Get this registered type's index.
    pub fn index(&self) -> VMSharedTypeIndex {
        self.index
    }
}

#[derive(Debug)]
struct OccupiedEntry {
    ty: Arc<WasmFuncType>,
    references: usize,
}

#[derive(Debug)]
enum RegistryEntry {
    /// An occupied entry containing a registered type.
    Occupied(OccupiedEntry),

    /// A vacant entry that is additionally a link in the free list of all
    /// vacant entries.
    Vacant {
        /// The next link in the free list of all vacant entries, if any.
        next_vacant: Option<VMSharedTypeIndex>,
    },
}

impl RegistryEntry {
    fn is_vacant(&self) -> bool {
        matches!(self, Self::Vacant { .. })
    }

    fn as_occupied(&self) -> Option<&OccupiedEntry> {
        match self {
            Self::Occupied(o) => Some(o),
            Self::Vacant { .. } => None,
        }
    }

    fn as_occupied_mut(&mut self) -> Option<&mut OccupiedEntry> {
        match self {
            Self::Occupied(o) => Some(o),
            Self::Vacant { .. } => None,
        }
    }

    fn unwrap_occupied_mut(&mut self) -> &mut OccupiedEntry {
        match self {
            Self::Occupied(o) => o,
            Self::Vacant { .. } => panic!("unwrap_occupied_mut on vacant entry"),
        }
    }

    fn unwrap_next_vacant(&self) -> Option<VMSharedTypeIndex> {
        match self {
            Self::Vacant { next_vacant } => *next_vacant,
            Self::Occupied(_) => panic!("unwrap_next_vacant on occupied entry"),
        }
    }
}

#[derive(Debug, Default)]
struct TypeRegistryInner {
    // A map from the Wasm function type to a `VMSharedTypeIndex`, for all
    // the Wasm function types we have already registered.
    map: HashMap<Arc<WasmFuncType>, VMSharedTypeIndex>,

    // A map from `VMSharedTypeIndex::bits()` to the type index's associated
    // Wasm type.
    entries: Vec<RegistryEntry>,

    // The head of the free list of the entries that are vacant and can
    // therefore (along with their associated `VMSharedTypeIndex`) be reused.
    //
    // This is a size optimization, and arguably not strictly necessary for
    // correctness, but is necessary to avoid unbounded memory growth: if we did
    // not reuse entries/indices, we would have holes in our `self.entries` list
    // and, as we load and unload new Wasm modules, `self.entries` would keep
    // growing indefinitely.
    first_vacant: Option<VMSharedTypeIndex>,
}

impl TypeRegistryInner {
    fn register_for_module(
        &mut self,
        types: &ModuleTypes,
    ) -> PrimaryMap<ModuleInternedTypeIndex, VMSharedTypeIndex> {
        log::trace!("Registering module types");
        let mut map = PrimaryMap::default();
        for (idx, ty) in types.wasm_types() {
            let (shared_type_index, _) = self.register_raw(ty);
            let map_idx = map.push(shared_type_index);
            assert_eq!(idx, map_idx);
        }
        map
    }

    /// Add a new type to this registry.
    ///
    /// Does not increment its reference count, that is the responsibility of
    /// callers.
    fn register_new(&mut self, ty: Arc<WasmFuncType>) -> VMSharedTypeIndex {
        let (index, entry) = match self.first_vacant.take() {
            Some(index) => {
                let i = usize::try_from(index.bits()).unwrap();
                let entry = &mut self.entries[i];
                self.first_vacant = entry.unwrap_next_vacant();
                (index, entry)
            }
            None => {
                debug_assert_eq!(self.entries.len(), self.map.len());

                let len = self.entries.len();
                let len = u32::try_from(len).unwrap();

                // Keep `index_map`'s length under `u32::MAX` because
                // `u32::MAX` is reserved for `VMSharedTypeIndex`'s
                // default value.
                assert!(
                    len < std::u32::MAX,
                    "Invariant check: self.entries.len() < std::u32::MAX"
                );

                let index = VMSharedTypeIndex::new(len);
                self.entries
                    .push(RegistryEntry::Vacant { next_vacant: None });

                (index, self.entries.last_mut().unwrap())
            }
        };

        let old_map_entry = self.map.insert(ty.clone(), index);
        assert!(old_map_entry.is_none());

        assert!(entry.is_vacant());
        *entry = RegistryEntry::Occupied(OccupiedEntry { ty, references: 0 });

        index
    }

    /// Register the given type, incrementing its reference count.
    fn register_raw(&mut self, ty: &WasmFuncType) -> (VMSharedTypeIndex, Arc<WasmFuncType>) {
        let index = if let Some(i) = self.map.get(ty) {
            *i
        } else {
            let ty = Arc::new(ty.clone());
            self.register_new(ty)
        };

        let i = usize::try_from(index.bits()).unwrap();
        let entry = self.entries[i].unwrap_occupied_mut();
        entry.references += 1;

        log::trace!("registered {index:?} (references -> {})", entry.references);

        (index, Arc::clone(&entry.ty))
    }

    fn unregister_types(&mut self, collection: &TypeCollection) {
        for (_, index) in collection.types.iter() {
            self.unregister_entry(*index, 1);
        }
    }

    fn unregister_entry(&mut self, index: VMSharedTypeIndex, count: usize) {
        let i = usize::try_from(index.bits()).unwrap();
        let entry = self.entries[i].unwrap_occupied_mut();

        assert!(entry.references >= count);
        entry.references -= count;
        log::trace!(
            "unregistered {index:?} by {count} (references -> {})",
            entry.references
        );

        if entry.references == 0 {
            self.map.remove(&entry.ty);
            self.entries[i] = RegistryEntry::Vacant {
                next_vacant: self.first_vacant.take(),
            };
            self.first_vacant = Some(index);
        }
    }
}

// `TypeRegistryInner` implements `Drop` in debug builds to assert that
// all types have been unregistered for the registry.
#[cfg(debug_assertions)]
impl Drop for TypeRegistryInner {
    fn drop(&mut self) {
        assert!(
            self.map.is_empty(),
            "type registry not empty: still have registered types in self.map"
        );
        assert!(
            self.entries.iter().all(|e| e.is_vacant()),
            "type registry not empty: not all entries are vacant"
        );
    }
}

/// Implements a shared type registry.
///
/// WebAssembly requires that the caller and callee types in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// types, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug)]
pub struct TypeRegistry(RwLock<TypeRegistryInner>);

impl TypeRegistry {
    /// Creates a new shared type registry.
    pub fn new() -> Self {
        Self(RwLock::new(TypeRegistryInner::default()))
    }

    /// Looks up a function type from a shared type index.
    ///
    /// This does *NOT* prevent the type from being unregistered while you are
    /// still using the resulting value! Use the `RegisteredType::root`
    /// constructor if you need to ensure that property and you don't have some
    /// other mechanism already keeping the type registered.
    pub fn borrow(&self, index: VMSharedTypeIndex) -> Option<Arc<WasmFuncType>> {
        let i = usize::try_from(index.bits()).unwrap();
        let inner = self.0.read().unwrap();
        let e = inner.entries.get(i)?;
        Some(e.as_occupied()?.ty.clone())
    }
}

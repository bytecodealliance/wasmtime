//! Implement a registry of types: function, struct, and array definitions.
//!
//! Helps implement fast indirect call signature checking, reference type
//! casting, and etc.

use crate::Engine;
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::Debug,
    sync::{Arc, RwLock},
};
use wasmtime_environ::{
    EngineOrModuleTypeIndex, ModuleInternedTypeIndex, ModuleTypes, PrimaryMap, TypeTrace,
    WasmFuncType,
};
use wasmtime_runtime::VMSharedTypeIndex;

// ### Notes on the Lifetime Management of Types
//
// (The below refers to recursion groups even though at the time of writing we
// don't support Wasm GC yet, which introduced recursion groups. Until that
// time, you can think of each type implicitly being in a singleton recursion
// group, so types and recursion groups are effectively one to one.)
//
// All defined types from all Wasm modules loaded into Wasmtime are interned
// into their engine's `TypeRegistry`.
//
// With Wasm MVP, managing type lifetimes within the registry was easy: we only
// cared about canonicalizing types so that `call_indirect` was fast and we
// didn't waste memory on many copies of the same function type definition.
// Function types could only take and return simple scalars (i32/f64/etc...) and
// there were no type-to-type references. We could simply deduplicate types and
// reference count their entries in the registry.
//
// The typed function references and GC proposals change everything. The former
// introduced function types that take a reference to a function of another
// specific type. This is a type-to-type reference. The latter introduces struct
// and array types that can have references to other struct, array, and function
// types, as well as recursion groups that allow cyclic references between
// types. Now type canonicalization additionally enables fast type checks
// *across* modules: so that two modules which define the same struct type, for
// example, can pass instances of that struct type to each other, and we can
// quickly check that those instances are in fact of the expected types.
//
// But how do we manage the lifetimes of types that can reference other types as
// Wasm modules are dynamically loaded and unloaded from the engine? These
// modules can define subsets of the same types and there can be cyclic type
// references. Dynamic lifetimes, sharing, and cycles is a classic combination
// of constraints that push a design towards a tracing garbage collector (or,
// equivalently, a reference-counting collector with a cycle collector).
//
// However, we can rely on the following properties:
//
// 1. The unit of type canonicalization is a whole recursion group.
//
// 2. Type-to-type reference cycles may only happen within a recursion group and
//    therefore type-to-type references across recursion groups are acyclic.
//
// Therefore, our type registry makes the following design decisions:
//
// * We manage the lifetime of whole recursion groups, not individual
//   types. That is, every type in the recursion group stays alive as long as
//   any type in the recursion group is kept alive. This is effectively mandated
//   by property (1) and the hash consing it implies.
//
// * We still use naive reference counting to manage the lifetimes of recursion
//   groups. A type-to-type reference that crosses the boundary from recursion
//   group A to recursion group B will increment B's reference count when A is
//   first registered and decrement B's reference count when A is removed from
//   the registry. Because of property (2) we don't need to worry about cycles,
//   which are the classic weakness of reference counting.

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

#[inline]
fn entry_index(index: VMSharedTypeIndex) -> usize {
    usize::try_from(index.bits()).unwrap()
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
            let i = entry_index(self.index);
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
            .unregister_entry(self.index);
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
    pub fn new(engine: &Engine, ty: WasmFuncType) -> RegisteredType {
        let (index, ty) = {
            let mut inner = engine.signatures().0.write().unwrap();

            log::trace!("RegisteredType::new({ty:?})");

            // It shouldn't be possible for users to construct non-canonical types
            // via the embedding API, and the only other types they can get are
            // already-canonicalized types from modules, so we shouldn't ever get
            // non-canonical types here.
            assert!(
                inner.is_canonicalized(&ty),
                "ty is not already canonicalized: {ty:?}"
            );

            inner.register_canonicalized(ty)
        };
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
        let i = entry_index(index);
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
            let i = entry_index(index);
            let e = registry.entries[i].as_occupied().unwrap();
            e.references > 0
        });
        RegisteredType { engine, index, ty }
    }

    /// Get this registered type's index.
    pub fn index(&self) -> VMSharedTypeIndex {
        self.index
    }

    /// Get the engine whose registry this type is registered within.
    pub fn engine(&self) -> &Engine {
        &self.engine
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

    fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied(_))
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

    // An explicit stack of entries that we are in the middle of dropping. Used
    // to avoid recursion when dropping a type that is holding the last
    // reference to another type, etc...
    drop_stack: Vec<VMSharedTypeIndex>,
}

impl TypeRegistryInner {
    fn register_for_module(
        &mut self,
        types: &ModuleTypes,
    ) -> PrimaryMap<ModuleInternedTypeIndex, VMSharedTypeIndex> {
        log::trace!("Registering module types");
        let mut map = PrimaryMap::default();
        for (idx, ty) in types.wasm_types() {
            let mut ty = ty.clone();
            self.canonicalize(&map, &mut ty);
            let (shared_type_index, _) = self.register_canonicalized(ty);
            let map_idx = map.push(shared_type_index);
            assert_eq!(idx, map_idx);
        }
        map
    }

    /// Is the given type canonicalized for this registry?
    fn is_canonicalized(&self, ty: &WasmFuncType) -> bool {
        let result = ty.trace::<_, ()>(&mut |index| match index {
            EngineOrModuleTypeIndex::Module(_) => Err(()),
            EngineOrModuleTypeIndex::Engine(id) => {
                let id = VMSharedTypeIndex::new(id);
                let i = entry_index(id);
                assert!(
                    self.entries[i].is_occupied(),
                    "canonicalized in a different engine? {ty:?}"
                );
                Ok(())
            }
        });
        result.is_ok()
    }

    /// Canonicalize a type, such that its type-to-type references are via
    /// engine-level `VMSharedTypeIndex`es rather than module-local via
    /// `ModuleInternedTypeIndex`es.
    ///
    /// This makes the type suitable for deduplication in the registry.
    ///
    /// Panics on already-canonicalized types. They might be canonicalized for
    /// another engine's registry, and we wouldn't know how to recanonicalize
    /// them for this registry.
    fn canonicalize(
        &self,
        module_to_shared: &PrimaryMap<ModuleInternedTypeIndex, VMSharedTypeIndex>,
        ty: &mut WasmFuncType,
    ) {
        ty.trace_mut::<_, ()>(&mut |index| match index {
            EngineOrModuleTypeIndex::Engine(_) => unreachable!("already canonicalized?"),
            EngineOrModuleTypeIndex::Module(module_index) => {
                *index = EngineOrModuleTypeIndex::Engine(module_to_shared[*module_index].bits());
                Ok(())
            }
        })
        .unwrap();

        debug_assert!(self.is_canonicalized(ty))
    }

    /// Allocate a vacant entry, either from the free list, or creating a new
    /// entry.
    fn alloc_vacant_entry(&mut self) -> VMSharedTypeIndex {
        match self.first_vacant.take() {
            // Pop a vacant entry off the free list when we can.
            Some(index) => {
                let i = entry_index(index);
                let entry = &mut self.entries[i];
                self.first_vacant = entry.unwrap_next_vacant();
                index
            }

            // Otherwise, allocate a new entry.
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

                index
            }
        }
    }

    /// Add a new type to this registry.
    ///
    /// The type must be canonicalized and must not already exist in the
    /// registry.
    ///
    /// Does not increment the new entry's reference count, that is the
    /// responsibility of callers.
    fn register_new(&mut self, ty: Arc<WasmFuncType>) -> VMSharedTypeIndex {
        assert!(
            self.is_canonicalized(&ty),
            "ty is not already canonicalized: {ty:?}"
        );

        let index = self.alloc_vacant_entry();
        let old_map_entry = self.map.insert(ty.clone(), index);
        assert!(old_map_entry.is_none());

        // Increment the ref count of each existing type that is referenced from
        // this new type. Those types shouldn't be dropped while this type is
        // still alive.
        ty.trace::<_, ()>(&mut |idx| match idx {
            EngineOrModuleTypeIndex::Engine(id) => {
                let id = VMSharedTypeIndex::new(id);
                let i = entry_index(id);
                let e = self.entries[i].unwrap_occupied_mut();
                e.references += 1;
                log::trace!(
                    "new type has edge to {id:?} (references -> {})",
                    e.references
                );
                Ok(())
            }
            EngineOrModuleTypeIndex::Module(_) => unreachable!("should be canonicalized"),
        })
        .unwrap();

        let i = entry_index(index);
        assert!(self.entries[i].is_vacant());
        self.entries[i] = RegistryEntry::Occupied(OccupiedEntry {
            ty,
            // NB: It is the caller's responsibility to increment this.
            references: 0,
        });

        index
    }

    /// Register the given canonicalized type, incrementing its reference count.
    fn register_canonicalized(
        &mut self,
        ty: WasmFuncType,
    ) -> (VMSharedTypeIndex, Arc<WasmFuncType>) {
        assert!(
            self.is_canonicalized(&ty),
            "ty is not already canonicalized: {ty:?}"
        );

        let index = if let Some(i) = self.map.get(&ty) {
            *i
        } else {
            self.register_new(Arc::new(ty))
        };

        let i = entry_index(index);
        let entry = self.entries[i].unwrap_occupied_mut();
        entry.references += 1;

        log::trace!(
            "registered {index:?} = {:?} (references -> {})",
            entry.ty,
            entry.references
        );

        (index, Arc::clone(&entry.ty))
    }

    fn unregister_types(&mut self, collection: &TypeCollection) {
        for (_, index) in collection.types.iter() {
            self.unregister_entry(*index);
        }
    }

    fn unregister_entry(&mut self, index: VMSharedTypeIndex) {
        log::trace!("unregistering {index:?}");

        debug_assert!(self.drop_stack.is_empty());
        self.drop_stack.push(index);

        while let Some(id) = self.drop_stack.pop() {
            let i = entry_index(id);
            let entry = self.entries[i].unwrap_occupied_mut();

            assert!(entry.references > 0);
            entry.references -= 1;
            log::trace!(
                "unregistered {index:?} (references -> {})",
                entry.references
            );

            if entry.references == 0 {
                // Enqueue the other types that are (shallowly/non-transitively)
                // referenced from this type for having their ref count
                // decremented as well. This type is no longer holding them
                // alive.
                entry
                    .ty
                    .trace::<_, ()>(&mut |idx| match idx {
                        EngineOrModuleTypeIndex::Engine(child_id) => {
                            let child_id = VMSharedTypeIndex::new(child_id);
                            log::trace!("dropping {id:?} enqueues {child_id:?} for unregistration");
                            self.drop_stack.push(child_id);
                            Ok(())
                        }
                        EngineOrModuleTypeIndex::Module(_) => {
                            unreachable!("should be canonicalized")
                        }
                    })
                    .unwrap();

                self.map.remove(&entry.ty);
                self.entries[i] = RegistryEntry::Vacant {
                    next_vacant: self.first_vacant.take(),
                };
                self.first_vacant = Some(index);
            }
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
        let i = entry_index(index);
        let inner = self.0.read().unwrap();
        let e = inner.entries.get(i)?;
        Some(e.as_occupied()?.ty.clone())
    }
}

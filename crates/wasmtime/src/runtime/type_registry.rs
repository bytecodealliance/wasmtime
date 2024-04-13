//! Implement a registry of types: function, struct, and array definitions.
//!
//! Helps implement fast indirect call signature checking, reference type
//! casting, and etc.

use crate::Engine;
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::Deref,
    sync::{
        atomic::{
            AtomicUsize,
            Ordering::{AcqRel, Acquire},
        },
        Arc, RwLock,
    },
};
use wasmtime_environ::{
    EngineOrModuleTypeIndex, ModuleInternedTypeIndex, ModuleTypes, PrimaryMap, TypeTrace,
    VMSharedTypeIndex, WasmFuncType,
};
use wasmtime_slab::{Id as SlabId, Slab};

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
                .unregister_type_collection(self);
        }
    }
}

#[inline]
fn shared_type_index_to_slab_id(index: VMSharedTypeIndex) -> SlabId {
    SlabId::from_raw(index.bits())
}

#[inline]
fn slab_id_to_shared_type_index(id: SlabId) -> VMSharedTypeIndex {
    VMSharedTypeIndex::new(id.into_raw())
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
    entry: Entry,
}

impl Debug for RegisteredType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let RegisteredType { engine: _, entry } = self;
        f.debug_struct("RegisteredType")
            .field("entry", entry)
            .finish_non_exhaustive()
    }
}

impl Clone for RegisteredType {
    fn clone(&self) -> Self {
        self.entry.incref("cloning RegisteredType");
        RegisteredType {
            engine: self.engine.clone(),
            entry: self.entry.clone(),
        }
    }
}

impl Drop for RegisteredType {
    fn drop(&mut self) {
        if self.entry.decref("dropping RegisteredType") {
            self.engine
                .signatures()
                .0
                .write()
                .unwrap()
                .unregister_entry(self.entry.0.index);
        }
    }
}

impl std::ops::Deref for RegisteredType {
    type Target = WasmFuncType;

    fn deref(&self) -> &Self::Target {
        &self.entry.0.ty
    }
}

impl PartialEq for RegisteredType {
    fn eq(&self, other: &Self) -> bool {
        let eq = Arc::ptr_eq(&self.entry.0, &other.entry.0);

        if cfg!(debug_assertions) {
            if eq {
                assert!(Engine::same(&self.engine, &other.engine));
            } else {
                assert!(
                    self.entry.0.index != other.entry.0.index
                        || !Engine::same(&self.engine, &other.engine)
                );
            }
        }

        eq
    }
}

impl Eq for RegisteredType {}

impl Hash for RegisteredType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.entry.0);
        ptr.hash(state);
    }
}

impl RegisteredType {
    /// Constructs a new `RegisteredType`, registering the given type with the
    /// engine's `TypeRegistry`.
    pub fn new(engine: &Engine, ty: WasmFuncType) -> RegisteredType {
        let entry = {
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
        RegisteredType::from_parts(engine.clone(), entry)
    }

    /// Create an owning handle to the given index's associated type.
    ///
    /// This will prevent the associated type from being unregistered as long as
    /// the returned `RegisteredType` is kept alive.
    ///
    /// Returns `None` if `index` is not registered in the given engine's
    /// registry.
    pub fn root(engine: &Engine, index: VMSharedTypeIndex) -> Option<RegisteredType> {
        let entry = {
            let id = shared_type_index_to_slab_id(index);
            let inner = engine.signatures().0.read().unwrap();
            let e = inner.entries.get(id)?;

            // NB: make sure to incref while the lock is held to prevent:
            //
            // * This thread: read locks registry, gets entry E, unlocks registry
            // * Other thread: drops `RegisteredType` for entry E, decref
            //   reaches zero, write locks registry, unregisters entry
            // * This thread: increfs entry, but it isn't in the registry anymore
            e.incref("RegisteredType::root");

            e.clone()
        };

        Some(RegisteredType::from_parts(engine.clone(), entry))
    }

    /// Construct a new `RegisteredType`.
    ///
    /// It is the caller's responsibility to ensure that the entry's reference
    /// count has already been incremented.
    fn from_parts(engine: Engine, entry: Entry) -> Self {
        debug_assert!(entry.0.registrations.load(Acquire) != 0);
        RegisteredType { engine, entry }
    }

    /// Get this registered type's index.
    pub fn index(&self) -> VMSharedTypeIndex {
        self.entry.0.index
    }

    /// Get the engine whose registry this type is registered within.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}

/// A Wasm function type, its `VMSharedTypeIndex`, and its registration count.
#[derive(Debug)]
struct EntryInner {
    ty: WasmFuncType,
    index: VMSharedTypeIndex,
    registrations: AtomicUsize,
}

/// Implements `Borrow`, `Eq`, and `Hash` by forwarding to the underlying Wasm
/// function type, so that this can be a hash consing key in
/// `TypeRegistryInner::map`.
#[derive(Clone, Debug)]
struct Entry(Arc<EntryInner>);

impl Deref for Entry {
    type Target = WasmFuncType;

    fn deref(&self) -> &Self::Target {
        &self.0.ty
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.0.ty == other.0.ty
    }
}

impl Eq for Entry {}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.ty.hash(state);
    }
}

impl Borrow<WasmFuncType> for Entry {
    fn borrow(&self) -> &WasmFuncType {
        &self.0.ty
    }
}

impl Entry {
    /// Increment the registration count.
    fn incref(&self, why: &str) {
        let old_count = self.0.registrations.fetch_add(1, AcqRel);
        log::trace!(
            "increment registration count for {:?} (registrations -> {}): {why}",
            self.0.index,
            old_count + 1
        );
    }

    /// Decrement the registration count and return `true` if the registration
    /// count reached zero and this entry should be removed from the registry.
    #[must_use = "caller must remove entry from registry if `decref` returns `true`"]
    fn decref(&self, why: &str) -> bool {
        let old_count = self.0.registrations.fetch_sub(1, AcqRel);
        debug_assert_ne!(old_count, 0);
        log::trace!(
            "decrement registration count for {:?} (registrations -> {}): {why}",
            self.0.index,
            old_count - 1
        );
        old_count == 1
    }
}

#[derive(Debug, Default)]
struct TypeRegistryInner {
    // A map from the Wasm function type to a `VMSharedTypeIndex`, for all
    // the Wasm function types we have already registered.
    map: HashSet<Entry>,

    // A map from `VMSharedTypeIndex::bits()` to the type index's associated
    // Wasm type.
    entries: Slab<Entry>,

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
            let entry = self.register_canonicalized(ty);
            let map_idx = map.push(entry.0.index);
            assert_eq!(idx, map_idx);
        }
        map
    }

    /// Is the given type canonicalized for this registry?
    fn is_canonicalized(&self, ty: &WasmFuncType) -> bool {
        let result = ty.trace::<_, ()>(&mut |index| match index {
            EngineOrModuleTypeIndex::Module(_) => Err(()),
            EngineOrModuleTypeIndex::Engine(id) => {
                let id = shared_type_index_to_slab_id(id);
                assert!(
                    self.entries.contains(id),
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
        ty.canonicalize(&mut |module_index| module_to_shared[module_index]);
        debug_assert!(self.is_canonicalized(ty))
    }

    /// Add a new type to this registry.
    ///
    /// The type must be canonicalized and must not already exist in the
    /// registry.
    ///
    /// Initializes the new entry's registration count to one, and callers
    /// should not further increment the registration count.
    fn register_new(&mut self, ty: WasmFuncType) -> Entry {
        assert!(
            self.is_canonicalized(&ty),
            "ty is not already canonicalized: {ty:?}"
        );

        // Increment the ref count of each existing type that is referenced from
        // this new type. Those types shouldn't be dropped while this type is
        // still alive.
        ty.trace::<_, ()>(&mut |idx| match idx {
            EngineOrModuleTypeIndex::Engine(id) => {
                let i = shared_type_index_to_slab_id(id);
                let e = &self.entries[i];
                e.incref("new type references existing type in TypeRegistryInner::register_new");
                Ok(())
            }
            EngineOrModuleTypeIndex::Module(_) => unreachable!("should be canonicalized"),
        })
        .unwrap();

        let id = self.entries.next_id();
        let index = slab_id_to_shared_type_index(id);
        log::trace!("create {index:?} = {ty:?} (registrations -> 1)");
        let entry = Entry(Arc::new(EntryInner {
            ty,
            index,
            registrations: AtomicUsize::new(1),
        }));
        let is_new_entry = self.map.insert(entry.clone());
        assert!(is_new_entry);

        let id = self.entries.alloc(entry.clone());
        assert_eq!(id, shared_type_index_to_slab_id(index));

        entry
    }

    /// Register the given canonicalized type, incrementing its reference count.
    fn register_canonicalized(&mut self, ty: WasmFuncType) -> Entry {
        assert!(
            self.is_canonicalized(&ty),
            "type is not already canonicalized: {ty:?}"
        );

        if let Some(entry) = self.map.get(&ty) {
            entry.incref(
                "registering already-registered type in TypeRegistryInner::register_canonicalized",
            );
            entry.clone()
        } else {
            self.register_new(ty)
        }
    }

    fn unregister_type_collection(&mut self, collection: &TypeCollection) {
        for (_, id) in collection.types.iter() {
            let i = shared_type_index_to_slab_id(*id);
            let e = &self.entries[i];
            if e.decref("TypeRegistryInner::unregister_type_collection") {
                self.unregister_entry(*id);
            }
        }
    }

    /// Remove an entry from the registry.
    ///
    /// This does *not* decrement the entry's registration count, it should
    /// instead be invoked after a previous decrement operation observed zero
    /// remaining registrations.
    fn unregister_entry(&mut self, index: VMSharedTypeIndex) {
        log::trace!("unregistering {index:?}");

        debug_assert!(self.drop_stack.is_empty());
        self.drop_stack.push(index);

        while let Some(index) = self.drop_stack.pop() {
            let slab_id = shared_type_index_to_slab_id(index);
            let entry = &self.entries[slab_id];

            // We need to double check whether the entry is still at zero
            // registrations: Between the time that we observed a zero and
            // acquired the lock to call this function, another thread could
            // have registered the type and found the 0-registrations entry in
            // `self.map` and incremented its count.
            //
            // We don't need to worry about any concurrent increments during
            // this function's invocation after we check for zero because we
            // have exclusive access to `&mut self` and therefore no one can
            // create a new reference to this entry and bring it back to life.
            let registrations = entry.0.registrations.load(Acquire);
            if registrations != 0 {
                log::trace!(
                    "{index:?} was concurrently resurrected and no longer has zero \
                     registrations (registrations -> {registrations})"
                );
                continue;
            }

            // Decrement any other types that this type was
            // (shallowly/non-transitively) keeping alive.
            entry
                .0
                .ty
                .trace::<_, ()>(&mut |child_index| match child_index {
                    EngineOrModuleTypeIndex::Engine(child_index) => {
                        let child_slab_id = shared_type_index_to_slab_id(child_index);
                        let child_entry = &self.entries[child_slab_id];
                        if child_entry.decref(
                            "referenced by unregistered type in TypeCollection::unregister_entry",
                        ) {
                            self.drop_stack.push(child_index);
                        }
                        Ok(())
                    }
                    EngineOrModuleTypeIndex::Module(_) => {
                        unreachable!("should be canonicalized")
                    }
                })
                .unwrap();

            log::trace!("removing {index:?} from registry");
            self.map.remove(entry);
            self.entries.dealloc(slab_id);
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
            self.entries.is_empty(),
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
    pub fn borrow(&self, index: VMSharedTypeIndex) -> Option<impl Deref<Target = WasmFuncType>> {
        let id = shared_type_index_to_slab_id(index);
        let inner = self.0.read().unwrap();
        inner.entries.get(id).cloned()
    }
}

//! Implement a registry of types: function, struct, and array definitions.
//!
//! Helps implement fast indirect call signature checking, reference type
//! downcasting, and etc...

use crate::Engine;
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::Range,
    sync::{
        atomic::{
            AtomicUsize,
            Ordering::{AcqRel, Acquire},
        },
        Arc, RwLock,
    },
};
use wasmtime_environ::{
    iter_entity_range, EngineOrModuleTypeIndex, ModuleInternedTypeIndex, ModuleTypes, PrimaryMap,
    TypeTrace, VMSharedTypeIndex, WasmRecGroup, WasmSubType,
};
use wasmtime_slab::{Id as SlabId, Slab};

// ### Notes on the Lifetime Management of Types
//
// All defined types from all Wasm modules loaded into Wasmtime are interned
// into their engine's `TypeRegistry`.
//
// With Wasm MVP, managing type lifetimes within the registry was easy: we only
// cared about canonicalizing types so that `call_indirect` was fast and we
// didn't waste memory on many copies of the same function type definition.
// Function types could only take and return simple scalars (i32/f64/etc...) and
// there were no type-to-type references. We could simply deduplicate function
// types and reference count their entries in the registry.
//
// The typed function references and GC proposals change everything. The former
// introduced function types that take a reference to a function of another
// specific type. This is a type-to-type reference. The latter introduces struct
// and array types that can have references to other struct, array, and function
// types, as well as recursion groups that allow cyclic references between
// types. Now type canonicalization additionally enables fast type checks and
// downcasts *across* modules: so that two modules which define the same struct
// type, for example, can pass instances of that struct type to each other, and
// we can quickly check that those instances are in fact of the expected types.
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
    rec_groups: Vec<RecGroupEntry>,
    types: PrimaryMap<ModuleInternedTypeIndex, VMSharedTypeIndex>,
    reverse_types: HashMap<VMSharedTypeIndex, ModuleInternedTypeIndex>,
}

impl Debug for TypeCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let TypeCollection {
            engine: _,
            rec_groups,
            types,
            reverse_types: _,
        } = self;
        f.debug_struct("TypeCollection")
            .field("rec_groups", rec_groups)
            .field("types", types)
            .finish_non_exhaustive()
    }
}

impl TypeCollection {
    /// Creates a type collection for a module given the module's types.
    pub fn new_for_module(engine: &Engine, types: &ModuleTypes) -> Self {
        let engine = engine.clone();
        let registry = engine.signatures();
        let (rec_groups, types) = registry.0.write().unwrap().register_module_types(types);
        let reverse_types = types.iter().map(|(k, v)| (*v, k)).collect();

        Self {
            engine,
            rec_groups,
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
        if !self.rec_groups.is_empty() {
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
/// Dereferences to its underlying `WasmSubType`.
pub struct RegisteredType {
    engine: Engine,
    entry: RecGroupEntry,
    ty: Arc<WasmSubType>,
    index: VMSharedTypeIndex,
}

impl Debug for RegisteredType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let RegisteredType {
            engine: _,
            entry: _,
            ty,
            index,
        } = self;
        f.debug_struct("RegisteredType")
            .field("index", index)
            .field("ty", ty)
            .finish_non_exhaustive()
    }
}

impl Clone for RegisteredType {
    fn clone(&self) -> Self {
        self.entry.incref("cloning RegisteredType");
        RegisteredType {
            engine: self.engine.clone(),
            entry: self.entry.clone(),
            ty: self.ty.clone(),
            index: self.index,
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
                .unregister_entry(self.entry.clone());
        }
    }
}

impl std::ops::Deref for RegisteredType {
    type Target = WasmSubType;

    fn deref(&self) -> &Self::Target {
        &self.ty
    }
}

impl PartialEq for RegisteredType {
    fn eq(&self, other: &Self) -> bool {
        let eq = Arc::ptr_eq(&self.entry.0, &other.entry.0);

        if cfg!(debug_assertions) {
            if eq {
                assert!(Engine::same(&self.engine, &other.engine));
                assert_eq!(self.ty, other.ty);
            } else {
                assert!(self.ty != other.ty || !Engine::same(&self.engine, &other.engine));
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
    pub fn new(engine: &Engine, ty: WasmSubType) -> RegisteredType {
        let (entry, index, ty) = {
            log::trace!("RegisteredType::new({ty:?})");

            let mut inner = engine.signatures().0.write().unwrap();

            // It shouldn't be possible for users to construct non-canonical
            // types via the embedding API, and the only other types they can
            // get are already-canonicalized types from modules, so we shouldn't
            // ever get non-canonical types here. Furthermore, this is only
            // called internally to Wasmtime, so we shouldn't ever have an
            // engine mismatch; those should be caught earlier.
            inner.assert_canonicalized_for_runtime_usage_in_this_registry(&ty);

            let entry = inner.register_singleton_rec_group(ty);

            let index = entry.0.shared_type_indices[0];
            let id = shared_type_index_to_slab_id(index);
            let ty = inner.types[id].clone();

            (entry, index, ty)
        };

        RegisteredType::from_parts(engine.clone(), entry, index, ty)
    }

    /// Create an owning handle to the given index's associated type.
    ///
    /// This will prevent the associated type from being unregistered as long as
    /// the returned `RegisteredType` is kept alive.
    ///
    /// Returns `None` if `index` is not registered in the given engine's
    /// registry.
    pub fn root(engine: &Engine, index: VMSharedTypeIndex) -> Option<RegisteredType> {
        let (entry, ty) = {
            let id = shared_type_index_to_slab_id(index);
            let inner = engine.signatures().0.read().unwrap();

            let ty = inner.types.get(id)?.clone();
            let entry = inner.type_to_rec_group[&index].clone();

            // NB: make sure to incref while the lock is held to prevent:
            //
            // * This thread: read locks registry, gets entry E, unlocks registry
            // * Other thread: drops `RegisteredType` for entry E, decref
            //   reaches zero, write locks registry, unregisters entry
            // * This thread: increfs entry, but it isn't in the registry anymore
            entry.incref("RegisteredType::root");

            (entry, ty)
        };

        Some(RegisteredType::from_parts(engine.clone(), entry, index, ty))
    }

    /// Construct a new `RegisteredType`.
    ///
    /// It is the caller's responsibility to ensure that the entry's reference
    /// count has already been incremented.
    fn from_parts(
        engine: Engine,
        entry: RecGroupEntry,
        index: VMSharedTypeIndex,
        ty: Arc<WasmSubType>,
    ) -> Self {
        debug_assert!(entry.0.registrations.load(Acquire) != 0);
        RegisteredType {
            engine,
            entry,
            ty,
            index,
        }
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

/// An entry in the type registry.
///
/// Implements `Borrow`, `Eq`, and `Hash` by forwarding to the underlying Wasm
/// rec group, so that this can be a hash consing key. (We can't use
/// `Arc<RecGroupEntryInner>` directly for this purpose because `Arc<T>` doesn't
/// implement `Borrow<U>` when `T: Borrow<U>`).
#[derive(Clone)]
struct RecGroupEntry(Arc<RecGroupEntryInner>);

impl Debug for RecGroupEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct Ptr<'a, P>(&'a P);
        impl<P: std::fmt::Pointer> Debug for Ptr<'_, P> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:#p}", *self.0)
            }
        }

        f.debug_struct("RecGroupEntry")
            .field("ptr", &Ptr(&self.0))
            .field("shared_type_indices", &self.0.shared_type_indices)
            .field("hash_consing_key", &self.0.hash_consing_key)
            .field("registrations", &self.0.registrations.load(Acquire))
            .finish()
    }
}

struct RecGroupEntryInner {
    /// The Wasm rec group, canonicalized for hash consing.
    hash_consing_key: WasmRecGroup,
    shared_type_indices: Box<[VMSharedTypeIndex]>,
    registrations: AtomicUsize,
}

impl PartialEq for RecGroupEntry {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash_consing_key == other.0.hash_consing_key
    }
}

impl Eq for RecGroupEntry {}

impl Hash for RecGroupEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash_consing_key.hash(state);
    }
}

impl Borrow<WasmRecGroup> for RecGroupEntry {
    fn borrow(&self) -> &WasmRecGroup {
        &self.0.hash_consing_key
    }
}

impl RecGroupEntry {
    /// Increment the registration count.
    fn incref(&self, why: &str) {
        let old_count = self.0.registrations.fetch_add(1, AcqRel);
        log::trace!(
            "increment registration count for {self:?} (registrations -> {}): {why}",
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
            "decrement registration count for {self:?} (registrations -> {}): {why}",
            old_count - 1
        );
        old_count == 1
    }
}

#[derive(Debug, Default)]
struct TypeRegistryInner {
    // A hash map from a canonicalized-for-hash-consing rec group to its
    // `VMSharedTypeIndex`es.
    //
    // There is an entry in this map for every rec group we have already
    // registered. Before registering new rec groups, we first check this map to
    // see if we've already registered an identical rec group that we should
    // reuse instead.
    hash_consing_map: HashSet<RecGroupEntry>,

    // A map from `VMSharedTypeIndex::bits()` to the type index's associated
    // Wasm type.
    //
    // These types are always canonicalized for runtime usage.
    types: Slab<Arc<WasmSubType>>,

    // A map that lets you walk backwards from a `VMSharedTypeIndex` to its
    // `RecGroupEntry`.
    type_to_rec_group: HashMap<VMSharedTypeIndex, RecGroupEntry>,

    // An explicit stack of entries that we are in the middle of dropping. Used
    // to avoid recursion when dropping a type that is holding the last
    // reference to another type, etc...
    drop_stack: Vec<RecGroupEntry>,
}

impl TypeRegistryInner {
    fn register_module_types(
        &mut self,
        types: &ModuleTypes,
    ) -> (
        Vec<RecGroupEntry>,
        PrimaryMap<ModuleInternedTypeIndex, VMSharedTypeIndex>,
    ) {
        log::trace!("Start registering module types");

        let mut entries = Vec::with_capacity(types.rec_groups().len());
        let mut map = PrimaryMap::<ModuleInternedTypeIndex, VMSharedTypeIndex>::with_capacity(
            types.wasm_types().len(),
        );

        for (_rec_group_index, module_group) in types.rec_groups() {
            let entry = self.register_rec_group(
                &map,
                module_group.clone(),
                iter_entity_range(module_group.clone()).map(|ty| types.get(ty).clone()),
            );

            for (module_ty, engine_ty) in
                iter_entity_range(module_group).zip(entry.0.shared_type_indices.iter())
            {
                let module_ty2 = map.push(*engine_ty);
                assert_eq!(module_ty, module_ty2);
            }

            entries.push(entry);
        }

        log::trace!("End registering module types");

        (entries, map)
    }

    /// Register a rec group in this registry.
    ///
    /// The rec group may be either module-level canonical (i.e. straight from
    /// `wasmparser`) or engine-level canonicalized for runtime usage in this
    /// registry. It may *not* be engine-level canonicalized for hash consing or
    /// engine-level canonicalized for a different type registry instance.
    ///
    /// If this rec group is determined to be a duplicate of an
    /// already-registered rec group, the existing rec group is reused.
    ///
    /// Parameters:
    ///
    /// * `map`: A map that we use to canonicalize inter-group type references
    ///   from module-canonical to engine-canonical indices. This must contain
    ///   entries for each inter-group type reference that this rec group
    ///   contains.
    ///
    /// * `range`: The range of (module-level) types defined by this rec
    ///   group. This is used to determine which type references inside this rec
    ///   group are inter- vs intra-group.
    ///
    /// * `types`: The types defined within this rec group. Must have the same
    ///   length as `range`.
    ///
    /// The returned entry will have already had its reference count incremented
    /// on behalf of callers.
    fn register_rec_group(
        &mut self,
        map: &PrimaryMap<ModuleInternedTypeIndex, VMSharedTypeIndex>,
        range: Range<ModuleInternedTypeIndex>,
        types: impl ExactSizeIterator<Item = WasmSubType>,
    ) -> RecGroupEntry {
        debug_assert_eq!(iter_entity_range(range.clone()).len(), types.len());

        let mut non_canon_types = Vec::with_capacity(types.len());
        let hash_consing_key = WasmRecGroup {
            types: types
                .zip(iter_entity_range(range.clone()))
                .map(|(mut ty, module_index)| {
                    non_canon_types.push((module_index, ty.clone()));
                    ty.canonicalize_for_hash_consing(range.clone(), &mut |idx| {
                        debug_assert!(idx < range.clone().start);
                        map[idx]
                    });
                    ty
                })
                .collect::<Box<[_]>>(),
        };

        // If we've already registered this rec group before, reuse it.
        if let Some(entry) = self.hash_consing_map.get(&hash_consing_key) {
            entry.incref(
                "hash consed to already-registered type in `TypeRegistryInner::register_rec_group`",
            );
            return entry.clone();
        }

        // Inter-group edges: increment the referenced group's ref
        // count, because these other rec groups shouldn't be dropped
        // while this rec group is still alive.
        hash_consing_key
            .trace_engine_indices::<_, ()>(&mut |index| {
                let entry = &self.type_to_rec_group[&index];
                entry.incref(
                    "new cross-group type reference to existing type in `register_rec_group`",
                );
                Ok(())
            })
            .unwrap();

        // Register the individual types.
        //
        // Note that we can't update the reverse type-to-rec-group map until
        // after we've constructed the `RecGroupEntry`, since that map needs to
        // the fully-constructed entry for its values.
        let module_rec_group_start = range.start;
        let engine_rec_group_start = u32::try_from(self.types.len()).unwrap();
        let shared_type_indices = non_canon_types
            .into_iter()
            .map(|(module_index, mut ty)| {
                ty.canonicalize_for_runtime_usage(&mut |idx| {
                    if idx < module_rec_group_start {
                        map[idx]
                    } else {
                        let rec_group_offset = idx.as_u32() - module_rec_group_start.as_u32();
                        VMSharedTypeIndex::from_u32(engine_rec_group_start + rec_group_offset)
                    }
                });
                self.insert_one_type_from_rec_group(module_index, ty)
            })
            .collect();

        let entry = RecGroupEntry(Arc::new(RecGroupEntryInner {
            hash_consing_key,
            shared_type_indices,
            registrations: AtomicUsize::new(1),
        }));
        log::trace!("create new entry {entry:?} (registrations -> 1)");

        let is_new_entry = self.hash_consing_map.insert(entry.clone());
        debug_assert!(is_new_entry);

        // Now that we've construct the entry, we can update the reverse
        // type-to-rec-group map.
        for ty in entry.0.shared_type_indices.iter() {
            let old_entry = self.type_to_rec_group.insert(*ty, entry.clone());
            debug_assert!(old_entry.is_none());
        }

        entry
    }

    /// Is the given type canonicalized for runtime usage this registry?
    fn assert_canonicalized_for_runtime_usage_in_this_registry(&self, ty: &WasmSubType) {
        ty.trace::<_, ()>(&mut |index| match index {
            EngineOrModuleTypeIndex::RecGroup(_) | EngineOrModuleTypeIndex::Module(_) => {
                panic!("not canonicalized for runtime usage: {ty:?}")
            }
            EngineOrModuleTypeIndex::Engine(idx) => {
                let id = shared_type_index_to_slab_id(idx);
                assert!(
                    self.types.contains(id),
                    "canonicalized in a different engine? {ty:?}"
                );
                Ok(())
            }
        })
        .unwrap();
    }

    /// Insert a new type as part of registering a new rec group.
    ///
    /// The type must be canonicalized for runtime usage in this registry and
    /// its rec group must be a new one that we are currently registering, not
    /// an already-registered rec group.
    fn insert_one_type_from_rec_group(
        &mut self,
        module_index: ModuleInternedTypeIndex,
        ty: WasmSubType,
    ) -> VMSharedTypeIndex {
        // Despite being canonicalized for runtime usage, this type may still
        // have forward references to other types in the rec group we haven't
        // yet registered. Therefore, we can't use our usual
        // `assert_canonicalized_for_runtime_usage_in_this_registry` helper here
        // as that will see the forward references and think they must be
        // references to types in other registries.
        assert!(
            ty.is_canonicalized_for_runtime_usage(),
            "type is not canonicalized for runtime usage: {ty:?}"
        );

        let id = self.types.alloc(Arc::new(ty));
        let engine_index = slab_id_to_shared_type_index(id);
        log::trace!(
            "registered type {module_index:?} as {engine_index:?} = {:?}",
            &self.types[id]
        );
        engine_index
    }

    /// Register a rec group consisting of a single type.
    ///
    /// The type must already be canonicalized for runtime usage in this
    /// registry.
    ///
    /// The returned entry will have already had its reference count incremented
    /// on behalf of callers.
    fn register_singleton_rec_group(&mut self, ty: WasmSubType) -> RecGroupEntry {
        self.assert_canonicalized_for_runtime_usage_in_this_registry(&ty);

        // This type doesn't have any module-level type references, since it is
        // already canonicalized for runtime usage in this registry, so an empty
        // map suffices.
        let map = PrimaryMap::default();

        // This must have `range.len() == 1`, even though we know this type
        // doesn't have any intra-group type references, to satisfy
        // `register_rec_group`'s preconditions.
        let range = ModuleInternedTypeIndex::from_bits(u32::MAX - 1)
            ..ModuleInternedTypeIndex::from_bits(u32::MAX);

        self.register_rec_group(&map, range, std::iter::once(ty))
    }

    /// Unregister all of a type collection's rec groups.
    fn unregister_type_collection(&mut self, collection: &TypeCollection) {
        for entry in &collection.rec_groups {
            if entry.decref("TypeRegistryInner::unregister_type_collection") {
                self.unregister_entry(entry.clone());
            }
        }
    }

    /// Remove a zero-refcount entry from the registry.
    ///
    /// This does *not* decrement the entry's registration count, it should
    /// instead be invoked only after a previous decrement operation observed
    /// zero remaining registrations.
    fn unregister_entry(&mut self, entry: RecGroupEntry) {
        debug_assert!(self.drop_stack.is_empty());
        self.drop_stack.push(entry);

        while let Some(entry) = self.drop_stack.pop() {
            log::trace!("Start unregistering {entry:?}");

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
                    "{entry:?} was concurrently resurrected and no longer has \
                     zero registrations (registrations -> {registrations})",
                );
                continue;
            }

            // Decrement any other types that this type was shallowly
            // (i.e. non-transitively) referencing and keeping alive. If this
            // was the last thing keeping them registered, its okay to
            // unregister them as well now.
            debug_assert!(entry.0.hash_consing_key.is_canonicalized_for_hash_consing());
            entry
                .0
                .hash_consing_key
                .trace_engine_indices::<_, ()>(&mut |other_index| {
                    let other_entry = self.type_to_rec_group[&other_index].clone();
                    if other_entry.decref(
                        "referenced by dropped entry in \
                         `TypeCollection::unregister_entry`",
                    ) {
                        self.drop_stack.push(other_entry);
                    }
                    Ok(())
                })
                .unwrap();

            // Remove the entry from the hash-consing map. If we register a
            // duplicate definition of this rec group again in the future, it
            // will be as if it is the first time it has ever been registered,
            // and it will be inserted into the hash-consing map again at that
            // time.
            self.hash_consing_map.remove(&entry);

            // Similarly, remove the rec group's types from the registry, as
            // well as their entries from the reverse type-to-rec-group map.
            for ty in entry.0.shared_type_indices.iter() {
                log::trace!("removing {ty:?} from registry");

                let removed_entry = self.type_to_rec_group.remove(ty);
                debug_assert_eq!(removed_entry.unwrap(), entry);

                let id = shared_type_index_to_slab_id(*ty);
                self.types.dealloc(id);
            }

            log::trace!("End unregistering {entry:?}");
        }
    }
}

// `TypeRegistryInner` implements `Drop` in debug builds to assert that
// all types have been unregistered for the registry.
#[cfg(debug_assertions)]
impl Drop for TypeRegistryInner {
    fn drop(&mut self) {
        let TypeRegistryInner {
            hash_consing_map,
            types,
            type_to_rec_group,
            drop_stack,
        } = self;
        assert!(
            hash_consing_map.is_empty(),
            "type registry not empty: hash consing map is not empty"
        );
        assert!(
            types.is_empty(),
            "type registry not empty: types slab is not empty"
        );
        assert!(
            type_to_rec_group.is_empty(),
            "type registry not empty: type-to-rec-group map is not empty"
        );
        assert!(
            drop_stack.is_empty(),
            "type registry not empty: drop stack is not empty"
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
    pub fn borrow(&self, index: VMSharedTypeIndex) -> Option<Arc<WasmSubType>> {
        let id = shared_type_index_to_slab_id(index);
        let inner = self.0.read().unwrap();
        inner.types.get(id).cloned()
    }
}

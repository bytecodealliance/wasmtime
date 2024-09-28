//! Implement a registry of types: function, struct, and array definitions.
//!
//! Helps implement fast indirect call signature checking, reference type
//! downcasting, and etc...

use crate::hash_set::HashSet;
use crate::prelude::*;
use crate::sync::RwLock;
use crate::vm::GcRuntime;
use crate::Engine;
use alloc::borrow::Cow;
use alloc::sync::Arc;
use core::iter;
use core::{
    borrow::Borrow,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    ops::Range,
    sync::atomic::{
        AtomicUsize,
        Ordering::{AcqRel, Acquire},
    },
};
use wasmtime_environ::{
    iter_entity_range, packed_option::PackedOption, EngineOrModuleTypeIndex, GcLayout,
    ModuleInternedTypeIndex, ModuleTypes, PrimaryMap, SecondaryMap, TypeTrace, VMSharedTypeIndex,
    WasmRecGroup, WasmSubType,
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
    trampolines: SecondaryMap<VMSharedTypeIndex, PackedOption<ModuleInternedTypeIndex>>,
}

impl Debug for TypeCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let TypeCollection {
            engine: _,
            rec_groups,
            types,
            trampolines,
        } = self;
        f.debug_struct("TypeCollection")
            .field("rec_groups", rec_groups)
            .field("types", types)
            .field("trampolines", trampolines)
            .finish_non_exhaustive()
    }
}

impl TypeCollection {
    /// Creates a type collection for a module given the module's types.
    pub fn new_for_module(engine: &Engine, module_types: &ModuleTypes) -> Self {
        let engine = engine.clone();
        let registry = engine.signatures();
        let gc_runtime = engine.gc_runtime();
        let (rec_groups, types) = registry
            .0
            .write()
            .register_module_types(&**gc_runtime, module_types);

        let mut trampolines = SecondaryMap::with_capacity(types.len());
        for (module_ty, trampoline) in module_types.trampoline_types() {
            let shared_ty = types[module_ty];
            let trampoline_ty = registry.trampoline_type(shared_ty);
            trampolines[trampoline_ty] = Some(trampoline).into();
        }

        Self {
            engine,
            rec_groups,
            types,
            trampolines,
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

    /// Get the module-level type index of the trampoline type for the given
    /// engine-level function type, if any.
    ///
    /// This allows callers to look up the pre-compiled wasm-to-native
    /// trampoline in this type collection's associated module.
    ///
    /// See the docs for `WasmFuncType::trampoline_type` for details on
    /// trampoline types.
    #[inline]
    pub fn trampoline_type(&self, ty: VMSharedTypeIndex) -> Option<ModuleInternedTypeIndex> {
        let trampoline_ty = self.trampolines[ty].expand();
        log::trace!("TypeCollection::trampoline_type({ty:?}) -> {trampoline_ty:?}");
        trampoline_ty
    }
}

impl Drop for TypeCollection {
    fn drop(&mut self) {
        if !self.rec_groups.is_empty() {
            self.engine
                .signatures()
                .0
                .write()
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
    layout: Option<GcLayout>,
}

impl Debug for RegisteredType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let RegisteredType {
            engine: _,
            entry: _,
            ty,
            index,
            layout,
        } = self;
        f.debug_struct("RegisteredType")
            .field("index", index)
            .field("ty", ty)
            .field("layout", layout)
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
            layout: self.layout.clone(),
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
                .unregister_entry(self.entry.clone());
        }
    }
}

impl core::ops::Deref for RegisteredType {
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
        let (entry, index, ty, layout) = {
            log::trace!("RegisteredType::new({ty:?})");

            let gc_runtime = engine.gc_runtime();
            let mut inner = engine.signatures().0.write();

            // It shouldn't be possible for users to construct non-canonical
            // types via the embedding API, and the only other types they can
            // get are already-canonicalized types from modules, so we shouldn't
            // ever get non-canonical types here. Furthermore, this is only
            // called internally to Wasmtime, so we shouldn't ever have an
            // engine mismatch; those should be caught earlier.
            inner.assert_canonicalized_for_runtime_usage_in_this_registry(&ty);

            let entry = inner.register_singleton_rec_group(&**gc_runtime, ty);

            let index = entry.0.shared_type_indices[0];
            let id = shared_type_index_to_slab_id(index);
            let ty = inner.types[id].clone();
            let layout = inner.type_to_gc_layout.get(index).and_then(|l| l.clone());

            (entry, index, ty, layout)
        };

        RegisteredType::from_parts(engine.clone(), entry, index, ty, layout)
    }

    /// Create an owning handle to the given index's associated type.
    ///
    /// This will prevent the associated type from being unregistered as long as
    /// the returned `RegisteredType` is kept alive.
    ///
    /// Returns `None` if `index` is not registered in the given engine's
    /// registry.
    pub fn root(engine: &Engine, index: VMSharedTypeIndex) -> Option<RegisteredType> {
        let (entry, ty, layout) = {
            let id = shared_type_index_to_slab_id(index);
            let inner = engine.signatures().0.read();

            let ty = inner.types.get(id)?.clone();
            let entry = inner.type_to_rec_group[index].clone().unwrap();
            let layout = inner.type_to_gc_layout.get(index).and_then(|l| l.clone());

            // NB: make sure to incref while the lock is held to prevent:
            //
            // * This thread: read locks registry, gets entry E, unlocks registry
            // * Other thread: drops `RegisteredType` for entry E, decref
            //   reaches zero, write locks registry, unregisters entry
            // * This thread: increfs entry, but it isn't in the registry anymore
            entry.incref("RegisteredType::root");

            (entry, ty, layout)
        };

        Some(RegisteredType::from_parts(
            engine.clone(),
            entry,
            index,
            ty,
            layout,
        ))
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
        layout: Option<GcLayout>,
    ) -> Self {
        debug_assert!(entry.0.registrations.load(Acquire) != 0);
        RegisteredType {
            engine,
            entry,
            ty,
            index,
            layout,
        }
    }

    /// Get the engine whose registry this type is registered within.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get this registered type's index.
    pub fn index(&self) -> VMSharedTypeIndex {
        self.index
    }

    /// Get this registered type's GC layout, if any.
    ///
    /// Only struct and array types have GC layouts; function types do not have
    /// layouts.
    pub fn layout(&self) -> Option<&GcLayout> {
        self.layout.as_ref()
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Ptr<'a, P>(&'a P);
        impl<P: fmt::Pointer> Debug for Ptr<'_, P> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    type_to_rec_group: SecondaryMap<VMSharedTypeIndex, Option<RecGroupEntry>>,

    // A map from a registered type to its complete list of supertypes.
    //
    // The supertypes are ordered from super- to subtype, i.e. the immediate
    // parent supertype is the last element and the least-upper-bound of all
    // supertypes is the first element.
    //
    // Types without any supertypes are omitted from this map. This means that
    // we never allocate any backing storage for this map when Wasm GC is not in
    // use.
    type_to_supertypes: SecondaryMap<VMSharedTypeIndex, Option<Box<[VMSharedTypeIndex]>>>,

    // A map from each registered function type to its trampoline type.
    //
    // Note that when a function type is its own trampoline type, then we omit
    // the entry in this map as a memory optimization. This means that if only
    // core Wasm function types are ever used, then we will never allocate any
    // backing storage for this map. As a nice bonus, this also avoids cycles (a
    // function type referencing itself) that our naive reference counting
    // doesn't play well with.
    type_to_trampoline: SecondaryMap<VMSharedTypeIndex, PackedOption<VMSharedTypeIndex>>,

    // A map from each registered GC type to its layout.
    //
    // Function types do not have an entry in this map. Similar to the
    // `type_to_{supertypes,trampoline}` maps, we completely omit the `None`
    // entries for these types as a memory optimization.
    type_to_gc_layout: SecondaryMap<VMSharedTypeIndex, Option<GcLayout>>,

    // An explicit stack of entries that we are in the middle of dropping. Used
    // to avoid recursion when dropping a type that is holding the last
    // reference to another type, etc...
    drop_stack: Vec<RecGroupEntry>,
}

impl TypeRegistryInner {
    fn register_module_types(
        &mut self,
        gc_runtime: &dyn GcRuntime,
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
                gc_runtime,
                &map,
                module_group.clone(),
                iter_entity_range(module_group.clone()).map(|ty| types[ty].clone()),
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
        gc_runtime: &dyn GcRuntime,
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
                let entry = &self.type_to_rec_group[index].as_ref().unwrap();
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
        let shared_type_indices: Box<[_]> = non_canon_types
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
                self.insert_one_type_from_rec_group(gc_runtime, module_index, ty)
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

        // Now that we've constructed the entry, we can update the reverse
        // type-to-rec-group map.
        for ty in entry.0.shared_type_indices.iter().copied() {
            debug_assert!(self.type_to_rec_group[ty].is_none());
            self.type_to_rec_group[ty] = Some(entry.clone());
        }

        // Finally, make sure to register the trampoline type for each function
        // type in the rec group.
        for shared_type_index in entry.0.shared_type_indices.iter().copied() {
            let slab_id = shared_type_index_to_slab_id(shared_type_index);
            if let Some(f) = self.types[slab_id].as_func() {
                match f.trampoline_type() {
                    Cow::Borrowed(_) => {
                        // The function type is its own trampoline type. Leave
                        // its entry in `type_to_trampoline` empty to signal
                        // this.
                    }
                    Cow::Owned(trampoline) => {
                        // This will recursively call into rec group
                        // registration, but at most once since trampoline
                        // function types are their own trampoline type.
                        let trampoline_entry = self.register_singleton_rec_group(
                            gc_runtime,
                            WasmSubType {
                                is_final: true,
                                supertype: None,
                                composite_type: wasmtime_environ::WasmCompositeType::Func(
                                    trampoline,
                                ),
                            },
                        );
                        let trampoline_index = trampoline_entry.0.shared_type_indices[0];
                        log::trace!(
                            "Registering trampoline {trampoline_index:?} for function type {shared_type_index:?}"
                        );
                        debug_assert_ne!(shared_type_index, trampoline_index);
                        self.type_to_trampoline[shared_type_index] = Some(trampoline_index).into();
                    }
                }
            }
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
        gc_runtime: &dyn GcRuntime,
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

        let gc_layout = match &ty.composite_type {
            wasmtime_environ::WasmCompositeType::Func(_) => None,
            wasmtime_environ::WasmCompositeType::Array(a) => {
                Some(gc_runtime.layouts().array_layout(a).into())
            }
            wasmtime_environ::WasmCompositeType::Struct(s) => {
                Some(gc_runtime.layouts().struct_layout(s).into())
            }
        };

        // Add the type to our slab.
        let id = self.types.alloc(Arc::new(ty));
        let engine_index = slab_id_to_shared_type_index(id);
        log::trace!(
            "registered type {module_index:?} as {engine_index:?} = {:?}",
            &self.types[id]
        );

        // Create the supertypes list for this type.
        if let Some(supertype) = self.types[id].supertype {
            let supertype = supertype.unwrap_engine_type_index();
            let supers_supertypes = self.supertypes(supertype);
            let mut supertypes = Vec::with_capacity(supers_supertypes.len() + 1);
            supertypes.extend(
                supers_supertypes
                    .iter()
                    .copied()
                    .chain(iter::once(supertype)),
            );
            self.type_to_supertypes[engine_index] = Some(supertypes.into_boxed_slice());
        }

        // Only write the type-to-gc-layout entry if we have a GC layout, so
        // that the map can avoid any heap allocation for backing storage in the
        // case where Wasm GC is disabled.
        if let Some(layout) = gc_layout {
            self.type_to_gc_layout[engine_index] = Some(layout);
        }

        engine_index
    }

    /// Get the supertypes list for the given type.
    ///
    /// The supertypes are listed in super-to-sub order. `ty` itself is not
    /// included in the list.
    fn supertypes(&self, ty: VMSharedTypeIndex) -> &[VMSharedTypeIndex] {
        self.type_to_supertypes
            .get(ty)
            .and_then(|s| s.as_deref())
            .unwrap_or(&[])
    }

    /// Register a rec group consisting of a single type.
    ///
    /// The type must already be canonicalized for runtime usage in this
    /// registry.
    ///
    /// The returned entry will have already had its reference count incremented
    /// on behalf of callers.
    fn register_singleton_rec_group(
        &mut self,
        gc_runtime: &dyn GcRuntime,
        ty: WasmSubType,
    ) -> RecGroupEntry {
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

        self.register_rec_group(gc_runtime, &map, range, iter::once(ty))
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
                    let other_entry = self.type_to_rec_group[other_index].as_ref().unwrap();
                    if other_entry.decref(
                        "referenced by dropped entry in \
                         `TypeCollection::unregister_entry`",
                    ) {
                        self.drop_stack.push(other_entry.clone());
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
            // well as their entries from the reverse type-to-rec-group
            // map. Additionally, stop holding a strong reference from each
            // function type in the rec group to that function type's trampoline
            // type.
            for ty in entry.0.shared_type_indices.iter().copied() {
                log::trace!("removing {ty:?} from registry");

                let removed_entry = self.type_to_rec_group[ty].take();
                debug_assert_eq!(removed_entry.unwrap(), entry);

                // Remove the associated trampoline type, if any.
                if let Some(trampoline_ty) =
                    self.type_to_trampoline.get(ty).and_then(|x| x.expand())
                {
                    self.type_to_trampoline[ty] = None.into();
                    let trampoline_entry = self.type_to_rec_group[trampoline_ty].as_ref().unwrap();
                    if trampoline_entry
                        .decref("removing reference from a function type to its trampoline type")
                    {
                        self.drop_stack.push(trampoline_entry.clone());
                    }
                }

                // Remove the type's supertypes list, if any. Take care to guard
                // this assignment so that we don't accidentally force the
                // secondary map to allocate even when we never actually use
                // Wasm GC.
                if self.type_to_supertypes.get(ty).is_some() {
                    self.type_to_supertypes[ty] = None;
                }

                // Same as above, but for the type's GC layout.
                if self.type_to_gc_layout.get(ty).is_some() {
                    self.type_to_gc_layout[ty] = None;
                }

                let id = shared_type_index_to_slab_id(ty);
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
        log::trace!("Dropping type registry: {self:#?}");
        let TypeRegistryInner {
            hash_consing_map,
            types,
            type_to_rec_group,
            type_to_supertypes,
            type_to_trampoline,
            type_to_gc_layout,
            drop_stack,
        } = self;
        assert!(
            hash_consing_map.is_empty(),
            "type registry not empty: hash consing map is not empty: {hash_consing_map:#?}"
        );
        assert!(
            types.is_empty(),
            "type registry not empty: types slab is not empty: {types:#?}"
        );
        assert!(
            type_to_rec_group.is_empty() || type_to_rec_group.values().all(|x| x.is_none()),
            "type registry not empty: type-to-rec-group map is not empty: {type_to_rec_group:#?}"
        );
        assert!(
            type_to_supertypes.is_empty() || type_to_supertypes.values().all(|x| x.is_none()),
            "type registry not empty: type-to-supertypes map is not empty: {type_to_supertypes:#?}"
        );
        assert!(
            type_to_trampoline.is_empty() || type_to_trampoline.values().all(|x| x.is_none()),
            "type registry not empty: type-to-trampoline map is not empty: {type_to_trampoline:#?}"
        );
        assert!(
            type_to_gc_layout.is_empty() || type_to_gc_layout.values().all(|x| x.is_none()),
            "type registry not empty: type-to-gc-layout map is not empty: {type_to_gc_layout:#?}"
        );
        assert!(
            drop_stack.is_empty(),
            "type registry not empty: drop stack is not empty: {drop_stack:#?}"
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
        let inner = self.0.read();
        inner.types.get(id).cloned()
    }

    /// Get the GC layout for the given index's type.
    ///
    /// Returns `None` for types that do not have GC layouts (i.e. function
    /// types).
    pub fn layout(&self, index: VMSharedTypeIndex) -> Option<GcLayout> {
        let inner = self.0.read();
        inner.type_to_gc_layout.get(index).and_then(|l| l.clone())
    }

    /// Get the trampoline type for the given function type index.
    ///
    /// Panics for non-function type indices.
    pub fn trampoline_type(&self, index: VMSharedTypeIndex) -> VMSharedTypeIndex {
        let slab_id = shared_type_index_to_slab_id(index);
        let inner = self.0.read();

        let ty = &inner.types[slab_id];
        debug_assert!(
            ty.is_func(),
            "cannot get the trampoline type of a non-function type: {index:?} = {ty:?}"
        );

        let trampoline_ty = match inner.type_to_trampoline.get(index).and_then(|x| x.expand()) {
            Some(ty) => ty,
            None => {
                // The function type is its own trampoline type.
                index
            }
        };
        log::trace!("TypeRegistry::trampoline_type({index:?}) -> {trampoline_ty:?}");
        trampoline_ty
    }

    /// Is type `sub` a subtype of `sup`?
    pub fn is_subtype(&self, sub: VMSharedTypeIndex, sup: VMSharedTypeIndex) -> bool {
        if sub == sup {
            return true;
        }

        // Do the O(1) subtype checking trick:
        //
        // In a type system with single inheritance, the subtyping relationships
        // between all types form a set of trees. The root of each tree is a
        // type that has no supertype; each node's immediate children are the
        // types that directly subtype that node.
        //
        // For example, consider these types:
        //
        //     class Base {}
        //     class A subtypes Base {}
        //     class B subtypes Base {}
        //     class C subtypes A {}
        //     class D subtypes A {}
        //     class E subtypes C {}
        //
        // These types produce the following tree:
        //
        //                Base
        //               /    \
        //              A      B
        //             / \
        //            C   D
        //           /
        //          E
        //
        // Note the following properties:
        //
        // 1. If `sub` is a subtype of `sup` (either directly or transitively)
        //    then `sup` *must* be on the path from `sub` up to the root of
        //    `sub`'s tree.
        //
        // 2. Additionally, `sup` *must* be the `i`th node down from the root in
        //    that path, where `i` is the length of the path from `sup` to its
        //    tree's root.
        //
        // Therefore, if we have the path to the root for each type (we do) then
        // we can simply check if `sup` is at index `supertypes(sup).len()`
        // within `supertypes(sub)`.
        let inner = self.0.read();
        let sub_supertypes = inner.supertypes(sub);
        let sup_supertypes = inner.supertypes(sup);
        sub_supertypes.get(sup_supertypes.len()) == Some(&sup)
    }
}

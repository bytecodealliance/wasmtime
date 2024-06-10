use crate::component::matching::InstanceType;
use crate::component::resources::{HostResourceData, HostResourceIndex, HostResourceTables};
use crate::component::ResourceType;
use crate::prelude::*;
use crate::runtime::vm::component::{
    CallContexts, ComponentInstance, InstanceFlags, ResourceTable, ResourceTables,
};
use crate::runtime::vm::{VMFuncRef, VMMemoryDefinition};
use crate::store::{StoreId, StoreOpaque};
use crate::{FuncType, StoreContextMut};
use alloc::sync::Arc;
use anyhow::{bail, Result};
use core::ptr::NonNull;
use wasmtime_environ::component::{ComponentTypes, StringEncoding, TypeResourceTableIndex};

/// Runtime representation of canonical ABI options in the component model.
///
/// This structure packages up the runtime representation of each option from
/// memories to reallocs to string encodings. Note that this is a "standalone"
/// structure which has raw pointers internally. This allows it to be created
/// out of thin air for a host function import, for example. The `store_id`
/// field, however, is what is used to pair this set of options with a store
/// reference to actually use the pointers.
#[derive(Copy, Clone)]
pub struct Options {
    /// The store from which this options originated from.
    store_id: StoreId,

    /// An optional pointer for the memory that this set of options is referring
    /// to. This option is not required to be specified in the canonical ABI
    /// hence the `Option`.
    ///
    /// Note that this pointer cannot be safely dereferenced unless a store,
    /// verified with `self.store_id`, has the appropriate borrow available.
    memory: Option<NonNull<VMMemoryDefinition>>,

    /// Similar to `memory` but corresponds to the `canonical_abi_realloc`
    /// function.
    ///
    /// Safely using this pointer has the same restrictions as `memory` above.
    realloc: Option<NonNull<VMFuncRef>>,

    /// The encoding used for strings, if found.
    ///
    /// This defaults to utf-8 but can be changed if necessary.
    string_encoding: StringEncoding,
}

// The `Options` structure stores raw pointers but they're never used unless a
// `Store` is available so this should be threadsafe and largely inherit the
// thread-safety story of `Store<T>` itself.
unsafe impl Send for Options {}
unsafe impl Sync for Options {}

impl Options {
    // TODO: prevent a ctor where the memory is memory64

    /// Creates a new set of options with the specified components.
    ///
    /// # Unsafety
    ///
    /// This is unsafety as there is no way to statically verify the validity of
    /// the arguments. For example pointers must be valid pointers, the
    /// `StoreId` must be valid for the pointers, etc.
    pub unsafe fn new(
        store_id: StoreId,
        memory: Option<NonNull<VMMemoryDefinition>>,
        realloc: Option<NonNull<VMFuncRef>>,
        string_encoding: StringEncoding,
    ) -> Options {
        Options {
            store_id,
            memory,
            realloc,
            string_encoding,
        }
    }

    fn realloc<'a, T>(
        &self,
        store: &'a mut StoreContextMut<'_, T>,
        realloc_ty: &FuncType,
        old: usize,
        old_size: usize,
        old_align: u32,
        new_size: usize,
    ) -> Result<(&'a mut [u8], usize)> {
        self.store_id.assert_belongs_to(store.0.id());

        let realloc = self.realloc.unwrap();

        let params = (
            u32::try_from(old).err2anyhow()?,
            u32::try_from(old_size).err2anyhow()?,
            old_align,
            u32::try_from(new_size).err2anyhow()?,
        );

        type ReallocFunc = crate::TypedFunc<(u32, u32, u32, u32), u32>;

        // This call doesn't take any GC refs, and therefore we shouldn't ever
        // need to GC before entering Wasm.
        debug_assert!(!ReallocFunc::need_gc_before_call_raw(store.0, &params));

        // Invoke the wasm malloc function using its raw and statically known
        // signature.
        let result = unsafe { ReallocFunc::call_raw(store, realloc_ty, realloc, params)? };

        if result % old_align != 0 {
            bail!("realloc return: result not aligned");
        }
        let result = usize::try_from(result).err2anyhow()?;

        let memory = self.memory_mut(store.0);

        let result_slice = match memory.get_mut(result..).and_then(|s| s.get_mut(..new_size)) {
            Some(end) => end,
            None => bail!("realloc return: beyond end of memory"),
        };

        Ok((result_slice, result))
    }

    /// Asserts that this function has an associated memory attached to it and
    /// then returns the slice of memory tied to the lifetime of the provided
    /// store.
    pub fn memory<'a>(&self, store: &'a StoreOpaque) -> &'a [u8] {
        self.store_id.assert_belongs_to(store.id());

        // The unsafety here is intended to be encapsulated by the two
        // preceding assertions. Namely we assert that the `store` is the same
        // as the original store of this `Options`, meaning that we safely have
        // either a shared reference or a mutable reference (as below) which
        // means it's safe to view the memory (aka it's not a different store
        // where our original store is on some other thread or something like
        // that).
        //
        // Additionally the memory itself is asserted to be present as memory
        // is an optional configuration in canonical ABI options.
        unsafe {
            let memory = self.memory.unwrap().as_ref();
            core::slice::from_raw_parts(memory.base, memory.current_length())
        }
    }

    /// Same as above, just `_mut`
    pub fn memory_mut<'a>(&self, store: &'a mut StoreOpaque) -> &'a mut [u8] {
        self.store_id.assert_belongs_to(store.id());

        // See comments in `memory` about the unsafety
        unsafe {
            let memory = self.memory.unwrap().as_ref();
            core::slice::from_raw_parts_mut(memory.base, memory.current_length())
        }
    }

    /// Returns the underlying encoding used for strings in this
    /// lifting/lowering.
    pub fn string_encoding(&self) -> StringEncoding {
        self.string_encoding
    }

    /// Returns the id of the store that this `Options` is connected to.
    pub fn store_id(&self) -> StoreId {
        self.store_id
    }
}

/// A helper structure which is a "package" of the context used during lowering
/// values into a component (or storing them into memory).
///
/// This type is used by the `Lower` trait extensively and contains any
/// contextual information necessary related to the context in which the
/// lowering is happening.
#[doc(hidden)]
pub struct LowerContext<'a, T> {
    /// Lowering may involve invoking memory allocation functions so part of the
    /// context here is carrying access to the entire store that wasm is
    /// executing within. This store serves as proof-of-ability to actually
    /// execute wasm safely.
    pub store: StoreContextMut<'a, T>,

    /// Lowering always happens into a function that's been `canon lift`'d or
    /// `canon lower`'d, both of which specify a set of options for the
    /// canonical ABI. For example details like string encoding are contained
    /// here along with which memory pointers are relative to or what the memory
    /// allocation function is.
    pub options: &'a Options,

    /// Lowering happens within the context of a component instance and this
    /// field stores the type information of that component instance. This is
    /// used for type lookups and general type queries during the
    /// lifting/lowering process.
    pub types: &'a ComponentTypes,

    /// A raw unsafe pointer to the component instance that's being lowered
    /// into.
    ///
    /// This pointer is required to be owned by the `store` provided.
    instance: *mut ComponentInstance,
}

#[doc(hidden)]
impl<'a, T> LowerContext<'a, T> {
    /// Creates a new lowering context from the specified parameters.
    ///
    /// # Unsafety
    ///
    /// This function is unsafe as it needs to be guaranteed by the caller that
    /// the `instance` here is is valid within `store` and is a valid component
    /// instance.
    pub unsafe fn new(
        store: StoreContextMut<'a, T>,
        options: &'a Options,
        types: &'a ComponentTypes,
        instance: *mut ComponentInstance,
    ) -> LowerContext<'a, T> {
        LowerContext {
            store,
            options,
            types,
            instance,
        }
    }

    /// Returns a view into memory as a mutable slice of bytes.
    ///
    /// # Panics
    ///
    /// This will panic if memory has not been configured for this lowering
    /// (e.g. it wasn't present during the specification of canonical options).
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        self.options.memory_mut(self.store.0)
    }

    /// Invokes the memory allocation function (which is style after `realloc`)
    /// with the specified parameters.
    ///
    /// # Panics
    ///
    /// This will panic if realloc hasn't been configured for this lowering via
    /// its canonical options.
    pub fn realloc(
        &mut self,
        old: usize,
        old_size: usize,
        old_align: u32,
        new_size: usize,
    ) -> Result<usize> {
        let realloc_func_ty = Arc::clone(unsafe { (*self.instance).realloc_func_ty() });
        let realloc_func_ty = realloc_func_ty.downcast_ref::<FuncType>().unwrap();
        self.options
            .realloc(
                &mut self.store,
                &realloc_func_ty,
                old,
                old_size,
                old_align,
                new_size,
            )
            .map(|(_, ptr)| ptr)
    }

    /// Returns a fixed mutable slice of memory `N` bytes large starting at
    /// offset `N`, panicking on out-of-bounds.
    ///
    /// It should be previously verified that `offset` is in-bounds via
    /// bounds-checks.
    ///
    /// # Panics
    ///
    /// This will panic if memory has not been configured for this lowering
    /// (e.g. it wasn't present during the specification of canonical options).
    pub fn get<const N: usize>(&mut self, offset: usize) -> &mut [u8; N] {
        // FIXME: this bounds check shouldn't actually be necessary, all
        // callers of `ComponentType::store` have already performed a bounds
        // check so we're guaranteed that `offset..offset+N` is in-bounds. That
        // being said we at least should do bounds checks in debug mode and
        // it's not clear to me how to easily structure this so that it's
        // "statically obvious" the bounds check isn't necessary.
        //
        // For now I figure we can leave in this bounds check and if it becomes
        // an issue we can optimize further later, probably with judicious use
        // of `unsafe`.
        (&mut self.as_slice_mut()[offset..][..N])
            .try_into()
            .unwrap()
    }

    /// Lowers an `own` resource into the guest, converting the `rep` specified
    /// into a guest-local index.
    ///
    /// The `ty` provided is which table to put this into.
    pub fn guest_resource_lower_own(
        &mut self,
        ty: TypeResourceTableIndex,
        rep: u32,
    ) -> Result<u32> {
        self.resource_tables().guest_resource_lower_own(rep, ty)
    }

    /// Lowers a `borrow` resource into the guest, converting the `rep` to a
    /// guest-local index in the `ty` table specified.
    pub fn guest_resource_lower_borrow(
        &mut self,
        ty: TypeResourceTableIndex,
        rep: u32,
    ) -> Result<u32> {
        // Implement `lower_borrow`'s special case here where if a borrow is
        // inserted into a table owned by the instance which implemented the
        // original resource then no borrow tracking is employed and instead the
        // `rep` is returned "raw".
        //
        // This check is performed by comparing the owning instance of `ty`
        // against the owning instance of the resource that `ty` is working
        // with.
        //
        // Note that the unsafety here should be valid given the contract of
        // `LowerContext::new`.
        if unsafe { (*self.instance).resource_owned_by_own_instance(ty) } {
            return Ok(rep);
        }
        self.resource_tables().guest_resource_lower_borrow(rep, ty)
    }

    /// Lifts a host-owned `own` resource at the `idx` specified into the
    /// representation of that resource.
    pub fn host_resource_lift_own(&mut self, idx: HostResourceIndex) -> Result<u32> {
        self.resource_tables().host_resource_lift_own(idx)
    }

    /// Lifts a host-owned `borrow` resource at the `idx` specified into the
    /// representation of that resource.
    pub fn host_resource_lift_borrow(&mut self, idx: HostResourceIndex) -> Result<u32> {
        self.resource_tables().host_resource_lift_borrow(idx)
    }

    /// Lowers a resource into the host-owned table, returning the index it was
    /// inserted at.
    ///
    /// Note that this is a special case for `Resource<T>`. Most of the time a
    /// host value shouldn't be lowered with a lowering context.
    pub fn host_resource_lower_own(
        &mut self,
        rep: u32,
        dtor: Option<NonNull<VMFuncRef>>,
        flags: Option<InstanceFlags>,
    ) -> Result<HostResourceIndex> {
        self.resource_tables()
            .host_resource_lower_own(rep, dtor, flags)
    }

    /// Returns the underlying resource type for the `ty` table specified.
    pub fn resource_type(&self, ty: TypeResourceTableIndex) -> ResourceType {
        self.instance_type().resource_type(ty)
    }

    /// Returns the instance type information corresponding to the instance that
    /// this context is lowering into.
    pub fn instance_type(&self) -> InstanceType<'_> {
        // Note that the unsafety here should be valid given the contract of
        // `LowerContext::new`.
        InstanceType::new(unsafe { &*self.instance })
    }

    fn resource_tables(&mut self) -> HostResourceTables<'_> {
        let (calls, host_table, host_resource_data) = self.store.0.component_resource_state();
        HostResourceTables::from_parts(
            ResourceTables {
                host_table: Some(host_table),
                calls,
                // Note that the unsafety here should be valid given the contract of
                // `LowerContext::new`.
                tables: Some(unsafe { (*self.instance).component_resource_tables() }),
            },
            host_resource_data,
        )
    }

    /// See [`HostResourceTables::enter_call`].
    #[inline]
    pub fn enter_call(&mut self) {
        self.resource_tables().enter_call()
    }

    /// See [`HostResourceTables::exit_call`].
    #[inline]
    pub fn exit_call(&mut self) -> Result<()> {
        self.resource_tables().exit_call()
    }
}

/// Contextual information used when lifting a type from a component into the
/// host.
///
/// This structure is the analogue of `LowerContext` except used during lifting
/// operations (or loading from memory).
#[doc(hidden)]
pub struct LiftContext<'a> {
    /// Like lowering, lifting always has options configured.
    pub options: &'a Options,

    /// Instance type information, like with lowering.
    pub types: &'a Arc<ComponentTypes>,

    memory: Option<&'a [u8]>,

    instance: *mut ComponentInstance,

    host_table: &'a mut ResourceTable,
    host_resource_data: &'a mut HostResourceData,

    calls: &'a mut CallContexts,
}

#[doc(hidden)]
impl<'a> LiftContext<'a> {
    /// Creates a new lifting context given the provided context.
    ///
    /// # Unsafety
    ///
    /// This is unsafe for the same reasons as `LowerContext::new` where the
    /// validity of `instance` is required to be upheld by the caller.
    #[inline]
    pub unsafe fn new(
        store: &'a mut StoreOpaque,
        options: &'a Options,
        types: &'a Arc<ComponentTypes>,
        instance: *mut ComponentInstance,
    ) -> LiftContext<'a> {
        // From `&mut StoreOpaque` provided the goal here is to project out
        // three different disjoint fields owned by the store: memory,
        // `CallContexts`, and `ResourceTable`. There's no native API for that
        // so it's hacked around a bit. This unsafe pointer cast could be fixed
        // with more methods in more places, but it doesn't seem worth doing it
        // at this time.
        let (calls, host_table, host_resource_data) =
            (&mut *(store as *mut StoreOpaque)).component_resource_state();
        let memory = options.memory.map(|_| options.memory(store));

        LiftContext {
            memory,
            options,
            types,
            instance,
            calls,
            host_table,
            host_resource_data,
        }
    }

    /// Returns the entire contents of linear memory for this set of lifting
    /// options.
    ///
    /// # Panics
    ///
    /// This will panic if memory has not been configured for this lifting
    /// operation.
    pub fn memory(&self) -> &'a [u8] {
        self.memory.unwrap()
    }

    /// Returns an identifier for the store from which this `LiftContext` was
    /// created.
    pub fn store_id(&self) -> StoreId {
        self.options.store_id
    }

    /// Returns the component instance raw pointer that is being lifted from.
    pub fn instance_ptr(&self) -> *mut ComponentInstance {
        self.instance
    }

    /// Lifts an `own` resource from the guest at the `idx` specified into its
    /// representation.
    ///
    /// Additionally returns a destructor/instance flags to go along with the
    /// representation so the host knows how to destroy this resource.
    pub fn guest_resource_lift_own(
        &mut self,
        ty: TypeResourceTableIndex,
        idx: u32,
    ) -> Result<(u32, Option<NonNull<VMFuncRef>>, Option<InstanceFlags>)> {
        let idx = self.resource_tables().guest_resource_lift_own(idx, ty)?;
        // Note that the unsafety here should be valid given the contract of
        // `LiftContext::new`.
        let (dtor, flags) = unsafe { (*self.instance).dtor_and_flags(ty) };
        Ok((idx, dtor, flags))
    }

    /// Lifts a `borrow` resource from the guest at the `idx` specified.
    pub fn guest_resource_lift_borrow(
        &mut self,
        ty: TypeResourceTableIndex,
        idx: u32,
    ) -> Result<u32> {
        self.resource_tables().guest_resource_lift_borrow(idx, ty)
    }

    /// Lowers a resource into the host-owned table, returning the index it was
    /// inserted at.
    pub fn host_resource_lower_own(
        &mut self,
        rep: u32,
        dtor: Option<NonNull<VMFuncRef>>,
        flags: Option<InstanceFlags>,
    ) -> Result<HostResourceIndex> {
        self.resource_tables()
            .host_resource_lower_own(rep, dtor, flags)
    }

    /// Lowers a resource into the host-owned table, returning the index it was
    /// inserted at.
    pub fn host_resource_lower_borrow(&mut self, rep: u32) -> Result<HostResourceIndex> {
        self.resource_tables().host_resource_lower_borrow(rep)
    }

    /// Returns the underlying type of the resource table specified by `ty`.
    pub fn resource_type(&self, ty: TypeResourceTableIndex) -> ResourceType {
        self.instance_type().resource_type(ty)
    }

    /// Returns instance type information for the component instance that is
    /// being lifted from.
    pub fn instance_type(&self) -> InstanceType<'_> {
        // Note that the unsafety here should be valid given the contract of
        // `LiftContext::new`.
        InstanceType::new(unsafe { &*self.instance })
    }

    fn resource_tables(&mut self) -> HostResourceTables<'_> {
        HostResourceTables::from_parts(
            ResourceTables {
                host_table: Some(self.host_table),
                calls: self.calls,
                // Note that the unsafety here should be valid given the contract of
                // `LiftContext::new`.
                tables: Some(unsafe { (*self.instance).component_resource_tables() }),
            },
            self.host_resource_data,
        )
    }

    /// See [`HostResourceTables::enter_call`].
    #[inline]
    pub fn enter_call(&mut self) {
        self.resource_tables().enter_call()
    }

    /// See [`HostResourceTables::exit_call`].
    #[inline]
    pub fn exit_call(&mut self) -> Result<()> {
        self.resource_tables().exit_call()
    }
}

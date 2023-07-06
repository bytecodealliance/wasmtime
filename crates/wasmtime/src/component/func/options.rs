use crate::component::matching::InstanceType;
use crate::component::ResourceType;
use crate::store::{StoreId, StoreOpaque};
use crate::StoreContextMut;
use anyhow::{bail, Result};
use std::ptr::NonNull;
use std::sync::Arc;
use wasmtime_environ::component::{ComponentTypes, StringEncoding, TypeResourceTableIndex};
use wasmtime_runtime::component::ComponentInstance;
use wasmtime_runtime::{VMFuncRef, VMMemoryDefinition};

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
        old: usize,
        old_size: usize,
        old_align: u32,
        new_size: usize,
    ) -> Result<(&'a mut [u8], usize)> {
        self.store_id.assert_belongs_to(store.0.id());

        let realloc = self.realloc.unwrap();

        // Invoke the wasm malloc function using its raw and statically known
        // signature.
        let result = unsafe {
            crate::TypedFunc::<(u32, u32, u32, u32), u32>::call_raw(
                store,
                realloc,
                (
                    u32::try_from(old)?,
                    u32::try_from(old_size)?,
                    old_align,
                    u32::try_from(new_size)?,
                ),
            )?
        };

        if result % old_align != 0 {
            bail!("realloc return: result not aligned");
        }
        let result = usize::try_from(result)?;

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
            std::slice::from_raw_parts(memory.base, memory.current_length())
        }
    }

    /// Same as above, just `_mut`
    pub fn memory_mut<'a>(&self, store: &'a mut StoreOpaque) -> &'a mut [u8] {
        self.store_id.assert_belongs_to(store.id());

        // See comments in `memory` about the unsafety
        unsafe {
            let memory = self.memory.unwrap().as_ref();
            std::slice::from_raw_parts_mut(memory.base, memory.current_length())
        }
    }

    /// Returns the underlying encoding used for strings in this
    /// lifting/lowering.
    pub fn string_encoding(&self) -> StringEncoding {
        self.string_encoding
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

    /// TODO
    instance: *mut ComponentInstance,
}

#[doc(hidden)]
impl<'a, T> LowerContext<'a, T> {
    /// TODO
    ///
    /// # Unsafety
    ///
    /// TODO: ptr must be valid
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
        self.options
            .realloc(&mut self.store, old, old_size, old_align, new_size)
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

    /// TODO
    pub fn resource_lower_own(&mut self, ty: TypeResourceTableIndex, rep: u32) -> u32 {
        unsafe { (*self.instance).resource_lower_own(ty, rep) }
    }

    pub fn resource_type(&self, ty: TypeResourceTableIndex) -> ResourceType {
        InstanceType::new(self.store.0, unsafe { &*self.instance }).resource_type(ty)
    }
}

/// Contextual information used when lifting a type from a component into the
/// host.
///
/// This structure is the analogue of `LowerContext` except used during lifting
/// operations (or loading from memory).
#[doc(hidden)]
pub struct LiftContext<'a> {
    /// Unlike `LowerContext` lifting doesn't ever need to execute wasm, so a
    /// full store isn't required here and only a shared reference is required.
    pub store: &'a StoreOpaque,

    /// Like lowering, lifting always has options configured.
    pub options: &'a Options,

    /// Instance type information, like with lowering.
    pub types: &'a Arc<ComponentTypes>,

    instance: *mut ComponentInstance,
}

#[doc(hidden)]
impl<'a> LiftContext<'a> {
    /// TODO
    ///
    /// # Unsafety
    ///
    /// TODO: ptr must be valid
    pub unsafe fn new(
        store: &'a StoreOpaque,
        options: &'a Options,
        types: &'a Arc<ComponentTypes>,
        instance: *mut ComponentInstance,
    ) -> LiftContext<'a> {
        LiftContext {
            store,
            options,
            types,
            instance,
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
        self.options.memory(self.store)
    }

    /// TODO
    pub fn instance_ptr(&self) -> *mut ComponentInstance {
        self.instance
    }

    /// TODO
    pub fn resource_lift_own(
        &self,
        ty: TypeResourceTableIndex,
        idx: u32,
    ) -> Result<(u32, Option<NonNull<VMFuncRef>>)> {
        // TODO: document unsafe
        unsafe { (*self.instance).resource_lift_own(ty, idx) }
    }

    pub fn resource_type(&self, ty: TypeResourceTableIndex) -> ResourceType {
        InstanceType::new(self.store, unsafe { &*self.instance }).resource_type(ty)
    }
}

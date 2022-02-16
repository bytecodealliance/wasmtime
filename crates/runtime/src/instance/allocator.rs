use crate::imports::Imports;
use crate::instance::{Instance, InstanceHandle, RuntimeMemoryCreator};
use crate::memory::{DefaultMemoryCreator, Memory};
use crate::table::Table;
use crate::traphandlers::Trap;
use crate::ModuleRuntimeInfo;
use crate::Store;
use anyhow::Result;
use std::alloc;
use std::any::Any;
use std::convert::TryFrom;
use std::ptr;
use std::slice;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_environ::{
    DefinedMemoryIndex, DefinedTableIndex, HostPtr, InitMemory, MemoryInitialization,
    MemoryInitializer, Module, PrimaryMap, TableInitialization, TableInitializer, TrapCode,
    VMOffsets, WasmType, WASM_PAGE_SIZE,
};

#[cfg(feature = "pooling-allocator")]
mod pooling;

#[cfg(feature = "pooling-allocator")]
pub use self::pooling::{
    InstanceLimits, ModuleLimits, PoolingAllocationStrategy, PoolingInstanceAllocator,
};

/// Represents a request for a new runtime instance.
pub struct InstanceAllocationRequest<'a> {
    /// The info related to the compiled version of this module,
    /// needed for instantiation: function metadata, JIT code
    /// addresses, precomputed images for lazy memory and table
    /// initialization, and the like. This Arc is cloned and held for
    /// the lifetime of the instance.
    pub runtime_info: &'a Arc<dyn ModuleRuntimeInfo>,

    /// The imports to use for the instantiation.
    pub imports: Imports<'a>,

    /// The host state to associate with the instance.
    pub host_state: Box<dyn Any + Send + Sync>,

    /// A pointer to the "store" for this instance to be allocated. The store
    /// correlates with the `Store` in wasmtime itself, and lots of contextual
    /// information about the execution of wasm can be learned through the store.
    ///
    /// Note that this is a raw pointer and has a static lifetime, both of which
    /// are a bit of a lie. This is done purely so a store can learn about
    /// itself when it gets called as a host function, and additionally so this
    /// runtime can access internals as necessary (such as the
    /// VMExternRefActivationsTable or the resource limiter methods).
    ///
    /// Note that this ends up being a self-pointer to the instance when stored.
    /// The reason is that the instance itself is then stored within the store.
    /// We use a number of `PhantomPinned` declarations to indicate this to the
    /// compiler. More info on this in `wasmtime/src/store.rs`
    pub store: StorePtr,
}

/// A pointer to a Store. This Option<*mut dyn Store> is wrapped in a struct
/// so that the function to create a &mut dyn Store is a method on a member of
/// InstanceAllocationRequest, rather than on a &mut InstanceAllocationRequest
/// itself, because several use-sites require a split mut borrow on the
/// InstanceAllocationRequest.
pub struct StorePtr(Option<*mut dyn Store>);
impl StorePtr {
    /// A pointer to no Store.
    pub fn empty() -> Self {
        Self(None)
    }
    /// A pointer to a Store.
    pub fn new(ptr: *mut dyn Store) -> Self {
        Self(Some(ptr))
    }
    /// The raw contents of this struct
    pub fn as_raw(&self) -> Option<*mut dyn Store> {
        self.0.clone()
    }
    /// Use the StorePtr as a mut ref to the Store.
    /// Safety: must not be used outside the original lifetime of the borrow.
    pub(crate) unsafe fn get(&mut self) -> Option<&mut dyn Store> {
        match self.0 {
            Some(ptr) => Some(&mut *ptr),
            None => None,
        }
    }
}

/// An link error while instantiating a module.
#[derive(Error, Debug)]
#[error("Link error: {0}")]
pub struct LinkError(pub String);

/// An error while instantiating a module.
#[derive(Error, Debug)]
pub enum InstantiationError {
    /// Insufficient resources available for execution.
    #[error("Insufficient resources: {0}")]
    Resource(anyhow::Error),

    /// A wasm link error occured.
    #[error("Failed to link module")]
    Link(#[from] LinkError),

    /// A trap ocurred during instantiation, after linking.
    #[error("Trap occurred during instantiation")]
    Trap(Trap),

    /// A limit on how many instances are supported has been reached.
    #[error("Limit of {0} concurrent instances has been reached")]
    Limit(u32),
}

/// An error while creating a fiber stack.
#[cfg(feature = "async")]
#[derive(Error, Debug)]
pub enum FiberStackError {
    /// Insufficient resources available for the request.
    #[error("Insufficient resources: {0}")]
    Resource(anyhow::Error),
    /// An error for when the allocator doesn't support fiber stacks.
    #[error("fiber stacks are not supported by the allocator")]
    NotSupported,
    /// A limit on how many fibers are supported has been reached.
    #[error("Limit of {0} concurrent fibers has been reached")]
    Limit(u32),
}

/// Represents a runtime instance allocator.
///
/// # Safety
///
/// This trait is unsafe as it requires knowledge of Wasmtime's runtime internals to implement correctly.
pub unsafe trait InstanceAllocator: Send + Sync {
    /// Validates that a module is supported by the allocator.
    fn validate(&self, module: &Module) -> Result<()> {
        drop(module);
        Ok(())
    }

    /// Adjusts the tunables prior to creation of any JIT compiler.
    ///
    /// This method allows the instance allocator control over tunables passed to a `wasmtime_jit::Compiler`.
    fn adjust_tunables(&self, tunables: &mut wasmtime_environ::Tunables) {
        drop(tunables);
    }

    /// Allocates an instance for the given allocation request.
    ///
    /// # Safety
    ///
    /// This method is not inherently unsafe, but care must be made to ensure
    /// pointers passed in the allocation request outlive the returned instance.
    unsafe fn allocate(
        &self,
        req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError>;

    /// Finishes the instantiation process started by an instance allocator.
    ///
    /// # Safety
    ///
    /// This method is only safe to call immediately after an instance has been allocated.
    unsafe fn initialize(
        &self,
        handle: &mut InstanceHandle,
        module: &Module,
        is_bulk_memory: bool,
    ) -> Result<(), InstantiationError>;

    /// Deallocates a previously allocated instance.
    ///
    /// # Safety
    ///
    /// This function is unsafe because there are no guarantees that the given handle
    /// is the only owner of the underlying instance to deallocate.
    ///
    /// Use extreme care when deallocating an instance so that there are no dangling instance pointers.
    unsafe fn deallocate(&self, handle: &InstanceHandle);

    /// Allocates a fiber stack for calling async functions on.
    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack, FiberStackError>;

    /// Deallocates a fiber stack that was previously allocated with `allocate_fiber_stack`.
    ///
    /// # Safety
    ///
    /// The provided stack is required to have been allocated with `allocate_fiber_stack`.
    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, stack: &wasmtime_fiber::FiberStack);
}

fn get_table_init_start(
    init: &TableInitializer,
    instance: &Instance,
) -> Result<u32, InstantiationError> {
    match init.base {
        Some(base) => {
            let val = unsafe {
                if let Some(def_index) = instance.module().defined_global_index(base) {
                    *instance.global(def_index).as_u32()
                } else {
                    *(*instance.imported_global(base).from).as_u32()
                }
            };

            init.offset.checked_add(val).ok_or_else(|| {
                InstantiationError::Link(LinkError(
                    "element segment global base overflows".to_owned(),
                ))
            })
        }
        None => Ok(init.offset),
    }
}

fn check_table_init_bounds(
    instance: &mut Instance,
    module: &Module,
) -> Result<(), InstantiationError> {
    match &module.table_initialization {
        TableInitialization::FuncTable { segments, .. }
        | TableInitialization::Segments { segments } => {
            for segment in segments {
                let table = unsafe { &*instance.get_table(segment.table_index) };
                let start = get_table_init_start(segment, instance)?;
                let start = usize::try_from(start).unwrap();
                let end = start.checked_add(segment.elements.len());

                match end {
                    Some(end) if end <= table.size() as usize => {
                        // Initializer is in bounds
                    }
                    _ => {
                        return Err(InstantiationError::Link(LinkError(
                            "table out of bounds: elements segment does not fit".to_owned(),
                        )))
                    }
                }
            }
        }
    }

    Ok(())
}

fn initialize_tables(instance: &mut Instance, module: &Module) -> Result<(), InstantiationError> {
    // Note: if the module's table initializer state is in
    // FuncTable mode, we will lazily initialize tables based on
    // any statically-precomputed image of FuncIndexes, but there
    // may still be "leftover segments" that could not be
    // incorporated. So we have a unified handler here that
    // iterates over all segments (Segments mode) or leftover
    // segments (FuncTable mode) to initialize.
    match &module.table_initialization {
        TableInitialization::FuncTable { segments, .. }
        | TableInitialization::Segments { segments } => {
            for segment in segments {
                instance
                    .table_init_segment(
                        segment.table_index,
                        &segment.elements,
                        get_table_init_start(segment, instance)?,
                        0,
                        segment.elements.len() as u32,
                    )
                    .map_err(InstantiationError::Trap)?;
            }
        }
    }

    Ok(())
}

fn get_memory_init_start(
    init: &MemoryInitializer,
    instance: &Instance,
) -> Result<u64, InstantiationError> {
    match init.base {
        Some(base) => {
            let mem64 = instance.module().memory_plans[init.memory_index]
                .memory
                .memory64;
            let val = unsafe {
                let global = if let Some(def_index) = instance.module().defined_global_index(base) {
                    instance.global(def_index)
                } else {
                    &*instance.imported_global(base).from
                };
                if mem64 {
                    *global.as_u64()
                } else {
                    u64::from(*global.as_u32())
                }
            };

            init.offset.checked_add(val).ok_or_else(|| {
                InstantiationError::Link(LinkError("data segment global base overflows".to_owned()))
            })
        }
        None => Ok(init.offset),
    }
}

fn check_memory_init_bounds(
    instance: &Instance,
    initializers: &[MemoryInitializer],
) -> Result<(), InstantiationError> {
    for init in initializers {
        let memory = instance.get_memory(init.memory_index);
        let start = get_memory_init_start(init, instance)?;
        let end = usize::try_from(start)
            .ok()
            .and_then(|start| start.checked_add(init.data.len()));

        match end {
            Some(end) if end <= memory.current_length => {
                // Initializer is in bounds
            }
            _ => {
                return Err(InstantiationError::Link(LinkError(
                    "memory out of bounds: data segment does not fit".into(),
                )))
            }
        }
    }

    Ok(())
}

fn initialize_memories(instance: &mut Instance, module: &Module) -> Result<(), InstantiationError> {
    let memory_size_in_pages =
        &|memory| (instance.get_memory(memory).current_length as u64) / u64::from(WASM_PAGE_SIZE);

    // Loads the `global` value and returns it as a `u64`, but sign-extends
    // 32-bit globals which can be used as the base for 32-bit memories.
    let get_global_as_u64 = &|global| unsafe {
        let def = if let Some(def_index) = instance.module().defined_global_index(global) {
            instance.global(def_index)
        } else {
            &*instance.imported_global(global).from
        };
        if module.globals[global].wasm_ty == WasmType::I64 {
            *def.as_u64()
        } else {
            u64::from(*def.as_u32())
        }
    };

    // Delegates to the `init_memory` method which is sort of a duplicate of
    // `instance.memory_init_segment` but is used at compile-time in other
    // contexts so is shared here to have only one method of memory
    // initialization.
    //
    // This call to `init_memory` notably implements all the bells and whistles
    // so errors only happen if an out-of-bounds segment is found, in which case
    // a trap is returned.
    let ok = module.memory_initialization.init_memory(
        InitMemory::Runtime {
            memory_size_in_pages,
            get_global_as_u64,
        },
        &mut |memory_index, init| {
            // If this initializer applies to a defined memory but that memory
            // doesn't need initialization, due to something like uffd or memfd
            // pre-initializing it via mmap magic, then this initializer can be
            // skipped entirely.
            if let Some(memory_index) = module.defined_memory_index(memory_index) {
                if !instance.memories[memory_index].needs_init() {
                    return true;
                }
            }
            let memory = instance.get_memory(memory_index);
            let dst_slice =
                unsafe { slice::from_raw_parts_mut(memory.base, memory.current_length) };
            let dst = &mut dst_slice[usize::try_from(init.offset).unwrap()..][..init.data.len()];
            dst.copy_from_slice(instance.wasm_data(init.data.clone()));
            true
        },
    );
    if !ok {
        return Err(InstantiationError::Trap(Trap::wasm(
            TrapCode::HeapOutOfBounds,
        )));
    }

    Ok(())
}

fn check_init_bounds(instance: &mut Instance, module: &Module) -> Result<(), InstantiationError> {
    check_table_init_bounds(instance, module)?;

    match &instance.module().memory_initialization {
        MemoryInitialization::Segmented(initializers) => {
            check_memory_init_bounds(instance, initializers)?;
        }
        // Statically validated already to have everything in-bounds.
        MemoryInitialization::Paged { .. } | MemoryInitialization::Static { .. } => {}
    }

    Ok(())
}

fn initialize_instance(
    instance: &mut Instance,
    module: &Module,
    is_bulk_memory: bool,
) -> Result<(), InstantiationError> {
    // If bulk memory is not enabled, bounds check the data and element segments before
    // making any changes. With bulk memory enabled, initializers are processed
    // in-order and side effects are observed up to the point of an out-of-bounds
    // initializer, so the early checking is not desired.
    if !is_bulk_memory {
        check_init_bounds(instance, module)?;
    }

    // Initialize the tables
    initialize_tables(instance, module)?;

    // Initialize the memories
    initialize_memories(instance, &module)?;

    Ok(())
}

/// Represents the on-demand instance allocator.
#[derive(Clone)]
pub struct OnDemandInstanceAllocator {
    mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>,
    #[cfg(feature = "async")]
    stack_size: usize,
}

impl OnDemandInstanceAllocator {
    /// Creates a new on-demand instance allocator.
    pub fn new(mem_creator: Option<Arc<dyn RuntimeMemoryCreator>>, stack_size: usize) -> Self {
        drop(stack_size); // suppress unused warnings w/o async feature
        Self {
            mem_creator,
            #[cfg(feature = "async")]
            stack_size,
        }
    }

    fn create_tables(
        store: &mut StorePtr,
        runtime_info: &Arc<dyn ModuleRuntimeInfo>,
    ) -> Result<PrimaryMap<DefinedTableIndex, Table>, InstantiationError> {
        let module = runtime_info.module();
        let num_imports = module.num_imported_tables;
        let mut tables: PrimaryMap<DefinedTableIndex, _> =
            PrimaryMap::with_capacity(module.table_plans.len() - num_imports);
        for (_, table) in module.table_plans.iter().skip(num_imports) {
            tables.push(
                Table::new_dynamic(table, unsafe {
                    store
                        .get()
                        .expect("if module has table plans, store is not empty")
                })
                .map_err(InstantiationError::Resource)?,
            );
        }
        Ok(tables)
    }

    fn create_memories(
        &self,
        store: &mut StorePtr,
        runtime_info: &Arc<dyn ModuleRuntimeInfo>,
    ) -> Result<PrimaryMap<DefinedMemoryIndex, Memory>, InstantiationError> {
        let module = runtime_info.module();
        let creator = self
            .mem_creator
            .as_deref()
            .unwrap_or_else(|| &DefaultMemoryCreator);
        let num_imports = module.num_imported_memories;
        let mut memories: PrimaryMap<DefinedMemoryIndex, _> =
            PrimaryMap::with_capacity(module.memory_plans.len() - num_imports);
        for (memory_idx, plan) in module.memory_plans.iter().skip(num_imports) {
            // Create a MemFdSlot if there is an image for this memory.
            let defined_memory_idx = module
                .defined_memory_index(memory_idx)
                .expect("Skipped imports, should never be None");
            let memfd_image = runtime_info
                .memfd_image(defined_memory_idx)
                .map_err(|err| InstantiationError::Resource(err.into()))?;

            memories.push(
                Memory::new_dynamic(
                    plan,
                    creator,
                    unsafe {
                        store
                            .get()
                            .expect("if module has memory plans, store is not empty")
                    },
                    memfd_image,
                )
                .map_err(InstantiationError::Resource)?,
            );
        }
        Ok(memories)
    }
}

impl Default for OnDemandInstanceAllocator {
    fn default() -> Self {
        Self {
            mem_creator: None,
            #[cfg(feature = "async")]
            stack_size: 0,
        }
    }
}

unsafe impl InstanceAllocator for OnDemandInstanceAllocator {
    unsafe fn allocate(
        &self,
        mut req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError> {
        let memories = self.create_memories(&mut req.store, &req.runtime_info)?;
        let tables = Self::create_tables(&mut req.store, &req.runtime_info)?;
        let module = req.runtime_info.module();
        let offsets = VMOffsets::new(HostPtr, module);
        let layout = Instance::alloc_layout(&offsets);
        let instance_ptr = alloc::alloc(layout) as *mut Instance;

        Instance::new_at(instance_ptr, layout.size(), offsets, req, memories, tables);

        Ok(InstanceHandle {
            instance: instance_ptr,
        })
    }

    unsafe fn initialize(
        &self,
        handle: &mut InstanceHandle,
        module: &Module,
        is_bulk_memory: bool,
    ) -> Result<(), InstantiationError> {
        initialize_instance(handle.instance_mut(), module, is_bulk_memory)
    }

    unsafe fn deallocate(&self, handle: &InstanceHandle) {
        let layout = Instance::alloc_layout(&handle.instance().offsets);
        ptr::drop_in_place(handle.instance);
        alloc::dealloc(handle.instance.cast(), layout);
    }

    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack, FiberStackError> {
        if self.stack_size == 0 {
            return Err(FiberStackError::NotSupported);
        }

        wasmtime_fiber::FiberStack::new(self.stack_size)
            .map_err(|e| FiberStackError::Resource(e.into()))
    }

    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, _stack: &wasmtime_fiber::FiberStack) {
        // The on-demand allocator has no further bookkeeping for fiber stacks
    }
}

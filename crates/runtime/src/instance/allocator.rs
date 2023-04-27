use crate::imports::Imports;
use crate::instance::{Instance, InstanceHandle, RuntimeMemoryCreator};
use crate::memory::{DefaultMemoryCreator, Memory};
use crate::table::Table;
use crate::{CompiledModuleId, ModuleRuntimeInfo, Store};
use anyhow::{anyhow, bail, Result};
use std::alloc;
use std::any::Any;
use std::convert::TryFrom;
use std::ptr;
use std::sync::Arc;
use wasmtime_environ::{
    DefinedMemoryIndex, DefinedTableIndex, HostPtr, InitMemory, MemoryInitialization,
    MemoryInitializer, Module, PrimaryMap, TableInitialization, TableInitializer, Trap, VMOffsets,
    WasmType, WASM_PAGE_SIZE,
};

#[cfg(feature = "pooling-allocator")]
mod pooling;

#[cfg(feature = "pooling-allocator")]
pub use self::pooling::{InstanceLimits, PoolingInstanceAllocator, PoolingInstanceAllocatorConfig};

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

/// Represents a runtime instance allocator.
///
/// # Safety
///
/// This trait is unsafe as it requires knowledge of Wasmtime's runtime internals to implement correctly.
pub unsafe trait InstanceAllocator {
    /// Validates that a module is supported by the allocator.
    fn validate(&self, module: &Module, offsets: &VMOffsets<HostPtr>) -> Result<()> {
        drop((module, offsets));
        Ok(())
    }

    /// Allocates a fresh `InstanceHandle` for the `req` given.
    ///
    /// This will allocate memories and tables internally from this allocator
    /// and weave that altogether into a final and complete `InstanceHandle`
    /// ready to be registered with a store.
    ///
    /// Note that the returned instance must still have `.initialize(..)` called
    /// on it to complete the instantiation process.
    fn allocate(&self, mut req: InstanceAllocationRequest) -> Result<InstanceHandle> {
        let index = self.allocate_index(&req)?;
        let module = req.runtime_info.module();
        let mut memories =
            PrimaryMap::with_capacity(module.memory_plans.len() - module.num_imported_memories);
        let mut tables =
            PrimaryMap::with_capacity(module.table_plans.len() - module.num_imported_tables);

        let result = self
            .allocate_memories(index, &mut req, &mut memories)
            .and_then(|()| self.allocate_tables(index, &mut req, &mut tables));
        if let Err(e) = result {
            self.deallocate_memories(index, &mut memories);
            self.deallocate_tables(index, &mut tables);
            self.deallocate_index(index);
            return Err(e);
        }

        unsafe { Ok(Instance::new(req, index, memories, tables)) }
    }

    /// Deallocates the provided instance.
    ///
    /// This will null-out the pointer within `handle` and otherwise reclaim
    /// resources such as tables, memories, and the instance memory itself.
    fn deallocate(&self, handle: &mut InstanceHandle) {
        let index = handle.instance().index;
        self.deallocate_memories(index, &mut handle.instance_mut().memories);
        self.deallocate_tables(index, &mut handle.instance_mut().tables);
        unsafe {
            let layout = Instance::alloc_layout(handle.instance().offsets());
            ptr::drop_in_place(handle.instance);
            alloc::dealloc(handle.instance.cast(), layout);
            handle.instance = std::ptr::null_mut();
        }
        self.deallocate_index(index);
    }

    /// Optionally allocates an allocator-defined index for the `req` provided.
    ///
    /// The return value here, if successful, is passed to the various methods
    /// below for memory/table allocation/deallocation.
    fn allocate_index(&self, req: &InstanceAllocationRequest) -> Result<usize>;

    /// Deallocates indices allocated by `allocate_index`.
    fn deallocate_index(&self, index: usize);

    /// Attempts to allocate all defined linear memories for a module.
    ///
    /// Pushes all memories for `req` onto the `mems` storage provided which is
    /// already appropriately allocated to contain all memories.
    ///
    /// Note that this is allowed to fail. Failure can additionally happen after
    /// some memories have already been successfully allocated. All memories
    /// pushed onto `mem` are guaranteed to one day make their way to
    /// `deallocate_memories`.
    fn allocate_memories(
        &self,
        index: usize,
        req: &mut InstanceAllocationRequest,
        mems: &mut PrimaryMap<DefinedMemoryIndex, Memory>,
    ) -> Result<()>;

    /// Deallocates all memories provided, optionally reclaiming resources for
    /// the pooling allocator for example.
    fn deallocate_memories(&self, index: usize, mems: &mut PrimaryMap<DefinedMemoryIndex, Memory>);

    /// Same as `allocate_memories`, but for tables.
    fn allocate_tables(
        &self,
        index: usize,
        req: &mut InstanceAllocationRequest,
        tables: &mut PrimaryMap<DefinedTableIndex, Table>,
    ) -> Result<()>;

    /// Same as `deallocate_memories`, but for tables.
    fn deallocate_tables(&self, index: usize, tables: &mut PrimaryMap<DefinedTableIndex, Table>);

    /// Allocates a fiber stack for calling async functions on.
    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack>;

    /// Deallocates a fiber stack that was previously allocated with `allocate_fiber_stack`.
    ///
    /// # Safety
    ///
    /// The provided stack is required to have been allocated with `allocate_fiber_stack`.
    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, stack: &wasmtime_fiber::FiberStack);

    /// Purges all lingering resources related to `module` from within this
    /// allocator.
    ///
    /// Primarily present for the pooling allocator to remove mappings of
    /// this module from slots in linear memory.
    fn purge_module(&self, module: CompiledModuleId);
}

fn get_table_init_start(init: &TableInitializer, instance: &mut Instance) -> Result<u32> {
    match init.base {
        Some(base) => {
            let val = unsafe { *(*instance.defined_or_imported_global_ptr(base)).as_u32() };

            init.offset
                .checked_add(val)
                .ok_or_else(|| anyhow!("element segment global base overflows"))
        }
        None => Ok(init.offset),
    }
}

fn check_table_init_bounds(instance: &mut Instance, module: &Module) -> Result<()> {
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
                        bail!("table out of bounds: elements segment does not fit")
                    }
                }
            }
        }
    }

    Ok(())
}

fn initialize_tables(instance: &mut Instance, module: &Module) -> Result<()> {
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
                let start = get_table_init_start(segment, instance)?;
                instance.table_init_segment(
                    segment.table_index,
                    &segment.elements,
                    start,
                    0,
                    segment.elements.len() as u32,
                )?;
            }
        }
    }

    Ok(())
}

fn get_memory_init_start(init: &MemoryInitializer, instance: &mut Instance) -> Result<u64> {
    match init.base {
        Some(base) => {
            let mem64 = instance.module().memory_plans[init.memory_index]
                .memory
                .memory64;
            let val = unsafe {
                let global = instance.defined_or_imported_global_ptr(base);
                if mem64 {
                    *(*global).as_u64()
                } else {
                    u64::from(*(*global).as_u32())
                }
            };

            init.offset
                .checked_add(val)
                .ok_or_else(|| anyhow!("data segment global base overflows"))
        }
        None => Ok(init.offset),
    }
}

fn check_memory_init_bounds(
    instance: &mut Instance,
    initializers: &[MemoryInitializer],
) -> Result<()> {
    for init in initializers {
        let memory = instance.get_memory(init.memory_index);
        let start = get_memory_init_start(init, instance)?;
        let end = usize::try_from(start)
            .ok()
            .and_then(|start| start.checked_add(init.data.len()));

        match end {
            Some(end) if end <= memory.current_length() => {
                // Initializer is in bounds
            }
            _ => {
                bail!("memory out of bounds: data segment does not fit")
            }
        }
    }

    Ok(())
}

fn initialize_memories(instance: &mut Instance, module: &Module) -> Result<()> {
    let memory_size_in_pages = &|instance: &mut Instance, memory| {
        (instance.get_memory(memory).current_length() as u64) / u64::from(WASM_PAGE_SIZE)
    };

    // Loads the `global` value and returns it as a `u64`, but sign-extends
    // 32-bit globals which can be used as the base for 32-bit memories.
    let get_global_as_u64 = &mut |instance: &mut Instance, global| unsafe {
        let def = instance.defined_or_imported_global_ptr(global);
        if module.globals[global].wasm_ty == WasmType::I64 {
            *(*def).as_u64()
        } else {
            u64::from(*(*def).as_u32())
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
        instance,
        InitMemory::Runtime {
            memory_size_in_pages,
            get_global_as_u64,
        },
        |instance, memory_index, init| {
            // If this initializer applies to a defined memory but that memory
            // doesn't need initialization, due to something like copy-on-write
            // pre-initializing it via mmap magic, then this initializer can be
            // skipped entirely.
            if let Some(memory_index) = module.defined_memory_index(memory_index) {
                if !instance.memories[memory_index].needs_init() {
                    return true;
                }
            }
            let memory = instance.get_memory(memory_index);

            unsafe {
                let src = instance.wasm_data(init.data.clone());
                let dst = memory.base.add(usize::try_from(init.offset).unwrap());
                // FIXME audit whether this is safe in the presence of shared
                // memory
                // (https://github.com/bytecodealliance/wasmtime/issues/4203).
                ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len())
            }
            true
        },
    );
    if !ok {
        return Err(Trap::MemoryOutOfBounds.into());
    }

    Ok(())
}

fn check_init_bounds(instance: &mut Instance, module: &Module) -> Result<()> {
    check_table_init_bounds(instance, module)?;

    match &module.memory_initialization {
        MemoryInitialization::Segmented(initializers) => {
            check_memory_init_bounds(instance, initializers)?;
        }
        // Statically validated already to have everything in-bounds.
        MemoryInitialization::Static { .. } => {}
    }

    Ok(())
}

pub(super) fn initialize_instance(
    instance: &mut Instance,
    module: &Module,
    is_bulk_memory: bool,
) -> Result<()> {
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
    fn allocate_index(&self, _req: &InstanceAllocationRequest) -> Result<usize> {
        Ok(0)
    }

    fn deallocate_index(&self, index: usize) {
        assert_eq!(index, 0);
    }

    fn allocate_memories(
        &self,
        _index: usize,
        req: &mut InstanceAllocationRequest,
        memories: &mut PrimaryMap<DefinedMemoryIndex, Memory>,
    ) -> Result<()> {
        let module = req.runtime_info.module();
        let creator = self
            .mem_creator
            .as_deref()
            .unwrap_or_else(|| &DefaultMemoryCreator);
        let num_imports = module.num_imported_memories;
        for (memory_idx, plan) in module.memory_plans.iter().skip(num_imports) {
            let defined_memory_idx = module
                .defined_memory_index(memory_idx)
                .expect("Skipped imports, should never be None");
            let image = req.runtime_info.memory_image(defined_memory_idx)?;

            memories.push(Memory::new_dynamic(
                plan,
                creator,
                unsafe {
                    req.store
                        .get()
                        .expect("if module has memory plans, store is not empty")
                },
                image,
            )?);
        }
        Ok(())
    }

    fn deallocate_memories(
        &self,
        _index: usize,
        _mems: &mut PrimaryMap<DefinedMemoryIndex, Memory>,
    ) {
        // normal destructors do cleanup here
    }

    fn allocate_tables(
        &self,
        _index: usize,
        req: &mut InstanceAllocationRequest,
        tables: &mut PrimaryMap<DefinedTableIndex, Table>,
    ) -> Result<()> {
        let module = req.runtime_info.module();
        let num_imports = module.num_imported_tables;
        for (_, table) in module.table_plans.iter().skip(num_imports) {
            tables.push(Table::new_dynamic(table, unsafe {
                req.store
                    .get()
                    .expect("if module has table plans, store is not empty")
            })?);
        }
        Ok(())
    }

    fn deallocate_tables(&self, _index: usize, _tables: &mut PrimaryMap<DefinedTableIndex, Table>) {
        // normal destructors do cleanup here
    }

    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack> {
        if self.stack_size == 0 {
            bail!("fiber stacks are not supported by the allocator")
        }

        let stack = wasmtime_fiber::FiberStack::new(self.stack_size)?;
        Ok(stack)
    }

    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, _stack: &wasmtime_fiber::FiberStack) {
        // The on-demand allocator has no further bookkeeping for fiber stacks
    }

    fn purge_module(&self, _: CompiledModuleId) {}
}

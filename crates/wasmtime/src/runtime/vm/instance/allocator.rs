use crate::prelude::*;
use crate::runtime::vm::const_expr::{ConstEvalContext, ConstExprEvaluator};
use crate::runtime::vm::imports::Imports;
use crate::runtime::vm::instance::{Instance, InstanceHandle};
use crate::runtime::vm::memory::Memory;
use crate::runtime::vm::mpk::ProtectionKey;
use crate::runtime::vm::table::Table;
use crate::runtime::vm::{CompiledModuleId, ModuleRuntimeInfo, VMFuncRef, VMGcRef, VMStore};
use crate::store::AutoAssertNoGc;
use crate::vm::VMGlobalDefinition;
use core::{any::Any, mem, ptr};
use wasmtime_environ::{
    DefinedMemoryIndex, DefinedTableIndex, HostPtr, InitMemory, MemoryInitialization,
    MemoryInitializer, MemoryPlan, Module, PrimaryMap, SizeOverflow, TableInitialValue, TablePlan,
    Trap, VMOffsets, WasmHeapTopType,
};

#[cfg(feature = "gc")]
use crate::runtime::vm::{GcHeap, GcRuntime};

#[cfg(feature = "component-model")]
use wasmtime_environ::{
    component::{Component, VMComponentOffsets},
    StaticModuleIndex,
};

mod on_demand;
pub use self::on_demand::OnDemandInstanceAllocator;

#[cfg(feature = "pooling-allocator")]
mod pooling;
#[cfg(feature = "pooling-allocator")]
pub use self::pooling::{
    InstanceLimits, PoolConcurrencyLimitError, PoolingInstanceAllocator,
    PoolingInstanceAllocatorConfig,
};

/// Represents a request for a new runtime instance.
pub struct InstanceAllocationRequest<'a> {
    /// The info related to the compiled version of this module,
    /// needed for instantiation: function metadata, JIT code
    /// addresses, precomputed images for lazy memory and table
    /// initialization, and the like. This Arc is cloned and held for
    /// the lifetime of the instance.
    pub runtime_info: &'a ModuleRuntimeInfo,

    /// The imports to use for the instantiation.
    pub imports: Imports<'a>,

    /// The host state to associate with the instance.
    pub host_state: Box<dyn Any + Send + Sync>,

    /// A pointer to the "store" for this instance to be allocated. The store
    /// correlates with the `Store` in wasmtime itself, and lots of contextual
    /// information about the execution of wasm can be learned through the
    /// store.
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

    /// Indicates '--wmemcheck' flag.
    #[cfg_attr(not(feature = "wmemcheck"), allow(dead_code))]
    pub wmemcheck: bool,

    /// Request that the instance's memories be protected by a specific
    /// protection key.
    pub pkey: Option<ProtectionKey>,
}

/// A pointer to a Store. This Option<*mut dyn Store> is wrapped in a struct
/// so that the function to create a &mut dyn Store is a method on a member of
/// InstanceAllocationRequest, rather than on a &mut InstanceAllocationRequest
/// itself, because several use-sites require a split mut borrow on the
/// InstanceAllocationRequest.
pub struct StorePtr(Option<*mut dyn VMStore>);

impl StorePtr {
    /// A pointer to no Store.
    pub fn empty() -> Self {
        Self(None)
    }

    /// A pointer to a Store.
    pub fn new(ptr: *mut dyn VMStore) -> Self {
        Self(Some(ptr))
    }

    /// The raw contents of this struct
    pub fn as_raw(&self) -> Option<*mut dyn VMStore> {
        self.0
    }

    /// Use the StorePtr as a mut ref to the Store.
    ///
    /// Safety: must not be used outside the original lifetime of the borrow.
    pub(crate) unsafe fn get(&mut self) -> Option<&mut dyn VMStore> {
        match self.0 {
            Some(ptr) => Some(&mut *ptr),
            None => None,
        }
    }
}

/// The index of a memory allocation within an `InstanceAllocator`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct MemoryAllocationIndex(u32);

impl Default for MemoryAllocationIndex {
    fn default() -> Self {
        // A default `MemoryAllocationIndex` that can be used with
        // `InstanceAllocator`s that don't actually need indices.
        MemoryAllocationIndex(u32::MAX)
    }
}

impl MemoryAllocationIndex {
    /// Get the underlying index of this `MemoryAllocationIndex`.
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// The index of a table allocation within an `InstanceAllocator`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct TableAllocationIndex(u32);

impl Default for TableAllocationIndex {
    fn default() -> Self {
        // A default `TableAllocationIndex` that can be used with
        // `InstanceAllocator`s that don't actually need indices.
        TableAllocationIndex(u32::MAX)
    }
}

impl TableAllocationIndex {
    /// Get the underlying index of this `TableAllocationIndex`.
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// The index of a table allocation within an `InstanceAllocator`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct GcHeapAllocationIndex(u32);

impl Default for GcHeapAllocationIndex {
    fn default() -> Self {
        // A default `GcHeapAllocationIndex` that can be used with
        // `InstanceAllocator`s that don't actually need indices.
        GcHeapAllocationIndex(u32::MAX)
    }
}

impl GcHeapAllocationIndex {
    /// Get the underlying index of this `GcHeapAllocationIndex`.
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Trait that represents the hooks needed to implement an instance allocator.
///
/// Implement this trait when implementing new instance allocators, but don't
/// use this trait when you need an instance allocator. Instead use the
/// `InstanceAllocator` trait for that, which has additional helper methods and
/// a blanket implementation for all types that implement this trait.
///
/// # Safety
///
/// This trait is unsafe as it requires knowledge of Wasmtime's runtime
/// internals to implement correctly.
pub unsafe trait InstanceAllocatorImpl {
    /// Validate whether a component (including all of its contained core
    /// modules) is allocatable by this instance allocator.
    #[cfg(feature = "component-model")]
    fn validate_component_impl<'a>(
        &self,
        component: &Component,
        offsets: &VMComponentOffsets<HostPtr>,
        get_module: &'a dyn Fn(StaticModuleIndex) -> &'a Module,
    ) -> Result<()>;

    /// Validate whether a module is allocatable by this instance allocator.
    fn validate_module_impl(&self, module: &Module, offsets: &VMOffsets<HostPtr>) -> Result<()>;

    /// Increment the count of concurrent component instances that are currently
    /// allocated, if applicable.
    ///
    /// Not all instance allocators will have limits for the maximum number of
    /// concurrent component instances that can be live at the same time, and
    /// these allocators may implement this method with a no-op.
    //
    // Note: It would be nice to have an associated type that on construction
    // does the increment and on drop does the decrement but there are two
    // problems with this:
    //
    // 1. This trait's implementations are always used as trait objects, and
    //    associated types are not object safe.
    //
    // 2. We would want a parameterized `Drop` implementation so that we could
    //    pass in the `InstanceAllocatorImpl` on drop, but this doesn't exist in
    //    Rust. Therefore, we would be forced to add reference counting and
    //    stuff like that to keep a handle on the instance allocator from this
    //    theoretical type. That's a bummer.
    fn increment_component_instance_count(&self) -> Result<()>;

    /// The dual of `increment_component_instance_count`.
    fn decrement_component_instance_count(&self);

    /// Increment the count of concurrent core module instances that are
    /// currently allocated, if applicable.
    ///
    /// Not all instance allocators will have limits for the maximum number of
    /// concurrent core module instances that can be live at the same time, and
    /// these allocators may implement this method with a no-op.
    fn increment_core_instance_count(&self) -> Result<()>;

    /// The dual of `increment_core_instance_count`.
    fn decrement_core_instance_count(&self);

    /// Allocate a memory for an instance.
    ///
    /// # Unsafety
    ///
    /// The memory and its associated module must have already been validated by
    /// `Self::validate_module` and passed that validation.
    unsafe fn allocate_memory(
        &self,
        request: &mut InstanceAllocationRequest,
        memory_plan: &MemoryPlan,
        memory_index: DefinedMemoryIndex,
    ) -> Result<(MemoryAllocationIndex, Memory)>;

    /// Deallocate an instance's previously allocated memory.
    ///
    /// # Unsafety
    ///
    /// The memory must have previously been allocated by
    /// `Self::allocate_memory`, be at the given index, and must currently be
    /// allocated. It must never be used again.
    unsafe fn deallocate_memory(
        &self,
        memory_index: DefinedMemoryIndex,
        allocation_index: MemoryAllocationIndex,
        memory: Memory,
    );

    /// Allocate a table for an instance.
    ///
    /// # Unsafety
    ///
    /// The table and its associated module must have already been validated by
    /// `Self::validate_module` and passed that validation.
    unsafe fn allocate_table(
        &self,
        req: &mut InstanceAllocationRequest,
        table_plan: &TablePlan,
        table_index: DefinedTableIndex,
    ) -> Result<(TableAllocationIndex, Table)>;

    /// Deallocate an instance's previously allocated table.
    ///
    /// # Unsafety
    ///
    /// The table must have previously been allocated by `Self::allocate_table`,
    /// be at the given index, and must currently be allocated. It must never be
    /// used again.
    unsafe fn deallocate_table(
        &self,
        table_index: DefinedTableIndex,
        allocation_index: TableAllocationIndex,
        table: Table,
    );

    /// Allocates a fiber stack for calling async functions on.
    #[cfg(feature = "async")]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack>;

    /// Deallocates a fiber stack that was previously allocated with
    /// `allocate_fiber_stack`.
    ///
    /// # Safety
    ///
    /// The provided stack is required to have been allocated with
    /// `allocate_fiber_stack`.
    #[cfg(feature = "async")]
    unsafe fn deallocate_fiber_stack(&self, stack: wasmtime_fiber::FiberStack);

    /// Allocate a GC heap for allocating Wasm GC objects within.
    #[cfg(feature = "gc")]
    fn allocate_gc_heap(
        &self,
        gc_runtime: &dyn GcRuntime,
    ) -> Result<(GcHeapAllocationIndex, Box<dyn GcHeap>)>;

    /// Deallocate a GC heap that was previously allocated with
    /// `allocate_gc_heap`.
    #[cfg(feature = "gc")]
    fn deallocate_gc_heap(&self, allocation_index: GcHeapAllocationIndex, gc_heap: Box<dyn GcHeap>);

    /// Purges all lingering resources related to `module` from within this
    /// allocator.
    ///
    /// Primarily present for the pooling allocator to remove mappings of
    /// this module from slots in linear memory.
    fn purge_module(&self, module: CompiledModuleId);

    /// Use the next available protection key.
    ///
    /// The pooling allocator can use memory protection keys (MPK) for
    /// compressing the guard regions protecting against OOB. Each
    /// pool-allocated store needs its own key.
    fn next_available_pkey(&self) -> Option<ProtectionKey>;

    /// Restrict access to memory regions protected by `pkey`.
    ///
    /// This is useful for the pooling allocator, which can use memory
    /// protection keys (MPK). Note: this may still allow access to other
    /// protection keys, such as the default kernel key; see implementations of
    /// this.
    fn restrict_to_pkey(&self, pkey: ProtectionKey);

    /// Allow access to memory regions protected by any protection key.
    fn allow_all_pkeys(&self);
}

/// A thing that can allocate instances.
///
/// Don't implement this trait directly, instead implement
/// `InstanceAllocatorImpl` and you'll get this trait for free via a blanket
/// impl.
pub trait InstanceAllocator: InstanceAllocatorImpl {
    /// Validate whether a component (including all of its contained core
    /// modules) is allocatable with this instance allocator.
    #[cfg(feature = "component-model")]
    fn validate_component<'a>(
        &self,
        component: &Component,
        offsets: &VMComponentOffsets<HostPtr>,
        get_module: &'a dyn Fn(StaticModuleIndex) -> &'a Module,
    ) -> Result<()> {
        InstanceAllocatorImpl::validate_component_impl(self, component, offsets, get_module)
    }

    /// Validate whether a core module is allocatable with this instance
    /// allocator.
    fn validate_module(&self, module: &Module, offsets: &VMOffsets<HostPtr>) -> Result<()> {
        InstanceAllocatorImpl::validate_module_impl(self, module, offsets)
    }

    /// Allocates a fresh `InstanceHandle` for the `req` given.
    ///
    /// This will allocate memories and tables internally from this allocator
    /// and weave that altogether into a final and complete `InstanceHandle`
    /// ready to be registered with a store.
    ///
    /// Note that the returned instance must still have `.initialize(..)` called
    /// on it to complete the instantiation process.
    ///
    /// # Unsafety
    ///
    /// The request's associated module, memories, tables, and vmctx must have
    /// already have been validated by `Self::validate_module`.
    unsafe fn allocate_module(
        &self,
        mut request: InstanceAllocationRequest,
    ) -> Result<InstanceHandle> {
        let module = request.runtime_info.env_module();

        #[cfg(debug_assertions)]
        InstanceAllocatorImpl::validate_module_impl(self, module, request.runtime_info.offsets())
            .expect("module should have already been validated before allocation");

        self.increment_core_instance_count()?;

        let num_defined_memories = module.memory_plans.len() - module.num_imported_memories;
        let mut memories = PrimaryMap::with_capacity(num_defined_memories);

        let num_defined_tables = module.table_plans.len() - module.num_imported_tables;
        let mut tables = PrimaryMap::with_capacity(num_defined_tables);

        match (|| {
            self.allocate_memories(&mut request, &mut memories)?;
            self.allocate_tables(&mut request, &mut tables)?;
            Ok(())
        })() {
            Ok(_) => Ok(Instance::new(
                request,
                memories,
                tables,
                &module.memory_plans,
            )),
            Err(e) => {
                self.deallocate_memories(&mut memories);
                self.deallocate_tables(&mut tables);
                self.decrement_core_instance_count();
                Err(e)
            }
        }
    }

    /// Deallocates the provided instance.
    ///
    /// This will null-out the pointer within `handle` and otherwise reclaim
    /// resources such as tables, memories, and the instance memory itself.
    ///
    /// # Unsafety
    ///
    /// The instance must have previously been allocated by `Self::allocate`.
    unsafe fn deallocate_module(&self, handle: &mut InstanceHandle) {
        self.deallocate_memories(&mut handle.instance_mut().memories);
        self.deallocate_tables(&mut handle.instance_mut().tables);

        let layout = Instance::alloc_layout(handle.instance().offsets());
        let ptr = handle.instance.take().unwrap();
        ptr::drop_in_place(ptr.as_ptr());
        alloc::alloc::dealloc(ptr.as_ptr().cast(), layout);

        self.decrement_core_instance_count();
    }

    /// Allocate the memories for the given instance allocation request, pushing
    /// them into `memories`.
    ///
    /// # Unsafety
    ///
    /// The request's associated module and memories must have previously been
    /// validated by `Self::validate_module`.
    unsafe fn allocate_memories(
        &self,
        request: &mut InstanceAllocationRequest,
        memories: &mut PrimaryMap<DefinedMemoryIndex, (MemoryAllocationIndex, Memory)>,
    ) -> Result<()> {
        let module = request.runtime_info.env_module();

        #[cfg(debug_assertions)]
        InstanceAllocatorImpl::validate_module_impl(self, module, request.runtime_info.offsets())
            .expect("module should have already been validated before allocation");

        for (memory_index, memory_plan) in module
            .memory_plans
            .iter()
            .skip(module.num_imported_memories)
        {
            let memory_index = module
                .defined_memory_index(memory_index)
                .expect("should be a defined memory since we skipped imported ones");

            memories.push(self.allocate_memory(request, memory_plan, memory_index)?);
        }

        Ok(())
    }

    /// Deallocate all the memories in the given primary map.
    ///
    /// # Unsafety
    ///
    /// The memories must have previously been allocated by
    /// `Self::allocate_memories`.
    unsafe fn deallocate_memories(
        &self,
        memories: &mut PrimaryMap<DefinedMemoryIndex, (MemoryAllocationIndex, Memory)>,
    ) {
        for (memory_index, (allocation_index, memory)) in mem::take(memories) {
            // Because deallocating memory is infallible, we don't need to worry
            // about leaking subsequent memories if the first memory failed to
            // deallocate. If deallocating memory ever becomes fallible, we will
            // need to be careful here!
            self.deallocate_memory(memory_index, allocation_index, memory);
        }
    }

    /// Allocate tables for the given instance allocation request, pushing them
    /// into `tables`.
    ///
    /// # Unsafety
    ///
    /// The request's associated module and tables must have previously been
    /// validated by `Self::validate_module`.
    unsafe fn allocate_tables(
        &self,
        request: &mut InstanceAllocationRequest,
        tables: &mut PrimaryMap<DefinedTableIndex, (TableAllocationIndex, Table)>,
    ) -> Result<()> {
        let module = request.runtime_info.env_module();

        #[cfg(debug_assertions)]
        InstanceAllocatorImpl::validate_module_impl(self, module, request.runtime_info.offsets())
            .expect("module should have already been validated before allocation");

        for (index, plan) in module.table_plans.iter().skip(module.num_imported_tables) {
            let def_index = module
                .defined_table_index(index)
                .expect("should be a defined table since we skipped imported ones");

            tables.push(self.allocate_table(request, plan, def_index)?);
        }

        Ok(())
    }

    /// Deallocate all the tables in the given primary map.
    ///
    /// # Unsafety
    ///
    /// The tables must have previously been allocated by
    /// `Self::allocate_tables`.
    unsafe fn deallocate_tables(
        &self,
        tables: &mut PrimaryMap<DefinedTableIndex, (TableAllocationIndex, Table)>,
    ) {
        for (table_index, (allocation_index, table)) in mem::take(tables) {
            self.deallocate_table(table_index, allocation_index, table);
        }
    }
}

// Every `InstanceAllocatorImpl` is an `InstanceAllocator` when used
// correctly. Also, no one is allowed to override this trait's methods, they
// must use the defaults. This blanket impl provides both of those things.
impl<T: InstanceAllocatorImpl> InstanceAllocator for T {}

fn check_table_init_bounds(instance: &mut Instance, module: &Module) -> Result<()> {
    let mut const_evaluator = ConstExprEvaluator::default();

    for segment in module.table_initialization.segments.iter() {
        let table = unsafe { &*instance.get_table(segment.table_index) };
        let mut context = ConstEvalContext::new(instance);
        let start = unsafe {
            const_evaluator
                .eval(&mut context, &segment.offset)
                .expect("const expression should be valid")
        };
        let start = usize::try_from(start.get_u32()).unwrap();
        let end = start.checked_add(usize::try_from(segment.elements.len()).unwrap());

        match end {
            Some(end) if end <= table.size() => {
                // Initializer is in bounds
            }
            _ => {
                bail!("table out of bounds: elements segment does not fit")
            }
        }
    }

    Ok(())
}

fn initialize_tables(
    context: &mut ConstEvalContext<'_>,
    const_evaluator: &mut ConstExprEvaluator,
    module: &Module,
) -> Result<()> {
    for (table, init) in module.table_initialization.initial_values.iter() {
        match init {
            // Tables are always initially null-initialized at this time
            TableInitialValue::Null { precomputed: _ } => {}

            TableInitialValue::Expr(expr) => {
                let raw = unsafe {
                    const_evaluator
                        .eval(context, expr)
                        .expect("const expression should be valid")
                };
                let idx = module.table_index(table);
                let table = unsafe { context.instance.get_defined_table(table).as_mut().unwrap() };
                match module.table_plans[idx].table.ref_type.heap_type.top() {
                    WasmHeapTopType::Extern => {
                        let gc_ref = VMGcRef::from_raw_u32(raw.get_externref());
                        let gc_store = unsafe { (*context.instance.store()).gc_store_mut()? };
                        let items = (0..table.size())
                            .map(|_| gc_ref.as_ref().map(|r| gc_store.clone_gc_ref(r)));
                        table.init_gc_refs(0, items).err2anyhow()?;
                    }

                    WasmHeapTopType::Any => {
                        let gc_ref = VMGcRef::from_raw_u32(raw.get_anyref());
                        let gc_store = unsafe { (*context.instance.store()).gc_store_mut()? };
                        let items = (0..table.size())
                            .map(|_| gc_ref.as_ref().map(|r| gc_store.clone_gc_ref(r)));
                        table.init_gc_refs(0, items).err2anyhow()?;
                    }

                    WasmHeapTopType::Func => {
                        let funcref = raw.get_funcref().cast::<VMFuncRef>();
                        let items = (0..table.size()).map(|_| funcref);
                        table.init_func(0, items).err2anyhow()?;
                    }
                }
            }
        }
    }

    // Note: if the module's table initializer state is in
    // FuncTable mode, we will lazily initialize tables based on
    // any statically-precomputed image of FuncIndexes, but there
    // may still be "leftover segments" that could not be
    // incorporated. So we have a unified handler here that
    // iterates over all segments (Segments mode) or leftover
    // segments (FuncTable mode) to initialize.
    for segment in module.table_initialization.segments.iter() {
        let start = unsafe {
            const_evaluator
                .eval(context, &segment.offset)
                .expect("const expression should be valid")
        };
        context
            .instance
            .table_init_segment(
                const_evaluator,
                segment.table_index,
                &segment.elements,
                start.get_u64(),
                0,
                segment.elements.len(),
            )
            .err2anyhow()?;
    }

    Ok(())
}

fn get_memory_init_start(init: &MemoryInitializer, instance: &mut Instance) -> Result<u64> {
    let mut context = ConstEvalContext::new(instance);
    let mut const_evaluator = ConstExprEvaluator::default();
    unsafe { const_evaluator.eval(&mut context, &init.offset) }.map(|v| {
        match instance.env_module().memory_plans[init.memory_index]
            .memory
            .idx_type
        {
            wasmtime_environ::IndexType::I32 => v.get_u32().into(),
            wasmtime_environ::IndexType::I64 => v.get_u64(),
        }
    })
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

fn initialize_memories(
    context: &mut ConstEvalContext<'_>,
    const_evaluator: &mut ConstExprEvaluator,
    module: &Module,
) -> Result<()> {
    // Delegates to the `init_memory` method which is sort of a duplicate of
    // `instance.memory_init_segment` but is used at compile-time in other
    // contexts so is shared here to have only one method of memory
    // initialization.
    //
    // This call to `init_memory` notably implements all the bells and whistles
    // so errors only happen if an out-of-bounds segment is found, in which case
    // a trap is returned.

    struct InitMemoryAtInstantiation<'a, 'b> {
        module: &'a Module,
        context: &'a mut ConstEvalContext<'b>,
        const_evaluator: &'a mut ConstExprEvaluator,
    }

    impl InitMemory for InitMemoryAtInstantiation<'_, '_> {
        fn memory_size_in_bytes(
            &mut self,
            memory: wasmtime_environ::MemoryIndex,
        ) -> Result<u64, SizeOverflow> {
            let len = self.context.instance.get_memory(memory).current_length();
            let len = u64::try_from(len).unwrap();
            Ok(len)
        }

        fn eval_offset(
            &mut self,
            memory: wasmtime_environ::MemoryIndex,
            expr: &wasmtime_environ::ConstExpr,
        ) -> Option<u64> {
            let val = unsafe { self.const_evaluator.eval(self.context, expr) }
                .expect("const expression should be valid");
            Some(
                match self.context.instance.env_module().memory_plans[memory]
                    .memory
                    .idx_type
                {
                    wasmtime_environ::IndexType::I32 => val.get_u32().into(),
                    wasmtime_environ::IndexType::I64 => val.get_u64(),
                },
            )
        }

        fn write(
            &mut self,
            memory_index: wasmtime_environ::MemoryIndex,
            init: &wasmtime_environ::StaticMemoryInitializer,
        ) -> bool {
            // If this initializer applies to a defined memory but that memory
            // doesn't need initialization, due to something like copy-on-write
            // pre-initializing it via mmap magic, then this initializer can be
            // skipped entirely.
            if let Some(memory_index) = self.module.defined_memory_index(memory_index) {
                if !self.context.instance.memories[memory_index].1.needs_init() {
                    return true;
                }
            }
            let memory = self.context.instance.get_memory(memory_index);

            unsafe {
                let src = self.context.instance.wasm_data(init.data.clone());
                let offset = usize::try_from(init.offset).unwrap();
                let dst = memory.base.add(offset);

                assert!(offset + src.len() <= memory.current_length());

                // FIXME audit whether this is safe in the presence of shared
                // memory
                // (https://github.com/bytecodealliance/wasmtime/issues/4203).
                ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len())
            }
            true
        }
    }

    let ok = module
        .memory_initialization
        .init_memory(&mut InitMemoryAtInstantiation {
            module,
            context,
            const_evaluator,
        });
    if !ok {
        return Err(Trap::MemoryOutOfBounds).err2anyhow();
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

fn initialize_globals(
    context: &mut ConstEvalContext<'_>,
    const_evaluator: &mut ConstExprEvaluator,
    module: &Module,
) -> Result<()> {
    assert!(core::ptr::eq(&**context.instance.env_module(), module));

    for (index, init) in module.global_initializers.iter() {
        let raw = unsafe {
            const_evaluator
                .eval(context, init)
                .expect("should be a valid const expr")
        };

        let to = context.instance.global_ptr(index);
        let wasm_ty = module.globals[module.global_index(index)].wasm_ty;

        #[cfg(feature = "wmemcheck")]
        if index.as_bits() == 0 && wasm_ty == wasmtime_environ::WasmValType::I32 {
            if let Some(wmemcheck) = &mut context.instance.wmemcheck_state {
                let size = usize::try_from(raw.get_i32()).unwrap();
                wmemcheck.set_stack_size(size);
            }
        }

        let store = unsafe { (*context.instance.store()).store_opaque_mut() };
        let mut store = AutoAssertNoGc::new(store);

        // This write is safe because we know we have the correct module for
        // this instance and its vmctx due to the assert above.
        unsafe {
            ptr::write(
                to,
                VMGlobalDefinition::from_val_raw(&mut store, wasm_ty, raw)?,
            )
        };
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

    let mut context = ConstEvalContext::new(instance);
    let mut const_evaluator = ConstExprEvaluator::default();

    initialize_globals(&mut context, &mut const_evaluator, module)?;
    initialize_tables(&mut context, &mut const_evaluator, module)?;
    initialize_memories(&mut context, &mut const_evaluator, &module)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_traits_are_object_safe() {
        fn _instance_allocator(_: &dyn InstanceAllocatorImpl) {}
        fn _instance_allocator_ext(_: &dyn InstanceAllocator) {}
    }
}

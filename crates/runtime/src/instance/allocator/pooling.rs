//! Implements the pooling instance allocator.
//!
//! The pooling instance allocator maps memory in advance
//! and allocates instances, memories, tables, and stacks from
//! a pool of available resources.
//!
//! Using the pooling instance allocator can speed up module instantiation
//! when modules can be constrained based on configurable limits.

use super::{
    initialize_instance, initialize_vmcontext, FiberStackError, InstanceAllocationRequest,
    InstanceAllocator, InstanceHandle, InstantiationError,
};
use crate::{instance::Instance, table::max_table_element_size, Memory, Mmap, Table, VMContext};
use rand::Rng;
use std::cell::RefCell;
use std::cmp::min;
use std::convert::TryFrom;
use std::mem;
use std::sync::{Arc, Mutex};
use wasmtime_environ::{
    entity::{EntitySet, PrimaryMap},
    MemoryStyle, Module, ModuleTranslation, Tunables, VMOffsets, WASM_PAGE_SIZE,
};

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod windows;
        use windows as imp;
    } else if #[cfg(all(feature = "uffd", target_os = "linux"))] {
        mod uffd;
        use uffd as imp;
        use imp::{PageFaultHandler, reset_guard_page};
        use super::{check_init_bounds, initialize_tables};
        use wasmtime_environ::MemoryInitialization;
        use std::sync::atomic::{AtomicBool, Ordering};
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        use linux as imp;
    } else {
        mod unix;
        use unix as imp;
    }
}

use imp::{create_memory_map, decommit, make_accessible};

fn round_up_to_pow2(n: usize, to: usize) -> usize {
    debug_assert!(to > 0);
    debug_assert!(to.is_power_of_two());
    (n + to - 1) & !(to - 1)
}

/// Represents the limits placed on a module for compiling with the pooling instance allocator.
#[derive(Debug, Copy, Clone)]
pub struct ModuleLimits {
    /// The maximum number of imported functions for a module (default is 1000).
    pub imported_functions: u32,

    /// The maximum number of imported tables for a module (default is 0).
    pub imported_tables: u32,

    /// The maximum number of imported memories for a module (default is 0).
    pub imported_memories: u32,

    /// The maximum number of imported globals for a module (default is 0).
    pub imported_globals: u32,

    /// The maximum number of defined types for a module (default is 100).
    pub types: u32,

    /// The maximum number of defined functions for a module (default is 10000).
    pub functions: u32,

    /// The maximum number of defined tables for a module (default is 1).
    pub tables: u32,

    /// The maximum number of defined memories for a module (default is 1).
    pub memories: u32,

    /// The maximum number of defined globals for a module (default is 10).
    pub globals: u32,

    /// The maximum table elements for any table defined in a module (default is 10000).
    ///
    /// If a table's minimum element limit is greater than this value, the module will
    /// fail to compile.
    ///
    /// If a table's maximum element limit is unbounded or greater than this value,
    /// the maximum will be `table_elements` for the purpose of any `table.grow` instruction.
    pub table_elements: u32,

    /// The maximum number of pages for any memory defined in a module (default is 160).
    ///
    /// The default of 160 means at most 10 MiB of host memory may be committed for each instance.
    ///
    /// If a memory's minimum page limit is greater than this value, the module will
    /// fail to compile.
    ///
    /// If a memory's maximum page limit is unbounded or greater than this value,
    /// the maximum will be `memory_pages` for the purpose of any `memory.grow` instruction.
    ///
    /// This value cannot exceed any address space limits placed on instances.
    pub memory_pages: u32,
}

impl ModuleLimits {
    fn validate_module(&self, module: &Module) -> Result<(), String> {
        if module.num_imported_funcs > self.imported_functions as usize {
            return Err(format!(
                "imported function count of {} exceeds the limit of {}",
                module.num_imported_funcs, self.imported_functions
            ));
        }

        if module.num_imported_tables > self.imported_tables as usize {
            return Err(format!(
                "imported tables count of {} exceeds the limit of {}",
                module.num_imported_tables, self.imported_tables
            ));
        }

        if module.num_imported_memories > self.imported_memories as usize {
            return Err(format!(
                "imported memories count of {} exceeds the limit of {}",
                module.num_imported_memories, self.imported_memories
            ));
        }

        if module.num_imported_globals > self.imported_globals as usize {
            return Err(format!(
                "imported globals count of {} exceeds the limit of {}",
                module.num_imported_globals, self.imported_globals
            ));
        }

        if module.types.len() > self.types as usize {
            return Err(format!(
                "defined types count of {} exceeds the limit of {}",
                module.types.len(),
                self.types
            ));
        }

        let functions = module.functions.len() - module.num_imported_funcs;
        if functions > self.functions as usize {
            return Err(format!(
                "defined functions count of {} exceeds the limit of {}",
                functions, self.functions
            ));
        }

        let tables = module.table_plans.len() - module.num_imported_tables;
        if tables > self.tables as usize {
            return Err(format!(
                "defined tables count of {} exceeds the limit of {}",
                tables, self.tables
            ));
        }

        let memories = module.memory_plans.len() - module.num_imported_memories;
        if memories > self.memories as usize {
            return Err(format!(
                "defined memories count of {} exceeds the limit of {}",
                memories, self.memories
            ));
        }

        let globals = module.globals.len() - module.num_imported_globals;
        if globals > self.globals as usize {
            return Err(format!(
                "defined globals count of {} exceeds the limit of {}",
                globals, self.globals
            ));
        }

        for (i, plan) in module.table_plans.values().as_slice()[module.num_imported_tables..]
            .iter()
            .enumerate()
        {
            if plan.table.minimum > self.table_elements {
                return Err(format!(
                    "table index {} has a minimum element size of {} which exceeds the limit of {}",
                    i, plan.table.minimum, self.table_elements
                ));
            }
        }

        for (i, plan) in module.memory_plans.values().as_slice()[module.num_imported_memories..]
            .iter()
            .enumerate()
        {
            if plan.memory.minimum > self.memory_pages {
                return Err(format!(
                    "memory index {} has a minimum page size of {} which exceeds the limit of {}",
                    i, plan.memory.minimum, self.memory_pages
                ));
            }

            if let MemoryStyle::Dynamic = plan.style {
                return Err(format!(
                    "memory index {} has an unsupported dynamic memory plan style",
                    i,
                ));
            }
        }

        Ok(())
    }
}

impl Default for ModuleLimits {
    fn default() -> Self {
        // See doc comments for `ModuleLimits` for these default values
        Self {
            imported_functions: 1000,
            imported_tables: 0,
            imported_memories: 0,
            imported_globals: 0,
            types: 100,
            functions: 10000,
            tables: 1,
            memories: 1,
            globals: 10,
            table_elements: 10000,
            memory_pages: 160,
        }
    }
}

/// Represents the limits placed on instances by the pooling instance allocator.
#[derive(Debug, Copy, Clone)]
pub struct InstanceLimits {
    /// The maximum number of concurrent instances supported (default is 1000).
    pub count: u32,

    /// The maximum reserved host address space size to use for each instance in bytes.
    ///
    /// Note: this value has important performance ramifications.
    ///
    /// On 64-bit platforms, the default for this value will be 6 GiB.  A value of less than 4 GiB will
    /// force runtime bounds checking for memory accesses and thus will negatively impact performance.
    /// Any value above 4 GiB will start eliding bounds checks provided the `offset` of the memory access is
    /// less than (`address_space_size` - 4 GiB).  A value of 8 GiB will completely elide *all* bounds
    /// checks; consequently, 8 GiB will be the maximum supported value. The default of 6 GiB reserves
    /// less host address space for each instance, but a memory access with an offet above 2 GiB will incur
    /// runtime bounds checks.
    ///
    /// On 32-bit platforms, the default for this value will be 10 MiB. A 32-bit host has very limited address
    /// space to reserve for a lot of concurrent instances.  As a result, runtime bounds checking will be used
    /// for all memory accesses.  For better runtime performance, a 64-bit host is recommended.
    ///
    /// This value will be rounded up by the WebAssembly page size (64 KiB).
    pub address_space_size: u64,
}

impl Default for InstanceLimits {
    fn default() -> Self {
        // See doc comments for `InstanceLimits` for these default values
        Self {
            count: 1000,
            #[cfg(target_pointer_width = "32")]
            address_space_size: 0xA00000,
            #[cfg(target_pointer_width = "64")]
            address_space_size: 0x180000000,
        }
    }
}

/// The allocation strategy to use for the pooling instance allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolingAllocationStrategy {
    /// Allocate from the next available instance.
    NextAvailable,
    /// Allocate from a random available instance.
    Random,
}

impl PoolingAllocationStrategy {
    fn next(&self, free_count: usize) -> usize {
        debug_assert!(free_count > 0);

        match self {
            Self::NextAvailable => free_count - 1,
            Self::Random => rand::thread_rng().gen_range(0, free_count),
        }
    }
}

impl Default for PoolingAllocationStrategy {
    fn default() -> Self {
        Self::NextAvailable
    }
}

// Used to iterate the base address of instance memories and tables.
struct BasePointerIterator {
    base: *mut u8,
    current: usize,
    num: usize,
    size: usize,
}

impl BasePointerIterator {
    fn new(base: *mut u8, num: usize, size: usize) -> Self {
        Self {
            base,
            current: 0,
            num,
            size,
        }
    }
}

impl Iterator for BasePointerIterator {
    type Item = *mut u8;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        if current == self.num {
            return None;
        }

        self.current += 1;

        Some(unsafe { self.base.add(current * self.size) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.num - self.current;
        (remaining, Some(remaining))
    }
}

/// Represents a pool of maximal `Instance` structures.
///
/// Each index in the pool provides enough space for a maximal `Instance`
/// structure depending on the limits used to create the pool.
///
/// The pool maintains a free list for fast instance allocation.
///
/// The userfault handler relies on how instances are stored in the mapping,
/// so make sure the uffd implementation is kept up-to-date.
#[derive(Debug)]
struct InstancePool {
    mapping: Mmap,
    offsets: VMOffsets,
    instance_size: usize,
    max_instances: usize,
    free_list: Mutex<Vec<usize>>,
    memories: MemoryPool,
    tables: TablePool,
}

impl InstancePool {
    fn new(module_limits: &ModuleLimits, instance_limits: &InstanceLimits) -> Result<Self, String> {
        let page_size = region::page::size();

        // Calculate the maximum size of an Instance structure given the limits
        let offsets = VMOffsets {
            pointer_size: std::mem::size_of::<*const u8>() as u8,
            num_signature_ids: module_limits.types,
            num_imported_functions: module_limits.imported_functions,
            num_imported_tables: module_limits.imported_tables,
            num_imported_memories: module_limits.imported_memories,
            num_imported_globals: module_limits.imported_globals,
            num_defined_functions: module_limits.functions,
            num_defined_tables: module_limits.tables,
            num_defined_memories: module_limits.memories,
            num_defined_globals: module_limits.globals,
        };

        let instance_size = round_up_to_pow2(
            mem::size_of::<Instance>()
                .checked_add(offsets.size_of_vmctx() as usize)
                .ok_or_else(|| "instance size exceeds addressable memory".to_string())?,
            page_size,
        );

        let max_instances = instance_limits.count as usize;

        let allocation_size = instance_size
            .checked_mul(max_instances)
            .ok_or_else(|| "total size of instance data exceeds addressable memory".to_string())?;

        let pool = Self {
            mapping: create_memory_map(allocation_size, allocation_size)?,
            offsets,
            instance_size,
            max_instances,
            free_list: Mutex::new((0..max_instances).collect()),
            memories: MemoryPool::new(module_limits, instance_limits)?,
            tables: TablePool::new(module_limits, instance_limits)?,
        };

        // Use a default module to initialize the instances to start
        let module = Arc::new(Module::default());
        for i in 0..instance_limits.count as usize {
            pool.initialize(i, &module);
        }

        Ok(pool)
    }

    fn initialize(&self, index: usize, module: &Arc<Module>) {
        debug_assert!(index < self.max_instances);

        unsafe {
            let instance_ptr = self.mapping.as_mut_ptr().add(index * self.instance_size);

            // Write a default instance with preallocated memory/table map storage to the ptr
            std::ptr::write(
                instance_ptr as _,
                Instance {
                    module: module.clone(),
                    offsets: self.offsets,
                    memories: PrimaryMap::with_capacity(self.offsets.num_defined_memories as usize),
                    tables: PrimaryMap::with_capacity(self.offsets.num_defined_tables as usize),
                    dropped_elements: RefCell::new(EntitySet::new()),
                    dropped_data: RefCell::new(EntitySet::new()),
                    host_state: Box::new(()),
                    #[cfg(all(feature = "uffd", target_os = "linux"))]
                    guard_page_faults: RefCell::new(Vec::new()),
                    vmctx: VMContext {},
                },
            );
        }
    }

    fn allocate(
        &self,
        strategy: PoolingAllocationStrategy,
        req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError> {
        let index = {
            let mut free_list = self.free_list.lock().unwrap();
            if free_list.is_empty() {
                return Err(InstantiationError::Limit(self.max_instances as u32));
            }
            let free_index = strategy.next(free_list.len());
            free_list.swap_remove(free_index)
        };

        unsafe {
            debug_assert!(index < self.max_instances);
            let instance =
                &mut *(self.mapping.as_mut_ptr().add(index * self.instance_size) as *mut Instance);

            instance.module = req.module;
            instance.offsets = VMOffsets::new(
                std::mem::size_of::<*const u8>() as u8,
                instance.module.as_ref(),
            );
            instance.host_state = req.host_state;

            Self::set_instance_memories(
                instance,
                self.memories.get(index),
                self.memories.max_wasm_pages,
            )?;
            Self::set_instance_tables(instance, self.tables.get(index), self.tables.max_elements)?;

            initialize_vmcontext(
                instance,
                req.imports.functions,
                req.imports.tables,
                req.imports.memories,
                req.imports.globals,
                req.finished_functions,
                req.lookup_shared_signature,
                req.interrupts,
                req.externref_activations_table,
                req.stack_map_registry,
                &|index| instance.memories[index].vmmemory(),
                &|index| instance.tables[index].vmtable(),
            );

            Ok(InstanceHandle::new(instance as _))
        }
    }

    fn deallocate(&self, handle: &InstanceHandle) {
        let addr = handle.instance as usize;
        let base = self.mapping.as_ptr() as usize;

        debug_assert!(addr >= base && addr < base + self.mapping.len());
        debug_assert!((addr - base) % self.instance_size == 0);

        let index = (addr - base) / self.instance_size;
        debug_assert!(index < self.max_instances);

        unsafe {
            // Decommit any linear memories that were used
            for (mem, base) in (*handle.instance)
                .memories
                .values()
                .zip(self.memories.get(index))
            {
                let size = (mem.size() * WASM_PAGE_SIZE) as usize;
                if size > 0 {
                    decommit(base, size);
                }
            }

            // Decommit any tables that were used
            let table_element_size = max_table_element_size();
            for (table, base) in (*handle.instance)
                .tables
                .values()
                .zip(self.tables.get(index))
            {
                let size = round_up_to_pow2(
                    table.size() as usize * table_element_size,
                    self.tables.page_size,
                );
                if size > 0 {
                    decommit(base, size);
                }
            }
        }

        {
            self.free_list.lock().unwrap().push(index);
        }
    }

    fn set_instance_memories(
        instance: &mut Instance,
        mut memories: BasePointerIterator,
        max_pages: u32,
    ) -> Result<(), InstantiationError> {
        let module = instance.module.as_ref();

        // Reset all guard pages before reusing the instance
        #[cfg(all(feature = "uffd", target_os = "linux"))]
        instance
            .reset_guard_pages()
            .map_err(InstantiationError::Resource)?;

        instance.memories.clear();

        for plan in
            (&module.memory_plans.values().as_slice()[module.num_imported_memories..]).iter()
        {
            instance.memories.push(
                Memory::new_static(plan, memories.next().unwrap(), max_pages, make_accessible)
                    .map_err(InstantiationError::Resource)?,
            );
        }

        let mut dropped_data = instance.dropped_data.borrow_mut();
        dropped_data.clear();
        dropped_data.resize(module.passive_data.len());

        Ok(())
    }

    fn set_instance_tables(
        instance: &mut Instance,
        mut tables: BasePointerIterator,
        max_elements: u32,
    ) -> Result<(), InstantiationError> {
        let module = instance.module.as_ref();

        instance.tables.clear();

        for plan in (&module.table_plans.values().as_slice()[module.num_imported_tables..]).iter() {
            let base = tables.next().unwrap();

            // Make the table data accessible
            if unsafe { !make_accessible(base, max_elements as usize * max_table_element_size()) } {
                return Err(InstantiationError::Resource(
                    "failed to make instance memory accessible".into(),
                ));
            }

            instance
                .tables
                .push(Table::new_static(plan, base, max_elements));
        }

        let mut dropped_elements = instance.dropped_elements.borrow_mut();
        dropped_elements.clear();
        dropped_elements.resize(module.passive_elements.len());

        Ok(())
    }
}

impl Drop for InstancePool {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.max_instances {
                let ptr = self.mapping.as_mut_ptr().add(i * self.instance_size) as *mut Instance;
                std::ptr::drop_in_place(ptr);
            }
        }
    }
}

/// Represents a pool of WebAssembly linear memories.
///
/// A linear memory is divided into accessible pages and guard pages.
///
/// Each instance index into the pool returns an iterator over the base addresses
/// of the instance's linear memories.
///
///
/// The userfault handler relies on how memories are stored in the mapping,
/// so make sure the uffd implementation is kept up-to-date.
#[derive(Debug)]
struct MemoryPool {
    mapping: Mmap,
    memory_size: usize,
    max_memories: usize,
    max_instances: usize,
    max_wasm_pages: u32,
}

impl MemoryPool {
    fn new(module_limits: &ModuleLimits, instance_limits: &InstanceLimits) -> Result<Self, String> {
        let memory_size = usize::try_from(instance_limits.address_space_size)
            .map_err(|_| "address space size exceeds addressable memory".to_string())?;

        debug_assert!(
            memory_size % region::page::size() == 0,
            "memory size {} is not a multiple of system page size",
            memory_size
        );

        let max_instances = instance_limits.count as usize;
        let max_memories = module_limits.memories as usize;

        let allocation_size = memory_size
            .checked_mul(max_memories)
            .and_then(|c| c.checked_mul(max_instances))
            .ok_or_else(|| {
                "total size of instance address space exceeds addressable memory".to_string()
            })?;

        Ok(Self {
            mapping: create_memory_map(0, allocation_size)?,
            memory_size,
            max_memories,
            max_instances,
            max_wasm_pages: module_limits.memory_pages,
        })
    }

    fn get(&self, instance_index: usize) -> BasePointerIterator {
        debug_assert!(instance_index < self.max_instances);

        let base = unsafe {
            self.mapping
                .as_mut_ptr()
                .add(instance_index * self.memory_size * self.max_memories) as _
        };

        BasePointerIterator::new(base, self.max_memories, self.memory_size)
    }
}

/// Represents a pool of WebAssembly tables.
///
/// Each instance index into the pool returns an iterator over the base addresses
/// of the instance's tables.
///
/// The userfault handler relies on how tables are stored in the mapping,
/// so make sure the uffd implementation is kept up-to-date.
#[derive(Debug)]
struct TablePool {
    mapping: Mmap,
    table_size: usize,
    max_tables: usize,
    max_instances: usize,
    page_size: usize,
    max_elements: u32,
}

impl TablePool {
    fn new(module_limits: &ModuleLimits, instance_limits: &InstanceLimits) -> Result<Self, String> {
        let page_size = region::page::size();

        let table_size = round_up_to_pow2(
            max_table_element_size()
                .checked_mul(module_limits.table_elements as usize)
                .ok_or_else(|| "table size exceeds addressable memory".to_string())?,
            page_size,
        );

        let max_instances = instance_limits.count as usize;
        let max_tables = module_limits.tables as usize;

        let allocation_size = table_size
            .checked_mul(max_tables)
            .and_then(|c| c.checked_mul(max_instances))
            .ok_or_else(|| {
                "total size of instance tables exceeds addressable memory".to_string()
            })?;

        Ok(Self {
            mapping: create_memory_map(0, allocation_size)?,
            table_size,
            max_tables,
            max_instances,
            page_size: region::page::size(),
            max_elements: module_limits.table_elements,
        })
    }

    fn get(&self, instance_index: usize) -> BasePointerIterator {
        debug_assert!(instance_index < self.max_instances);

        let base = unsafe {
            self.mapping
                .as_mut_ptr()
                .add(instance_index * self.table_size * self.max_tables) as _
        };

        BasePointerIterator::new(base, self.max_tables, self.table_size)
    }
}

/// Represents a pool of execution stacks (used for the async fiber implementation).
///
/// Each index into the pool represents a single execution stack. The maximum number of
/// stacks is the same as the maximum number of instances.
///
/// As stacks grow downwards, each stack starts (lowest address) with a guard page
/// that can be used to detect stack overflow.
///
/// The top of the stack (starting stack pointer) is returned when a stack is allocated
/// from the pool.
///
/// The userfault handler relies on how stacks are stored in the mapping,
/// so make sure the uffd implementation is kept up-to-date.
#[derive(Debug)]
struct StackPool {
    mapping: Mmap,
    stack_size: usize,
    max_instances: usize,
    page_size: usize,
    free_list: Mutex<Vec<usize>>,
    #[cfg(all(feature = "uffd", target_os = "linux"))]
    faulted_guard_pages: Arc<[AtomicBool]>,
}

impl StackPool {
    fn new(instance_limits: &InstanceLimits, stack_size: usize) -> Result<Self, String> {
        let page_size = region::page::size();

        // On Windows, don't allocate any fiber stacks as native fibers are always used
        // Add a page to the stack size for the guard page when using fiber stacks
        let stack_size = if cfg!(windows) || stack_size == 0 {
            0
        } else {
            round_up_to_pow2(stack_size, page_size)
                .checked_add(page_size)
                .ok_or_else(|| "stack size exceeds addressable memory".to_string())?
        };

        let max_instances = instance_limits.count as usize;

        let allocation_size = stack_size.checked_mul(max_instances).ok_or_else(|| {
            "total size of execution stacks exceeds addressable memory".to_string()
        })?;

        Ok(Self {
            mapping: create_memory_map(0, allocation_size)?,
            stack_size,
            max_instances,
            page_size,
            free_list: Mutex::new((0..max_instances).collect()),
            #[cfg(all(feature = "uffd", target_os = "linux"))]
            faulted_guard_pages: std::iter::repeat_with(|| false.into())
                .take(max_instances)
                .collect::<Vec<_>>()
                .into(),
        })
    }

    fn allocate(&self, strategy: PoolingAllocationStrategy) -> Result<*mut u8, FiberStackError> {
        // Stacks are not supported if nothing was allocated
        if self.stack_size == 0 {
            return Err(FiberStackError::NotSupported);
        }

        let index = {
            let mut free_list = self.free_list.lock().unwrap();
            if free_list.is_empty() {
                return Err(FiberStackError::Limit(self.max_instances as u32));
            }
            let free_index = strategy.next(free_list.len());
            free_list.swap_remove(free_index)
        };

        debug_assert!(index < self.max_instances);

        unsafe {
            // Remove the guard page from the size
            let size_without_guard = self.stack_size - self.page_size;

            let bottom_of_stack = self
                .mapping
                .as_mut_ptr()
                .add((index * self.stack_size) + self.page_size);

            cfg_if::cfg_if! {
                if #[cfg(all(feature = "uffd", target_os = "linux"))] {
                    // Check to see if a guard page needs to be reset
                    if self.faulted_guard_pages[index].swap(false, Ordering::SeqCst) {
                        if !reset_guard_page(bottom_of_stack.sub(self.page_size), self.page_size) {
                            return Err(FiberStackError::Resource(
                                "failed to reset stack guard page".into(),
                            ));
                        }
                    }

                } else {
                    // Make the stack accessible (excluding the guard page)
                    if !make_accessible(bottom_of_stack, size_without_guard) {
                        return Err(FiberStackError::Resource(
                            "failed to make instance memory accessible".into(),
                        ));
                    }
                }
            }

            // The top of the stack should be returned
            Ok(bottom_of_stack.add(size_without_guard))
        }
    }

    fn deallocate(&self, top_of_stack: *mut u8) {
        debug_assert!(!top_of_stack.is_null());

        unsafe {
            // Remove the guard page from the size
            let stack_size = self.stack_size - self.page_size;
            let bottom_of_stack = top_of_stack.sub(stack_size);

            let base = self.mapping.as_ptr() as usize;
            let start_of_stack = (bottom_of_stack as usize) - self.page_size;

            debug_assert!(start_of_stack >= base && start_of_stack < (base + self.mapping.len()));
            debug_assert!((start_of_stack - base) % self.stack_size == 0);

            let index = (start_of_stack - base) / self.stack_size;
            debug_assert!(index < self.max_instances);

            decommit(bottom_of_stack, stack_size);

            {
                self.free_list.lock().unwrap().push(index);
            }
        }
    }
}

/// Implements the pooling instance allocator.
///
/// This allocator interinally maintains pools of instances, memories, tables, and stacks.
///
/// Note: the resource pools are manually dropped so that the fault handler terminates correctly.
#[derive(Debug)]
pub struct PoolingInstanceAllocator {
    strategy: PoolingAllocationStrategy,
    module_limits: ModuleLimits,
    instance_limits: InstanceLimits,
    instances: mem::ManuallyDrop<InstancePool>,
    stacks: mem::ManuallyDrop<StackPool>,
    #[cfg(all(feature = "uffd", target_os = "linux"))]
    _fault_handler: PageFaultHandler,
}

impl PoolingInstanceAllocator {
    /// Creates a new pooling instance allocator with the given strategy and limits.
    pub fn new(
        strategy: PoolingAllocationStrategy,
        module_limits: ModuleLimits,
        mut instance_limits: InstanceLimits,
        stack_size: usize,
    ) -> Result<Self, String> {
        if instance_limits.count == 0 {
            return Err("the instance count limit cannot be zero".into());
        }

        // Round the instance address space size to the nearest Wasm page size
        instance_limits.address_space_size = u64::try_from(round_up_to_pow2(
            usize::try_from(instance_limits.address_space_size).unwrap(),
            WASM_PAGE_SIZE as usize,
        ))
        .unwrap();

        // Cap the instance address space size to 8 GiB (maximum 4 GiB address space + 4 GiB of guard region)
        instance_limits.address_space_size = min(instance_limits.address_space_size, 0x200000000);

        // The maximum module memory page count cannot exceed 65536 pages
        if module_limits.memory_pages > 0x10000 {
            return Err(format!(
                "module memory page limit of {} exceeds the maximum of 65536",
                module_limits.memory_pages
            ));
        }

        // The maximum module memory page count cannot exceed the instance address space size
        if (module_limits.memory_pages * WASM_PAGE_SIZE) as u64 > instance_limits.address_space_size
        {
            return Err(format!(
                "module memory page limit of {} pages exeeds the instance address space size limit of {} bytes",
                module_limits.memory_pages,
                instance_limits.address_space_size
            ));
        }

        let instances = InstancePool::new(&module_limits, &instance_limits)?;
        let stacks = StackPool::new(&instance_limits, stack_size)?;

        #[cfg(all(feature = "uffd", target_os = "linux"))]
        let _fault_handler = PageFaultHandler::new(&instances, &stacks)?;

        Ok(Self {
            strategy,
            module_limits,
            instance_limits,
            instances: mem::ManuallyDrop::new(instances),
            stacks: mem::ManuallyDrop::new(stacks),
            #[cfg(all(feature = "uffd", target_os = "linux"))]
            _fault_handler,
        })
    }
}

impl Drop for PoolingInstanceAllocator {
    fn drop(&mut self) {
        // Manually drop the pools before the fault handler (if uffd is enabled)
        // This ensures that any fault handler thread monitoring the pool memory terminates
        unsafe {
            mem::ManuallyDrop::drop(&mut self.instances);
            mem::ManuallyDrop::drop(&mut self.stacks);
        }
    }
}

unsafe impl InstanceAllocator for PoolingInstanceAllocator {
    fn validate_module(&self, translation: &ModuleTranslation) -> Result<(), String> {
        self.module_limits.validate_module(&translation.module)
    }

    fn adjust_tunables(&self, tunables: &mut Tunables) {
        let address_space_size = self.instance_limits.address_space_size;

        // For address spaces larger than 4 GiB, use a guard region to elide
        if address_space_size >= 0x100000000 {
            tunables.static_memory_bound = 0x10000; // in Wasm pages
            tunables.static_memory_offset_guard_size = address_space_size - 0x100000000;
        } else {
            tunables.static_memory_bound =
                u32::try_from(address_space_size).unwrap() / WASM_PAGE_SIZE;
            tunables.static_memory_offset_guard_size = 0;
        }

        // Treat the static memory bound as the maximum for unbounded Wasm memories
        // Because we guarantee a module cannot compile unless it fits in the limits of
        // the pool allocator, this ensures all memories are treated as static (i.e. immovable).
        tunables.static_memory_bound_is_maximum = true;
    }

    unsafe fn allocate(
        &self,
        req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError> {
        self.instances.allocate(self.strategy, req)
    }

    unsafe fn initialize(
        &self,
        handle: &InstanceHandle,
        is_bulk_memory: bool,
    ) -> Result<(), InstantiationError> {
        let instance = handle.instance();

        cfg_if::cfg_if! {
            if #[cfg(all(feature = "uffd", target_os = "linux"))] {
                match instance.module.memory_initialization {
                    Some(MemoryInitialization::Paged{ .. }) => {
                        if !is_bulk_memory {
                            check_init_bounds(instance)?;
                        }

                        // Initialize the tables
                        initialize_tables(instance)?;

                        // Don't initialize the memory; the fault handler will fill the pages when accessed
                        Ok(())
                    },
                    _ => initialize_instance(instance, is_bulk_memory)
                }
            } else {
                initialize_instance(instance, is_bulk_memory)
            }
        }
    }

    unsafe fn deallocate(&self, handle: &InstanceHandle) {
        self.instances.deallocate(handle);
    }

    fn allocate_fiber_stack(&self) -> Result<*mut u8, FiberStackError> {
        self.stacks.allocate(self.strategy)
    }

    unsafe fn deallocate_fiber_stack(&self, stack: *mut u8) {
        self.stacks.deallocate(stack);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Imports, VMSharedSignatureIndex};
    use wasmtime_environ::{
        entity::EntityRef,
        ir::Type,
        wasm::{Global, GlobalInit, Memory, SignatureIndex, Table, TableElementType, WasmType},
        MemoryPlan, ModuleType, TablePlan, TableStyle,
    };

    #[test]
    fn test_module_imported_functions_limit() {
        let limits = ModuleLimits {
            imported_functions: 0,
            ..Default::default()
        };

        let mut module = Module::default();

        module.functions.push(SignatureIndex::new(0));
        assert_eq!(limits.validate_module(&module), Ok(()));

        module.num_imported_funcs = 1;
        assert_eq!(
            limits.validate_module(&module),
            Err("imported function count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_imported_tables_limit() {
        let limits = ModuleLimits {
            imported_tables: 0,
            ..Default::default()
        };

        let mut module = Module::default();

        module.table_plans.push(TablePlan {
            style: TableStyle::CallerChecksSignature,
            table: Table {
                wasm_ty: WasmType::FuncRef,
                ty: TableElementType::Func,
                minimum: 0,
                maximum: None,
            },
        });

        assert_eq!(limits.validate_module(&module), Ok(()));

        module.num_imported_tables = 1;
        assert_eq!(
            limits.validate_module(&module),
            Err("imported tables count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_imported_memories_limit() {
        let limits = ModuleLimits {
            imported_memories: 0,
            ..Default::default()
        };

        let mut module = Module::default();

        module.memory_plans.push(MemoryPlan {
            style: MemoryStyle::Static { bound: 0 },
            memory: Memory {
                minimum: 0,
                maximum: None,
                shared: false,
            },
            offset_guard_size: 0,
        });

        assert_eq!(limits.validate_module(&module), Ok(()));

        module.num_imported_memories = 1;
        assert_eq!(
            limits.validate_module(&module),
            Err("imported memories count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_imported_globals_limit() {
        let limits = ModuleLimits {
            imported_globals: 0,
            ..Default::default()
        };

        let mut module = Module::default();

        module.globals.push(Global {
            wasm_ty: WasmType::I32,
            ty: Type::int(32).unwrap(),
            mutability: false,
            initializer: GlobalInit::I32Const(0),
        });

        assert_eq!(limits.validate_module(&module), Ok(()));

        module.num_imported_globals = 1;
        assert_eq!(
            limits.validate_module(&module),
            Err("imported globals count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_defined_types_limit() {
        let limits = ModuleLimits {
            types: 0,
            ..Default::default()
        };

        let mut module = Module::default();
        assert_eq!(limits.validate_module(&module), Ok(()));

        module
            .types
            .push(ModuleType::Function(SignatureIndex::new(0)));
        assert_eq!(
            limits.validate_module(&module),
            Err("defined types count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_defined_functions_limit() {
        let limits = ModuleLimits {
            functions: 0,
            ..Default::default()
        };

        let mut module = Module::default();
        assert_eq!(limits.validate_module(&module), Ok(()));

        module.functions.push(SignatureIndex::new(0));
        assert_eq!(
            limits.validate_module(&module),
            Err("defined functions count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_defined_tables_limit() {
        let limits = ModuleLimits {
            tables: 0,
            ..Default::default()
        };

        let mut module = Module::default();
        assert_eq!(limits.validate_module(&module), Ok(()));

        module.table_plans.push(TablePlan {
            style: TableStyle::CallerChecksSignature,
            table: Table {
                wasm_ty: WasmType::FuncRef,
                ty: TableElementType::Func,
                minimum: 0,
                maximum: None,
            },
        });
        assert_eq!(
            limits.validate_module(&module),
            Err("defined tables count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_defined_memories_limit() {
        let limits = ModuleLimits {
            memories: 0,
            ..Default::default()
        };

        let mut module = Module::default();
        assert_eq!(limits.validate_module(&module), Ok(()));

        module.memory_plans.push(MemoryPlan {
            style: MemoryStyle::Static { bound: 0 },
            memory: Memory {
                minimum: 0,
                maximum: None,
                shared: false,
            },
            offset_guard_size: 0,
        });
        assert_eq!(
            limits.validate_module(&module),
            Err("defined memories count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_defined_globals_limit() {
        let limits = ModuleLimits {
            globals: 0,
            ..Default::default()
        };

        let mut module = Module::default();
        assert_eq!(limits.validate_module(&module), Ok(()));

        module.globals.push(Global {
            wasm_ty: WasmType::I32,
            ty: Type::int(32).unwrap(),
            mutability: false,
            initializer: GlobalInit::I32Const(0),
        });
        assert_eq!(
            limits.validate_module(&module),
            Err("defined globals count of 1 exceeds the limit of 0".into())
        );
    }

    #[test]
    fn test_module_table_minimum_elements_limit() {
        let limits = ModuleLimits {
            tables: 1,
            table_elements: 10,
            ..Default::default()
        };

        let mut module = Module::default();
        module.table_plans.push(TablePlan {
            style: TableStyle::CallerChecksSignature,
            table: Table {
                wasm_ty: WasmType::FuncRef,
                ty: TableElementType::Func,
                minimum: 11,
                maximum: None,
            },
        });
        assert_eq!(
            limits.validate_module(&module),
            Err(
                "table index 0 has a minimum element size of 11 which exceeds the limit of 10"
                    .into()
            )
        );
    }

    #[test]
    fn test_module_memory_minimum_size_limit() {
        let limits = ModuleLimits {
            memories: 1,
            memory_pages: 5,
            ..Default::default()
        };

        let mut module = Module::default();
        module.memory_plans.push(MemoryPlan {
            style: MemoryStyle::Static { bound: 0 },
            memory: Memory {
                minimum: 6,
                maximum: None,
                shared: false,
            },
            offset_guard_size: 0,
        });
        assert_eq!(
            limits.validate_module(&module),
            Err("memory index 0 has a minimum page size of 6 which exceeds the limit of 5".into())
        );
    }

    #[test]
    fn test_module_with_dynamic_memory_style() {
        let limits = ModuleLimits {
            memories: 1,
            memory_pages: 5,
            ..Default::default()
        };

        let mut module = Module::default();
        module.memory_plans.push(MemoryPlan {
            style: MemoryStyle::Dynamic,
            memory: Memory {
                minimum: 1,
                maximum: None,
                shared: false,
            },
            offset_guard_size: 0,
        });
        assert_eq!(
            limits.validate_module(&module),
            Err("memory index 0 has an unsupported dynamic memory plan style".into())
        );
    }

    #[test]
    fn test_next_available_allocation_strategy() {
        let strat = PoolingAllocationStrategy::NextAvailable;
        assert_eq!(strat.next(10), 9);
        assert_eq!(strat.next(5), 4);
        assert_eq!(strat.next(1), 0);
    }

    #[test]
    fn test_random_allocation_strategy() {
        let strat = PoolingAllocationStrategy::Random;
        assert!(strat.next(100) < 100);
        assert_eq!(strat.next(1), 0);
    }

    #[test]
    fn test_base_pointer_iterator() {
        let mut iter = BasePointerIterator::new(std::ptr::null_mut(), 5, 3);

        assert_eq!(iter.next(), Some(0usize as _));
        assert_eq!(iter.next(), Some(3usize as _));
        assert_eq!(iter.next(), Some(6usize as _));
        assert_eq!(iter.next(), Some(9usize as _));
        assert_eq!(iter.next(), Some(12usize as _));
        assert_eq!(iter.next(), None);

        let mut iter = BasePointerIterator::new(std::ptr::null_mut(), 0, 10);
        assert_eq!(iter.next(), None);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_instance_pool() -> Result<(), String> {
        let module_limits = ModuleLimits {
            imported_functions: 0,
            imported_tables: 0,
            imported_memories: 0,
            imported_globals: 0,
            types: 0,
            functions: 0,
            tables: 1,
            memories: 1,
            globals: 0,
            table_elements: 10,
            memory_pages: 10,
        };
        let instance_limits = InstanceLimits {
            count: 3,
            address_space_size: 4096,
        };

        let instances = InstancePool::new(&module_limits, &instance_limits)?;

        assert_eq!(
            instances.offsets.pointer_size,
            std::mem::size_of::<*const u8>() as u8
        );
        assert_eq!(instances.offsets.num_signature_ids, 0);
        assert_eq!(instances.offsets.num_imported_functions, 0);
        assert_eq!(instances.offsets.num_imported_tables, 0);
        assert_eq!(instances.offsets.num_imported_memories, 0);
        assert_eq!(instances.offsets.num_imported_globals, 0);
        assert_eq!(instances.offsets.num_defined_functions, 0);
        assert_eq!(instances.offsets.num_defined_tables, 1);
        assert_eq!(instances.offsets.num_defined_memories, 1);
        assert_eq!(instances.offsets.num_defined_globals, 0);
        assert_eq!(instances.instance_size, 4096);
        assert_eq!(instances.max_instances, 3);

        assert_eq!(
            &*instances
                .free_list
                .lock()
                .map_err(|_| "failed to lock".to_string())?,
            &[0, 1, 2],
        );

        let mut handles = Vec::new();
        let module = Arc::new(Module::default());
        let finished_functions = &PrimaryMap::new();

        for _ in (0..3).rev() {
            handles.push(
                instances
                    .allocate(
                        PoolingAllocationStrategy::NextAvailable,
                        InstanceAllocationRequest {
                            module: module.clone(),
                            finished_functions,
                            imports: Imports {
                                functions: &[],
                                tables: &[],
                                memories: &[],
                                globals: &[],
                            },
                            lookup_shared_signature: &|_| VMSharedSignatureIndex::default(),
                            host_state: Box::new(()),
                            interrupts: std::ptr::null(),
                            externref_activations_table: std::ptr::null_mut(),
                            stack_map_registry: std::ptr::null_mut(),
                        },
                    )
                    .expect("allocation should succeed"),
            );
        }

        assert_eq!(
            &*instances
                .free_list
                .lock()
                .map_err(|_| "failed to lock".to_string())?,
            &[],
        );

        match instances.allocate(
            PoolingAllocationStrategy::NextAvailable,
            InstanceAllocationRequest {
                module: module.clone(),
                finished_functions,
                imports: Imports {
                    functions: &[],
                    tables: &[],
                    memories: &[],
                    globals: &[],
                },
                lookup_shared_signature: &|_| VMSharedSignatureIndex::default(),
                host_state: Box::new(()),
                interrupts: std::ptr::null(),
                externref_activations_table: std::ptr::null_mut(),
                stack_map_registry: std::ptr::null_mut(),
            },
        ) {
            Err(InstantiationError::Limit(3)) => {}
            _ => panic!("unexpected error"),
        };

        for handle in handles.drain(..) {
            instances.deallocate(&handle);
        }

        assert_eq!(
            &*instances
                .free_list
                .lock()
                .map_err(|_| "failed to lock".to_string())?,
            &[2, 1, 0],
        );

        Ok(())
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_memory_pool() -> Result<(), String> {
        let pool = MemoryPool::new(
            &ModuleLimits {
                imported_functions: 0,
                imported_tables: 0,
                imported_memories: 0,
                imported_globals: 0,
                types: 0,
                functions: 0,
                tables: 0,
                memories: 3,
                globals: 0,
                table_elements: 0,
                memory_pages: 10,
            },
            &InstanceLimits {
                count: 5,
                address_space_size: WASM_PAGE_SIZE as u64,
            },
        )?;

        assert_eq!(pool.memory_size, WASM_PAGE_SIZE as usize);
        assert_eq!(pool.max_memories, 3);
        assert_eq!(pool.max_instances, 5);
        assert_eq!(pool.max_wasm_pages, 10);

        let base = pool.mapping.as_ptr() as usize;

        for i in 0..5 {
            let mut iter = pool.get(i);

            for j in 0..3 {
                assert_eq!(
                    iter.next().unwrap() as usize - base,
                    ((i * 3) + j) * pool.memory_size
                );
            }

            assert_eq!(iter.next(), None);
        }

        Ok(())
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_table_pool() -> Result<(), String> {
        let pool = TablePool::new(
            &ModuleLimits {
                imported_functions: 0,
                imported_tables: 0,
                imported_memories: 0,
                imported_globals: 0,
                types: 0,
                functions: 0,
                tables: 4,
                memories: 0,
                globals: 0,
                table_elements: 100,
                memory_pages: 0,
            },
            &InstanceLimits {
                count: 7,
                address_space_size: WASM_PAGE_SIZE as u64,
            },
        )?;

        assert_eq!(pool.table_size, 4096);
        assert_eq!(pool.max_tables, 4);
        assert_eq!(pool.max_instances, 7);
        assert_eq!(pool.page_size, 4096);
        assert_eq!(pool.max_elements, 100);

        let base = pool.mapping.as_ptr() as usize;

        for i in 0..7 {
            let mut iter = pool.get(i);

            for j in 0..4 {
                assert_eq!(
                    iter.next().unwrap() as usize - base,
                    ((i * 4) + j) * pool.table_size
                );
            }

            assert_eq!(iter.next(), None);
        }

        Ok(())
    }

    #[cfg(all(unix, target_pointer_width = "64"))]
    #[test]
    fn test_stack_pool() -> Result<(), String> {
        let pool = StackPool::new(
            &InstanceLimits {
                count: 10,
                address_space_size: 0,
            },
            1,
        )?;

        assert_eq!(pool.stack_size, 8192);
        assert_eq!(pool.max_instances, 10);
        assert_eq!(pool.page_size, 4096);

        assert_eq!(
            &*pool
                .free_list
                .lock()
                .map_err(|_| "failed to lock".to_string())?,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        );

        let base = pool.mapping.as_ptr() as usize;

        let mut stacks = Vec::new();
        for i in (0..10).rev() {
            let stack = pool
                .allocate(PoolingAllocationStrategy::NextAvailable)
                .expect("allocation should succeed");
            assert_eq!(((stack as usize - base) / pool.stack_size) - 1, i);
            stacks.push(stack);
        }

        assert_eq!(
            &*pool
                .free_list
                .lock()
                .map_err(|_| "failed to lock".to_string())?,
            &[],
        );

        match pool
            .allocate(PoolingAllocationStrategy::NextAvailable)
            .unwrap_err()
        {
            FiberStackError::Limit(10) => {}
            _ => panic!("unexpected error"),
        };

        for stack in stacks {
            pool.deallocate(stack);
        }

        assert_eq!(
            &*pool
                .free_list
                .lock()
                .map_err(|_| "failed to lock".to_string())?,
            &[9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        );

        Ok(())
    }

    #[test]
    fn test_pooling_allocator_with_zero_instance_count() {
        assert_eq!(
            PoolingInstanceAllocator::new(
                PoolingAllocationStrategy::Random,
                ModuleLimits::default(),
                InstanceLimits {
                    count: 0,
                    ..Default::default()
                },
                4096
            )
            .expect_err("expected a failure constructing instance allocator"),
            "the instance count limit cannot be zero"
        );
    }

    #[test]
    fn test_pooling_allocator_with_memory_pages_exeeded() {
        assert_eq!(
            PoolingInstanceAllocator::new(
                PoolingAllocationStrategy::Random,
                ModuleLimits {
                    memory_pages: 0x10001,
                    ..Default::default()
                },
                InstanceLimits {
                    count: 1,
                    address_space_size: 1,
                },
                4096
            )
            .expect_err("expected a failure constructing instance allocator"),
            "module memory page limit of 65537 exceeds the maximum of 65536"
        );
    }

    #[test]
    fn test_pooling_allocator_with_address_space_exeeded() {
        assert_eq!(
            PoolingInstanceAllocator::new(
                PoolingAllocationStrategy::Random,
                ModuleLimits {
                    memory_pages: 2,
                    ..Default::default()
                },
                InstanceLimits {
                    count: 1,
                    address_space_size: 1,
                },
                4096,
            )
            .expect_err("expected a failure constructing instance allocator"),
            "module memory page limit of 2 pages exeeds the instance address space size limit of 65536 bytes"
        );
    }

    #[cfg_attr(target_arch = "aarch64", ignore)] // https://github.com/bytecodealliance/wasmtime/pull/2518#issuecomment-747280133
    #[cfg(all(unix, target_pointer_width = "64"))]
    #[test]
    fn test_stack_zeroed() -> Result<(), String> {
        let allocator = PoolingInstanceAllocator::new(
            PoolingAllocationStrategy::NextAvailable,
            ModuleLimits {
                imported_functions: 0,
                types: 0,
                functions: 0,
                tables: 0,
                memories: 0,
                globals: 0,
                table_elements: 0,
                memory_pages: 0,
                ..Default::default()
            },
            InstanceLimits {
                count: 1,
                address_space_size: 1,
            },
            4096,
        )?;

        unsafe {
            for _ in 0..10 {
                let stack = allocator
                    .allocate_fiber_stack()
                    .map_err(|e| format!("failed to allocate stack: {}", e))?;

                // The stack pointer is at the top, so decerement it first
                let addr = stack.sub(1);

                assert_eq!(*addr, 0);
                *addr = 1;

                allocator.deallocate_fiber_stack(stack);
            }
        }

        Ok(())
    }
}

//! Implements the pooling instance allocator.
//!
//! The pooling instance allocator maps memory in advance
//! and allocates instances, memories, tables, and stacks from
//! a pool of available resources.
//!
//! Using the pooling instance allocator can speed up module instantiation
//! when modules can be constrained based on configurable limits.

use super::{
    initialize_instance, initialize_vmcontext, InstanceAllocationRequest, InstanceAllocator,
    InstanceHandle, InstantiationError,
};
use crate::MemFdSlot;
use crate::{instance::Instance, Memory, Mmap, ModuleMemFds, Table};
use anyhow::{anyhow, bail, Context, Result};
use libc::c_void;
use std::convert::TryFrom;
use std::mem;
use std::sync::Arc;
use std::sync::Mutex;
use wasmtime_environ::{
    HostPtr, MemoryIndex, MemoryStyle, Module, PrimaryMap, Tunables, VMOffsets, VMOffsetsFields,
    WASM_PAGE_SIZE,
};

mod index_allocator;
use index_allocator::{PoolingAllocationState, SlotId};

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod windows;
        use windows as imp;
    } else if #[cfg(all(feature = "uffd", target_os = "linux"))] {
        mod uffd;
        use uffd as imp;
        use imp::initialize_memory_pool;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        use linux as imp;
    } else {
        mod unix;
        use unix as imp;
    }
}

use imp::{commit_memory_pages, commit_table_pages, decommit_memory_pages, decommit_table_pages};

#[cfg(all(feature = "async", unix))]
use imp::{commit_stack_pages, decommit_stack_pages};

#[cfg(feature = "async")]
use super::FiberStackError;

fn round_up_to_pow2(n: usize, to: usize) -> usize {
    debug_assert!(to > 0);
    debug_assert!(to.is_power_of_two());
    (n + to - 1) & !(to - 1)
}

/// Represents the limits placed on a module for compiling with the pooling instance allocator.
#[derive(Debug, Copy, Clone)]
pub struct ModuleLimits {
    /// The maximum number of imported functions for a module.
    pub imported_functions: u32,

    /// The maximum number of imported tables for a module.
    pub imported_tables: u32,

    /// The maximum number of imported linear memories for a module.
    pub imported_memories: u32,

    /// The maximum number of imported globals for a module.
    pub imported_globals: u32,

    /// The maximum number of defined types for a module.
    pub types: u32,

    /// The maximum number of defined functions for a module.
    pub functions: u32,

    /// The maximum number of defined tables for a module.
    pub tables: u32,

    /// The maximum number of defined linear memories for a module.
    pub memories: u32,

    /// The maximum number of defined globals for a module.
    pub globals: u32,

    /// The maximum table elements for any table defined in a module.
    pub table_elements: u32,

    /// The maximum number of pages for any linear memory defined in a module.
    pub memory_pages: u64,
}

impl ModuleLimits {
    fn validate(&self, module: &Module) -> Result<()> {
        if module.num_imported_funcs > self.imported_functions as usize {
            bail!(
                "imported function count of {} exceeds the limit of {}",
                module.num_imported_funcs,
                self.imported_functions
            );
        }

        if module.num_imported_tables > self.imported_tables as usize {
            bail!(
                "imported tables count of {} exceeds the limit of {}",
                module.num_imported_tables,
                self.imported_tables
            );
        }

        if module.num_imported_memories > self.imported_memories as usize {
            bail!(
                "imported memories count of {} exceeds the limit of {}",
                module.num_imported_memories,
                self.imported_memories
            );
        }

        if module.num_imported_globals > self.imported_globals as usize {
            bail!(
                "imported globals count of {} exceeds the limit of {}",
                module.num_imported_globals,
                self.imported_globals
            );
        }

        if module.types.len() > self.types as usize {
            bail!(
                "defined types count of {} exceeds the limit of {}",
                module.types.len(),
                self.types
            );
        }

        let functions = module.functions.len() - module.num_imported_funcs;
        if functions > self.functions as usize {
            bail!(
                "defined functions count of {} exceeds the limit of {}",
                functions,
                self.functions
            );
        }

        let tables = module.table_plans.len() - module.num_imported_tables;
        if tables > self.tables as usize {
            bail!(
                "defined tables count of {} exceeds the limit of {}",
                tables,
                self.tables
            );
        }

        let memories = module.memory_plans.len() - module.num_imported_memories;
        if memories > self.memories as usize {
            bail!(
                "defined memories count of {} exceeds the limit of {}",
                memories,
                self.memories
            );
        }

        let globals = module.globals.len() - module.num_imported_globals;
        if globals > self.globals as usize {
            bail!(
                "defined globals count of {} exceeds the limit of {}",
                globals,
                self.globals
            );
        }

        for (i, plan) in module.table_plans.values().as_slice()[module.num_imported_tables..]
            .iter()
            .enumerate()
        {
            if plan.table.minimum > self.table_elements {
                bail!(
                    "table index {} has a minimum element size of {} which exceeds the limit of {}",
                    i,
                    plan.table.minimum,
                    self.table_elements
                );
            }
        }

        for (i, plan) in module.memory_plans.values().as_slice()[module.num_imported_memories..]
            .iter()
            .enumerate()
        {
            if plan.memory.minimum > self.memory_pages {
                bail!(
                    "memory index {} has a minimum page size of {} which exceeds the limit of {}",
                    i,
                    plan.memory.minimum,
                    self.memory_pages
                );
            }

            if let MemoryStyle::Dynamic { .. } = plan.style {
                bail!(
                    "memory index {} has an unsupported dynamic memory plan style",
                    i,
                );
            }
        }

        Ok(())
    }
}

impl Default for ModuleLimits {
    fn default() -> Self {
        // See doc comments for `wasmtime::ModuleLimits` for these default values
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
    /// The maximum number of concurrent instances supported.
    pub count: u32,
}

impl Default for InstanceLimits {
    fn default() -> Self {
        // See doc comments for `wasmtime::InstanceLimits` for these default values
        Self { count: 1000 }
    }
}

/// The allocation strategy to use for the pooling instance allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolingAllocationStrategy {
    /// Allocate from the next available instance.
    NextAvailable,
    /// Allocate from a random available instance.
    Random,
    /// Try to allocate an instance slot that was previously used for
    /// the same module, potentially enabling faster instantiation by
    /// reusing e.g. memory mappings.
    ReuseAffinity,
}

impl Default for PoolingAllocationStrategy {
    fn default() -> Self {
        if cfg!(memfd) {
            Self::ReuseAffinity
        } else {
            Self::NextAvailable
        }
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
    instance_size: usize,
    max_instances: usize,
    index_allocator: Mutex<PoolingAllocationState>,
    memories: MemoryPool,
    tables: TablePool,
}

impl InstancePool {
    fn new(
        strategy: PoolingAllocationStrategy,
        module_limits: &ModuleLimits,
        instance_limits: &InstanceLimits,
        tunables: &Tunables,
    ) -> Result<Self> {
        let page_size = region::page::size();

        // Calculate the maximum size of an Instance structure given the limits
        let offsets = VMOffsets::from(VMOffsetsFields {
            ptr: HostPtr,
            num_signature_ids: module_limits.types,
            num_imported_functions: module_limits.imported_functions,
            num_imported_tables: module_limits.imported_tables,
            num_imported_memories: module_limits.imported_memories,
            num_imported_globals: module_limits.imported_globals,
            num_defined_functions: module_limits.functions,
            num_defined_tables: module_limits.tables,
            num_defined_memories: module_limits.memories,
            num_defined_globals: module_limits.globals,
        });

        let instance_size = round_up_to_pow2(
            mem::size_of::<Instance>()
                .checked_add(offsets.size_of_vmctx() as usize)
                .ok_or_else(|| anyhow!("instance size exceeds addressable memory"))?,
            page_size,
        );

        let max_instances = instance_limits.count as usize;

        let allocation_size = instance_size
            .checked_mul(max_instances)
            .ok_or_else(|| anyhow!("total size of instance data exceeds addressable memory"))?;

        let mapping = Mmap::accessible_reserved(allocation_size, allocation_size)
            .context("failed to create instance pool mapping")?;

        let pool = Self {
            mapping,
            instance_size,
            max_instances,
            index_allocator: Mutex::new(PoolingAllocationState::new(strategy, max_instances)),
            memories: MemoryPool::new(module_limits, instance_limits, tunables)?,
            tables: TablePool::new(module_limits, instance_limits)?,
        };

        Ok(pool)
    }

    unsafe fn instance(&self, index: usize) -> &mut Instance {
        debug_assert!(index < self.max_instances);
        &mut *(self.mapping.as_mut_ptr().add(index * self.instance_size) as *mut Instance)
    }

    unsafe fn setup_instance(
        &self,
        index: usize,
        mut req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError> {
        let host_state = std::mem::replace(&mut req.host_state, Box::new(()));
        let instance_data = Instance::create_raw(
            &req.module,
            req.unique_id,
            &*req.wasm_data,
            PrimaryMap::default(),
            PrimaryMap::default(),
            host_state,
        );

        // Instances are uninitialized memory at first; we need to
        // write an empty but initialized `Instance` struct into the
        // chosen slot before we do anything else with it. (This is
        // paired with a `drop_in_place` in deallocate below.)
        let instance = self.instance(index);

        std::ptr::write(instance as _, instance_data);

        // set_instance_memories and _tables will need the store before we can completely
        // initialize the vmcontext.
        if let Some(store) = req.store.as_raw() {
            instance.set_store(store);
        }

        Self::set_instance_memories(
            index,
            instance,
            &self.memories,
            req.memfds,
            self.memories.max_wasm_pages,
        )?;

        Self::set_instance_tables(
            instance,
            self.tables.get(index).map(|x| x as *mut usize),
            self.tables.max_elements,
        )?;

        initialize_vmcontext(instance, req);

        Ok(InstanceHandle {
            instance: instance as _,
        })
    }

    fn allocate(
        &self,
        req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError> {
        let index = {
            let mut alloc = self.index_allocator.lock().unwrap();
            if alloc.is_empty() {
                return Err(InstantiationError::Limit(self.max_instances as u32));
            }
            alloc.alloc(req.unique_id).index()
        };

        unsafe {
            self.setup_instance(index, req).or_else(|e| {
                // Deallocate the allocated instance on error
                let instance = self.instance(index);
                self.deallocate(&InstanceHandle {
                    instance: instance as _,
                });
                Err(e)
            })
        }
    }

    fn deallocate(&self, handle: &InstanceHandle) {
        let addr = handle.instance as usize;
        let base = self.mapping.as_ptr() as usize;

        debug_assert!(addr >= base && addr < base + self.mapping.len());
        debug_assert!((addr - base) % self.instance_size == 0);

        let index = (addr - base) / self.instance_size;
        debug_assert!(index < self.max_instances);

        let instance = unsafe { &mut *handle.instance };

        // Decommit any linear memories that were used
        for ((def_mem_idx, memory), base) in
            instance.memories.iter_mut().zip(self.memories.get(index))
        {
            let mut memory = mem::take(memory);
            debug_assert!(memory.is_static());

            match memory {
                Memory::Static {
                    memfd_slot: Some(mut memfd_slot),
                    ..
                } => {
                    let mem_idx = instance.module.memory_index(def_mem_idx);
                    // If there was any error clearing the memfd, just
                    // drop it here, and let the drop handler for the
                    // MemFdSlot unmap in a way that retains the
                    // address space reservation.
                    if memfd_slot.clear_and_remain_ready().is_ok() {
                        self.memories.return_memfd_slot(index, mem_idx, memfd_slot);
                    }
                }

                _ => {
                    // Reset any faulted guard pages as the physical
                    // memory may be reused for another instance in
                    // the future.
                    #[cfg(all(feature = "uffd", target_os = "linux"))]
                    memory
                        .reset_guard_pages()
                        .expect("failed to reset guard pages");
                    // require mutable on all platforms, not just uffd
                    drop(&mut memory);

                    let size = memory.byte_size();
                    drop(memory);
                    decommit_memory_pages(base, size)
                        .expect("failed to decommit linear memory pages");
                }
            }
        }

        instance.memories.clear();
        instance.dropped_data.clear();

        // Decommit any tables that were used
        for (table, base) in instance.tables.values_mut().zip(self.tables.get(index)) {
            let table = mem::take(table);
            debug_assert!(table.is_static());

            let size = round_up_to_pow2(
                table.size() as usize * mem::size_of::<*mut u8>(),
                self.tables.page_size,
            );

            drop(table);
            decommit_table_pages(base, size).expect("failed to decommit table pages");
        }

        // We've now done all of the pooling-allocator-specific
        // teardown, so we can drop the Instance and let destructors
        // take care of any other fields (host state, globals, etc.).
        unsafe {
            std::ptr::drop_in_place(instance as *mut _);
        }
        // The instance is now uninitialized memory and cannot be
        // touched again until we write a fresh Instance in-place with
        // std::ptr::write in allocate() above.

        self.index_allocator.lock().unwrap().free(SlotId(index));
    }

    fn set_instance_memories(
        instance_idx: usize,
        instance: &mut Instance,
        memories: &MemoryPool,
        maybe_memfds: Option<&Arc<ModuleMemFds>>,
        max_pages: u64,
    ) -> Result<(), InstantiationError> {
        let module = instance.module.as_ref();

        debug_assert!(instance.memories.is_empty());

        for (memory_index, plan) in module
            .memory_plans
            .iter()
            .skip(module.num_imported_memories)
        {
            let defined_index = module
                .defined_memory_index(memory_index)
                .expect("should be a defined memory since we skipped imported ones");

            let memory = unsafe {
                std::slice::from_raw_parts_mut(
                    memories.get_base(instance_idx, memory_index),
                    (max_pages as usize) * (WASM_PAGE_SIZE as usize),
                )
            };

            if let Some(memfds) = maybe_memfds {
                let image = memfds.get_memory_image(defined_index);
                let mut slot = memories.take_memfd_slot(instance_idx, memory_index);
                let initial_size = plan.memory.minimum * WASM_PAGE_SIZE as u64;

                // If instantiation fails, we can propagate the error
                // upward and drop the slot. This will cause the Drop
                // handler to attempt to map the range with PROT_NONE
                // memory, to reserve the space while releasing any
                // stale mappings. The next use of this slot will then
                // create a new MemFdSlot that will try to map over
                // this, returning errors as well if the mapping
                // errors persist. The unmap-on-drop is best effort;
                // if it fails, then we can still soundly continue
                // using the rest of the pool and allowing the rest of
                // the process to continue, because we never perform a
                // mmap that would leave an open space for someone
                // else to come in and map something.
                slot.instantiate(initial_size as usize, image)
                    .map_err(|e| InstantiationError::Resource(e.into()))?;

                instance.memories.push(
                    Memory::new_static(plan, memory, None, Some(slot), unsafe {
                        &mut *instance.store()
                    })
                    .map_err(InstantiationError::Resource)?,
                );
            } else {
                instance.memories.push(
                    Memory::new_static(plan, memory, Some(commit_memory_pages), None, unsafe {
                        &mut *instance.store()
                    })
                    .map_err(InstantiationError::Resource)?,
                );
            }
        }

        debug_assert!(instance.dropped_data.is_empty());

        Ok(())
    }

    fn set_instance_tables(
        instance: &mut Instance,
        mut tables: impl Iterator<Item = *mut usize>,
        max_elements: u32,
    ) -> Result<(), InstantiationError> {
        let module = instance.module.as_ref();

        debug_assert!(instance.tables.is_empty());

        for plan in (&module.table_plans.values().as_slice()[module.num_imported_tables..]).iter() {
            let base = tables.next().unwrap();

            commit_table_pages(
                base as *mut u8,
                max_elements as usize * mem::size_of::<*mut u8>(),
            )
            .map_err(InstantiationError::Resource)?;

            let table = unsafe { std::slice::from_raw_parts_mut(base, max_elements as usize) };
            instance.tables.push(
                Table::new_static(plan, table, unsafe { &mut *instance.store() })
                    .map_err(InstantiationError::Resource)?,
            );
        }

        debug_assert!(instance.dropped_elements.is_empty());
        instance
            .dropped_elements
            .resize(module.passive_elements.len());

        Ok(())
    }
}

/// Represents a pool of WebAssembly linear memories.
///
/// A linear memory is divided into accessible pages and guard pages.
///
/// Each instance index into the pool returns an iterator over the base addresses
/// of the instance's linear memories.
///
/// The userfault handler relies on how memories are stored in the mapping,
/// so make sure the uffd implementation is kept up-to-date.
#[derive(Debug)]
struct MemoryPool {
    mapping: Mmap,
    // If using the memfd allocation scheme, the MemFd slots. We
    // dynamically transfer ownership of a slot to a Memory when in
    // use.
    memfd_slots: Vec<Mutex<Option<MemFdSlot>>>,
    // The size, in bytes, of each linear memory's reservation plus the guard
    // region allocated for it.
    memory_size: usize,
    // The size, in bytes, of the offset to the first linear memory in this
    // pool. This is here to help account for the first region of guard pages,
    // if desired, before the first linear memory.
    initial_memory_offset: usize,
    max_memories: usize,
    max_instances: usize,
    max_wasm_pages: u64,
}

impl MemoryPool {
    fn new(
        module_limits: &ModuleLimits,
        instance_limits: &InstanceLimits,
        tunables: &Tunables,
    ) -> Result<Self> {
        // The maximum module memory page count cannot exceed 65536 pages
        if module_limits.memory_pages > 0x10000 {
            bail!(
                "module memory page limit of {} exceeds the maximum of 65536",
                module_limits.memory_pages
            );
        }

        // The maximum module memory page count cannot exceed the memory reservation size
        if module_limits.memory_pages > tunables.static_memory_bound {
            bail!(
                "module memory page limit of {} pages exceeds maximum static memory limit of {} pages",
                module_limits.memory_pages,
                tunables.static_memory_bound,
            );
        }

        let memory_size = if module_limits.memory_pages > 0 {
            usize::try_from(
                u64::from(tunables.static_memory_bound) * u64::from(WASM_PAGE_SIZE)
                    + tunables.static_memory_offset_guard_size,
            )
            .map_err(|_| anyhow!("memory reservation size exceeds addressable memory"))?
        } else {
            0
        };

        debug_assert!(
            memory_size % region::page::size() == 0,
            "memory size {} is not a multiple of system page size",
            memory_size
        );

        let max_instances = instance_limits.count as usize;
        let max_memories = module_limits.memories as usize;
        let initial_memory_offset = if tunables.guard_before_linear_memory {
            usize::try_from(tunables.static_memory_offset_guard_size).unwrap()
        } else {
            0
        };

        // The entire allocation here is the size of each memory times the
        // max memories per instance times the number of instances allowed in
        // this pool, plus guard regions.
        //
        // Note, though, that guard regions are required to be after each linear
        // memory. If the `guard_before_linear_memory` setting is specified,
        // then due to the contiguous layout of linear memories the guard pages
        // after one memory are also guard pages preceding the next linear
        // memory. This means that we only need to handle pre-guard-page sizes
        // specially for the first linear memory, hence the
        // `initial_memory_offset` variable here. If guards aren't specified
        // before linear memories this is set to `0`, otherwise it's set to
        // the same size as guard regions for other memories.
        let allocation_size = memory_size
            .checked_mul(max_memories)
            .and_then(|c| c.checked_mul(max_instances))
            .and_then(|c| c.checked_add(initial_memory_offset))
            .ok_or_else(|| {
                anyhow!("total size of memory reservation exceeds addressable memory")
            })?;

        // Create a completely inaccessible region to start
        let mapping = Mmap::accessible_reserved(0, allocation_size)
            .context("failed to create memory pool mapping")?;

        let num_memfd_slots = if cfg!(memfd) {
            max_instances * max_memories
        } else {
            0
        };
        let memfd_slots: Vec<_> = std::iter::repeat_with(|| Mutex::new(None))
            .take(num_memfd_slots)
            .collect();

        let pool = Self {
            mapping,
            memfd_slots,
            memory_size,
            initial_memory_offset,
            max_memories,
            max_instances,
            max_wasm_pages: module_limits.memory_pages,
        };

        // uffd support requires some special setup for the memory pool
        #[cfg(all(feature = "uffd", target_os = "linux"))]
        initialize_memory_pool(&pool)?;

        Ok(pool)
    }

    fn get_base(&self, instance_index: usize, memory_index: MemoryIndex) -> *mut u8 {
        debug_assert!(instance_index < self.max_instances);
        let memory_index = memory_index.as_u32() as usize;
        debug_assert!(memory_index < self.max_memories);
        let idx = instance_index * self.max_memories + memory_index;
        let offset = self.initial_memory_offset + idx * self.memory_size;
        unsafe { self.mapping.as_mut_ptr().offset(offset as isize) }
    }

    fn get<'a>(&'a self, instance_index: usize) -> impl Iterator<Item = *mut u8> + 'a {
        (0..self.max_memories)
            .map(move |i| self.get_base(instance_index, MemoryIndex::from_u32(i as u32)))
    }

    /// Take ownership of the given memfd slot. Must be returned via
    /// `return_memfd_slot` when the instance is done using it.
    fn take_memfd_slot(&self, instance_index: usize, memory_index: MemoryIndex) -> MemFdSlot {
        let idx = instance_index * self.max_memories + (memory_index.as_u32() as usize);
        let maybe_slot = self.memfd_slots[idx].lock().unwrap().take();

        maybe_slot.unwrap_or_else(|| {
            MemFdSlot::create(
                self.get_base(instance_index, memory_index) as *mut c_void,
                0,
                self.memory_size,
            )
        })
    }

    /// Return ownership of the given memfd slot.
    fn return_memfd_slot(&self, instance_index: usize, memory_index: MemoryIndex, slot: MemFdSlot) {
        assert!(!slot.is_dirty());
        let idx = instance_index * self.max_memories + (memory_index.as_u32() as usize);
        *self.memfd_slots[idx].lock().unwrap() = Some(slot);
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        // Clear the `clear_no_drop` flag (i.e., ask to *not* clear on
        // drop) for all MemFdSlots, and then drop them here. This is
        // valid because the one `Mmap` that covers the whole region
        // can just do its one munmap.
        for mut memfd in std::mem::take(&mut self.memfd_slots) {
            if let Some(memfd_slot) = memfd.get_mut().unwrap() {
                memfd_slot.no_clear_on_drop();
            }
        }
    }
}

/// Represents a pool of WebAssembly tables.
///
/// Each instance index into the pool returns an iterator over the base addresses
/// of the instance's tables.
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
    fn new(module_limits: &ModuleLimits, instance_limits: &InstanceLimits) -> Result<Self> {
        let page_size = region::page::size();

        let table_size = if module_limits.table_elements > 0 {
            round_up_to_pow2(
                mem::size_of::<*mut u8>()
                    .checked_mul(module_limits.table_elements as usize)
                    .ok_or_else(|| anyhow!("table size exceeds addressable memory"))?,
                page_size,
            )
        } else {
            0
        };

        let max_instances = instance_limits.count as usize;
        let max_tables = module_limits.tables as usize;

        let allocation_size = table_size
            .checked_mul(max_tables)
            .and_then(|c| c.checked_mul(max_instances))
            .ok_or_else(|| anyhow!("total size of instance tables exceeds addressable memory"))?;

        let mapping = Mmap::accessible_reserved(allocation_size, allocation_size)
            .context("failed to create table pool mapping")?;

        Ok(Self {
            mapping,
            table_size,
            max_tables,
            max_instances,
            page_size,
            max_elements: module_limits.table_elements,
        })
    }

    fn get(&self, instance_index: usize) -> impl Iterator<Item = *mut u8> {
        debug_assert!(instance_index < self.max_instances);

        let base: *mut u8 = unsafe {
            self.mapping
                .as_mut_ptr()
                .add(instance_index * self.table_size * self.max_tables) as _
        };

        let size = self.table_size;
        (0..self.max_tables).map(move |i| unsafe { base.add(i * size) })
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
#[cfg(all(feature = "async", unix))]
#[derive(Debug)]
struct StackPool {
    mapping: Mmap,
    stack_size: usize,
    max_instances: usize,
    page_size: usize,
    index_allocator: Mutex<PoolingAllocationState>,
}

#[cfg(all(feature = "async", unix))]
impl StackPool {
    fn new(instance_limits: &InstanceLimits, stack_size: usize) -> Result<Self> {
        let page_size = region::page::size();

        // Add a page to the stack size for the guard page when using fiber stacks
        let stack_size = if stack_size == 0 {
            0
        } else {
            round_up_to_pow2(stack_size, page_size)
                .checked_add(page_size)
                .ok_or_else(|| anyhow!("stack size exceeds addressable memory"))?
        };

        let max_instances = instance_limits.count as usize;

        let allocation_size = stack_size
            .checked_mul(max_instances)
            .ok_or_else(|| anyhow!("total size of execution stacks exceeds addressable memory"))?;

        let mapping = Mmap::accessible_reserved(allocation_size, allocation_size)
            .context("failed to create stack pool mapping")?;

        // Set up the stack guard pages
        if allocation_size > 0 {
            unsafe {
                for i in 0..max_instances {
                    // Make the stack guard page inaccessible
                    let bottom_of_stack = mapping.as_mut_ptr().add(i * stack_size);
                    region::protect(bottom_of_stack, page_size, region::Protection::NONE)
                        .context("failed to protect stack guard page")?;
                }
            }
        }

        Ok(Self {
            mapping,
            stack_size,
            max_instances,
            page_size,
            // We always use a `NextAvailable` strategy for stack
            // allocation. We don't want or need an affinity policy
            // here: stacks do not benefit from being allocated to the
            // same compiled module with the same image (they always
            // start zeroed just the same for everyone).
            index_allocator: Mutex::new(PoolingAllocationState::new(
                PoolingAllocationStrategy::NextAvailable,
                max_instances,
            )),
        })
    }

    fn allocate(&self) -> Result<wasmtime_fiber::FiberStack, FiberStackError> {
        if self.stack_size == 0 {
            return Err(FiberStackError::NotSupported);
        }

        let index = {
            let mut alloc = self.index_allocator.lock().unwrap();
            if alloc.is_empty() {
                return Err(FiberStackError::Limit(self.max_instances as u32));
            }
            alloc.alloc(None).index()
        };

        debug_assert!(index < self.max_instances);

        unsafe {
            // Remove the guard page from the size
            let size_without_guard = self.stack_size - self.page_size;

            let bottom_of_stack = self
                .mapping
                .as_mut_ptr()
                .add((index * self.stack_size) + self.page_size);

            commit_stack_pages(bottom_of_stack, size_without_guard)
                .map_err(FiberStackError::Resource)?;

            wasmtime_fiber::FiberStack::from_top_ptr(bottom_of_stack.add(size_without_guard))
                .map_err(|e| FiberStackError::Resource(e.into()))
        }
    }

    fn deallocate(&self, stack: &wasmtime_fiber::FiberStack) {
        let top = stack
            .top()
            .expect("fiber stack not allocated from the pool") as usize;

        let base = self.mapping.as_ptr() as usize;
        let len = self.mapping.len();
        assert!(
            top > base && top <= (base + len),
            "fiber stack top pointer not in range"
        );

        // Remove the guard page from the size
        let stack_size = self.stack_size - self.page_size;
        let bottom_of_stack = top - stack_size;
        let start_of_stack = bottom_of_stack - self.page_size;
        debug_assert!(start_of_stack >= base && start_of_stack < (base + len));
        debug_assert!((start_of_stack - base) % self.stack_size == 0);

        let index = (start_of_stack - base) / self.stack_size;
        debug_assert!(index < self.max_instances);

        decommit_stack_pages(bottom_of_stack as _, stack_size).unwrap();

        self.index_allocator.lock().unwrap().free(SlotId(index));
    }
}

/// Implements the pooling instance allocator.
///
/// This allocator internally maintains pools of instances, memories, tables, and stacks.
///
/// Note: the resource pools are manually dropped so that the fault handler terminates correctly.
#[derive(Debug)]
pub struct PoolingInstanceAllocator {
    module_limits: ModuleLimits,
    // This is manually drop so that the pools unmap their memory before the page fault handler drops.
    instances: mem::ManuallyDrop<InstancePool>,
    #[cfg(all(feature = "async", unix))]
    stacks: StackPool,
    #[cfg(all(feature = "async", windows))]
    stack_size: usize,
    #[cfg(all(feature = "uffd", target_os = "linux"))]
    _fault_handler: imp::PageFaultHandler,
}

impl PoolingInstanceAllocator {
    /// Creates a new pooling instance allocator with the given strategy and limits.
    pub fn new(
        strategy: PoolingAllocationStrategy,
        module_limits: ModuleLimits,
        instance_limits: InstanceLimits,
        stack_size: usize,
        tunables: &Tunables,
    ) -> Result<Self> {
        if instance_limits.count == 0 {
            bail!("the instance count limit cannot be zero");
        }

        let instances = InstancePool::new(strategy, &module_limits, &instance_limits, tunables)?;

        #[cfg(all(feature = "uffd", target_os = "linux"))]
        let _fault_handler = imp::PageFaultHandler::new(&instances)?;

        drop(stack_size); // suppress unused warnings w/o async feature

        Ok(Self {
            module_limits,
            instances: mem::ManuallyDrop::new(instances),
            #[cfg(all(feature = "async", unix))]
            stacks: StackPool::new(&instance_limits, stack_size)?,
            #[cfg(all(feature = "async", windows))]
            stack_size,
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
        }
    }
}

unsafe impl InstanceAllocator for PoolingInstanceAllocator {
    fn validate(&self, module: &Module) -> Result<()> {
        self.module_limits.validate(module)
    }

    fn adjust_tunables(&self, tunables: &mut Tunables) {
        // Treat the static memory bound as the maximum for unbounded Wasm memories
        // Because we guarantee a module cannot compile unless it fits in the limits of
        // the pool allocator, this ensures all memories are treated as static (i.e. immovable).
        tunables.static_memory_bound_is_maximum = true;
    }

    unsafe fn allocate(
        &self,
        req: InstanceAllocationRequest,
    ) -> Result<InstanceHandle, InstantiationError> {
        self.instances.allocate(req)
    }

    unsafe fn initialize(
        &self,
        handle: &mut InstanceHandle,
        module: &Module,
        is_bulk_memory: bool,
    ) -> Result<(), InstantiationError> {
        let instance = handle.instance_mut();

        cfg_if::cfg_if! {
            if #[cfg(all(feature = "uffd", target_os = "linux"))] {
                match &module.memory_initialization {
                    wasmtime_environ::MemoryInitialization::Paged { .. } => {
                        if !is_bulk_memory {
                            super::check_init_bounds(instance, module)?;
                        }

                        // Initialize the tables
                        super::initialize_tables(instance, module)?;

                        // Don't initialize the memory; the fault handler will back the pages when accessed

                        Ok(())
                    },
                    _ => initialize_instance(instance, module, is_bulk_memory)
                }
            } else {
                initialize_instance(instance, module, is_bulk_memory)
            }
        }
    }

    unsafe fn deallocate(&self, handle: &InstanceHandle) {
        self.instances.deallocate(handle);
    }

    #[cfg(all(feature = "async", unix))]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack, FiberStackError> {
        self.stacks.allocate()
    }

    #[cfg(all(feature = "async", unix))]
    unsafe fn deallocate_fiber_stack(&self, stack: &wasmtime_fiber::FiberStack) {
        self.stacks.deallocate(stack);
    }

    #[cfg(all(feature = "async", windows))]
    fn allocate_fiber_stack(&self) -> Result<wasmtime_fiber::FiberStack, FiberStackError> {
        if self.stack_size == 0 {
            return Err(FiberStackError::NotSupported);
        }

        // On windows, we don't use a stack pool as we use the native fiber implementation
        wasmtime_fiber::FiberStack::new(self.stack_size)
            .map_err(|e| FiberStackError::Resource(e.into()))
    }

    #[cfg(all(feature = "async", windows))]
    unsafe fn deallocate_fiber_stack(&self, _stack: &wasmtime_fiber::FiberStack) {
        // A no-op as we don't own the fiber stack on Windows
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Imports, StorePtr, VMSharedSignatureIndex};
    use wasmtime_environ::{
        EntityRef, Global, GlobalInit, Memory, MemoryPlan, ModuleType, SignatureIndex, Table,
        TablePlan, TableStyle, WasmType,
    };

    #[test]
    fn test_module_imported_functions_limit() {
        let limits = ModuleLimits {
            imported_functions: 0,
            ..Default::default()
        };

        let mut module = Module::default();

        module.functions.push(SignatureIndex::new(0));
        assert!(limits.validate(&module).is_ok());

        module.num_imported_funcs = 1;
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
                minimum: 0,
                maximum: None,
            },
        });

        assert!(limits.validate(&module).is_ok());

        module.num_imported_tables = 1;
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
                memory64: false,
            },
            pre_guard_size: 0,
            offset_guard_size: 0,
        });

        assert!(limits.validate(&module).is_ok());

        module.num_imported_memories = 1;
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
            mutability: false,
            initializer: GlobalInit::I32Const(0),
        });

        assert!(limits.validate(&module).is_ok());

        module.num_imported_globals = 1;
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
        assert!(limits.validate(&module).is_ok());

        module
            .types
            .push(ModuleType::Function(SignatureIndex::new(0)));
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
        assert!(limits.validate(&module).is_ok());

        module.functions.push(SignatureIndex::new(0));
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
        assert!(limits.validate(&module).is_ok());

        module.table_plans.push(TablePlan {
            style: TableStyle::CallerChecksSignature,
            table: Table {
                wasm_ty: WasmType::FuncRef,
                minimum: 0,
                maximum: None,
            },
        });
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
        assert!(limits.validate(&module).is_ok());

        module.memory_plans.push(MemoryPlan {
            style: MemoryStyle::Static { bound: 0 },
            memory: Memory {
                minimum: 0,
                maximum: None,
                shared: false,
                memory64: false,
            },
            pre_guard_size: 0,
            offset_guard_size: 0,
        });
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
        assert!(limits.validate(&module).is_ok());

        module.globals.push(Global {
            wasm_ty: WasmType::I32,
            mutability: false,
            initializer: GlobalInit::I32Const(0),
        });
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
                minimum: 11,
                maximum: None,
            },
        });
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
                memory64: false,
            },
            pre_guard_size: 0,
            offset_guard_size: 0,
        });
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
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
            style: MemoryStyle::Dynamic { reserve: 0 },
            memory: Memory {
                minimum: 1,
                maximum: None,
                shared: false,
                memory64: false,
            },
            offset_guard_size: 0,
            pre_guard_size: 0,
        });
        assert_eq!(
            limits.validate(&module).map_err(|e| e.to_string()),
            Err("memory index 0 has an unsupported dynamic memory plan style".into())
        );
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_instance_pool() -> Result<()> {
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
            memory_pages: 1,
        };
        let instance_limits = InstanceLimits { count: 3 };

        let instances = InstancePool::new(
            PoolingAllocationStrategy::NextAvailable,
            &module_limits,
            &instance_limits,
            &Tunables {
                static_memory_bound: 1,
                ..Tunables::default()
            },
        )?;

        // As of April 2021, the instance struct's size is largely below the size of a single page,
        // so it's safe to assume it's been rounded to the size of a single memory page here.
        assert_eq!(instances.instance_size, region::page::size());
        assert_eq!(instances.max_instances, 3);

        assert_eq!(
            instances.index_allocator.lock().unwrap().testing_freelist(),
            &[SlotId(0), SlotId(1), SlotId(2)]
        );

        let mut handles = Vec::new();
        let module = Arc::new(Module::default());
        let functions = &PrimaryMap::new();

        for _ in (0..3).rev() {
            handles.push(
                instances
                    .allocate(InstanceAllocationRequest {
                        module: &module,
                        unique_id: None,
                        image_base: 0,
                        functions,
                        imports: Imports {
                            functions: &[],
                            tables: &[],
                            memories: &[],
                            globals: &[],
                        },
                        shared_signatures: VMSharedSignatureIndex::default().into(),
                        host_state: Box::new(()),
                        store: StorePtr::empty(),
                        wasm_data: &[],
                        memfds: None,
                    })
                    .expect("allocation should succeed"),
            );
        }

        assert_eq!(
            instances.index_allocator.lock().unwrap().testing_freelist(),
            &[]
        );

        match instances.allocate(InstanceAllocationRequest {
            module: &module,
            unique_id: None,
            functions,
            image_base: 0,
            imports: Imports {
                functions: &[],
                tables: &[],
                memories: &[],
                globals: &[],
            },
            shared_signatures: VMSharedSignatureIndex::default().into(),
            host_state: Box::new(()),
            store: StorePtr::empty(),
            wasm_data: &[],
            memfds: None,
        }) {
            Err(InstantiationError::Limit(3)) => {}
            _ => panic!("unexpected error"),
        };

        for handle in handles.drain(..) {
            instances.deallocate(&handle);
        }

        assert_eq!(
            instances.index_allocator.lock().unwrap().testing_freelist(),
            &[SlotId(2), SlotId(1), SlotId(0)]
        );

        Ok(())
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_memory_pool() -> Result<()> {
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
                memory_pages: 1,
            },
            &InstanceLimits { count: 5 },
            &Tunables {
                static_memory_bound: 1,
                static_memory_offset_guard_size: 0,
                ..Tunables::default()
            },
        )?;

        assert_eq!(pool.memory_size, WASM_PAGE_SIZE as usize);
        assert_eq!(pool.max_memories, 3);
        assert_eq!(pool.max_instances, 5);
        assert_eq!(pool.max_wasm_pages, 1);

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
    fn test_table_pool() -> Result<()> {
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
            &InstanceLimits { count: 7 },
        )?;

        let host_page_size = region::page::size();

        assert_eq!(pool.table_size, host_page_size);
        assert_eq!(pool.max_tables, 4);
        assert_eq!(pool.max_instances, 7);
        assert_eq!(pool.page_size, host_page_size);
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

    #[cfg(all(unix, target_pointer_width = "64", feature = "async"))]
    #[test]
    fn test_stack_pool() -> Result<()> {
        let pool = StackPool::new(&InstanceLimits { count: 10 }, 1)?;

        let native_page_size = region::page::size();
        assert_eq!(pool.stack_size, 2 * native_page_size);
        assert_eq!(pool.max_instances, 10);
        assert_eq!(pool.page_size, native_page_size);

        assert_eq!(
            pool.index_allocator.lock().unwrap().testing_freelist(),
            &[
                SlotId(0),
                SlotId(1),
                SlotId(2),
                SlotId(3),
                SlotId(4),
                SlotId(5),
                SlotId(6),
                SlotId(7),
                SlotId(8),
                SlotId(9)
            ],
        );

        let base = pool.mapping.as_ptr() as usize;

        let mut stacks = Vec::new();
        for i in (0..10).rev() {
            let stack = pool.allocate().expect("allocation should succeed");
            assert_eq!(
                ((stack.top().unwrap() as usize - base) / pool.stack_size) - 1,
                i
            );
            stacks.push(stack);
        }

        assert_eq!(pool.index_allocator.lock().unwrap().testing_freelist(), &[]);

        match pool.allocate().unwrap_err() {
            FiberStackError::Limit(10) => {}
            _ => panic!("unexpected error"),
        };

        for stack in stacks {
            pool.deallocate(&stack);
        }

        assert_eq!(
            pool.index_allocator.lock().unwrap().testing_freelist(),
            &[
                SlotId(9),
                SlotId(8),
                SlotId(7),
                SlotId(6),
                SlotId(5),
                SlotId(4),
                SlotId(3),
                SlotId(2),
                SlotId(1),
                SlotId(0)
            ],
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
                4096,
                &Tunables::default(),
            )
            .map_err(|e| e.to_string())
            .expect_err("expected a failure constructing instance allocator"),
            "the instance count limit cannot be zero"
        );
    }

    #[test]
    fn test_pooling_allocator_with_memory_pages_exceeded() {
        assert_eq!(
            PoolingInstanceAllocator::new(
                PoolingAllocationStrategy::Random,
                ModuleLimits {
                    memory_pages: 0x10001,
                    ..Default::default()
                },
                InstanceLimits { count: 1 },
                4096,
                &Tunables {
                    static_memory_bound: 1,
                    ..Tunables::default()
                },
            )
            .map_err(|e| e.to_string())
            .expect_err("expected a failure constructing instance allocator"),
            "module memory page limit of 65537 exceeds the maximum of 65536"
        );
    }

    #[test]
    fn test_pooling_allocator_with_reservation_size_exceeded() {
        assert_eq!(
            PoolingInstanceAllocator::new(
                PoolingAllocationStrategy::Random,
                ModuleLimits {
                    memory_pages: 2,
                    ..Default::default()
                },
                InstanceLimits { count: 1 },
                4096,
                &Tunables {
                    static_memory_bound: 1,
                    static_memory_offset_guard_size: 0,
                    ..Tunables::default()
                },
            )
            .map_err(|e| e.to_string())
            .expect_err("expected a failure constructing instance allocator"),
            "module memory page limit of 2 pages exceeds maximum static memory limit of 1 pages"
        );
    }

    #[cfg(all(unix, target_pointer_width = "64", feature = "async"))]
    #[test]
    fn test_stack_zeroed() -> Result<()> {
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
            InstanceLimits { count: 1 },
            4096,
            &Tunables::default(),
        )?;

        unsafe {
            for _ in 0..10 {
                let stack = allocator.allocate_fiber_stack()?;

                // The stack pointer is at the top, so decrement it first
                let addr = stack.top().unwrap().sub(1);

                assert_eq!(*addr, 0);
                *addr = 1;

                allocator.deallocate_fiber_stack(&stack);
            }
        }

        Ok(())
    }
}

//! Implements a memory pool using a single allocated memory slab.
//!
//! The pooling instance allocator maps one large slab of memory in advance and
//! allocates WebAssembly memories from this slab--a [`MemoryPool`]. Each
//! WebAssembly memory is allocated in its own slot (see uses of `index` and
//! [`SlotId`] in this module):
//!
//! ```text
//! ┌──────┬──────┬──────┬──────┬──────┐
//! │Slot 0│Slot 1│Slot 2│Slot 3│......│
//! └──────┴──────┴──────┴──────┴──────┘
//! ```
//!
//! Diving deeper, we note that a [`MemoryPool`] protects Wasmtime from
//! out-of-bounds memory accesses by inserting inaccessible guard regions
//! between memory slots. These guard regions are configured to raise a signal
//! if they are accessed--a WebAssembly out-of-bounds (OOB) memory access. The
//! [`MemoryPool`] documentation has a more detailed chart but one can think of
//! memory slots being laid out like the following:
//!
//! ```text
//! ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐
//! │Guard│Mem 0│Guard│Mem 1│Guard│Mem 2│.....│Guard│
//! └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘
//! ```
//!
//! But we can be more efficient about guard regions: with memory protection
//! keys (MPK) enabled, the interleaved guard regions can be smaller. If we
//! surround a memory with memories from other instances and each instance is
//! protected by different protection keys, the guard region can be smaller AND
//! the pool will still raise a signal on an OOB access. This complicates how we
//! lay out memory slots: we must store memories from the same instance in the
//! same "stripe". Each stripe is protected by a different protection key.
//!
//! This concept, dubbed [ColorGuard] in the original paper, relies on careful
//! calculation of the memory sizes to prevent any "overlapping access" (see
//! [`calculate`]): there are limited protection keys available (15) so the next
//! memory using the same key must be at least as far away as the guard region
//! we would insert otherwise. This ends up looking like the following, where a
//! store for instance 0 (`I0`) "stripes" two memories (`M0` and `M1`) with the
//! same protection key 1 and far enough apart to signal an OOB access:
//!
//! ```text
//! ┌─────┬─────┬─────┬─────┬────────────────┬─────┬─────┬─────┐
//! │.....│I0:M1│.....│.....│.<enough slots>.│I0:M2│.....│.....│
//! ├─────┼─────┼─────┼─────┼────────────────┼─────┼─────┼─────┤
//! │.....│key 1│key 2│key 3│..<more keys>...│key 1│key 2│.....│
//! └─────┴─────┴─────┴─────┴────────────────┴─────┴─────┴─────┘
//! ```
//!
//! [ColorGuard]: https://plas2022.github.io/files/pdf/SegueColorGuard.pdf

use super::{
    index_allocator::{MemoryInModule, ModuleAffinityIndexAllocator, SlotId},
    MemoryAllocationIndex,
};
use crate::prelude::*;
use crate::runtime::vm::mpk::{self, ProtectionKey, ProtectionMask};
use crate::runtime::vm::{
    CompiledModuleId, InstanceAllocationRequest, InstanceLimits, Memory, MemoryImageSlot, Mmap,
    MpkEnabled, PoolingInstanceAllocatorConfig,
};
use anyhow::{anyhow, bail, Context, Result};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use wasmtime_environ::{
    DefinedMemoryIndex, MemoryPlan, MemoryStyle, Module, Tunables, WASM_PAGE_SIZE,
};

/// A set of allocator slots.
///
/// The allocated slots can be split by striping them: e.g., with two stripe
/// colors 0 and 1, we would allocate all even slots using stripe 0 and all odd
/// slots using stripe 1.
///
/// This is helpful for the use of protection keys: (a) if a request comes to
/// allocate multiple instances, we can allocate them all from the same stripe
/// and (b) if a store wants to allocate more from the same stripe it can.
#[derive(Debug)]
struct Stripe {
    allocator: ModuleAffinityIndexAllocator,
    pkey: Option<ProtectionKey>,
}

/// Represents a pool of WebAssembly linear memories.
///
/// A linear memory is divided into accessible pages and guard pages. A memory
/// pool contains linear memories: each memory occupies a slot in an
/// allocated slab (i.e., `mapping`):
///
/// ```text
///          layout.max_memory_bytes                 layout.slot_bytes
///                    |                                   |
///              ◄─────┴────►                  ◄───────────┴──────────►
/// ┌───────────┬────────────┬───────────┐     ┌───────────┬───────────┬───────────┐
/// | PROT_NONE |            | PROT_NONE | ... |           | PROT_NONE | PROT_NONE |
/// └───────────┴────────────┴───────────┘     └───────────┴───────────┴───────────┘
/// |           |◄──────────────────┬─────────────────────────────────► ◄────┬────►
/// |           |                   |                                        |
/// mapping     |            `layout.num_slots` memories         layout.post_slab_guard_size
///             |
///   layout.pre_slab_guard_size
/// ```
#[derive(Debug)]
pub struct MemoryPool {
    mapping: Mmap,
    /// This memory pool is stripe-aware. If using  memory protection keys, this
    /// will contain one stripe per available key; otherwise, a single stripe
    /// with an empty key.
    stripes: Vec<Stripe>,

    /// If using a copy-on-write allocation scheme, the slot management. We
    /// dynamically transfer ownership of a slot to a Memory when in use.
    image_slots: Vec<Mutex<Option<MemoryImageSlot>>>,

    /// A description of the various memory sizes used in allocating the
    /// `mapping` slab.
    layout: SlabLayout,

    /// The maximum number of memories that a single core module instance may
    /// use.
    ///
    /// NB: this is needed for validation but does not affect the pool's size.
    memories_per_instance: usize,

    /// How much linear memory, in bytes, to keep resident after resetting for
    /// use with the next instance. This much memory will be `memset` to zero
    /// when a linear memory is deallocated.
    ///
    /// Memory exceeding this amount in the wasm linear memory will be released
    /// with `madvise` back to the kernel.
    ///
    /// Only applicable on Linux.
    pub(super) keep_resident: usize,

    /// Keep track of protection keys handed out to initialized stores; this
    /// allows us to round-robin the assignment of stores to stripes.
    next_available_pkey: AtomicUsize,
}

impl MemoryPool {
    /// Create a new `MemoryPool`.
    pub fn new(config: &PoolingInstanceAllocatorConfig, tunables: &Tunables) -> Result<Self> {
        if u64::try_from(config.limits.max_memory_size).unwrap()
            > tunables.static_memory_reservation
        {
            bail!(
                "maximum memory size of {:#x} bytes exceeds the configured \
                 static memory reservation of {:#x} bytes",
                config.limits.max_memory_size,
                tunables.static_memory_reservation
            );
        }
        let pkeys = match config.memory_protection_keys {
            MpkEnabled::Auto => {
                if mpk::is_supported() {
                    mpk::keys(config.max_memory_protection_keys)
                } else {
                    &[]
                }
            }
            MpkEnabled::Enable => {
                if mpk::is_supported() {
                    mpk::keys(config.max_memory_protection_keys)
                } else {
                    bail!("mpk is disabled on this system")
                }
            }
            MpkEnabled::Disable => &[],
        };

        // This is a tricky bit of global state: when creating a memory pool
        // that uses memory protection keys, we ensure here that any host code
        // will have access to all keys (i.e., stripes). It's only when we enter
        // the WebAssembly guest code (see `StoreInner::call_hook`) that we
        // enforce which keys/stripes can be accessed. Be forewarned about the
        // assumptions here:
        // - we expect this "allow all" configuration to reset the default
        //   process state (only allow key 0) _before_ any memories are accessed
        // - and we expect no other code (e.g., host-side code) to modify this
        //   global MPK configuration
        if !pkeys.is_empty() {
            mpk::allow(ProtectionMask::all());
        }

        // Create a slab layout and allocate it as a completely inaccessible
        // region to start--`PROT_NONE`.
        let constraints = SlabConstraints::new(&config.limits, tunables, pkeys.len())?;
        let layout = calculate(&constraints)?;
        log::debug!(
            "creating memory pool: {constraints:?} -> {layout:?} (total: {})",
            layout.total_slab_bytes()?
        );
        let mut mapping = Mmap::accessible_reserved(0, layout.total_slab_bytes()?)
            .context("failed to create memory pool mapping")?;

        // Then, stripe the memory with the available protection keys. This is
        // unnecessary if there is only one stripe color.
        if layout.num_stripes >= 2 {
            let mut cursor = layout.pre_slab_guard_bytes;
            let pkeys = &pkeys[..layout.num_stripes];
            for i in 0..constraints.num_slots {
                let pkey = &pkeys[i % pkeys.len()];
                let region = unsafe { mapping.slice_mut(cursor..cursor + layout.slot_bytes) };
                pkey.protect(region)?;
                cursor += layout.slot_bytes;
            }
            debug_assert_eq!(
                cursor + layout.post_slab_guard_bytes,
                layout.total_slab_bytes()?
            );
        }

        let image_slots: Vec<_> = std::iter::repeat_with(|| Mutex::new(None))
            .take(constraints.num_slots)
            .collect();

        let create_stripe = |i| {
            let num_slots = constraints.num_slots / layout.num_stripes
                + usize::from(constraints.num_slots % layout.num_stripes > i);
            let allocator = ModuleAffinityIndexAllocator::new(
                num_slots.try_into().unwrap(),
                config.max_unused_warm_slots,
            );
            Stripe {
                allocator,
                pkey: pkeys.get(i).cloned(),
            }
        };

        debug_assert!(layout.num_stripes > 0);
        let stripes: Vec<_> = (0..layout.num_stripes)
            .into_iter()
            .map(create_stripe)
            .collect();

        let pool = Self {
            stripes,
            mapping,
            image_slots,
            layout,
            memories_per_instance: usize::try_from(config.limits.max_memories_per_module).unwrap(),
            keep_resident: config.linear_memory_keep_resident,
            next_available_pkey: AtomicUsize::new(0),
        };

        Ok(pool)
    }

    /// Return a protection key that stores can use for requesting new
    pub fn next_available_pkey(&self) -> Option<ProtectionKey> {
        let index = self.next_available_pkey.fetch_add(1, Ordering::SeqCst) % self.stripes.len();
        debug_assert!(
            self.stripes.len() < 2 || self.stripes[index].pkey.is_some(),
            "if we are using stripes, we cannot have an empty protection key"
        );
        self.stripes[index].pkey.clone()
    }

    /// Validate whether this memory pool supports the given module.
    pub fn validate(&self, module: &Module) -> Result<()> {
        let memories = module.memory_plans.len() - module.num_imported_memories;
        if memories > usize::try_from(self.memories_per_instance).unwrap() {
            bail!(
                "defined memories count of {} exceeds the per-instance limit of {}",
                memories,
                self.memories_per_instance,
            );
        }

        let max_memory_pages = self.layout.max_memory_bytes / WASM_PAGE_SIZE as usize;
        for (i, plan) in module
            .memory_plans
            .iter()
            .skip(module.num_imported_memories)
        {
            match plan.style {
                MemoryStyle::Static { byte_reservation } => {
                    if u64::try_from(self.layout.bytes_to_next_stripe_slot()).unwrap()
                        < byte_reservation
                    {
                        bail!(
                            "memory size allocated per-memory is too small to \
                             satisfy static bound of {byte_reservation:#x} bytes"
                        );
                    }
                }
                MemoryStyle::Dynamic { .. } => {}
            }
            if plan.memory.minimum > u64::try_from(max_memory_pages).unwrap() {
                bail!(
                    "memory index {} has a minimum page size of {} which exceeds the limit of {}",
                    i.as_u32(),
                    plan.memory.minimum,
                    max_memory_pages,
                );
            }
        }
        Ok(())
    }

    /// Are zero slots in use right now?
    #[allow(unused)] // some cfgs don't use this
    pub fn is_empty(&self) -> bool {
        self.stripes.iter().all(|s| s.allocator.is_empty())
    }

    /// Allocate a single memory for the given instance allocation request.
    pub fn allocate(
        &self,
        request: &mut InstanceAllocationRequest,
        memory_plan: &MemoryPlan,
        memory_index: DefinedMemoryIndex,
    ) -> Result<(MemoryAllocationIndex, Memory)> {
        let stripe_index = if let Some(pkey) = &request.pkey {
            pkey.as_stripe()
        } else {
            debug_assert!(self.stripes.len() < 2);
            0
        };

        let striped_allocation_index = self.stripes[stripe_index]
            .allocator
            .alloc(
                request
                    .runtime_info
                    .unique_id()
                    .map(|id| MemoryInModule(id, memory_index)),
            )
            .map(|slot| StripedAllocationIndex(u32::try_from(slot.index()).unwrap()))
            .ok_or_else(|| {
                super::PoolConcurrencyLimitError::new(
                    self.stripes[stripe_index].allocator.len(),
                    format!("memory stripe {stripe_index}"),
                )
            })?;
        let allocation_index =
            striped_allocation_index.as_unstriped_slot_index(stripe_index, self.stripes.len());

        match (|| {
            // Double-check that the runtime requirements of the memory are
            // satisfied by the configuration of this pooling allocator. This
            // should be returned as an error through `validate_memory_plans`
            // but double-check here to be sure.
            match memory_plan.style {
                MemoryStyle::Static { byte_reservation } => {
                    assert!(
                        byte_reservation
                            <= u64::try_from(self.layout.bytes_to_next_stripe_slot()).unwrap()
                    );
                }
                MemoryStyle::Dynamic { .. } => {}
            }

            let base_ptr = self.get_base(allocation_index);
            let base_capacity = self.layout.max_memory_bytes;

            let mut slot = self.take_memory_image_slot(allocation_index);
            let image = request.runtime_info.memory_image(memory_index)?;
            let initial_size = memory_plan.memory.minimum * WASM_PAGE_SIZE as u64;

            // If instantiation fails, we can propagate the error
            // upward and drop the slot. This will cause the Drop
            // handler to attempt to map the range with PROT_NONE
            // memory, to reserve the space while releasing any
            // stale mappings. The next use of this slot will then
            // create a new slot that will try to map over
            // this, returning errors as well if the mapping
            // errors persist. The unmap-on-drop is best effort;
            // if it fails, then we can still soundly continue
            // using the rest of the pool and allowing the rest of
            // the process to continue, because we never perform a
            // mmap that would leave an open space for someone
            // else to come in and map something.
            slot.instantiate(initial_size as usize, image, memory_plan)?;

            Memory::new_static(
                memory_plan,
                base_ptr,
                base_capacity,
                slot,
                self.layout.bytes_to_next_stripe_slot(),
                unsafe { &mut *request.store.get().unwrap() },
            )
        })() {
            Ok(memory) => Ok((allocation_index, memory)),
            Err(e) => {
                self.stripes[stripe_index]
                    .allocator
                    .free(SlotId(striped_allocation_index.0));
                Err(e)
            }
        }
    }

    /// Deallocate a previously-allocated memory.
    ///
    /// # Safety
    ///
    /// The memory must have been previously allocated from this pool and
    /// assigned the given index, must currently be in an allocated state, and
    /// must never be used again.
    ///
    /// The caller must have already called `clear_and_remain_ready` on the
    /// memory's image and flushed any enqueued decommits for this memory.
    pub unsafe fn deallocate(
        &self,
        allocation_index: MemoryAllocationIndex,
        image: MemoryImageSlot,
    ) {
        self.return_memory_image_slot(allocation_index, image);

        let (stripe_index, striped_allocation_index) =
            StripedAllocationIndex::from_unstriped_slot_index(allocation_index, self.stripes.len());
        self.stripes[stripe_index]
            .allocator
            .free(SlotId(striped_allocation_index.0));
    }

    /// Purging everything related to `module`.
    pub fn purge_module(&self, module: CompiledModuleId) {
        // This primarily means clearing out all of its memory images present in
        // the virtual address space. Go through the index allocator for slots
        // affine to `module` and reset them, freeing up the index when we're
        // done.
        //
        // Note that this is only called when the specified `module` won't be
        // allocated further (the module is being dropped) so this shouldn't hit
        // any sort of infinite loop since this should be the final operation
        // working with `module`.
        //
        // TODO: We are given a module id, but key affinity by pair of module id
        // and defined memory index. We are missing any defined memory index or
        // count of how many memories the module defines here. Therefore, we
        // probe up to the maximum number of memories per instance. This is fine
        // because that maximum is generally relatively small. If this method
        // somehow ever gets hot because of unnecessary probing, we should
        // either pass in the actual number of defined memories for the given
        // module to this method, or keep a side table of all slots that are
        // associated with a module (not just module and memory). The latter
        // would require care to make sure that its maintenance wouldn't be too
        // expensive for normal allocation/free operations.
        for stripe in &self.stripes {
            for i in 0..self.memories_per_instance {
                use wasmtime_environ::EntityRef;
                let memory_index = DefinedMemoryIndex::new(i);
                while let Some(id) = stripe
                    .allocator
                    .alloc_affine_and_clear_affinity(module, memory_index)
                {
                    // Clear the image from the slot and, if successful, return it back
                    // to our state. Note that on failure here the whole slot will get
                    // paved over with an anonymous mapping.
                    let index = MemoryAllocationIndex(id.0);
                    let mut slot = self.take_memory_image_slot(index);
                    if slot.remove_image().is_ok() {
                        self.return_memory_image_slot(index, slot);
                    }

                    stripe.allocator.free(id);
                }
            }
        }
    }

    fn get_base(&self, allocation_index: MemoryAllocationIndex) -> *mut u8 {
        assert!(allocation_index.index() < self.layout.num_slots);
        let offset =
            self.layout.pre_slab_guard_bytes + allocation_index.index() * self.layout.slot_bytes;
        unsafe { self.mapping.as_ptr().offset(offset as isize).cast_mut() }
    }

    /// Take ownership of the given image slot. Must be returned via
    /// `return_memory_image_slot` when the instance is done using it.
    fn take_memory_image_slot(&self, allocation_index: MemoryAllocationIndex) -> MemoryImageSlot {
        let maybe_slot = self.image_slots[allocation_index.index()]
            .lock()
            .unwrap()
            .take();

        maybe_slot.unwrap_or_else(|| {
            MemoryImageSlot::create(
                self.get_base(allocation_index) as *mut c_void,
                0,
                self.layout.max_memory_bytes,
            )
        })
    }

    /// Return ownership of the given image slot.
    fn return_memory_image_slot(
        &self,
        allocation_index: MemoryAllocationIndex,
        slot: MemoryImageSlot,
    ) {
        assert!(!slot.is_dirty());
        *self.image_slots[allocation_index.index()].lock().unwrap() = Some(slot);
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        // Clear the `clear_no_drop` flag (i.e., ask to *not* clear on
        // drop) for all slots, and then drop them here. This is
        // valid because the one `Mmap` that covers the whole region
        // can just do its one munmap.
        for mut slot in std::mem::take(&mut self.image_slots) {
            if let Some(slot) = slot.get_mut().unwrap() {
                slot.no_clear_on_drop();
            }
        }
    }
}

/// The index of a memory allocation within an `InstanceAllocator`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct StripedAllocationIndex(u32);

impl StripedAllocationIndex {
    fn from_unstriped_slot_index(
        index: MemoryAllocationIndex,
        num_stripes: usize,
    ) -> (usize, Self) {
        let stripe_index = index.index() % num_stripes;
        let num_stripes: u32 = num_stripes.try_into().unwrap();
        let index_within_stripe = Self(index.0 / num_stripes);
        (stripe_index, index_within_stripe)
    }

    fn as_unstriped_slot_index(self, stripe: usize, num_stripes: usize) -> MemoryAllocationIndex {
        let num_stripes: u32 = num_stripes.try_into().unwrap();
        let stripe: u32 = stripe.try_into().unwrap();
        MemoryAllocationIndex(self.0 * num_stripes + stripe)
    }
}

#[derive(Clone, Debug)]
struct SlabConstraints {
    /// Essentially, the `static_memory_bound`: this is an assumption that the
    /// runtime and JIT compiler make about how much space will be guarded
    /// between slots.
    expected_slot_bytes: usize,
    /// The maximum size of any memory in the pool.
    max_memory_bytes: usize,
    num_slots: usize,
    num_pkeys_available: usize,
    guard_bytes: usize,
    guard_before_slots: bool,
}

impl SlabConstraints {
    fn new(
        limits: &InstanceLimits,
        tunables: &Tunables,
        num_pkeys_available: usize,
    ) -> Result<Self> {
        // `static_memory_bound` is the configured number of Wasm pages for a
        // static memory slot (see `Config::static_memory_maximum_size`); even
        // if the memory never grows to this size (e.g., it has a lower memory
        // maximum), codegen will assume that this unused memory is mapped
        // `PROT_NONE`. Typically `static_memory_bound` is 4G which helps elide
        // most bounds checks. `MemoryPool` must respect this bound, though not
        // explicitly: if we can achieve the same effect via MPK-protected
        // stripes, the slot size can be lower than the `static_memory_bound`.
        let expected_slot_bytes = tunables.static_memory_reservation;

        let constraints = SlabConstraints {
            max_memory_bytes: limits.max_memory_size,
            num_slots: limits
                .total_memories
                .try_into()
                .context("too many memories")?,
            expected_slot_bytes: expected_slot_bytes
                .try_into()
                .context("static memory bound is too large")?,
            num_pkeys_available,
            guard_bytes: tunables
                .static_memory_offset_guard_size
                .try_into()
                .context("guard region is too large")?,
            guard_before_slots: tunables.guard_before_linear_memory,
        };
        Ok(constraints)
    }
}

#[derive(Debug)]
struct SlabLayout {
    /// The total number of slots available in the memory pool slab.
    num_slots: usize,
    /// The size of each slot in the memory pool; this contains the maximum
    /// memory size (i.e., from WebAssembly or Wasmtime configuration) plus any
    /// guard region after the memory to catch OOB access. On these guard
    /// regions, note that:
    /// - users can configure how aggressively (or not) to elide bounds checks
    ///   via `Config::static_memory_guard_size` (see also:
    ///   `memory_and_guard_size`)
    /// - memory protection keys can compress the size of the guard region by
    ///   placing slots from a different key (i.e., a stripe) in the guard
    ///   region; this means the slot itself can be smaller and we can allocate
    ///   more of them.
    slot_bytes: usize,
    /// The maximum size that can become accessible, in bytes, for each linear
    /// memory. Guaranteed to be a whole number of Wasm pages.
    max_memory_bytes: usize,
    /// If necessary, the number of bytes to reserve as a guard region at the
    /// beginning of the slab.
    pre_slab_guard_bytes: usize,
    /// Like `pre_slab_guard_bytes`, but at the end of the slab.
    post_slab_guard_bytes: usize,
    /// The number of stripes needed in the slab layout.
    num_stripes: usize,
}

impl SlabLayout {
    /// Return the total size of the slab, using the final layout (where `n =
    /// num_slots`):
    ///
    /// ```text
    /// ┌────────────────────┬──────┬──────┬───┬──────┬─────────────────────┐
    /// │pre_slab_guard_bytes│slot 1│slot 2│...│slot n│post_slab_guard_bytes│
    /// └────────────────────┴──────┴──────┴───┴──────┴─────────────────────┘
    /// ```
    fn total_slab_bytes(&self) -> Result<usize> {
        self.slot_bytes
            .checked_mul(self.num_slots)
            .and_then(|c| c.checked_add(self.pre_slab_guard_bytes))
            .and_then(|c| c.checked_add(self.post_slab_guard_bytes))
            .ok_or_else(|| anyhow!("total size of memory reservation exceeds addressable memory"))
    }

    /// Returns the number of Wasm bytes from the beginning of one slot to the
    /// next slot in the same stripe--this is the striped equivalent of
    /// `static_memory_bound`. Recall that between slots of the same stripe we
    /// will see a slot from every other stripe.
    ///
    /// For example, in a 3-stripe pool, this function measures the distance
    /// from the beginning of slot 1 to slot 4, which are of the same stripe:
    ///
    /// ```text
    ///  ◄────────────────────►
    /// ┌────────┬──────┬──────┬────────┬───┐
    /// │*slot 1*│slot 2│slot 3│*slot 4*│...|
    /// └────────┴──────┴──────┴────────┴───┘
    /// ```
    fn bytes_to_next_stripe_slot(&self) -> usize {
        self.slot_bytes * self.num_stripes
    }
}

fn calculate(constraints: &SlabConstraints) -> Result<SlabLayout> {
    let SlabConstraints {
        max_memory_bytes,
        num_slots,
        expected_slot_bytes,
        num_pkeys_available,
        guard_bytes,
        guard_before_slots,
    } = *constraints;

    // If the user specifies a guard region, we always need to allocate a
    // `PROT_NONE` region for it before any memory slots. Recall that we can
    // avoid bounds checks for loads and stores with immediates up to
    // `guard_bytes`, but we rely on Wasmtime to emit bounds checks for any
    // accesses greater than this.
    let pre_slab_guard_bytes = if guard_before_slots { guard_bytes } else { 0 };

    // To calculate the slot size, we start with the default configured size and
    // attempt to chip away at this via MPK protection. Note here how we begin
    // to define a slot as "all of the memory and guard region."
    let faulting_region_bytes = expected_slot_bytes
        .max(max_memory_bytes)
        .saturating_add(guard_bytes);

    let (num_stripes, slot_bytes) = if guard_bytes == 0 || max_memory_bytes == 0 || num_slots == 0 {
        // In the uncommon case where the memory/guard regions are empty or we don't need any slots , we
        // will not need any stripes: we just lay out the slots back-to-back
        // using a single stripe.
        (1, faulting_region_bytes)
    } else if num_pkeys_available < 2 {
        // If we do not have enough protection keys to stripe the memory, we do
        // the same. We can't elide any of the guard bytes because we aren't
        // overlapping guard regions with other stripes...
        (1, faulting_region_bytes)
    } else {
        // ...but if we can create at least two stripes, we can use another
        // stripe (i.e., a different pkey) as this slot's guard region--this
        // reduces the guard bytes each slot has to allocate. We must make
        // sure, though, that if the size of that other stripe(s) does not
        // fully cover `guard_bytes`, we keep those around to prevent OOB
        // access.

        // We first calculate the number of stripes we need: we want to
        // minimize this so that there is less chance of a single store
        // running out of slots with its stripe--we need at least two,
        // though. But this is not just an optimization; we need to handle
        // the case when there are fewer slots than stripes. E.g., if our
        // pool is configured with only three slots (`num_memory_slots =
        // 3`), we will run into failures if we attempt to set up more than
        // three stripes.
        let needed_num_stripes = faulting_region_bytes / max_memory_bytes
            + usize::from(faulting_region_bytes % max_memory_bytes != 0);
        assert!(needed_num_stripes > 0);
        let num_stripes = num_pkeys_available.min(needed_num_stripes).min(num_slots);

        // Next, we try to reduce the slot size by "overlapping" the stripes: we
        // can make slot `n` smaller since we know that slot `n+1` and following
        // are in different stripes and will look just like `PROT_NONE` memory.
        // Recall that codegen expects a guarantee that at least
        // `faulting_region_bytes` will catch OOB accesses via segfaults.
        let needed_slot_bytes = faulting_region_bytes
            .checked_div(num_stripes)
            .unwrap_or(faulting_region_bytes)
            .max(max_memory_bytes);
        assert!(needed_slot_bytes >= max_memory_bytes);

        (num_stripes, needed_slot_bytes)
    };

    // The page-aligned slot size; equivalent to `memory_and_guard_size`.
    let page_alignment = crate::runtime::vm::page_size() - 1;
    let slot_bytes = slot_bytes
        .checked_add(page_alignment)
        .and_then(|slot_bytes| Some(slot_bytes & !page_alignment))
        .ok_or_else(|| anyhow!("slot size is too large"))?;

    // We may need another guard region (like `pre_slab_guard_bytes`) at the end
    // of our slab to maintain our `faulting_region_bytes` guarantee. We could
    // be conservative and just create it as large as `faulting_region_bytes`,
    // but because we know that the last slot's `slot_bytes` make up the first
    // part of that region, we reduce the final guard region by that much.
    let post_slab_guard_bytes = faulting_region_bytes.saturating_sub(slot_bytes);

    // Check that we haven't exceeded the slab we can calculate given the limits
    // of `usize`.
    let layout = SlabLayout {
        num_slots,
        slot_bytes,
        max_memory_bytes,
        pre_slab_guard_bytes,
        post_slab_guard_bytes,
        num_stripes,
    };
    match layout.total_slab_bytes() {
        Ok(_) => Ok(layout),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_memory_pool() -> Result<()> {
        let pool = MemoryPool::new(
            &PoolingInstanceAllocatorConfig {
                limits: InstanceLimits {
                    total_memories: 5,
                    max_tables_per_module: 0,
                    max_memories_per_module: 3,
                    table_elements: 0,
                    max_memory_size: 65536,
                    ..Default::default()
                },
                ..Default::default()
            },
            &Tunables {
                static_memory_reservation: 65536,
                static_memory_offset_guard_size: 0,
                ..Tunables::default_host()
            },
        )?;

        assert_eq!(pool.layout.slot_bytes, WASM_PAGE_SIZE as usize);
        assert_eq!(pool.layout.num_slots, 5);
        assert_eq!(pool.layout.max_memory_bytes, WASM_PAGE_SIZE as usize);

        let base = pool.mapping.as_ptr() as usize;

        for i in 0..5 {
            let index = MemoryAllocationIndex(i);
            let ptr = pool.get_base(index);
            assert_eq!(ptr as usize - base, i as usize * pool.layout.slot_bytes);
        }

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_pooling_allocator_striping() {
        if !mpk::is_supported() {
            println!("skipping `test_pooling_allocator_striping` test; mpk is not supported");
            return;
        }

        // Force the use of MPK.
        let config = PoolingInstanceAllocatorConfig {
            memory_protection_keys: MpkEnabled::Enable,
            ..PoolingInstanceAllocatorConfig::default()
        };
        let pool = MemoryPool::new(&config, &Tunables::default_host()).unwrap();
        assert!(pool.stripes.len() >= 2);

        let max_memory_slots = config.limits.total_memories;
        dbg!(pool.stripes[0].allocator.num_empty_slots());
        dbg!(pool.stripes[1].allocator.num_empty_slots());
        let available_memory_slots: usize = pool
            .stripes
            .iter()
            .map(|s| s.allocator.num_empty_slots())
            .sum();
        assert_eq!(
            max_memory_slots,
            u32::try_from(available_memory_slots).unwrap()
        );
    }

    #[test]
    fn check_known_layout_calculations() {
        for num_pkeys_available in 0..16 {
            for num_memory_slots in [0, 1, 10, 64] {
                for expected_slot_bytes in [0, 1 << 30 /* 1GB */, 4 << 30 /* 4GB */] {
                    for max_memory_bytes in
                        [0, 1 * WASM_PAGE_SIZE as usize, 10 * WASM_PAGE_SIZE as usize]
                    {
                        for guard_bytes in [0, 2 << 30 /* 2GB */] {
                            for guard_before_slots in [true, false] {
                                let constraints = SlabConstraints {
                                    max_memory_bytes,
                                    num_slots: num_memory_slots,
                                    expected_slot_bytes,
                                    num_pkeys_available,
                                    guard_bytes,
                                    guard_before_slots,
                                };
                                let layout = calculate(&constraints);
                                assert_slab_layout_invariants(constraints, layout.unwrap());
                            }
                        }
                    }
                }
            }
        }
    }

    proptest! {
        #[test]
        #[cfg_attr(miri, ignore)]
        fn check_random_layout_calculations(c in constraints()) {
            if let Ok(l) = calculate(&c) {
                assert_slab_layout_invariants(c, l);
            }
        }
    }

    fn constraints() -> impl Strategy<Value = SlabConstraints> {
        (
            any::<usize>(),
            any::<usize>(),
            any::<usize>(),
            any::<usize>(),
            any::<usize>(),
            any::<bool>(),
        )
            .prop_map(
                |(
                    max_memory_bytes,
                    num_memory_slots,
                    expected_slot_bytes,
                    num_pkeys_available,
                    guard_bytes,
                    guard_before_slots,
                )| {
                    SlabConstraints {
                        max_memory_bytes,
                        num_slots: num_memory_slots,
                        expected_slot_bytes,
                        num_pkeys_available,
                        guard_bytes,
                        guard_before_slots,
                    }
                },
            )
    }

    fn assert_slab_layout_invariants(c: SlabConstraints, s: SlabLayout) {
        // Check that all the sizes add up.
        assert_eq!(
            s.total_slab_bytes().unwrap(),
            s.pre_slab_guard_bytes + s.slot_bytes * c.num_slots + s.post_slab_guard_bytes,
            "the slab size does not add up: {c:?} => {s:?}"
        );
        assert!(
            s.slot_bytes >= s.max_memory_bytes,
            "slot is not big enough: {c:?} => {s:?}"
        );

        // Check that the various memory values are page-aligned.
        assert!(
            is_aligned(s.slot_bytes),
            "slot is not page-aligned: {c:?} => {s:?}",
        );
        assert!(
            is_aligned(s.max_memory_bytes),
            "slot guard region is not page-aligned: {c:?} => {s:?}",
        );
        assert!(
            is_aligned(s.pre_slab_guard_bytes),
            "pre-slab guard region is not page-aligned: {c:?} => {s:?}"
        );
        assert!(
            is_aligned(s.post_slab_guard_bytes),
            "post-slab guard region is not page-aligned: {c:?} => {s:?}"
        );
        assert!(
            is_aligned(s.total_slab_bytes().unwrap()),
            "slab is not page-aligned: {c:?} => {s:?}"
        );

        // Check that we use no more or less stripes than needed.
        assert!(s.num_stripes >= 1, "not enough stripes: {c:?} => {s:?}");
        if c.num_pkeys_available == 0 || c.num_slots == 0 {
            assert_eq!(
                s.num_stripes, 1,
                "expected at least one stripe: {c:?} => {s:?}"
            );
        } else {
            assert!(
                s.num_stripes <= c.num_pkeys_available,
                "layout has more stripes than available pkeys: {c:?} => {s:?}"
            );
            assert!(
                s.num_stripes <= c.num_slots,
                "layout has more stripes than memory slots: {c:?} => {s:?}"
            );
        }

        // Check that we use the minimum number of stripes/protection keys.
        // - if the next MPK-protected slot is bigger or the same as the
        //   required guard region, we only need two stripes
        // - if the next slot is smaller than the guard region, we only need
        //   enough stripes to add up to at least that guard region size.
        if c.num_pkeys_available > 1 && c.max_memory_bytes > 0 {
            assert!(
                s.num_stripes <= (c.guard_bytes / c.max_memory_bytes) + 2,
                "calculated more stripes than needed: {c:?} => {s:?}"
            );
        }

        // Check that the memory-striping will not allow OOB access.
        // - we may have reduced the slot size from `expected_slot_bytes` to
        //   `slot_bytes` assuming MPK striping; we check that our guaranteed
        //   "faulting region" is respected
        // - the last slot won't have MPK striping after it; we check that the
        //   `post_slab_guard_bytes` accounts for this
        assert!(
            s.bytes_to_next_stripe_slot()
                >= c.expected_slot_bytes.max(c.max_memory_bytes) + c.guard_bytes,
            "faulting region not large enough: {c:?} => {s:?}"
        );
        assert!(
            s.slot_bytes + s.post_slab_guard_bytes >= c.expected_slot_bytes,
            "last slot may allow OOB access: {c:?} => {s:?}"
        );
    }

    fn is_aligned(bytes: usize) -> bool {
        bytes % crate::runtime::vm::page_size() == 0
    }
}

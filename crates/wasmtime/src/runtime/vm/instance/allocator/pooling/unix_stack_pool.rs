#![cfg_attr(asan, allow(dead_code))]

use super::{
    index_allocator::{SimpleIndexAllocator, SlotId},
    round_up_to_pow2,
};
use crate::runtime::vm::sys::vm::commit_pages;
use crate::runtime::vm::{Mmap, PoolingInstanceAllocatorConfig};
use anyhow::{anyhow, bail, Context, Result};

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
#[derive(Debug)]
pub struct StackPool {
    mapping: Mmap,
    stack_size: usize,
    max_stacks: usize,
    page_size: usize,
    index_allocator: SimpleIndexAllocator,
    async_stack_zeroing: bool,
    async_stack_keep_resident: usize,
}

impl StackPool {
    pub fn new(config: &PoolingInstanceAllocatorConfig) -> Result<Self> {
        use rustix::mm::{mprotect, MprotectFlags};

        let page_size = crate::runtime::vm::page_size();

        // Add a page to the stack size for the guard page when using fiber stacks
        let stack_size = if config.stack_size == 0 {
            0
        } else {
            round_up_to_pow2(config.stack_size, page_size)
                .checked_add(page_size)
                .ok_or_else(|| anyhow!("stack size exceeds addressable memory"))?
        };

        let max_stacks = usize::try_from(config.limits.total_stacks).unwrap();

        let allocation_size = stack_size
            .checked_mul(max_stacks)
            .ok_or_else(|| anyhow!("total size of execution stacks exceeds addressable memory"))?;

        let mapping = Mmap::accessible_reserved(allocation_size, allocation_size)
            .context("failed to create stack pool mapping")?;

        // Set up the stack guard pages.
        if allocation_size > 0 {
            unsafe {
                for i in 0..max_stacks {
                    // Make the stack guard page inaccessible.
                    let bottom_of_stack = mapping.as_ptr().add(i * stack_size).cast_mut();
                    mprotect(bottom_of_stack.cast(), page_size, MprotectFlags::empty())
                        .context("failed to protect stack guard page")?;
                }
            }
        }

        Ok(Self {
            mapping,
            stack_size,
            max_stacks,
            page_size,
            async_stack_zeroing: config.async_stack_zeroing,
            async_stack_keep_resident: config.async_stack_keep_resident,
            index_allocator: SimpleIndexAllocator::new(config.limits.total_stacks),
        })
    }

    /// Are there zero slots in use right now?
    #[allow(unused)] // some cfgs don't use this
    pub fn is_empty(&self) -> bool {
        self.index_allocator.is_empty()
    }

    /// Allocate a new fiber.
    pub fn allocate(&self) -> Result<wasmtime_fiber::FiberStack> {
        if self.stack_size == 0 {
            bail!("pooling allocator not configured to enable fiber stack allocation");
        }

        let index = self
            .index_allocator
            .alloc()
            .ok_or_else(|| super::PoolConcurrencyLimitError::new(self.max_stacks, "fibers"))?
            .index();

        assert!(index < self.max_stacks);

        unsafe {
            // Remove the guard page from the size
            let size_without_guard = self.stack_size - self.page_size;

            let bottom_of_stack = self
                .mapping
                .as_ptr()
                .add((index * self.stack_size) + self.page_size)
                .cast_mut();

            commit_pages(bottom_of_stack, size_without_guard)?;

            let stack =
                wasmtime_fiber::FiberStack::from_raw_parts(bottom_of_stack, size_without_guard)?;
            Ok(stack)
        }
    }

    /// Zero the given stack, if we are configured to do so.
    ///
    /// This will call the given `decommit` function for each region of memory
    /// that should be decommitted. It is the caller's responsibility to ensure
    /// that those decommits happen before this stack is reused.
    ///
    /// # Safety
    ///
    /// The stack must no longer be in use, and ready for returning to the pool
    /// after it is zeroed and decommitted.
    pub unsafe fn zero_stack(
        &self,
        stack: &mut wasmtime_fiber::FiberStack,
        mut decommit: impl FnMut(*mut u8, usize),
    ) {
        assert!(stack.is_from_raw_parts());

        if !self.async_stack_zeroing {
            return;
        }

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
        assert!(start_of_stack >= base && start_of_stack < (base + len));
        assert!((start_of_stack - base) % self.stack_size == 0);

        // Manually zero the top of the stack to keep the pages resident in
        // memory and avoid future page faults. Use the system to deallocate
        // pages past this. This hopefully strikes a reasonable balance between:
        //
        // * memset for the whole range is probably expensive
        // * madvise for the whole range incurs expensive future page faults
        // * most threads probably don't use most of the stack anyway
        let size_to_memset = stack_size.min(self.async_stack_keep_resident);
        std::ptr::write_bytes(
            (bottom_of_stack + stack_size - size_to_memset) as *mut u8,
            0,
            size_to_memset,
        );

        // Use the system to reset remaining stack pages to zero.
        decommit(bottom_of_stack as _, stack_size - size_to_memset);
    }

    /// Deallocate a previously-allocated fiber.
    ///
    /// # Safety
    ///
    /// The fiber must have been allocated by this pool, must be in an allocated
    /// state, and must never be used again.
    ///
    /// The caller must have already called `zero_stack` on the fiber stack and
    /// flushed any enqueued decommits for this stack's memory.
    pub unsafe fn deallocate(&self, stack: wasmtime_fiber::FiberStack) {
        assert!(stack.is_from_raw_parts());

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
        assert!(start_of_stack >= base && start_of_stack < (base + len));
        assert!((start_of_stack - base) % self.stack_size == 0);

        let index = (start_of_stack - base) / self.stack_size;
        assert!(index < self.max_stacks);

        self.index_allocator.free(SlotId(index as u32));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::runtime::vm::InstanceLimits;

    #[cfg(all(unix, target_pointer_width = "64", feature = "async", not(miri)))]
    #[test]
    fn test_stack_pool() -> Result<()> {
        let config = PoolingInstanceAllocatorConfig {
            limits: InstanceLimits {
                total_stacks: 10,
                ..Default::default()
            },
            stack_size: 1,
            async_stack_zeroing: true,
            ..PoolingInstanceAllocatorConfig::default()
        };
        let pool = StackPool::new(&config)?;

        let native_page_size = crate::runtime::vm::page_size();
        assert_eq!(pool.stack_size, 2 * native_page_size);
        assert_eq!(pool.max_stacks, 10);
        assert_eq!(pool.page_size, native_page_size);

        assert_eq!(pool.index_allocator.testing_freelist(), []);

        let base = pool.mapping.as_ptr() as usize;

        let mut stacks = Vec::new();
        for i in 0..10 {
            let stack = pool.allocate().expect("allocation should succeed");
            assert_eq!(
                ((stack.top().unwrap() as usize - base) / pool.stack_size) - 1,
                i
            );
            stacks.push(stack);
        }

        assert_eq!(pool.index_allocator.testing_freelist(), []);

        assert!(pool.allocate().is_err(), "allocation should fail");

        for stack in stacks {
            unsafe {
                pool.deallocate(stack);
            }
        }

        assert_eq!(
            pool.index_allocator.testing_freelist(),
            [
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

        Ok(())
    }
}

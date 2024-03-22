use anyhow::{anyhow, ensure, Context, Result};
use std::{alloc::Layout, collections::BTreeMap, num::NonZeroU32, ops::Bound};

/// A very simple first-fit free list for use by our garbage collectors.
pub(crate) struct FreeList {
    /// The total capacity of the contiguous range of memory we are managing.
    capacity: usize,
    /// Our free blocks, as a map from index to length of the free block at that
    /// index.
    free_block_index_to_len: BTreeMap<u32, u32>,
}

/// Our minimum and maximum supported alignment. Every allocation is aligned to
/// this.
const ALIGN_USIZE: usize = 8;
const ALIGN_U32: u32 = ALIGN_USIZE as u32;

/// Our minimum allocation size.
const MIN_BLOCK_SIZE: u32 = 24;

impl FreeList {
    /// Create a new `FreeList` for a contiguous region of memory of the given
    /// size.
    pub fn new(capacity: usize) -> Self {
        let mut free_list = FreeList {
            capacity,
            free_block_index_to_len: BTreeMap::new(),
        };
        free_list.reset();
        free_list
    }

    fn max_size(&self) -> usize {
        let cap = std::cmp::min(self.capacity, usize::try_from(u32::MAX).unwrap());
        round_usize_down_to_pow2(cap.saturating_sub(ALIGN_USIZE), ALIGN_USIZE)
    }

    /// Check the given layout for compatibility with this free list and return
    /// the actual block size we will use for this layout.
    fn check_layout(&self, layout: Layout) -> Result<u32> {
        ensure!(
            layout.align() <= ALIGN_USIZE,
            "requested allocation's alignment of {} is greater than max supported alignment of {ALIGN_USIZE}",
            layout.align(),
        );

        ensure!(
            layout.size() <= self.max_size(),
            "requested allocation's size of {} is greater than the max supported size of {}",
            layout.size(),
            self.max_size(),
        );

        let alloc_size = u32::try_from(layout.size())
            .context("requested allocation's size does not fit in a u32")?;
        alloc_size
            .checked_next_multiple_of(ALIGN_U32)
            .ok_or_else(|| {
                anyhow!(
                    "failed to round allocation size of {alloc_size} up to next \
                     multiple of {ALIGN_USIZE}"
                )
            })
    }

    /// Find the first free block that can hold an allocation of the given size
    /// and remove it from the free list.
    fn first_fit(&mut self, alloc_size: u32) -> Option<(u32, u32)> {
        debug_assert_eq!(alloc_size % ALIGN_U32, 0);

        let (&block_index, &block_len) = self
            .free_block_index_to_len
            .iter()
            .find(|(_idx, len)| **len >= alloc_size)?;

        debug_assert_eq!(block_index % ALIGN_U32, 0);
        debug_assert_eq!(block_len % ALIGN_U32, 0);

        let entry = self.free_block_index_to_len.remove(&block_index);
        debug_assert!(entry.is_some());

        Some((block_index, block_len))
    }

    /// If the given allocated block is large enough such that we can split it
    /// and still have enough space left for future allocations, then split it.
    ///
    /// Returns the new length of the allocated block.
    fn maybe_split(&mut self, alloc_size: u32, block_index: u32, block_len: u32) -> u32 {
        debug_assert_eq!(alloc_size % ALIGN_U32, 0);
        debug_assert_eq!(block_index % ALIGN_U32, 0);
        debug_assert_eq!(block_len % ALIGN_U32, 0);

        if block_len - alloc_size < MIN_BLOCK_SIZE {
            // The block is not large enough to split.
            return block_len;
        }

        // The block is large enough to split. Split the block at exactly the
        // requested allocation size and put the tail back in the free list.
        let new_block_len = alloc_size;
        let split_start = block_index + alloc_size;
        let split_len = block_len - alloc_size;

        debug_assert_eq!(new_block_len % ALIGN_U32, 0);
        debug_assert_eq!(split_start % ALIGN_U32, 0);
        debug_assert_eq!(split_len % ALIGN_U32, 0);

        self.free_block_index_to_len.insert(split_start, split_len);

        new_block_len
    }

    /// Allocate space for an object of the given layout.
    ///
    /// Returns:
    ///
    /// * `Ok(Some(_))`: Allocation succeeded.
    ///
    /// * `Ok(None)`: Can't currently fulfill the allocation request, but might
    ///   be able to if some stuff was reallocated.
    ///
    /// * `Err(_)`:
    pub fn alloc(&mut self, layout: Layout) -> Result<Option<NonZeroU32>> {
        let alloc_size = self.check_layout(layout)?;
        debug_assert_eq!(alloc_size % ALIGN_U32, 0);

        let (block_index, block_len) = match self.first_fit(alloc_size) {
            None => return Ok(None),
            Some(tup) => tup,
        };
        debug_assert_ne!(block_index, 0);
        debug_assert_eq!(block_index % ALIGN_U32, 0);
        debug_assert!(block_len >= alloc_size);
        debug_assert_eq!(block_len % ALIGN_U32, 0);

        let block_len = self.maybe_split(alloc_size, block_index, block_len);
        debug_assert!(block_len >= alloc_size);
        debug_assert_eq!(block_len % ALIGN_U32, 0);

        // After we've mutated the free list, double check its integrity.
        #[cfg(debug_assertions)]
        self.check_integrity();

        Ok(Some(unsafe { NonZeroU32::new_unchecked(block_index) }))
    }

    /// Deallocate an object with the given layout.
    pub fn dealloc(&mut self, index: NonZeroU32, layout: Layout) {
        let index = index.get();
        debug_assert_eq!(index % ALIGN_U32, 0);

        let alloc_size = self.check_layout(layout).unwrap();
        debug_assert_eq!(alloc_size % ALIGN_U32, 0);

        let prev_block = self
            .free_block_index_to_len
            .range((Bound::Unbounded, Bound::Excluded(index)))
            .next_back()
            .map(|(idx, len)| (*idx, *len));

        let next_block = self
            .free_block_index_to_len
            .range((Bound::Excluded(index), Bound::Unbounded))
            .next()
            .map(|(idx, len)| (*idx, *len));

        // Try and merge this block with its previous and next blocks in the
        // free list, if any and if they are contiguous.
        match (prev_block, next_block) {
            // The prev, this, and next blocks are all contiguous: merge this
            // and next into prev.
            (Some((prev_index, prev_len)), Some((next_index, next_len)))
                if blocks_are_contiguous(prev_index, prev_len, index)
                    && blocks_are_contiguous(index, alloc_size, next_index) =>
            {
                self.free_block_index_to_len.remove(&next_index);
                let merged_block_len = next_index + next_len - prev_index;
                debug_assert_eq!(merged_block_len % ALIGN_U32, 0);
                *self.free_block_index_to_len.get_mut(&prev_index).unwrap() = merged_block_len;
            }

            // The prev and this blocks are contiguous: merge this into prev.
            (Some((prev_index, prev_len)), _)
                if blocks_are_contiguous(prev_index, prev_len, index) =>
            {
                let merged_block_len = index + alloc_size - prev_index;
                debug_assert_eq!(merged_block_len % ALIGN_U32, 0);
                *self.free_block_index_to_len.get_mut(&prev_index).unwrap() = merged_block_len;
            }

            // The this and next blocks are contiguous: merge next into this.
            (_, Some((next_index, next_len)))
                if blocks_are_contiguous(index, alloc_size, next_index) =>
            {
                self.free_block_index_to_len.remove(&next_index);
                let merged_block_len = next_index + next_len - index;
                debug_assert_eq!(merged_block_len % ALIGN_U32, 0);
                self.free_block_index_to_len.insert(index, merged_block_len);
            }

            // None of the blocks are contiguous: insert this block into the
            // free list.
            (_, _) => {
                self.free_block_index_to_len.insert(index, alloc_size);
            }
        }

        // After we've added to/mutated the free list, double check its
        // integrity.
        #[cfg(debug_assertions)]
        self.check_integrity();
    }

    /// Assert that the free list is valid:
    ///
    /// 1. All blocks are within `ALIGN..self.capacity`
    ///
    /// 2. No blocks are overlapping.
    ///
    /// 3. All blocks are aligned to `ALIGN`
    ///
    /// 4. All block sizes are a multiple of `ALIGN`
    #[cfg(debug_assertions)]
    fn check_integrity(&self) {
        let mut prev_end = None;
        for (&index, &len) in self.free_block_index_to_len.iter() {
            // (1)
            let end = index + len;
            assert!(usize::try_from(end).unwrap() <= self.capacity);

            // (2)
            if let Some(prev_end) = prev_end {
                // We could assert `prev_end <= index`, and that would be
                // correct, but it would also mean that we missed an opportunity
                // to merge the previous block and this current block
                // together. We don't want to allow that kind of fragmentation,
                // so do the stricter `prev_end < index` assert here.
                assert!(prev_end < index);
            }

            // (3)
            assert_eq!(index % ALIGN_U32, 0);

            // (4)
            assert_eq!(len % ALIGN_U32, 0);

            prev_end = Some(end);
        }
    }

    /// Reset this free list, making the whole range available for allocation.
    pub fn reset(&mut self) {
        let end = u32::try_from(self.capacity).unwrap_or_else(|_| {
            assert!(self.capacity > usize::try_from(u32::MAX).unwrap());
            u32::MAX
        });

        // Don't start at `0`. Reserve that for "null pointers" and this way we
        // can use `NonZeroU32` as out pointer type, giving us some more
        // bitpacking opportunities.
        let start = ALIGN_U32;

        let len = round_u32_down_to_pow2(end.saturating_sub(start), ALIGN_U32);

        let entire_range = if len >= MIN_BLOCK_SIZE {
            Some((start, len))
        } else {
            None
        };

        self.free_block_index_to_len.clear();
        self.free_block_index_to_len.extend(entire_range);
    }
}

#[inline]
fn blocks_are_contiguous(prev_index: u32, prev_len: u32, next_index: u32) -> bool {
    // NB: We might have decided *not* to split the prev block if it was larger
    // than the requested allocation size but not large enough such that if we
    // split it, the remainder could fulfill future allocations. In such cases,
    // the size of the `Layout` given to us upon deallocation (aka `prev_len`)
    // is smaller than the actual size of the block we allocated.
    let end_of_prev = prev_index + prev_len;
    debug_assert!(next_index >= end_of_prev);
    let delta_to_next = next_index - end_of_prev;
    delta_to_next < MIN_BLOCK_SIZE
}

#[inline]
fn round_u32_down_to_pow2(value: u32, divisor: u32) -> u32 {
    debug_assert!(divisor > 0);
    debug_assert!(divisor.is_power_of_two());
    value & !(divisor - 1)
}

#[inline]
fn round_usize_down_to_pow2(value: usize, divisor: usize) -> usize {
    debug_assert!(divisor > 0);
    debug_assert!(divisor.is_power_of_two());
    value & !(divisor - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::{collections::HashMap, num::NonZeroUsize};

    fn free_list_block_len_and_size(free_list: &FreeList) -> (usize, Option<usize>) {
        let len = free_list.free_block_index_to_len.len();
        let size = free_list
            .free_block_index_to_len
            .values()
            .next()
            .map(|s| usize::try_from(*s).unwrap());
        (len, size)
    }

    proptest! {
        /// This property test ensures that `FreeList` doesn't suffer from
        /// permanent fragmentation. That is, it can always merge neighboring
        /// free blocks together into a single, larger free block that can be
        /// used to satisfy larger allocations than either of those smaller
        /// blocks could have. In the limit, once we've freed all blocks, that
        /// means we should end up with a single block that represents the whole
        /// range of memory that the `FreeList` is portioning out (just like
        /// what we started with when we initially created the `FreeList`).
        #[test]
        #[cfg_attr(miri, ignore)]
        fn check_no_fragmentation((capacity, ops) in ops()) {
            // Map from allocation id to ptr.
            let mut live = HashMap::new();

            // Set of deferred deallocations, where the strategy told us to
            // deallocate an id before it was allocated. These simply get
            // deallocated en-mass at the end.
            let mut deferred = vec![];

            // The free list we are testing.
            let mut free_list = FreeList::new(capacity.get());

            let (initial_len, initial_size) = free_list_block_len_and_size(&free_list);
            assert!(initial_len == 0 || initial_len == 1);
            assert!(initial_size.unwrap_or(0) <= capacity.get());
            assert_eq!(initial_size.unwrap_or(0), free_list.max_size());

            // Run through the generated ops and perform each operation.
            for (id, op) in ops {
                match op {
                    Op::Alloc(layout) => {
                        if let Ok(Some(ptr)) = free_list.alloc(layout) {
                            live.insert(id, ptr);
                        }
                    }
                    Op::Dealloc(layout) => {
                        if let Some(ptr) = live.remove(&id) {
                            free_list.dealloc(ptr, layout);
                        } else {
                            deferred.push((id, layout));
                        }
                    }
                }
            }

            // Now that we've completed all allocations, perform the deferred
            // deallocations.
            for (id, layout) in deferred {
                // NB: not all IDs necessarily got successful allocations, so
                // there might not be a live pointer for this ID, even after
                // we've already performed all the allocation operations.
                if let Some(ptr) = live.remove(&id) {
                    free_list.dealloc(ptr, layout);
                }
            }

            // Now we can assert various properties that should hold after we
            // have deallocated everything that was allocated.
            //
            // First, assert we did in fact deallocate everything.
            assert!(live.is_empty());

            let (final_len, final_size) = free_list_block_len_and_size(&free_list);

            // The free list should have a single chunk again (or no chunks if
            // the capacity was too small).
            assert_eq!(final_len, initial_len);

            // And the size of that chunk should be the same as the initial size.
            assert_eq!(final_size, initial_size);
        }
    }

    #[derive(Clone, Debug)]
    enum Op {
        Alloc(Layout),
        Dealloc(Layout),
    }

    /// Map an arbitrary `x` to a power of 2 that is less than or equal to
    /// `max`, but with as little bias as possible (e.g. rounding `min(x, max)`
    /// to the nearest power of 2 is unacceptable because it would majorly bias
    /// the distribution towards `max` when `max` is much smaller than
    /// `usize::MAX`).
    fn clamp_to_pow2_in_range(x: usize, max: usize) -> usize {
        let log_x = max.ilog2() as usize;
        if log_x == 0 {
            return 1;
        }
        let divisor = usize::MAX / log_x;
        let y = 1_usize << (x / divisor);
        assert!(y.is_power_of_two(), "{y} is not a power of two");
        assert!(y <= max, "{y} is larger than {max}");
        y
    }

    /// Helper to turn a pair of arbitrary `usize`s into a valid `Layout` of
    /// reasonable size for use with quickchecks.
    fn arbitrary_layout(max_size: NonZeroUsize, size: usize, align: usize) -> Layout {
        // The maximum size cannot be larger than `isize::MAX` because `Layout`
        // imposes that constraint on its size.
        let max_size = std::cmp::min(max_size.get(), usize::try_from(isize::MAX).unwrap());

        // Ensure that the alignment is a power of 2 that is less than or equal
        // to the maximum alignment that `FreeList` supports.
        let align = clamp_to_pow2_in_range(align, super::ALIGN_USIZE);

        // Ensure that `size` is less than or equal to `max_size`.
        let size = size % (max_size + 1);

        // Ensure that `size` is a multiple of `align`.
        //
        // NB: We round `size` *down* to the previous multiple of `align` to
        // preserve `size <= max_size`.
        let size = round_usize_down_to_pow2(size, align);
        assert!(size <= max_size);

        // Double check that we satisfied all of `Layout::from_size_align`'s
        // success requirements.
        assert_ne!(align, 0);
        assert!(align.is_power_of_two());
        assert_eq!(size % align, 0);
        assert!(size <= usize::try_from(isize::MAX).unwrap());

        Layout::from_size_align(size, align).unwrap()
    }

    /// Proptest strategy to generate a free list capacity and a series of
    /// allocation operations to perform in a free list of that capacity.
    fn ops() -> impl Strategy<Value = (NonZeroUsize, Vec<(usize, Op)>)> {
        any::<usize>().prop_flat_map(|capacity| {
            let capacity =
                NonZeroUsize::new(capacity).unwrap_or_else(|| NonZeroUsize::new(1 << 31).unwrap());

            (
                Just(capacity),
                (any::<usize>(), any::<usize>(), any::<usize>())
                    .prop_flat_map(move |(id, size, align)| {
                        let layout = arbitrary_layout(capacity, size, align);
                        vec![
                            Just((id, Op::Alloc(layout))),
                            Just((id, Op::Dealloc(layout))),
                        ]
                    })
                    .prop_shuffle(),
            )
        })
    }

    #[test]
    fn allocate_no_split() {
        // Create a free list with the capacity to allocate two blocks of size
        // `MIN_BLOCK_SIZE`.
        let mut free_list =
            FreeList::new(ALIGN_USIZE + usize::try_from(MIN_BLOCK_SIZE).unwrap() * 2);

        assert_eq!(free_list.free_block_index_to_len.len(), 1);
        assert_eq!(
            free_list.max_size(),
            usize::try_from(MIN_BLOCK_SIZE).unwrap() * 2
        );

        // Allocate a block such that the remainder is not worth splitting.
        free_list
            .alloc(
                Layout::from_size_align(
                    usize::try_from(MIN_BLOCK_SIZE).unwrap() + ALIGN_USIZE,
                    ALIGN_USIZE,
                )
                .unwrap(),
            )
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");

        // Should not have split the block.
        assert_eq!(free_list.free_block_index_to_len.len(), 0);
    }

    #[test]
    fn allocate_and_split() {
        // Create a free list with the capacity to allocate three blocks of size
        // `MIN_BLOCK_SIZE`.
        let mut free_list =
            FreeList::new(ALIGN_USIZE + usize::try_from(MIN_BLOCK_SIZE).unwrap() * 3);

        assert_eq!(free_list.free_block_index_to_len.len(), 1);
        assert_eq!(
            free_list.max_size(),
            usize::try_from(MIN_BLOCK_SIZE).unwrap() * 3
        );

        // Allocate a block such that the remainder is not worth splitting.
        free_list
            .alloc(
                Layout::from_size_align(
                    usize::try_from(MIN_BLOCK_SIZE).unwrap() + ALIGN_USIZE,
                    ALIGN_USIZE,
                )
                .unwrap(),
            )
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");

        // Should have split the block.
        assert_eq!(free_list.free_block_index_to_len.len(), 1);
    }

    #[test]
    fn dealloc_merge_prev_and_next() {
        let layout =
            Layout::from_size_align(usize::try_from(MIN_BLOCK_SIZE).unwrap(), ALIGN_USIZE).unwrap();

        let mut free_list =
            FreeList::new(ALIGN_USIZE + usize::try_from(MIN_BLOCK_SIZE).unwrap() * 100);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "initially one big free block"
        );

        let a = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "should have split the block to allocate `a`"
        );

        let b = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "should have split the block to allocate `b`"
        );

        free_list.dealloc(a, layout);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            2,
            "should have two non-contiguous free blocks after deallocating `a`"
        );

        free_list.dealloc(b, layout);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "should have merged `a` and `b` blocks with the rest to form a \
             single, contiguous free block after deallocating `b`"
        );
    }

    #[test]
    fn dealloc_merge_with_prev_and_not_next() {
        let layout =
            Layout::from_size_align(usize::try_from(MIN_BLOCK_SIZE).unwrap(), ALIGN_USIZE).unwrap();

        let mut free_list =
            FreeList::new(ALIGN_USIZE + usize::try_from(MIN_BLOCK_SIZE).unwrap() * 100);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "initially one big free block"
        );

        let a = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        let b = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        let c = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "should have split the block to allocate `a`, `b`, and `c`"
        );

        free_list.dealloc(a, layout);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            2,
            "should have two non-contiguous free blocks after deallocating `a`"
        );

        free_list.dealloc(b, layout);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            2,
            "should have merged `a` and `b` blocks, but not merged with the \
             rest of the free space"
        );

        let _ = c;
    }

    #[test]
    fn dealloc_merge_with_next_and_not_prev() {
        let layout =
            Layout::from_size_align(usize::try_from(MIN_BLOCK_SIZE).unwrap(), ALIGN_USIZE).unwrap();

        let mut free_list =
            FreeList::new(ALIGN_USIZE + usize::try_from(MIN_BLOCK_SIZE).unwrap() * 100);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "initially one big free block"
        );

        let a = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        let b = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        let c = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "should have split the block to allocate `a`, `b`, and `c`"
        );

        free_list.dealloc(a, layout);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            2,
            "should have two non-contiguous free blocks after deallocating `a`"
        );

        free_list.dealloc(c, layout);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            2,
            "should have merged `c` block with rest of the free space, but not \
             with `a` block"
        );

        let _ = b;
    }

    #[test]
    fn dealloc_no_merge() {
        let layout =
            Layout::from_size_align(usize::try_from(MIN_BLOCK_SIZE).unwrap(), ALIGN_USIZE).unwrap();

        let mut free_list =
            FreeList::new(ALIGN_USIZE + usize::try_from(MIN_BLOCK_SIZE).unwrap() * 100);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "initially one big free block"
        );

        let a = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        let b = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        let c = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        let d = free_list
            .alloc(layout)
            .expect("allocation within 'static' free list limits")
            .expect("have free space available for allocation");
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            1,
            "should have split the block to allocate `a`, `b`, `c`, and `d`"
        );

        free_list.dealloc(a, layout);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            2,
            "should have two non-contiguous free blocks after deallocating `a`"
        );

        free_list.dealloc(c, layout);
        assert_eq!(
            free_list.free_block_index_to_len.len(),
            3,
            "should not have merged `c` block `a` block or rest of the free \
             space"
        );

        let _ = (b, d);
    }

    #[test]
    fn alloc_size_too_large() {
        // Free list with room for 10 min-sized blocks.
        let mut free_list =
            FreeList::new(ALIGN_USIZE + usize::try_from(MIN_BLOCK_SIZE).unwrap() * 10);
        assert_eq!(
            free_list.max_size(),
            usize::try_from(MIN_BLOCK_SIZE).unwrap() * 10
        );

        // Attempt to allocate something that is 20 times the size of our
        // min-sized block.
        assert!(free_list
            .alloc(
                Layout::from_size_align(usize::try_from(MIN_BLOCK_SIZE).unwrap() * 20, ALIGN_USIZE)
                    .unwrap(),
            )
            .is_err());
    }

    #[test]
    fn alloc_align_too_large() {
        // Free list with room for 10 min-sized blocks.
        let mut free_list =
            FreeList::new(ALIGN_USIZE + usize::try_from(MIN_BLOCK_SIZE).unwrap() * 10);
        assert_eq!(
            free_list.max_size(),
            usize::try_from(MIN_BLOCK_SIZE).unwrap() * 10
        );

        // Attempt to allocate something that requires larger alignment than
        // `FreeList` supports.
        assert!(free_list
            .alloc(
                Layout::from_size_align(usize::try_from(MIN_BLOCK_SIZE).unwrap(), ALIGN_USIZE * 2)
                    .unwrap(),
            )
            .is_err());
    }
}

use crate::InstanceState;
use crate::info::ModuleContext;
#[cfg(not(feature = "rayon"))]
use crate::rayoff::{IntoParallelIterator, ParallelExtend};
#[cfg(feature = "rayon")]
use rayon::iter::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use std::convert::TryFrom;
use std::ops::Range;

/// The maximum number of data segments that we will emit. Most
/// engines support more than this, but we want to leave some
/// headroom.
const MAX_DATA_SEGMENTS: usize = 10_000;

/// A "snapshot" of Wasm state from its default value after having been initialized.
pub struct Snapshot {
    /// Maps global index to its initialized value.
    ///
    /// Note that this only tracks defined mutable globals, not all globals.
    pub globals: Vec<(u32, SnapshotVal)>,

    /// A new minimum size for each memory (in units of pages).
    pub memory_mins: Vec<u64>,

    /// Segments of non-zero memory.
    pub data_segments: Vec<DataSegment>,
}

/// A value from a snapshot, currently a subset of wasm types that aren't
/// reference types.
#[expect(missing_docs, reason = "self-describing variants")]
pub enum SnapshotVal {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
    V128(u128),
}

/// A data segment initializer for a memory.
#[derive(Clone)]
pub struct DataSegment {
    /// The index of this data segment's memory.
    pub memory_index: u32,

    /// This data segment's initialized memory that it originated from.
    pub data: Vec<u8>,

    /// The offset within the memory that `data` should be copied to.
    pub offset: u64,

    /// Whether or not `memory_index` is a 64-bit memory.
    pub is64: bool,
}

/// Snapshot the given instance's globals, memories, and instances from the Wasm
/// defaults.
//
// TODO: when we support reference types, we will have to snapshot tables.
pub async fn snapshot(module: &ModuleContext<'_>, ctx: &mut impl InstanceState) -> Snapshot {
    log::debug!("Snapshotting the initialized state");

    let globals = snapshot_globals(module, ctx).await;
    let (memory_mins, data_segments) = snapshot_memories(module, ctx).await;

    Snapshot {
        globals,
        memory_mins,
        data_segments,
    }
}

/// Get the initialized values of all globals.
async fn snapshot_globals(
    module: &ModuleContext<'_>,
    ctx: &mut impl InstanceState,
) -> Vec<(u32, SnapshotVal)> {
    log::debug!("Snapshotting global values");

    let mut ret = Vec::new();
    for (i, name) in module.defined_global_exports.as_ref().unwrap().iter() {
        let val = ctx.global_get(&name).await;
        ret.push((*i, val));
    }
    ret
}

#[derive(Clone)]
struct DataSegmentRange {
    memory_index: u32,
    range: Range<usize>,
}

impl DataSegmentRange {
    /// What is the gap between two consecutive data segments?
    ///
    /// `self` must be in front of `other` and they must not overlap with each
    /// other.
    fn gap(&self, other: &Self) -> usize {
        debug_assert_eq!(self.memory_index, other.memory_index);
        debug_assert!(self.range.end <= other.range.start);
        other.range.start - self.range.end
    }

    /// Merge two consecutive data segments.
    ///
    /// `self` must be in front of `other` and they must not overlap with each
    /// other.
    fn merge(&mut self, other: &Self) {
        debug_assert_eq!(self.memory_index, other.memory_index);
        debug_assert!(self.range.end <= other.range.start);
        self.range.end = other.range.end;
    }
}

/// Find the initialized minimum page size of each memory, as well as all
/// regions of non-zero memory.
async fn snapshot_memories(
    module: &ModuleContext<'_>,
    instance: &mut impl InstanceState,
) -> (Vec<u64>, Vec<DataSegment>) {
    log::debug!("Snapshotting memories");

    // Find and record non-zero regions of memory (in parallel).
    let mut memory_mins = vec![];
    let mut data_segments = vec![];
    let iter = module
        .defined_memories()
        .zip(module.defined_memory_exports.as_ref().unwrap());
    for ((memory_index, ty), name) in iter {
        instance
            .memory_contents(&name, |memory| {
                let page_size = 1 << ty.page_size_log2.unwrap_or(16);
                let num_wasm_pages = memory.len() / page_size;
                memory_mins.push(num_wasm_pages as u64);

                let memory_data = &memory[..];

                // Consider each Wasm page in parallel. Create data segments for each
                // region of non-zero memory.
                data_segments.par_extend((0..num_wasm_pages).into_par_iter().flat_map(|i| {
                    let page_end = (i + 1) * page_size;
                    let mut start = i * page_size;
                    let mut segments = vec![];
                    while start < page_end {
                        let nonzero = match memory_data[start..page_end]
                            .iter()
                            .position(|byte| *byte != 0)
                        {
                            None => break,
                            Some(i) => i,
                        };
                        start += nonzero;
                        let end = memory_data[start..page_end]
                            .iter()
                            .position(|byte| *byte == 0)
                            .map_or(page_end, |zero| start + zero);
                        segments.push(DataSegmentRange {
                            memory_index,
                            range: start..end,
                        });
                        start = end;
                    }
                    segments
                }));
            })
            .await;
    }

    if data_segments.is_empty() {
        return (memory_mins, Vec::new());
    }

    // Sort data segments to enforce determinism in the face of the
    // parallelism above.
    data_segments.sort_by_key(|s| (s.memory_index, s.range.start));

    // Merge any contiguous segments (caused by spanning a Wasm page boundary,
    // and therefore created in separate logical threads above) or pages that
    // are within four bytes of each other. Four because this is the minimum
    // overhead of defining a new active data segment: one for the memory index
    // LEB, two for the memory offset init expression (one for the `i32.const`
    // opcode and another for the constant immediate LEB), and finally one for
    // the data length LEB).
    const MIN_ACTIVE_SEGMENT_OVERHEAD: usize = 4;
    let mut merged_data_segments = Vec::with_capacity(data_segments.len());
    merged_data_segments.push(data_segments[0].clone());
    for b in &data_segments[1..] {
        let a = merged_data_segments.last_mut().unwrap();

        // Only merge segments for the same memory.
        if a.memory_index != b.memory_index {
            merged_data_segments.push(b.clone());
            continue;
        }

        // Only merge segments if they are contiguous or if it is definitely
        // more size efficient than leaving them apart.
        let gap = a.gap(b);
        if gap > MIN_ACTIVE_SEGMENT_OVERHEAD {
            merged_data_segments.push(b.clone());
            continue;
        }

        // Okay, merge them together into `a` (so that the next iteration can
        // merge it with its predecessor) and then omit `b`!
        a.merge(b);
    }

    remove_excess_segments(&mut merged_data_segments);

    // With the final set of data segments now extract the actual data of each
    // memory, copying it into a `DataSegment`, to return the final list of
    // segments.
    //
    // Here the memories are iterated over again and, in tandem, the
    // `merged_data_segments` list is traversed to extract a `DataSegment` for
    // each range that `merged_data_segments` indicates. This relies on
    // `merged_data_segments` being a sorted list by `memory_index` at least.
    let mut final_data_segments = Vec::with_capacity(merged_data_segments.len());
    let mut merged = merged_data_segments.iter().peekable();
    let iter = module
        .defined_memories()
        .zip(module.defined_memory_exports.as_ref().unwrap());
    for ((memory_index, ty), name) in iter {
        instance
            .memory_contents(&name, |memory| {
                while let Some(segment) = merged.next_if(|s| s.memory_index == memory_index) {
                    final_data_segments.push(DataSegment {
                        memory_index,
                        data: memory[segment.range.clone()].to_vec(),
                        offset: segment.range.start.try_into().unwrap(),
                        is64: ty.memory64,
                    });
                }
            })
            .await;
    }
    assert!(merged.next().is_none());

    (memory_mins, final_data_segments)
}

/// Engines apply a limit on how many segments a module may contain, and Wizer
/// can run afoul of it. When that happens, we need to merge data segments
/// together until our number of data segments fits within the limit.
fn remove_excess_segments(merged_data_segments: &mut Vec<DataSegmentRange>) {
    if merged_data_segments.len() < MAX_DATA_SEGMENTS {
        return;
    }

    // We need to remove `excess` number of data segments.
    let excess = merged_data_segments.len() - MAX_DATA_SEGMENTS;

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct GapIndex {
        gap: u32,
        // Use a `u32` instead of `usize` to fit `GapIndex` within a word on
        // 64-bit systems, using less memory.
        index: u32,
    }

    // Find the gaps between the start of one segment and the next (if they are
    // both in the same memory). We will merge the `excess` segments with the
    // smallest gaps together. Because they are the smallest gaps, this will
    // bloat the size of our data segment the least.
    let mut smallest_gaps = Vec::with_capacity(merged_data_segments.len() - 1);
    for (index, w) in merged_data_segments.windows(2).enumerate() {
        if w[0].memory_index != w[1].memory_index {
            continue;
        }
        let gap = match u32::try_from(w[0].gap(&w[1])) {
            Ok(gap) => gap,
            // If the gap is larger than 4G then don't consider these two data
            // segments for merging and assume there's enough other data
            // segments close enough together to still consider for merging to
            // get under the limit.
            Err(_) => continue,
        };
        let index = u32::try_from(index).unwrap();
        smallest_gaps.push(GapIndex { gap, index });
    }
    smallest_gaps.sort_unstable_by_key(|g| g.gap);
    smallest_gaps.truncate(excess);

    // Now merge the chosen segments together in reverse index order so that
    // merging two segments doesn't mess up the index of the next segments we
    // will to merge.
    smallest_gaps.sort_unstable_by(|a, b| a.index.cmp(&b.index).reverse());
    for GapIndex { index, .. } in smallest_gaps {
        let index = usize::try_from(index).unwrap();
        let [a, b] = merged_data_segments
            .get_disjoint_mut([index, index + 1])
            .unwrap();
        a.merge(b);

        // Okay to use `swap_remove` here because, even though it makes
        // `merged_data_segments` unsorted, the segments are still sorted within
        // the range `0..index` and future iterations will only operate within
        // that subregion because we are iterating over largest to smallest
        // indices.
        merged_data_segments.swap_remove(index + 1);
    }

    // Finally, sort the data segments again so that our output is
    // deterministic.
    merged_data_segments.sort_by_key(|s| (s.memory_index, s.range.start));
}

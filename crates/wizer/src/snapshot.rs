use rayon::iter::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use std::{collections::BinaryHeap, convert::TryFrom};

const WASM_PAGE_SIZE: u32 = 65_536;

/// The maximum number of data segments that most engines support.
const MAX_DATA_SEGMENTS: usize = 100_000;

/// A "snapshot" of Wasm state from its default value after having been initialized.
pub struct Snapshot<'a> {
    /// Maps global index to its initialized value.
    pub globals: Vec<wasmtime::Val>,

    /// A new minimum size for each memory (in units of pages).
    pub memory_mins: Vec<u32>,

    /// Segments of non-zero memory.
    pub data_segments: Vec<DataSegment<'a>>,

    /// Snapshots for each nested instantiation.
    pub instantiations: Vec<Snapshot<'a>>,
}

/// A data segment initializer for a memory.
#[derive(Clone, Copy)]
pub struct DataSegment<'a> {
    /// The index of this data segment's memory.
    pub memory_index: u32,
    /// The offset within the memory that `data` should be copied to.
    pub offset: u32,
    /// This segment's (non-zero) data.
    pub data: &'a [u8],
}

impl<'a> DataSegment<'a> {
    /// What is the gap between two consecutive data segments?
    ///
    /// `self` must be in front of `other` and they must not overlap with each
    /// other.
    fn gap(&self, other: &Self) -> u32 {
        let self_len = u32::try_from(self.data.len()).unwrap();
        debug_assert!(self.offset + self_len <= other.offset);
        other.offset - (self.offset + self_len)
    }

    /// Get a slice of the merged data for two consecutive data segments.
    ///
    /// # Safety
    ///
    /// Both `self` and `other` must be segments whose data are from the same
    /// Wasm linear memory.
    ///
    /// `self` must be in front of `other` and they must not overlap with each
    /// other.
    unsafe fn merge(&self, other: &Self) -> &'a [u8] {
        debug_assert_eq!(self.memory_index, other.memory_index);

        let gap = self.gap(other);
        let gap = usize::try_from(gap).unwrap();

        debug_assert_eq!(
            self.data
                .as_ptr()
                .offset(isize::try_from(self.data.len() + gap).unwrap()),
            other.data.as_ptr(),
        );
        std::slice::from_raw_parts(self.data.as_ptr(), self.data.len() + gap + other.data.len())
    }
}

/// Snapshot the given instance's globals, memories, and instances from the Wasm
/// defaults.
//
// TODO: when we support reference types, we will have to snapshot tables.
pub fn snapshot<'a>(store: &'a wasmtime::Store, instance: &wasmtime::Instance) -> Snapshot<'a> {
    log::debug!("Snapshotting the initialized state");

    assert!(wasmtime::Store::same(store, &instance.store()));

    let globals = snapshot_globals(instance);
    let (memory_mins, data_segments) = snapshot_memories(instance);
    let instantiations = snapshot_instantiations(store, instance);

    Snapshot {
        globals,
        memory_mins,
        data_segments,
        instantiations,
    }
}

/// Get the initialized values of all globals.
fn snapshot_globals(instance: &wasmtime::Instance) -> Vec<wasmtime::Val> {
    log::debug!("Snapshotting global values");
    let mut globals = vec![];
    let mut index = 0;
    loop {
        let name = format!("__wizer_global_{}", index);
        match instance.get_global(&name) {
            None => break,
            Some(global) => {
                globals.push(global.get());
                index += 1;
            }
        }
    }
    globals
}

/// Find the initialized minimum page size of each memory, as well as all
/// regions of non-zero memory.
fn snapshot_memories<'a>(instance: &wasmtime::Instance) -> (Vec<u32>, Vec<DataSegment<'a>>) {
    log::debug!("Snapshotting memories");

    // Find and record non-zero regions of memory (in parallel).
    let mut memory_mins = vec![];
    let mut data_segments = vec![];
    let mut memory_index = 0;
    loop {
        let name = format!("__wizer_memory_{}", memory_index);
        let memory = match instance.get_memory(&name) {
            None => break,
            Some(memory) => memory,
        };
        memory_mins.push(memory.size());

        let num_wasm_pages = memory.size();

        let memory: &'a [u8] = unsafe {
            // Safe because no one else has a (potentially mutable)
            // view to this memory and we know the memory will live
            // as long as the store is alive.
            std::slice::from_raw_parts(memory.data_ptr(), memory.data_size())
        };

        // Consider each Wasm page in parallel. Create data segments for each
        // region of non-zero memory.
        data_segments.par_extend((0..num_wasm_pages).into_par_iter().flat_map(|i| {
            let page_end = ((i + 1) * WASM_PAGE_SIZE) as usize;
            let mut start = (i * WASM_PAGE_SIZE) as usize;
            let mut segments = vec![];
            while start < page_end {
                let nonzero = match memory[start..page_end].iter().position(|byte| *byte != 0) {
                    None => break,
                    Some(i) => i,
                };
                start += nonzero;
                let end = memory[start..page_end]
                    .iter()
                    .position(|byte| *byte == 0)
                    .map_or(page_end, |zero| start + zero);
                segments.push(DataSegment {
                    memory_index,
                    offset: start as u32,
                    data: &memory[start..end],
                });
                start = end;
            }
            segments
        }));

        memory_index += 1;
    }

    if data_segments.is_empty() {
        return (memory_mins, data_segments);
    }

    // Sort data segments to enforce determinism in the face of the
    // parallelism above.
    data_segments.sort_by_key(|s| (s.memory_index, s.offset));

    // Merge any contiguous segments (caused by spanning a Wasm page boundary,
    // and therefore created in separate logical threads above) or pages that
    // are within four bytes of each other. Four because this is the minimum
    // overhead of defining a new active data segment: one for the memory index
    // LEB, two for the memory offset init expression (one for the `i32.const`
    // opcode and another for the constant immediate LEB), and finally one for
    // the data length LEB).
    const MIN_ACTIVE_SEGMENT_OVERHEAD: u32 = 4;
    let mut merged_data_segments = Vec::with_capacity(data_segments.len());
    merged_data_segments.push(data_segments[0]);
    for b in &data_segments[1..] {
        let a = merged_data_segments.last_mut().unwrap();

        // Only merge segments for the same memory.
        if a.memory_index != b.memory_index {
            merged_data_segments.push(*b);
            continue;
        }

        // Only merge segments if they are contiguous or if it is definitely
        // more size efficient than leaving them apart.
        let gap = a.gap(b);
        if gap > MIN_ACTIVE_SEGMENT_OVERHEAD {
            merged_data_segments.push(*b);
            continue;
        }

        // Okay, merge them together into `a` (so that the next iteration can
        // merge it with its predecessor) and then omit `b`!
        let merged_data = unsafe { a.merge(b) };
        a.data = merged_data;
    }

    // Engines apply a limit on how many segments a module may contain, and
    // Wizer can run afoul of it. In this case, we need to merge data segments
    // together until our number of data segments fits within the limit.
    if merged_data_segments.len() >= MAX_DATA_SEGMENTS {
        // We need to remove `excess` data segments. Find the `excess` smallest
        // gaps between the start of one segment and the next. We will merge
        // these segments together. Because they are the smallest gaps, this
        // will bloat the size of our data segment the least.
        let excess = merged_data_segments.len() - MAX_DATA_SEGMENTS;

        #[derive(Clone, Copy, PartialEq, Eq)]
        struct GapIndex {
            gap: u32,
            index: usize,
        }
        impl Ord for GapIndex {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                // NB: Bigger gaps first.
                other
                    .gap
                    .cmp(&self.gap)
                    .then_with(|| self.index.cmp(&other.index))
            }
        }
        impl PartialOrd for GapIndex {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut smallest_gaps = BinaryHeap::<GapIndex>::with_capacity(excess + 1);
        for (i, w) in merged_data_segments.windows(2).enumerate() {
            debug_assert!(smallest_gaps.len() <= excess);
            let gap = w[0].gap(&w[1]);
            smallest_gaps.push(GapIndex { gap, index: i });
            if smallest_gaps.len() > excess {
                let entry = smallest_gaps.pop();
                debug_assert!(entry.unwrap().gap >= gap);
                debug_assert_eq!(smallest_gaps.len(), excess);
            }
        }

        // Now merge the chosen segments together in reverse order so that
        // merging two segments doesn't mess up the index of the next segments
        // we will to merge.
        let mut indices: Vec<_> = smallest_gaps.into_iter().map(|g| g.index).collect();
        indices.sort_unstable_by(|a, b| a.cmp(b).reverse());
        for i in indices {
            let merged_data =
                unsafe { merged_data_segments[i].merge(&merged_data_segments[i + 1]) };
            merged_data_segments[i].data = merged_data;
            merged_data_segments.remove(i + 1);
        }
    }

    (memory_mins, merged_data_segments)
}

fn snapshot_instantiations<'a>(
    store: &'a wasmtime::Store,
    instance: &wasmtime::Instance,
) -> Vec<Snapshot<'a>> {
    log::debug!("Snapshotting nested instantiations");
    let mut instantiations = vec![];
    loop {
        let name = format!("__wizer_instance_{}", instantiations.len());
        match instance.get_export(&name) {
            None => break,
            Some(wasmtime::Extern::Instance(instance)) => {
                instantiations.push(snapshot(store, &instance));
            }
            Some(_) => unreachable!(),
        }
    }
    instantiations
}

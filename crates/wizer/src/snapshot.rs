use rayon::iter::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use std::convert::TryFrom;

const WASM_PAGE_SIZE: u32 = 65_536;
const NATIVE_PAGE_SIZE: u32 = 4_096;

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
pub struct DataSegment<'a> {
    /// The index of this data segment's memory.
    pub memory_index: u32,
    /// The offset within the memory that `data` should be copied to.
    pub offset: u32,
    /// This segment's (non-zero) data.
    pub data: &'a [u8],
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
        match instance.get_memory(&name) {
            None => break,
            Some(memory) => {
                memory_mins.push(memory.size());

                let num_wasm_pages = memory.size();
                let num_native_pages = num_wasm_pages * (WASM_PAGE_SIZE / NATIVE_PAGE_SIZE);

                let memory: &'a [u8] = unsafe {
                    // Safe because no one else has a (potentially mutable)
                    // view to this memory and we know the memory will live
                    // as long as the store is alive.
                    std::slice::from_raw_parts(memory.data_ptr(), memory.data_size())
                };

                // Consider each "native" page of the memory. (Scare quotes
                // because we have no guarantee that anyone isn't using huge
                // page sizes or something). Process each page in
                // parallel. If any byte has changed, add the whole page as
                // a data segment. This means that the resulting Wasm module
                // should instantiate faster, since there are fewer segments
                // to bounds check on instantiation. Engines could even
                // theoretically recognize that each of these segments is
                // page sized and aligned, and use lazy copy-on-write
                // initialization of each instance's memory.
                data_segments.par_extend((0..num_native_pages).into_par_iter().filter_map(|i| {
                    let start = i * NATIVE_PAGE_SIZE;
                    let end = ((i + 1) * NATIVE_PAGE_SIZE) as usize;
                    let page = &memory[start as usize..end];
                    for byte in page {
                        if *byte != 0 {
                            return Some(DataSegment {
                                memory_index,
                                offset: start as u32,
                                data: page,
                            });
                        }
                    }
                    None
                }));

                memory_index += 1;
            }
        }
    }

    // Sort data segments to enforce determinism in the face of the
    // parallelism above.
    data_segments.sort_by_key(|s| (s.memory_index, s.offset));

    // Merge any contiguous pages, so that the engine can initialize them
    // all at once (ideally with a single copy-on-write `mmap`) rather than
    // initializing each data segment individually.
    for i in (1..data_segments.len()).rev() {
        let a = &data_segments[i - 1];
        let b = &data_segments[i];

        // Only merge segments for the same memory.
        if a.memory_index != b.memory_index {
            continue;
        }

        // Only merge segments if they are contiguous.
        if a.offset + u32::try_from(a.data.len()).unwrap() != b.offset {
            continue;
        }

        // Okay, merge them together into `a` (so that the next iteration
        // can merge it with its predecessor) and then remove `b`!
        data_segments[i - 1].data = unsafe {
            debug_assert_eq!(
                a.data
                    .as_ptr()
                    .offset(isize::try_from(a.data.len()).unwrap()),
                b.data.as_ptr()
            );
            std::slice::from_raw_parts(a.data.as_ptr(), a.data.len() + b.data.len())
        };
        data_segments.remove(i);
    }

    (memory_mins, data_segments)
}

fn snapshot_instantiations<'a>(
    store: &'a wasmtime::Store,
    instance: &wasmtime::Instance,
) -> Vec<Snapshot<'a>> {
    log::debug!("Snapshotting nested instantiations");
    let mut instantiations = vec![];
    let mut index = 0;
    loop {
        let name = format!("__wizer_instance_{}", index);
        match instance.get_export(&name) {
            None => break,
            Some(wasmtime::Extern::Instance(instance)) => {
                instantiations.push(snapshot(store, &instance));
                index += 1;
            }
            Some(_) => unreachable!(),
        }
    }
    instantiations
}

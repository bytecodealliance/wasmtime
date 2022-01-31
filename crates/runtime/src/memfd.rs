//! memfd support.

use anyhow::Result;
use memfd::{Memfd, MemfdOptions};
use rustix::fs::FileExt;
use std::convert::TryFrom;
use std::sync::Arc;
use wasmtime_environ::{
    DefinedMemoryIndex, MemoryInitialization, MemoryInitializer, MemoryPlan, Module, PrimaryMap,
};

/// MemFDs containing backing images for certain memories in a module.
///
/// This is meant to be built once, when a module is first
/// loaded/constructed, and then used many times for instantiation.
pub struct ModuleMemFds {
    memories: PrimaryMap<DefinedMemoryIndex, Option<Arc<MemoryMemFd>>>,
}

const MAX_MEMFD_IMAGE_SIZE: u64 = 1024 * 1024 * 1024; // limit to 1GiB.

impl ModuleMemFds {
    pub(crate) fn get_memory_image(
        &self,
        defined_index: DefinedMemoryIndex,
    ) -> Option<&Arc<MemoryMemFd>> {
        self.memories[defined_index].as_ref()
    }
}

/// One backing image for one memory.
#[derive(Debug)]
pub struct MemoryMemFd {
    /// The actual memfd image: an anonymous file in memory which we
    /// use as the backing content for a copy-on-write (CoW) mapping
    /// in the memory region.
    pub fd: Memfd,
    /// Length of image. Note that initial memory size may be larger;
    /// leading and trailing zeroes are truncated (handled by
    /// anonymous backing memfd).
    pub len: usize,
    /// Image starts this many bytes into heap space. Note that the
    /// memfd's offsets are always equal to the heap offsets, so we
    /// map at an offset into the fd as well. (This simplifies
    /// construction.)
    pub offset: usize,
}

fn unsupported_initializer(segment: &MemoryInitializer, plan: &MemoryPlan) -> bool {
    // If the segment has a base that is dynamically determined
    // (by a global value, which may be a function of an imported
    // module, for example), then we cannot build a single static
    // image that is used for every instantiation. So we skip this
    // memory entirely.
    let end = match segment.end() {
        None => {
            return true;
        }
        Some(end) => end,
    };

    // Cannot be out-of-bounds. If there is a *possibility* it may
    // be, then we just fall back on ordinary initialization.
    if plan.initializer_possibly_out_of_bounds(segment) {
        return true;
    }

    // Must fit in our max size.
    if end > MAX_MEMFD_IMAGE_SIZE {
        return true;
    }

    false
}

impl ModuleMemFds {
    /// Create a new `ModuleMemFds` for the given module. This can be
    /// passed in as part of a `InstanceAllocationRequest` to speed up
    /// instantiation and execution by using memfd-backed memories.
    pub fn new(module: &Module, wasm_data: &[u8]) -> Result<Option<Arc<ModuleMemFds>>> {
        let page_size = region::page::size() as u64;
        let num_defined_memories = module.memory_plans.len() - module.num_imported_memories;

        // Allocate a memfd file initially for every memory. We'll
        // release those and set `excluded_memories` for those that we
        // determine during initializer processing we cannot support a
        // static image (e.g. due to dynamically-located segments).
        let mut memfds: PrimaryMap<DefinedMemoryIndex, Option<Memfd>> = PrimaryMap::default();
        let mut sizes: PrimaryMap<DefinedMemoryIndex, u64> = PrimaryMap::default();
        let mut excluded_memories: PrimaryMap<DefinedMemoryIndex, bool> = PrimaryMap::new();

        for _ in 0..num_defined_memories {
            memfds.push(None);
            sizes.push(0);
            excluded_memories.push(false);
        }

        fn create_memfd() -> Result<Memfd> {
            // Create the memfd. It needs a name, but the
            // documentation for `memfd_create()` says that names can
            // be duplicated with no issues.
            MemfdOptions::new()
                .allow_sealing(true)
                .create("wasm-memory-image")
                .map_err(|e| e.into())
        }
        let round_up_page = |len: u64| (len + page_size - 1) & !(page_size - 1);

        match &module.memory_initialization {
            &MemoryInitialization::Segmented(ref segments) => {
                for (i, segment) in segments.iter().enumerate() {
                    let defined_memory = match module.defined_memory_index(segment.memory_index) {
                        Some(defined_memory) => defined_memory,
                        None => continue,
                    };
                    if excluded_memories[defined_memory] {
                        continue;
                    }

                    if unsupported_initializer(segment, &module.memory_plans[segment.memory_index])
                    {
                        memfds[defined_memory] = None;
                        excluded_memories[defined_memory] = true;
                        continue;
                    }

                    if memfds[defined_memory].is_none() {
                        memfds[defined_memory] = Some(create_memfd()?);
                    }
                    let memfd = memfds[defined_memory].as_mut().unwrap();

                    let end = round_up_page(segment.end().expect("must have statically-known end"));
                    if end > sizes[defined_memory] {
                        sizes[defined_memory] = end;
                        memfd.as_file().set_len(end)?;
                    }

                    let base = segments[i].offset;
                    let data = &wasm_data[segment.data.start as usize..segment.data.end as usize];
                    memfd.as_file().write_at(data, base)?;
                }
            }
            &MemoryInitialization::Paged { ref map, .. } => {
                for (defined_memory, pages) in map {
                    let top = pages
                        .iter()
                        .map(|(base, range)| *base + range.len() as u64)
                        .max()
                        .unwrap_or(0);

                    let memfd = create_memfd()?;
                    memfd.as_file().set_len(top)?;

                    for (base, range) in pages {
                        let data = &wasm_data[range.start as usize..range.end as usize];
                        memfd.as_file().write_at(data, *base)?;
                    }

                    memfds[defined_memory] = Some(memfd);
                    sizes[defined_memory] = top;
                }
            }
        }

        // Now finalize each memory.
        let mut memories: PrimaryMap<DefinedMemoryIndex, Option<Arc<MemoryMemFd>>> =
            PrimaryMap::default();
        for (defined_memory, maybe_memfd) in memfds {
            let memfd = match maybe_memfd {
                Some(memfd) => memfd,
                None => {
                    memories.push(None);
                    continue;
                }
            };
            let size = sizes[defined_memory];

            // Find leading and trailing zero data so that the mmap
            // can precisely map only the nonzero data; anon-mmap zero
            // memory is faster for anything that doesn't actually
            // have content.
            let mut page_data = vec![0; page_size as usize];
            let mut page_is_nonzero = |page| {
                let offset = page_size * page;
                memfd.as_file().read_at(&mut page_data[..], offset).unwrap();
                page_data.iter().any(|byte| *byte != 0)
            };
            let n_pages = size / page_size;

            let mut offset = 0;
            for page in 0..n_pages {
                if page_is_nonzero(page) {
                    break;
                }
                offset += page_size;
            }
            let len = if offset == size {
                0
            } else {
                let mut len = 0;
                for page in (0..n_pages).rev() {
                    if page_is_nonzero(page) {
                        len = (page + 1) * page_size - offset;
                        break;
                    }
                }
                len
            };

            // Seal the memfd's data and length.
            //
            // This is a defense-in-depth security mitigation. The
            // memfd will serve as the starting point for the heap of
            // every instance of this module. If anything were to
            // write to this, it could affect every execution. The
            // memfd object itself is owned by the machinery here and
            // not exposed elsewhere, but it is still an ambient open
            // file descriptor at the syscall level, so some other
            // vulnerability that allowed writes to arbitrary fds
            // could modify it. Or we could have some issue with the
            // way that we map it into each instance. To be
            // extra-super-sure that it never changes, and because
            // this costs very little, we use the kernel's "seal" API
            // to make the memfd image permanently read-only.
            memfd.add_seal(memfd::FileSeal::SealGrow)?;
            memfd.add_seal(memfd::FileSeal::SealShrink)?;
            memfd.add_seal(memfd::FileSeal::SealWrite)?;
            memfd.add_seal(memfd::FileSeal::SealSeal)?;

            memories.push(Some(Arc::new(MemoryMemFd {
                fd: memfd,
                offset: usize::try_from(offset).unwrap(),
                len: usize::try_from(len).unwrap(),
            })));
        }

        Ok(Some(Arc::new(ModuleMemFds { memories })))
    }
}

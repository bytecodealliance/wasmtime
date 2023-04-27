//! Data structures for representing decoded wasm modules.

use crate::{ModuleTranslation, PrimaryMap, Tunables, WASM_PAGE_SIZE};
use cranelift_entity::{packed_option::ReservedValue, EntityRef};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::mem;
use std::ops::Range;
use wasmtime_types::*;

/// Implementation styles for WebAssembly linear memory.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub enum MemoryStyle {
    /// The actual memory can be resized and moved.
    Dynamic {
        /// Extra space to reserve when a memory must be moved due to growth.
        reserve: u64,
    },
    /// Address space is allocated up front.
    Static {
        /// The number of mapped and unmapped pages.
        bound: u64,
    },
}

impl MemoryStyle {
    /// Decide on an implementation style for the given `Memory`.
    pub fn for_memory(memory: Memory, tunables: &Tunables) -> (Self, u64) {
        // A heap with a maximum that doesn't exceed the static memory bound specified by the
        // tunables make it static.
        //
        // If the module doesn't declare an explicit maximum treat it as 4GiB when not
        // requested to use the static memory bound itself as the maximum.
        let absolute_max_pages = if memory.memory64 {
            crate::WASM64_MAX_PAGES
        } else {
            crate::WASM32_MAX_PAGES
        };
        let maximum = std::cmp::min(
            memory.maximum.unwrap_or(absolute_max_pages),
            if tunables.static_memory_bound_is_maximum {
                std::cmp::min(tunables.static_memory_bound, absolute_max_pages)
            } else {
                absolute_max_pages
            },
        );

        // Ensure the minimum is less than the maximum; the minimum might exceed the maximum
        // when the memory is artificially bounded via `static_memory_bound_is_maximum` above
        if memory.minimum <= maximum && maximum <= tunables.static_memory_bound {
            return (
                Self::Static {
                    bound: tunables.static_memory_bound,
                },
                tunables.static_memory_offset_guard_size,
            );
        }

        // Otherwise, make it dynamic.
        (
            Self::Dynamic {
                reserve: tunables.dynamic_memory_growth_reserve,
            },
            tunables.dynamic_memory_offset_guard_size,
        )
    }
}

/// A WebAssembly linear memory description along with our chosen style for
/// implementing it.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct MemoryPlan {
    /// The WebAssembly linear memory description.
    pub memory: Memory,
    /// Our chosen implementation style.
    pub style: MemoryStyle,
    /// Chosen size of a guard page before the linear memory allocation.
    pub pre_guard_size: u64,
    /// Our chosen offset-guard size.
    pub offset_guard_size: u64,
}

impl MemoryPlan {
    /// Draw up a plan for implementing a `Memory`.
    pub fn for_memory(memory: Memory, tunables: &Tunables) -> Self {
        let (style, offset_guard_size) = MemoryStyle::for_memory(memory, tunables);
        Self {
            memory,
            style,
            offset_guard_size,
            pre_guard_size: if tunables.guard_before_linear_memory {
                offset_guard_size
            } else {
                0
            },
        }
    }
}

/// A WebAssembly linear memory initializer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryInitializer {
    /// The index of a linear memory to initialize.
    pub memory_index: MemoryIndex,
    /// Optionally, a global variable giving a base index.
    pub base: Option<GlobalIndex>,
    /// The offset to add to the base.
    pub offset: u64,
    /// The range of the data to write within the linear memory.
    ///
    /// This range indexes into a separately stored data section which will be
    /// provided with the compiled module's code as well.
    pub data: Range<u32>,
}

/// Similar to the above `MemoryInitializer` but only used when memory
/// initializers are statically known to be valid.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaticMemoryInitializer {
    /// The 64-bit offset, in bytes, of where this initializer starts.
    pub offset: u64,

    /// The range of data to write at `offset`, where these indices are indexes
    /// into the compiled wasm module's data section.
    pub data: Range<u32>,
}

/// The type of WebAssembly linear memory initialization to use for a module.
#[derive(Debug, Serialize, Deserialize)]
pub enum MemoryInitialization {
    /// Memory initialization is segmented.
    ///
    /// Segmented initialization can be used for any module, but it is required
    /// if:
    ///
    /// * A data segment referenced an imported memory.
    /// * A data segment uses a global base.
    ///
    /// Segmented initialization is performed by processing the complete set of
    /// data segments when the module is instantiated.
    ///
    /// This is the default memory initialization type.
    Segmented(Vec<MemoryInitializer>),

    /// Memory initialization is statically known and involves a single `memcpy`
    /// or otherwise simply making the defined data visible.
    ///
    /// To be statically initialized everything must reference a defined memory
    /// and all data segments have a statically known in-bounds base (no
    /// globals).
    ///
    /// This form of memory initialization is a more optimized version of
    /// `Segmented` where memory can be initialized with one of a few methods:
    ///
    /// * First it could be initialized with a single `memcpy` of data from the
    ///   module to the linear memory.
    /// * Otherwise techniques like `mmap` are also possible to make this data,
    ///   which might reside in a compiled module on disk, available immediately
    ///   in a linear memory's address space.
    ///
    /// To facilitate the latter of these techniques the `try_static_init`
    /// function below, which creates this variant, takes a host page size
    /// argument which can page-align everything to make mmap-ing possible.
    Static {
        /// The initialization contents for each linear memory.
        ///
        /// This array has, for each module's own linear memory, the contents
        /// necessary to initialize it. If the memory has a `None` value then no
        /// initialization is necessary (it's zero-filled). Otherwise with
        /// `Some` the first element of the tuple is the offset in memory to
        /// start the initialization and the `Range` is the range within the
        /// final data section of the compiled module of bytes to copy into the
        /// memory.
        ///
        /// The offset, range base, and range end are all guaranteed to be page
        /// aligned to the page size passed in to `try_static_init`.
        map: PrimaryMap<MemoryIndex, Option<StaticMemoryInitializer>>,
    },
}

impl ModuleTranslation<'_> {
    /// Attempts to convert segmented memory initialization into static
    /// initialization for the module that this translation represents.
    ///
    /// If this module's memory initialization is not compatible with paged
    /// initialization then this won't change anything. Otherwise if it is
    /// compatible then the `memory_initialization` field will be updated.
    ///
    /// Takes a `page_size` argument in order to ensure that all
    /// initialization is page-aligned for mmap-ability, and
    /// `max_image_size_always_allowed` to control how we decide
    /// whether to use static init.
    ///
    /// We will try to avoid generating very sparse images, which are
    /// possible if e.g. a module has an initializer at offset 0 and a
    /// very high offset (say, 1 GiB). To avoid this, we use a dual
    /// condition: we always allow images less than
    /// `max_image_size_always_allowed`, and the embedder of Wasmtime
    /// can set this if desired to ensure that static init should
    /// always be done if the size of the module or its heaps is
    /// otherwise bounded by the system. We also allow images with
    /// static init data bigger than that, but only if it is "dense",
    /// defined as having at least half (50%) of its pages with some
    /// data.
    ///
    /// We could do something slightly better by building a dense part
    /// and keeping a sparse list of outlier/leftover segments (see
    /// issue #3820). This would also allow mostly-static init of
    /// modules that have some dynamically-placed data segments. But,
    /// for now, this is sufficient to allow a system that "knows what
    /// it's doing" to always get static init.
    pub fn try_static_init(&mut self, page_size: u64, max_image_size_always_allowed: u64) {
        // This method only attempts to transform a `Segmented` memory init
        // into a `Static` one, no other state.
        if !self.module.memory_initialization.is_segmented() {
            return;
        }

        // First a dry run of memory initialization is performed. This
        // collects information about the extent of memory initialized for each
        // memory as well as the size of all data segments being copied in.
        struct Memory {
            data_size: u64,
            min_addr: u64,
            max_addr: u64,
            // The `usize` here is a pointer into `self.data` which is the list
            // of data segments corresponding to what was found in the original
            // wasm module.
            segments: Vec<(usize, StaticMemoryInitializer)>,
        }
        let mut info = PrimaryMap::with_capacity(self.module.memory_plans.len());
        for _ in 0..self.module.memory_plans.len() {
            info.push(Memory {
                data_size: 0,
                min_addr: u64::MAX,
                max_addr: 0,
                segments: Vec::new(),
            });
        }
        let mut idx = 0;
        let ok = self.module.memory_initialization.init_memory(
            &mut (),
            InitMemory::CompileTime(&self.module),
            |(), memory, init| {
                // Currently `Static` only applies to locally-defined memories,
                // so if a data segment references an imported memory then
                // transitioning to a `Static` memory initializer is not
                // possible.
                if self.module.defined_memory_index(memory).is_none() {
                    return false;
                };
                let info = &mut info[memory];
                let data_len = u64::from(init.data.end - init.data.start);
                if data_len > 0 {
                    info.data_size += data_len;
                    info.min_addr = info.min_addr.min(init.offset);
                    info.max_addr = info.max_addr.max(init.offset + data_len);
                    info.segments.push((idx, init.clone()));
                }
                idx += 1;
                true
            },
        );
        if !ok {
            return;
        }

        // Validate that the memory information collected is indeed valid for
        // static memory initialization.
        for info in info.values().filter(|i| i.data_size > 0) {
            let image_size = info.max_addr - info.min_addr;

            // If the range of memory being initialized is less than twice the
            // total size of the data itself then it's assumed that static
            // initialization is ok. This means we'll at most double memory
            // consumption during the memory image creation process, which is
            // currently assumed to "probably be ok" but this will likely need
            // tweaks over time.
            if image_size < info.data_size.saturating_mul(2) {
                continue;
            }

            // If the memory initialization image is larger than the size of all
            // data, then we still allow memory initialization if the image will
            // be of a relatively modest size, such as 1MB here.
            if image_size < max_image_size_always_allowed {
                continue;
            }

            // At this point memory initialization is concluded to be too
            // expensive to do at compile time so it's entirely deferred to
            // happen at runtime.
            return;
        }

        // Here's where we've now committed to changing to static memory. The
        // memory initialization image is built here from the page data and then
        // it's converted to a single initializer.
        let data = mem::replace(&mut self.data, Vec::new());
        let mut map = PrimaryMap::with_capacity(info.len());
        let mut module_data_size = 0u32;
        for (memory, info) in info.iter() {
            // Create the in-memory `image` which is the initialized contents of
            // this linear memory.
            let extent = if info.segments.len() > 0 {
                (info.max_addr - info.min_addr) as usize
            } else {
                0
            };
            let mut image = Vec::with_capacity(extent);
            for (idx, init) in info.segments.iter() {
                let data = &data[*idx];
                assert_eq!(data.len(), init.data.len());
                let offset = usize::try_from(init.offset - info.min_addr).unwrap();
                if image.len() < offset {
                    image.resize(offset, 0u8);
                    image.extend_from_slice(data);
                } else {
                    image.splice(
                        offset..(offset + data.len()).min(image.len()),
                        data.iter().copied(),
                    );
                }
            }
            assert_eq!(image.len(), extent);
            assert_eq!(image.capacity(), extent);
            let mut offset = if info.segments.len() > 0 {
                info.min_addr
            } else {
                0
            };

            // Chop off trailing zeros from the image as memory is already
            // zero-initialized. Note that `i` is the position of a nonzero
            // entry here, so to not lose it we truncate to `i + 1`.
            if let Some(i) = image.iter().rposition(|i| *i != 0) {
                image.truncate(i + 1);
            }

            // Also chop off leading zeros, if any.
            if let Some(i) = image.iter().position(|i| *i != 0) {
                offset += i as u64;
                image.drain(..i);
            }
            let mut len = u64::try_from(image.len()).unwrap();

            // The goal is to enable mapping this image directly into memory, so
            // the offset into linear memory must be a multiple of the page
            // size. If that's not already the case then the image is padded at
            // the front and back with extra zeros as necessary
            if offset % page_size != 0 {
                let zero_padding = offset % page_size;
                self.data.push(vec![0; zero_padding as usize].into());
                offset -= zero_padding;
                len += zero_padding;
            }
            self.data.push(image.into());
            if len % page_size != 0 {
                let zero_padding = page_size - (len % page_size);
                self.data.push(vec![0; zero_padding as usize].into());
                len += zero_padding;
            }

            // Offset/length should now always be page-aligned.
            assert!(offset % page_size == 0);
            assert!(len % page_size == 0);

            // Create the `StaticMemoryInitializer` which describes this image,
            // only needed if the image is actually present and has a nonzero
            // length. The `offset` has been calculates above, originally
            // sourced from `info.min_addr`. The `data` field is the extent
            // within the final data segment we'll emit to an ELF image, which
            // is the concatenation of `self.data`, so here it's the size of
            // the section-so-far plus the current segment we're appending.
            let len = u32::try_from(len).unwrap();
            let init = if len > 0 {
                Some(StaticMemoryInitializer {
                    offset,
                    data: module_data_size..module_data_size + len,
                })
            } else {
                None
            };
            let idx = map.push(init);
            assert_eq!(idx, memory);
            module_data_size += len;
        }
        self.data_align = Some(page_size);
        self.module.memory_initialization = MemoryInitialization::Static { map };
    }

    /// Attempts to convert the module's table initializers to
    /// FuncTable form where possible. This enables lazy table
    /// initialization later by providing a one-to-one map of initial
    /// table values, without having to parse all segments.
    pub fn try_func_table_init(&mut self) {
        // This should be large enough to support very large Wasm
        // modules with huge funcref tables, but small enough to avoid
        // OOMs or DoS on truly sparse tables.
        const MAX_FUNC_TABLE_SIZE: u32 = 1024 * 1024;

        let segments = match &self.module.table_initialization {
            TableInitialization::Segments { segments } => segments,
            TableInitialization::FuncTable { .. } => {
                // Already done!
                return;
            }
        };

        // Build the table arrays per-table.
        let mut tables = PrimaryMap::with_capacity(self.module.table_plans.len());
        // Keep the "leftovers" for eager init.
        let mut leftovers = vec![];

        for segment in segments {
            // Skip imported tables: we can't provide a preconstructed
            // table for them, because their values depend on the
            // imported table overlaid with whatever segments we have.
            if self
                .module
                .defined_table_index(segment.table_index)
                .is_none()
            {
                leftovers.push(segment.clone());
                continue;
            }

            // If this is not a funcref table, then we can't support a
            // pre-computed table of function indices.
            if self.module.table_plans[segment.table_index].table.wasm_ty != WasmType::FuncRef {
                leftovers.push(segment.clone());
                continue;
            }

            // If the base of this segment is dynamic, then we can't
            // include it in the statically-built array of initial
            // contents.
            if segment.base.is_some() {
                leftovers.push(segment.clone());
                continue;
            }

            // Get the end of this segment. If out-of-bounds, or too
            // large for our dense table representation, then skip the
            // segment.
            let top = match segment.offset.checked_add(segment.elements.len() as u32) {
                Some(top) => top,
                None => {
                    leftovers.push(segment.clone());
                    continue;
                }
            };
            let table_size = self.module.table_plans[segment.table_index].table.minimum;
            if top > table_size || top > MAX_FUNC_TABLE_SIZE {
                leftovers.push(segment.clone());
                continue;
            }

            // We can now incorporate this segment into the initializers array.
            while tables.len() <= segment.table_index.index() {
                tables.push(vec![]);
            }
            let elements = &mut tables[segment.table_index];
            if elements.is_empty() {
                elements.resize(table_size as usize, FuncIndex::reserved_value());
            }

            let dst = &mut elements[(segment.offset as usize)..(top as usize)];
            dst.copy_from_slice(&segment.elements[..]);
        }

        self.module.table_initialization = TableInitialization::FuncTable {
            tables,
            segments: leftovers,
        };
    }
}

impl Default for MemoryInitialization {
    fn default() -> Self {
        Self::Segmented(Vec::new())
    }
}

impl MemoryInitialization {
    /// Returns whether this initialization is of the form
    /// `MemoryInitialization::Segmented`.
    pub fn is_segmented(&self) -> bool {
        match self {
            MemoryInitialization::Segmented(_) => true,
            _ => false,
        }
    }

    /// Performs the memory initialization steps for this set of initializers.
    ///
    /// This will perform wasm initialization in compliance with the wasm spec
    /// and how data segments are processed. This doesn't need to necessarily
    /// only be called as part of initialization, however, as it's structured to
    /// allow learning about memory ahead-of-time at compile time possibly.
    ///
    /// The various callbacks provided here are used to drive the smaller bits
    /// of initialization, such as:
    ///
    /// * `get_cur_size_in_pages` - gets the current size, in wasm pages, of the
    ///   memory specified. For compile-time purposes this would be the memory
    ///   type's minimum size.
    ///
    /// * `get_global` - gets the value of the global specified. This is
    ///   statically, via validation, a pointer to the global of the correct
    ///   type (either u32 or u64 depending on the memory), but the value
    ///   returned here is `u64`. A `None` value can be returned to indicate
    ///   that the global's value isn't known yet.
    ///
    /// * `write` - a callback used to actually write data. This indicates that
    ///   the specified memory must receive the specified range of data at the
    ///   specified offset. This can internally return an false error if it
    ///   wants to fail.
    ///
    /// This function will return true if all memory initializers are processed
    /// successfully. If any initializer hits an error or, for example, a
    /// global value is needed but `None` is returned, then false will be
    /// returned. At compile-time this typically means that the "error" in
    /// question needs to be deferred to runtime, and at runtime this means
    /// that an invalid initializer has been found and a trap should be
    /// generated.
    pub fn init_memory<T>(
        &self,
        state: &mut T,
        init: InitMemory<'_, T>,
        mut write: impl FnMut(&mut T, MemoryIndex, &StaticMemoryInitializer) -> bool,
    ) -> bool {
        let initializers = match self {
            // Fall through below to the segmented memory one-by-one
            // initialization.
            MemoryInitialization::Segmented(list) => list,

            // If previously switched to static initialization then pass through
            // all those parameters here to the `write` callback.
            //
            // Note that existence of `Static` already guarantees that all
            // indices are in-bounds.
            MemoryInitialization::Static { map } => {
                for (index, init) in map {
                    if let Some(init) = init {
                        let result = write(state, index, init);
                        if !result {
                            return result;
                        }
                    }
                }
                return true;
            }
        };

        for initializer in initializers {
            let MemoryInitializer {
                memory_index,
                base,
                offset,
                ref data,
            } = *initializer;

            // First up determine the start/end range and verify that they're
            // in-bounds for the initial size of the memory at `memory_index`.
            // Note that this can bail if we don't have access to globals yet
            // (e.g. this is a task happening before instantiation at
            // compile-time).
            let base = match base {
                Some(index) => match &init {
                    InitMemory::Runtime {
                        get_global_as_u64, ..
                    } => get_global_as_u64(state, index),
                    InitMemory::CompileTime(_) => return false,
                },
                None => 0,
            };
            let start = match base.checked_add(offset) {
                Some(start) => start,
                None => return false,
            };
            let len = u64::try_from(data.len()).unwrap();
            let end = match start.checked_add(len) {
                Some(end) => end,
                None => return false,
            };

            let cur_size_in_pages = match &init {
                InitMemory::CompileTime(module) => module.memory_plans[memory_index].memory.minimum,
                InitMemory::Runtime {
                    memory_size_in_pages,
                    ..
                } => memory_size_in_pages(state, memory_index),
            };

            // Note that this `minimum` can overflow if `minimum` is
            // `1 << 48`, the maximum number of minimum pages for 64-bit
            // memories. If this overflow happens, though, then there's no need
            // to check the `end` value since `end` fits in a `u64` and it is
            // naturally less than the overflowed value.
            //
            // This is a bit esoteric though because it's impossible to actually
            // create a memory of `u64::MAX + 1` bytes, so this is largely just
            // here to avoid having the multiplication here overflow in debug
            // mode.
            if let Some(max) = cur_size_in_pages.checked_mul(u64::from(WASM_PAGE_SIZE)) {
                if end > max {
                    return false;
                }
            }

            // The limits of the data segment have been validated at this point
            // so the `write` callback is called with the range of data being
            // written. Any erroneous result is propagated upwards.
            let init = StaticMemoryInitializer {
                offset: start,
                data: data.clone(),
            };
            let result = write(state, memory_index, &init);
            if !result {
                return result;
            }
        }

        return true;
    }
}

/// Argument to [`MemoryInitialization::init_memory`] indicating the current
/// status of the instance.
pub enum InitMemory<'a, T> {
    /// This evaluation of memory initializers is happening at compile time.
    /// This means that the current state of memories is whatever their initial
    /// state is, and additionally globals are not available if data segments
    /// have global offsets.
    CompileTime(&'a Module),

    /// Evaluation of memory initializers is happening at runtime when the
    /// instance is available, and callbacks are provided to learn about the
    /// instance's state.
    Runtime {
        /// Returns the size, in wasm pages, of the the memory specified.
        memory_size_in_pages: &'a dyn Fn(&mut T, MemoryIndex) -> u64,
        /// Returns the value of the global, as a `u64`. Note that this may
        /// involve zero-extending a 32-bit global to a 64-bit number.
        get_global_as_u64: &'a dyn Fn(&mut T, GlobalIndex) -> u64,
    },
}

/// Implementation styles for WebAssembly tables.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub enum TableStyle {
    /// Signatures are stored in the table and checked in the caller.
    CallerChecksSignature,
}

impl TableStyle {
    /// Decide on an implementation style for the given `Table`.
    pub fn for_table(_table: Table, _tunables: &Tunables) -> Self {
        Self::CallerChecksSignature
    }
}

/// A WebAssembly table description along with our chosen style for
/// implementing it.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct TablePlan {
    /// The WebAssembly table description.
    pub table: Table,
    /// Our chosen implementation style.
    pub style: TableStyle,
}

impl TablePlan {
    /// Draw up a plan for implementing a `Table`.
    pub fn for_table(table: Table, tunables: &Tunables) -> Self {
        let style = TableStyle::for_table(table, tunables);
        Self { table, style }
    }
}

/// A WebAssembly table initializer segment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableInitializer {
    /// The index of a table to initialize.
    pub table_index: TableIndex,
    /// Optionally, a global variable giving a base index.
    pub base: Option<GlobalIndex>,
    /// The offset to add to the base.
    pub offset: u32,
    /// The values to write into the table elements.
    pub elements: Box<[FuncIndex]>,
}

/// Table initialization data for all tables in the module.
#[derive(Debug, Serialize, Deserialize)]
pub enum TableInitialization {
    /// "Segment" mode: table initializer segments, possibly with
    /// dynamic bases, possibly applying to an imported memory.
    ///
    /// Every kind of table initialization is supported by the
    /// Segments mode.
    Segments {
        /// The segment initializers. All apply to the table for which
        /// this TableInitialization is specified.
        segments: Vec<TableInitializer>,
    },

    /// "FuncTable" mode: a single array per table, with a function
    /// index or null per slot. This is only possible to provide for a
    /// given table when it is defined by the module itself, and can
    /// only include data from initializer segments that have
    /// statically-knowable bases (i.e., not dependent on global
    /// values).
    ///
    /// Any segments that are not compatible with this mode are held
    /// in the `segments` array of "leftover segments", which are
    /// still processed eagerly.
    ///
    /// This mode facilitates lazy initialization of the tables. It is
    /// thus "nice to have", but not necessary for correctness.
    FuncTable {
        /// For each table, an array of function indices (or
        /// FuncIndex::reserved_value(), meaning no initialized value,
        /// hence null by default). Array elements correspond
        /// one-to-one to table elements; i.e., `elements[i]` is the
        /// initial value for `table[i]`.
        tables: PrimaryMap<TableIndex, Vec<FuncIndex>>,

        /// Leftover segments that need to be processed eagerly on
        /// instantiation. These either apply to an imported table (so
        /// we can't pre-build a full image of the table from this
        /// overlay) or have dynamically (at instantiation time)
        /// determined bases.
        segments: Vec<TableInitializer>,
    },
}

impl Default for TableInitialization {
    fn default() -> Self {
        TableInitialization::Segments { segments: vec![] }
    }
}

/// Different types that can appear in a module.
///
/// Note that each of these variants are intended to index further into a
/// separate table.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ModuleType {
    Function(SignatureIndex),
}

impl ModuleType {
    /// Asserts this is a `ModuleType::Function`, returning the underlying
    /// `SignatureIndex`.
    pub fn unwrap_function(&self) -> SignatureIndex {
        match self {
            ModuleType::Function(f) => *f,
        }
    }
}

/// A translated WebAssembly module, excluding the function bodies and
/// memory initializers.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Module {
    /// The name of this wasm module, often found in the wasm file.
    pub name: Option<String>,

    /// All import records, in the order they are declared in the module.
    pub initializers: Vec<Initializer>,

    /// Exported entities.
    pub exports: IndexMap<String, EntityIndex>,

    /// The module "start" function, if present.
    pub start_func: Option<FuncIndex>,

    /// WebAssembly table initialization data, per table.
    pub table_initialization: TableInitialization,

    /// WebAssembly linear memory initializer.
    pub memory_initialization: MemoryInitialization,

    /// WebAssembly passive elements.
    pub passive_elements: Vec<Box<[FuncIndex]>>,

    /// The map from passive element index (element segment index space) to index in `passive_elements`.
    pub passive_elements_map: BTreeMap<ElemIndex, usize>,

    /// The map from passive data index (data segment index space) to index in `passive_data`.
    pub passive_data_map: BTreeMap<DataIndex, Range<u32>>,

    /// Types declared in the wasm module.
    pub types: PrimaryMap<TypeIndex, ModuleType>,

    /// Number of imported or aliased functions in the module.
    pub num_imported_funcs: usize,

    /// Number of imported or aliased tables in the module.
    pub num_imported_tables: usize,

    /// Number of imported or aliased memories in the module.
    pub num_imported_memories: usize,

    /// Number of imported or aliased globals in the module.
    pub num_imported_globals: usize,

    /// Number of functions that "escape" from this module may need to have a
    /// `VMCallerCheckedAnyfunc` constructed for them.
    ///
    /// This is also the number of functions in the `functions` array below with
    /// an `anyfunc` index (and is the maximum anyfunc index).
    pub num_escaped_funcs: usize,

    /// Types of functions, imported and local.
    pub functions: PrimaryMap<FuncIndex, FunctionType>,

    /// WebAssembly tables.
    pub table_plans: PrimaryMap<TableIndex, TablePlan>,

    /// WebAssembly linear memory plans.
    pub memory_plans: PrimaryMap<MemoryIndex, MemoryPlan>,

    /// WebAssembly global variables.
    pub globals: PrimaryMap<GlobalIndex, Global>,
}

/// Initialization routines for creating an instance, encompassing imports,
/// modules, instances, aliases, etc.
#[derive(Debug, Serialize, Deserialize)]
pub enum Initializer {
    /// An imported item is required to be provided.
    Import {
        /// Name of this import
        name: String,
        /// The field name projection of this import
        field: String,
        /// Where this import will be placed, which also has type information
        /// about the import.
        index: EntityIndex,
    },
}

impl Module {
    /// Allocates the module data structures.
    pub fn new() -> Self {
        Module::default()
    }

    /// Convert a `DefinedFuncIndex` into a `FuncIndex`.
    #[inline]
    pub fn func_index(&self, defined_func: DefinedFuncIndex) -> FuncIndex {
        FuncIndex::new(self.num_imported_funcs + defined_func.index())
    }

    /// Convert a `FuncIndex` into a `DefinedFuncIndex`. Returns None if the
    /// index is an imported function.
    #[inline]
    pub fn defined_func_index(&self, func: FuncIndex) -> Option<DefinedFuncIndex> {
        if func.index() < self.num_imported_funcs {
            None
        } else {
            Some(DefinedFuncIndex::new(
                func.index() - self.num_imported_funcs,
            ))
        }
    }

    /// Test whether the given function index is for an imported function.
    #[inline]
    pub fn is_imported_function(&self, index: FuncIndex) -> bool {
        index.index() < self.num_imported_funcs
    }

    /// Convert a `DefinedTableIndex` into a `TableIndex`.
    #[inline]
    pub fn table_index(&self, defined_table: DefinedTableIndex) -> TableIndex {
        TableIndex::new(self.num_imported_tables + defined_table.index())
    }

    /// Convert a `TableIndex` into a `DefinedTableIndex`. Returns None if the
    /// index is an imported table.
    #[inline]
    pub fn defined_table_index(&self, table: TableIndex) -> Option<DefinedTableIndex> {
        if table.index() < self.num_imported_tables {
            None
        } else {
            Some(DefinedTableIndex::new(
                table.index() - self.num_imported_tables,
            ))
        }
    }

    /// Test whether the given table index is for an imported table.
    #[inline]
    pub fn is_imported_table(&self, index: TableIndex) -> bool {
        index.index() < self.num_imported_tables
    }

    /// Convert a `DefinedMemoryIndex` into a `MemoryIndex`.
    #[inline]
    pub fn memory_index(&self, defined_memory: DefinedMemoryIndex) -> MemoryIndex {
        MemoryIndex::new(self.num_imported_memories + defined_memory.index())
    }

    /// Convert a `MemoryIndex` into a `DefinedMemoryIndex`. Returns None if the
    /// index is an imported memory.
    #[inline]
    pub fn defined_memory_index(&self, memory: MemoryIndex) -> Option<DefinedMemoryIndex> {
        if memory.index() < self.num_imported_memories {
            None
        } else {
            Some(DefinedMemoryIndex::new(
                memory.index() - self.num_imported_memories,
            ))
        }
    }

    /// Convert a `DefinedMemoryIndex` into an `OwnedMemoryIndex`. Returns None
    /// if the index is an imported memory.
    #[inline]
    pub fn owned_memory_index(&self, memory: DefinedMemoryIndex) -> OwnedMemoryIndex {
        assert!(
            memory.index() < self.memory_plans.len(),
            "non-shared memory must have an owned index"
        );

        // Once we know that the memory index is not greater than the number of
        // plans, we can iterate through the plans up to the memory index and
        // count how many are not shared (i.e., owned).
        let owned_memory_index = self
            .memory_plans
            .iter()
            .skip(self.num_imported_memories)
            .take(memory.index())
            .filter(|(_, mp)| !mp.memory.shared)
            .count();
        OwnedMemoryIndex::new(owned_memory_index)
    }

    /// Test whether the given memory index is for an imported memory.
    #[inline]
    pub fn is_imported_memory(&self, index: MemoryIndex) -> bool {
        index.index() < self.num_imported_memories
    }

    /// Convert a `DefinedGlobalIndex` into a `GlobalIndex`.
    #[inline]
    pub fn global_index(&self, defined_global: DefinedGlobalIndex) -> GlobalIndex {
        GlobalIndex::new(self.num_imported_globals + defined_global.index())
    }

    /// Convert a `GlobalIndex` into a `DefinedGlobalIndex`. Returns None if the
    /// index is an imported global.
    #[inline]
    pub fn defined_global_index(&self, global: GlobalIndex) -> Option<DefinedGlobalIndex> {
        if global.index() < self.num_imported_globals {
            None
        } else {
            Some(DefinedGlobalIndex::new(
                global.index() - self.num_imported_globals,
            ))
        }
    }

    /// Test whether the given global index is for an imported global.
    #[inline]
    pub fn is_imported_global(&self, index: GlobalIndex) -> bool {
        index.index() < self.num_imported_globals
    }

    /// Returns an iterator of all the imports in this module, along with their
    /// module name, field name, and type that's being imported.
    pub fn imports(&self) -> impl ExactSizeIterator<Item = (&str, &str, EntityType)> {
        self.initializers.iter().map(move |i| match i {
            Initializer::Import { name, field, index } => {
                (name.as_str(), field.as_str(), self.type_of(*index))
            }
        })
    }

    /// Returns the type of an item based on its index
    pub fn type_of(&self, index: EntityIndex) -> EntityType {
        match index {
            EntityIndex::Global(i) => EntityType::Global(self.globals[i]),
            EntityIndex::Table(i) => EntityType::Table(self.table_plans[i].table),
            EntityIndex::Memory(i) => EntityType::Memory(self.memory_plans[i].memory),
            EntityIndex::Function(i) => EntityType::Function(self.functions[i].signature),
        }
    }

    /// Appends a new function to this module with the given type information,
    /// used for functions that either don't escape or aren't certain whether
    /// they escape yet.
    pub fn push_function(&mut self, signature: SignatureIndex) -> FuncIndex {
        self.functions.push(FunctionType {
            signature,
            anyfunc: AnyfuncIndex::reserved_value(),
        })
    }

    /// Appends a new function to this module with the given type information.
    pub fn push_escaped_function(
        &mut self,
        signature: SignatureIndex,
        anyfunc: AnyfuncIndex,
    ) -> FuncIndex {
        self.functions.push(FunctionType { signature, anyfunc })
    }
}

/// Type information about functions in a wasm module.
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionType {
    /// The type of this function, indexed into the module-wide type tables for
    /// a module compilation.
    pub signature: SignatureIndex,
    /// The index into the anyfunc table, if present. Note that this is
    /// `reserved_value()` if the function does not escape from a module.
    pub anyfunc: AnyfuncIndex,
}

impl FunctionType {
    /// Returns whether this function's type is one that "escapes" the current
    /// module, meaning that the function is exported, used in `ref.func`, used
    /// in a table, etc.
    pub fn is_escaping(&self) -> bool {
        !self.anyfunc.is_reserved_value()
    }
}

/// Index into the anyfunc table within a VMContext for a function.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct AnyfuncIndex(u32);
cranelift_entity::entity_impl!(AnyfuncIndex);

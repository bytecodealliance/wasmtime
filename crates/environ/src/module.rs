//! Data structures for representing decoded wasm modules.

use crate::prelude::*;
use crate::{PrimaryMap, Tunables};
use alloc::collections::BTreeMap;
use core::ops::Range;
use cranelift_entity::{packed_option::ReservedValue, EntityRef};
use serde_derive::{Deserialize, Serialize};
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
        /// The number of bytes which are reserved for this linear memory. Only
        /// the lower bytes which represent the actual linear memory need be
        /// mapped, but other bytes must be guaranteed to be unmapped.
        byte_reservation: u64,
    },
}

impl MemoryStyle {
    /// Decide on an implementation style for the given `Memory`.
    pub fn for_memory(memory: Memory, tunables: &Tunables) -> (Self, u64) {
        let is_static =
            // Ideally we would compare against (an upper bound on) the target's
            // page size, but unfortunately that is a little hard to plumb
            // through here.
            memory.page_size_log2 >= Memory::DEFAULT_PAGE_SIZE_LOG2
            && tunables.signals_based_traps
            && match memory.maximum_byte_size() {
                Ok(mut maximum) => {
                    if tunables.static_memory_bound_is_maximum {
                        maximum = maximum.min(tunables.static_memory_reservation);
                    }

                    // Ensure the minimum is less than the maximum; the minimum might exceed the maximum
                    // when the memory is artificially bounded via `static_memory_bound_is_maximum` above
                    memory.minimum_byte_size().unwrap() <= maximum
                        && maximum <= tunables.static_memory_reservation
                }

                // If the maximum size of this memory is not representable with
                // `u64` then use the `static_memory_bound_is_maximum` to indicate
                // whether it's a static memory or not. It should be ok to discard
                // the linear memory's maximum size here as growth to the maximum is
                // always fallible and never guaranteed.
                Err(_) => tunables.static_memory_bound_is_maximum,
            };

        if is_static {
            return (
                Self::Static {
                    byte_reservation: tunables.static_memory_reservation,
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
    /// The base offset to start this segment at.
    pub offset: ConstExpr,
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
    /// This function will return true if all memory initializers are processed
    /// successfully. If any initializer hits an error or, for example, a
    /// global value is needed but `None` is returned, then false will be
    /// returned. At compile-time this typically means that the "error" in
    /// question needs to be deferred to runtime, and at runtime this means
    /// that an invalid initializer has been found and a trap should be
    /// generated.
    pub fn init_memory(&self, state: &mut dyn InitMemory) -> bool {
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
                        let result = state.write(index, init);
                        if !result {
                            return result;
                        }
                    }
                }
                return true;
            }
        };

        for initializer in initializers {
            let &MemoryInitializer {
                memory_index,
                ref offset,
                ref data,
            } = initializer;

            // First up determine the start/end range and verify that they're
            // in-bounds for the initial size of the memory at `memory_index`.
            // Note that this can bail if we don't have access to globals yet
            // (e.g. this is a task happening before instantiation at
            // compile-time).
            let start = match state.eval_offset(memory_index, offset) {
                Some(start) => start,
                None => return false,
            };
            let len = u64::try_from(data.len()).unwrap();
            let end = match start.checked_add(len) {
                Some(end) => end,
                None => return false,
            };

            match state.memory_size_in_bytes(memory_index) {
                Ok(max) => {
                    if end > max {
                        return false;
                    }
                }

                // Note that computing the minimum can overflow if the page size
                // is the default 64KiB and the memory's minimum size in pages
                // is `1 << 48`, the maximum number of minimum pages for 64-bit
                // memories. We don't return `false` to signal an error here and
                // instead defer the error to runtime, when it will be
                // impossible to allocate that much memory anyways.
                Err(_) => {}
            }

            // The limits of the data segment have been validated at this point
            // so the `write` callback is called with the range of data being
            // written. Any erroneous result is propagated upwards.
            let init = StaticMemoryInitializer {
                offset: start,
                data: data.clone(),
            };
            let result = state.write(memory_index, &init);
            if !result {
                return result;
            }
        }

        return true;
    }
}

/// The various callbacks provided here are used to drive the smaller bits of
/// memory initialization.
pub trait InitMemory {
    /// Returns the size, in bytes, of the memory specified. For compile-time
    /// purposes this would be the memory type's minimum size.
    fn memory_size_in_bytes(&mut self, memory_index: MemoryIndex) -> Result<u64, SizeOverflow>;

    /// Returns the value of the constant expression, as a `u64`. Note that
    /// this may involve zero-extending a 32-bit global to a 64-bit number. May
    /// return `None` to indicate that the expression involves a value which is
    /// not available yet.
    fn eval_offset(&mut self, memory_index: MemoryIndex, expr: &ConstExpr) -> Option<u64>;

    /// A callback used to actually write data. This indicates that the
    /// specified memory must receive the specified range of data at the
    /// specified offset. This can return false on failure.
    fn write(&mut self, memory_index: MemoryIndex, init: &StaticMemoryInitializer) -> bool;
}

/// Implementation styles for WebAssembly tables.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub enum TableStyle {
    /// Signatures are stored in the table and checked in the caller.
    CallerChecksSignature {
        /// Whether this table is initialized lazily and requires an
        /// initialization check on every access.
        lazy_init: bool,
    },
}

impl TableStyle {
    /// Decide on an implementation style for the given `Table`.
    pub fn for_table(_table: Table, tunables: &Tunables) -> Self {
        Self::CallerChecksSignature {
            lazy_init: tunables.table_lazy_init,
        }
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

/// Table initialization data for all tables in the module.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TableInitialization {
    /// Initial values for tables defined within the module itself.
    ///
    /// This contains the initial values and initializers for tables defined
    /// within a wasm, so excluding imported tables. This initializer can
    /// represent null-initialized tables, element-initialized tables (e.g. with
    /// the function-references proposal), or precomputed images of table
    /// initialization. For example table initializers to a table that are all
    /// in-bounds will get removed from `segment` and moved into
    /// `initial_values` here.
    pub initial_values: PrimaryMap<DefinedTableIndex, TableInitialValue>,

    /// Element segments present in the initial wasm module which are executed
    /// at instantiation time.
    ///
    /// These element segments are iterated over during instantiation to apply
    /// any segments that weren't already moved into `initial_values` above.
    pub segments: Vec<TableSegment>,
}

/// Initial value for all elements in a table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TableInitialValue {
    /// Initialize each table element to null, optionally setting some elements
    /// to non-null given the precomputed image.
    Null {
        /// A precomputed image of table initializers for this table.
        ///
        /// This image is constructed during `try_func_table_init` and
        /// null-initialized elements are represented with
        /// `FuncIndex::reserved_value()`. Note that this image is empty by
        /// default and may not encompass the entire span of the table in which
        /// case the elements are initialized to null.
        precomputed: Vec<FuncIndex>,
    },
    /// An arbitrary const expression.
    Expr(ConstExpr),
}

/// A WebAssembly table initializer segment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableSegment {
    /// The index of a table to initialize.
    pub table_index: TableIndex,
    /// The base offset to start this segment at.
    pub offset: ConstExpr,
    /// The values to write into the table elements.
    pub elements: TableSegmentElements,
}

/// Elements of a table segment, either a list of functions or list of arbitrary
/// expressions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TableSegmentElements {
    /// A sequential list of functions where `FuncIndex::reserved_value()`
    /// indicates a null function.
    Functions(Box<[FuncIndex]>),
    /// Arbitrary expressions, aka either functions, null or a load of a global.
    Expressions(Box<[ConstExpr]>),
}

impl TableSegmentElements {
    /// Returns the number of elements in this segment.
    pub fn len(&self) -> u64 {
        match self {
            Self::Functions(s) => u64::try_from(s.len()).unwrap(),
            Self::Expressions(s) => u64::try_from(s.len()).unwrap(),
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
    pub passive_elements: Vec<TableSegmentElements>,

    /// The map from passive element index (element segment index space) to index in `passive_elements`.
    pub passive_elements_map: BTreeMap<ElemIndex, usize>,

    /// The map from passive data index (data segment index space) to index in `passive_data`.
    pub passive_data_map: BTreeMap<DataIndex, Range<u32>>,

    /// Types declared in the wasm module.
    pub types: PrimaryMap<TypeIndex, ModuleInternedTypeIndex>,

    /// Number of imported or aliased functions in the module.
    pub num_imported_funcs: usize,

    /// Number of imported or aliased tables in the module.
    pub num_imported_tables: usize,

    /// Number of imported or aliased memories in the module.
    pub num_imported_memories: usize,

    /// Number of imported or aliased globals in the module.
    pub num_imported_globals: usize,

    /// Number of functions that "escape" from this module may need to have a
    /// `VMFuncRef` constructed for them.
    ///
    /// This is also the number of functions in the `functions` array below with
    /// an `func_ref` index (and is the maximum func_ref index).
    pub num_escaped_funcs: usize,

    /// Number of call-indirect caches.
    pub num_call_indirect_caches: usize,

    /// Types of functions, imported and local.
    pub functions: PrimaryMap<FuncIndex, FunctionType>,

    /// WebAssembly tables.
    pub table_plans: PrimaryMap<TableIndex, TablePlan>,

    /// WebAssembly linear memory plans.
    pub memory_plans: PrimaryMap<MemoryIndex, MemoryPlan>,

    /// WebAssembly global variables.
    pub globals: PrimaryMap<GlobalIndex, Global>,

    /// WebAssembly global initializers for locally-defined globals.
    pub global_initializers: PrimaryMap<DefinedGlobalIndex, ConstExpr>,
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
            EntityIndex::Function(i) => {
                EntityType::Function(EngineOrModuleTypeIndex::Module(self.functions[i].signature))
            }
        }
    }

    /// Appends a new function to this module with the given type information,
    /// used for functions that either don't escape or aren't certain whether
    /// they escape yet.
    pub fn push_function(&mut self, signature: ModuleInternedTypeIndex) -> FuncIndex {
        self.functions.push(FunctionType {
            signature,
            func_ref: FuncRefIndex::reserved_value(),
        })
    }

    /// Returns an iterator over all of the defined function indices in this
    /// module.
    pub fn defined_func_indices(&self) -> impl Iterator<Item = DefinedFuncIndex> {
        (0..self.functions.len() - self.num_imported_funcs).map(|i| DefinedFuncIndex::new(i))
    }
}

/// Type information about functions in a wasm module.
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionType {
    /// The type of this function, indexed into the module-wide type tables for
    /// a module compilation.
    pub signature: ModuleInternedTypeIndex,
    /// The index into the funcref table, if present. Note that this is
    /// `reserved_value()` if the function does not escape from a module.
    pub func_ref: FuncRefIndex,
}

impl FunctionType {
    /// Returns whether this function's type is one that "escapes" the current
    /// module, meaning that the function is exported, used in `ref.func`, used
    /// in a table, etc.
    pub fn is_escaping(&self) -> bool {
        !self.func_ref.is_reserved_value()
    }
}

/// Index into the funcref table within a VMContext for a function.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct FuncRefIndex(u32);
cranelift_entity::entity_impl!(FuncRefIndex);

use crate::prelude::*;
use crate::{ConstOp, IndexType, Limits, Memory, TripleExt};
use core::num::NonZeroU32;
use core::{fmt, str::FromStr};
use serde_derive::{Deserialize, Serialize};
use target_lexicon::{PointerWidth, Triple};
use wasmparser::Operator;

macro_rules! define_tunables {
    (
        $(#[$outer_attr:meta])*
        pub struct $tunables:ident {
            $(
                $(#[$field_attr:meta])*
                pub $field:ident : $field_ty:ty,
            )*
        }

        pub struct $config_tunables:ident {
            ...
        }
    ) => {
        $(#[$outer_attr])*
        pub struct $tunables {
            $(
                $(#[$field_attr])*
                pub $field: $field_ty,
            )*
        }

        /// Optional tunable configuration options used in `wasmtime::Config`
        #[derive(Default, Clone)]
        #[expect(missing_docs, reason = "macro-generated fields")]
        pub struct $config_tunables {
            $(pub $field: Option<$field_ty>,)*
        }

        impl $config_tunables {
            /// Formats configured fields into `f`.
            pub fn format(&self, f: &mut fmt::DebugStruct<'_,'_>) {
                $(
                    if let Some(val) = &self.$field {
                        f.field(stringify!($field), val);
                    }
                )*
            }

            /// Configure the `Tunables` provided.
            pub fn configure(&self, tunables: &mut Tunables) {
                $(
                    if let Some(val) = &self.$field {
                        tunables.$field = val.clone();
                    }
                )*
            }
        }
    };
}

define_tunables! {
    /// Tunable parameters for WebAssembly compilation.
    #[derive(Clone, Hash, Serialize, Deserialize, Debug)]
    pub struct Tunables {
        /// The garbage collector implementation to use, which implies the layout of
        /// GC objects and barriers that must be emitted in Wasm code.
        pub collector: Option<Collector>,

        /// Initial size, in bytes, to be allocated for linear memories.
        pub memory_reservation: u64,

        /// The size, in bytes, of the guard page region for linear memories.
        pub memory_guard_size: u64,

        /// The size, in bytes, to allocate at the end of a relocated linear
        /// memory for growth.
        pub memory_reservation_for_growth: u64,

        /// Whether or not to generate native DWARF debug information.
        pub debug_native: bool,

        /// Whether we are enabling precise Wasm-level debugging in
        /// the guest.
        pub debug_guest: bool,

        /// Whether we are enabling native symbols to get inserted into the
        /// final `*.cwasm`.
        pub debug_symbols: bool,

        /// Whether or not to retain DWARF sections in compiled modules.
        pub parse_wasm_debuginfo: bool,

        /// Whether or not fuel is enabled for generated code, meaning that fuel
        /// will be consumed every time a wasm instruction is executed.
        pub consume_fuel: bool,

        /// The cost of each operator. If fuel is not enabled, this is ignored.
        pub operator_cost: OperatorCostStrategy,

        /// Whether or not we use epoch-based interruption.
        pub epoch_interruption: bool,

        /// Whether or not linear memories are allowed to be reallocated after
        /// initial allocation at runtime.
        pub memory_may_move: bool,

        /// Whether or not linear memory allocations will have a guard region at the
        /// beginning of the allocation in addition to the end.
        pub guard_before_linear_memory: bool,

        /// Whether to initialize tables lazily, so that instantiation is fast but
        /// indirect calls are a little slower. If false, tables are initialized
        /// eagerly from any active element segments that apply to them during
        /// instantiation.
        pub table_lazy_init: bool,

        /// Indicates whether an address map from compiled native code back to wasm
        /// offsets in the original file is generated.
        pub generate_address_map: bool,

        /// Flag for the component module whether adapter modules have debug
        /// assertions baked into them.
        pub debug_adapter_modules: bool,

        /// Whether or not lowerings for relaxed simd instructions are forced to
        /// be deterministic.
        pub relaxed_simd_deterministic: bool,

        /// Whether or not Wasm functions target the winch abi.
        pub winch_callable: bool,

        /// Whether or not the host will be using native signals (e.g. SIGILL,
        /// SIGSEGV, etc) to implement traps.
        pub signals_based_traps: bool,

        /// Whether CoW images might be used to initialize linear memories.
        pub memory_init_cow: bool,

        /// Whether to enable inlining in Wasmtime's compilation orchestration
        /// or not.
        pub inlining: Inlining,

        /// The size of "small callees" that can be inlined regardless of the
        /// caller's size.
        pub inlining_small_callee_size: u32,

        /// The general size threshold for the sum of the caller's and callee's
        /// sizes, past which we will generally not inline calls anymore.
        pub inlining_sum_size_threshold: u32,

        /// Whether any component model feature related to concurrency is
        /// enabled.
        pub concurrency_support: bool,

        /// Whether recording in RR is enabled or not. This is used primarily
        /// to signal checksum computation for compiled artifacts.
        pub recording: bool,

        /// An allocation counter that triggers GC when it reaches zero.
        ///
        /// Decremented on every allocation and when it hits zero, a GC is
        /// forced and the counter is reset. Only effective when
        /// `cfg(gc_zeal)` is enabled.
        pub gc_zeal_alloc_counter: Option<NonZeroU32>,

        /// Initial size, in bytes, to be allocated for GC heaps.
        ///
        /// This is the same as `memory_reservation` but for GC heaps.
        pub gc_heap_reservation: u64,

        /// The size, in bytes, of the guard page region for GC heaps.
        ///
        /// This is the same as `memory_guard_size` but for GC heaps.
        pub gc_heap_guard_size: u64,

        /// The size, in bytes, to allocate at the end of a relocated GC heap
        /// for growth.
        ///
        /// This is the same as `memory_reservation_for_growth` but for GC
        /// heaps.
        pub gc_heap_reservation_for_growth: u64,

        /// The size, in bytes, to set as the minimum for GC heaps.
        pub gc_heap_initial_size: u64,

        /// Whether or not GC heaps are allowed to be reallocated after initial
        /// allocation at runtime.
        ///
        /// This is the same as `memory_may_move` but for GC heaps.
        pub gc_heap_may_move: bool,

        /// Boolean to track whether compiled code retains metadata necessary to
        /// report extra information on internal assertions failing.
        pub metadata_for_internal_asserts: bool,

        /// Boolean to track whether compiled code retains metadata necessary to
        /// report extra information on gc heap corruption being detected.
        pub metadata_for_gc_heap_corruption: bool,

        /// Whether `metadata.code.branch_hint` sections are parsed and used to
        /// mark cold blocks during compilation.
        pub branch_hinting: bool,
    }

    pub struct ConfigTunables {
        ...
    }
}

impl Tunables {
    /// Returns a `Tunables` configuration assumed for running code on the host.
    pub fn default_host() -> Self {
        if cfg!(miri) {
            Tunables::default_miri()
        } else if cfg!(target_pointer_width = "32") {
            Tunables::default_u32()
        } else if cfg!(target_pointer_width = "64") {
            Tunables::default_u64()
        } else {
            panic!("unsupported target_pointer_width");
        }
    }

    /// Returns the default set of tunables for the given target triple.
    pub fn default_for_target(target: &Triple) -> Result<Self> {
        if cfg!(miri) {
            return Ok(Tunables::default_miri());
        }
        let mut ret = match target
            .pointer_width()
            .map_err(|_| format_err!("failed to retrieve target pointer width"))?
        {
            PointerWidth::U32 => Tunables::default_u32(),
            PointerWidth::U64 => Tunables::default_u64(),
            _ => bail!("unsupported target pointer width"),
        };

        // Pulley targets never use signals-based-traps and also can't benefit
        // from guard pages, so disable them.
        if target.is_pulley() {
            ret.signals_based_traps = false;
            ret.memory_guard_size = 0;
            ret.gc_heap_guard_size = 0;
        }
        Ok(ret)
    }

    /// Returns the default set of tunables for running under MIRI.
    pub fn default_miri() -> Tunables {
        Tunables {
            collector: None,

            // No virtual memory tricks are available on miri so make these
            // limits quite conservative.
            memory_reservation: 1 << 20,
            memory_guard_size: 0,
            memory_reservation_for_growth: 0,

            // General options which have the same defaults regardless of
            // architecture.
            debug_native: false,
            parse_wasm_debuginfo: true,
            consume_fuel: false,
            operator_cost: OperatorCostStrategy::Default,
            epoch_interruption: false,
            memory_may_move: true,
            guard_before_linear_memory: true,
            table_lazy_init: true,
            generate_address_map: true,
            debug_adapter_modules: false,
            relaxed_simd_deterministic: false,
            winch_callable: false,
            signals_based_traps: false,
            memory_init_cow: true,
            inlining: Inlining::No,
            inlining_small_callee_size: 50,
            inlining_sum_size_threshold: 2000,
            debug_guest: false,
            concurrency_support: true,
            recording: false,
            gc_zeal_alloc_counter: None,
            gc_heap_reservation: 0,
            gc_heap_guard_size: 0,
            gc_heap_reservation_for_growth: 0,
            gc_heap_may_move: true,
            gc_heap_initial_size: 0,
            metadata_for_internal_asserts: false,
            metadata_for_gc_heap_corruption: true,
            branch_hinting: false,
            debug_symbols: true,
        }
    }

    /// Returns the default set of tunables for running under a 32-bit host.
    pub fn default_u32() -> Tunables {
        Tunables {
            // For 32-bit we scale way down to 10MB of reserved memory. This
            // impacts performance severely but allows us to have more than a
            // few instances running around.
            memory_reservation: 10 * (1 << 20),
            memory_guard_size: 0x1_0000,
            memory_reservation_for_growth: 1 << 20, // 1MB
            signals_based_traps: true,

            // GC heaps on 32-bit: conservative defaults similar to linear
            // memories.
            gc_heap_reservation: 10 * (1 << 20),
            gc_heap_guard_size: 0x1_0000,
            gc_heap_reservation_for_growth: 1 << 20, // 1MB

            ..Tunables::default_miri()
        }
    }

    /// Returns the default set of tunables for running under a 64-bit host.
    pub fn default_u64() -> Tunables {
        Tunables {
            // 64-bit has tons of address space to static memories can have 4gb
            // address space reservations liberally by default, allowing us to
            // help eliminate bounds checks.
            //
            // A 32MiB default guard size is then allocated so we can remove
            // explicit bounds checks if any static offset is less than this
            // value. SpiderMonkey found, for example, that in a large corpus of
            // wasm modules 20MiB was the maximum offset so this is the
            // power-of-two-rounded up from that and matches SpiderMonkey.
            memory_reservation: 1 << 32,
            memory_guard_size: 32 << 20,

            // We've got lots of address space on 64-bit so use a larger
            // grow-into-this area, but on 32-bit we aren't as lucky. Miri is
            // not exactly fast so reduce memory consumption instead of trying
            // to avoid memory movement.
            memory_reservation_for_growth: 2 << 30, // 2GB

            // GC heaps on 64-bit: use 4GiB reservation and 32MiB guard pages
            // to enable bounds check elision, matching linear memory defaults.
            gc_heap_reservation: 1 << 32,
            gc_heap_guard_size: 32 << 20,
            gc_heap_reservation_for_growth: 2 << 30, // 2GB

            signals_based_traps: true,
            ..Tunables::default_miri()
        }
    }

    /// Get the GC heap's memory type, given our configured tunables.
    pub fn gc_heap_memory_type(&self) -> Memory {
        // We *could* try to match the target architecture's page size, but that
        // would require exercising a page size for memories that we don't
        // otherwise support for Wasm; we conservatively avoid that, and just
        // use the default Wasm page size, for now.
        let page_size_log2 = 16;
        let min = self.gc_heap_initial_size.div_ceil(1 << page_size_log2);
        Memory {
            idx_type: IndexType::I32,
            limits: Limits { min, max: None },
            shared: false,
            page_size_log2,
        }
    }
}

/// Whether a heap is backing a linear memory or a GC heap.
///
/// This is used by [`MemoryTunables`] to select between the memory tunables and
/// the GC heap tunables.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemoryKind {
    /// A WebAssembly linear memory.
    LinearMemory,
    /// A GC heap for garbage-collected objects.
    GcHeap,
}

/// A view into a [`Tunables`] that selects the appropriate linear-memory or
/// GC-heap flavor of each tunable based on a [`MemoryKind`].
pub struct MemoryTunables<'a> {
    tunables: &'a Tunables,
    kind: MemoryKind,
}

impl<'a> MemoryTunables<'a> {
    /// Create a new `MemoryTunables` view.
    pub fn new(tunables: &'a Tunables, kind: MemoryKind) -> Self {
        Self { tunables, kind }
    }

    /// The virtual memory reservation for this kind of memory.
    pub fn reservation(&self) -> u64 {
        match self.kind {
            MemoryKind::LinearMemory => self.tunables.memory_reservation,
            MemoryKind::GcHeap => self.tunables.gc_heap_reservation,
        }
    }

    /// The size of the guard page region for this kind of memory.
    pub fn guard_size(&self) -> u64 {
        match self.kind {
            MemoryKind::LinearMemory => self.tunables.memory_guard_size,
            MemoryKind::GcHeap => self.tunables.gc_heap_guard_size,
        }
    }

    /// Extra virtual memory to reserve beyond the initially mapped pages for
    /// this kind of memory.
    pub fn reservation_for_growth(&self) -> u64 {
        match self.kind {
            MemoryKind::LinearMemory => self.tunables.memory_reservation_for_growth,
            MemoryKind::GcHeap => self.tunables.gc_heap_reservation_for_growth,
        }
    }

    /// Whether this kind of memory's base pointer may be relocated at runtime.
    pub fn may_move(&self) -> bool {
        match self.kind {
            MemoryKind::LinearMemory => self.tunables.memory_may_move,
            MemoryKind::GcHeap => self.tunables.gc_heap_may_move,
        }
    }

    /// Get the underlying tunables.
    ///
    /// This is ONLY for accessing tunable fields that DO NOT come in a
    /// linear-memory flavor and a GC-heap flavor.
    pub fn tunables(&self) -> &'a Tunables {
        self.tunables
    }
}

/// The garbage collector implementation to use.
#[derive(Clone, Copy, Hash, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Collector {
    /// The deferred reference-counting collector.
    DeferredReferenceCounting,
    /// The null collector.
    Null,
    /// The copying collector.
    Copying,
}

impl fmt::Display for Collector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Collector::DeferredReferenceCounting => write!(f, "deferred reference-counting"),
            Collector::Null => write!(f, "null"),
            Collector::Copying => write!(f, "copying"),
        }
    }
}

/// Inlining modes supported by Wasmtime.
#[derive(Clone, Copy, Hash, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Inlining {
    /// All inlining is enabled wherever possible.
    ///
    /// This includes inter-module inlining (across modules) as well as
    /// intra-module inlining (within a module).
    ///
    /// Note that backtraces may omit inlined stack frames.
    Yes,

    /// Inter-module inlining (across modules) is allowed, but intra-module
    /// (within a module) is only allowed when the module is using GC.
    ///
    /// Note that backtraces may omit inlined stack frames.
    InterModuleAndIntraGc,

    /// Inter-module inlining (across modules) is allowed, but intra-module
    /// (within a module) is not allowed.
    ///
    /// Note that backtraces may omit inlined stack frames.
    InterModule,

    /// No module inlining is allowed, either inter- or intra-module. Only
    /// inlining Wasmtime's intrinsics are allowed.
    ///
    /// This option, for example, never emits WebAssembly stack frames from
    /// backtraces.
    Intrinsics,

    /// Inlining is disabled entirely.
    No,
}

impl FromStr for Inlining {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "y" | "yes" | "true" => Ok(Self::Yes),
            "n" | "no" | "false" => Ok(Self::No),
            "gc" => Ok(Self::InterModuleAndIntraGc),
            "inter-module" => Ok(Self::InterModuleAndIntraGc),
            "intrinsics" => Ok(Self::Intrinsics),
            _ => bail!(
                "invalid intra-module inlining option string: `{s}`, \
                 only yes,no,gc,inter-module,intrinsics accepted"
            ),
        }
    }
}

impl fmt::Display for Inlining {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Inlining::Yes => write!(f, "yes"),
            Inlining::InterModuleAndIntraGc => write!(f, "gc"),
            Inlining::InterModule => write!(f, "inter-module"),
            Inlining::Intrinsics => write!(f, "intrinsics"),
            Inlining::No => write!(f, "no"),
        }
    }
}

/// The cost of each operator.
///
/// Note: a more dynamic approach (e.g. a user-supplied callback) can be
/// added as a variant in the future if needed.
#[derive(Clone, Hash, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub enum OperatorCostStrategy {
    /// A table of operator costs.
    Table(Box<OperatorCost>),

    /// Each cost defaults to 1 fuel unit, except `Nop`, `Drop` and
    /// a few control flow operators.
    #[default]
    Default,
}

impl OperatorCostStrategy {
    /// Create a new operator cost strategy with a table of costs.
    pub fn table(cost: OperatorCost) -> Self {
        OperatorCostStrategy::Table(Box::new(cost))
    }

    /// Get the cost of an operator.
    pub fn cost(&self, op: &Operator) -> i64 {
        match self {
            OperatorCostStrategy::Table(cost) => cost.cost(op),
            OperatorCostStrategy::Default => default_operator_cost(op),
        }
    }

    /// Get the cost of an operator inside a constant expression.
    ///
    /// Constant expressions are stored as [`ConstOp`] rather than
    /// `wasmparser::Operator`, so translate back and reuse
    /// [`OperatorCostStrategy::cost`].
    pub fn const_op_cost(&self, op: &ConstOp) -> i64 {
        self.cost(&const_op_as_operator(op))
    }

    /// Get the costs of work whose size is only known at runtime.
    pub fn variable(&self) -> &VariableOperatorCost {
        match self {
            OperatorCostStrategy::Table(cost) => &cost.variable,
            OperatorCostStrategy::Default => &DEFAULT_VARIABLE_OPERATOR_COST,
        }
    }
}

const DEFAULT_VARIABLE_OPERATOR_COST: VariableOperatorCost = VariableOperatorCost::new();

/// Fuel costs for operators whose work is proportional to a runtime operand.
///
/// These costs are charged in addition to the corresponding flat cost in
/// [`OperatorCost`].
#[derive(Clone, Hash, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct VariableOperatorCost {
    /// Cost per byte copied by `memory.copy`.
    pub memory_copy_per_byte: u8,
    /// Cost per byte written by `memory.fill`.
    pub memory_fill_per_byte: u8,
    /// Cost per byte copied by `memory.init`.
    pub memory_init_per_byte: u8,
    /// Cost per page requested by `memory.grow`.
    pub memory_grow_per_page: u8,

    /// Cost per element copied by `table.copy`.
    pub table_copy_per_element: u8,
    /// Cost per element written by `table.fill`.
    pub table_fill_per_element: u8,
    /// Cost per element copied by `table.init`.
    pub table_init_per_element: u8,
    /// Cost per element requested by `table.grow`.
    pub table_grow_per_element: u8,

    /// Cost per element copied by `array.copy`.
    pub array_copy_per_element: u8,
    /// Cost per element written by `array.fill`.
    pub array_fill_per_element: u8,
    /// Cost per element initialized by `array.new_data`.
    pub array_new_data_per_element: u8,
    /// Cost per element initialized by `array.init_data`.
    pub array_init_data_per_element: u8,
    /// Cost per element initialized by `array.new_elem`.
    pub array_new_elem_per_element: u8,
    /// Cost per element initialized by `array.init_elem`.
    pub array_init_elem_per_element: u8,
    /// Cost per element initialized by `array.new_default`.
    pub array_new_default_per_element: u8,
    /// Cost per element initialized by `array.new`.
    pub array_new_per_element: u8,
}

impl VariableOperatorCost {
    /// Creates the default variable-cost table.
    pub const fn new() -> Self {
        Self {
            memory_copy_per_byte: 1,
            memory_fill_per_byte: 1,
            memory_init_per_byte: 1,
            // `memory.grow` did not previously have a dynamic fuel charge.
            memory_grow_per_page: 0,
            table_copy_per_element: 1,
            table_fill_per_element: 1,
            table_init_per_element: 1,
            table_grow_per_element: 1,
            array_copy_per_element: 1,
            array_fill_per_element: 1,
            array_new_data_per_element: 1,
            array_init_data_per_element: 1,
            array_new_elem_per_element: 1,
            array_init_elem_per_element: 1,
            array_new_default_per_element: 1,
            array_new_per_element: 1,
        }
    }
}

impl Default for VariableOperatorCost {
    fn default() -> Self {
        Self::new()
    }
}

const fn default_operator_cost(op: &Operator) -> i64 {
    match op {
        // Nop and drop generate no code, so don't consume fuel for them.
        Operator::Nop | Operator::Drop => 0,

        // Control flow may create branches, but is generally cheap and
        // free, so don't consume fuel. Note the lack of `if` since some
        // cost is incurred with the conditional check.
        Operator::Block { .. }
        | Operator::Loop { .. }
        | Operator::Unreachable
        | Operator::Return
        | Operator::Else
        | Operator::End => 0,

        // Everything else, just call it one operation.
        _ => 1,
    }
}

macro_rules! default_cost {
    // Nop and drop generate no code, so don't consume fuel for them.
    (Nop) => {
        0
    };
    (Drop) => {
        0
    };

    // Control flow may create branches, but is generally cheap and
    // free, so don't consume fuel. Note the lack of `if` since some
    // cost is incurred with the conditional check.
    (Block) => {
        0
    };
    (Loop) => {
        0
    };
    (Unreachable) => {
        0
    };
    (Return) => {
        0
    };
    (Else) => {
        0
    };
    (End) => {
        0
    };

    // Everything else, just call it one operation.
    ($op:ident) => {
        1
    };
}

macro_rules! define_operator_cost {
    ($(@$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident ($($ann:tt)*) )*) => {
        /// The fuel cost of each operator in a table.
        #[derive(Clone, Hash, Serialize, Deserialize, Debug, PartialEq, Eq)]
        #[allow(missing_docs, non_snake_case, reason = "to avoid triggering clippy lints")]
        pub struct OperatorCost {
            $(
                pub $op: u8,
            )*
            /// Costs for work whose size is only known at runtime.
            pub variable: VariableOperatorCost,
        }

        impl OperatorCost {
            /// Returns the cost of the given operator.
            pub fn cost(&self, op: &Operator) -> i64 {
                match op {
                    $(
                        Operator::$op $({ $($arg: _),* })? => self.$op as i64,
                    )*
                    unknown => panic!("unknown op: {unknown:?}"),
                }
            }
        }

        impl OperatorCost {
            /// Creates a new `OperatorCost` table with default costs for each operator.
            pub const fn new() -> Self {
                Self {
                    $(
                        $op: default_cost!($op),
                    )*
                    variable: VariableOperatorCost::new(),
                }
            }
        }

        impl Default for OperatorCost {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

wasmparser::for_each_operator!(define_operator_cost);

/// Translate a [`ConstOp`] back into the `wasmparser::Operator` it was parsed
/// from, so that constant expressions can share the operator cost lookup with
/// function bodies.
///
/// Every immediate round-trips exactly except `ConstOp::RefNull`'s, which stores
/// a `WasmHeapType` lowered on the way in by `TypeConvert::convert_heap_type`
/// and has no reverse conversion, so a placeholder heap type stands in for it.
/// That is fine here because the cost lookup matches on the operator alone and
/// ignores its immediates.
fn const_op_as_operator(op: &ConstOp) -> Operator<'static> {
    use wasmparser::{AbstractHeapType, HeapType, Ieee32, Ieee64, V128};
    match op {
        ConstOp::I32Const(value) => Operator::I32Const { value: *value },
        ConstOp::I64Const(value) => Operator::I64Const { value: *value },
        ConstOp::F32Const(bits) => Operator::F32Const {
            value: Ieee32::from(f32::from_bits(*bits)),
        },
        ConstOp::F64Const(bits) => Operator::F64Const {
            value: Ieee64::from(f64::from_bits(*bits)),
        },
        ConstOp::V128Const(value) => Operator::V128Const {
            value: V128::from(*value as i128),
        },
        ConstOp::GlobalGet(index) => Operator::GlobalGet {
            global_index: index.as_u32(),
        },
        ConstOp::RefI31 => Operator::RefI31,
        ConstOp::RefNull(_) => Operator::RefNull {
            hty: HeapType::Abstract {
                shared: false,
                ty: AbstractHeapType::Any,
            },
        },
        ConstOp::RefFunc(index) => Operator::RefFunc {
            function_index: index.as_u32(),
        },
        ConstOp::I32Add => Operator::I32Add,
        ConstOp::I32Sub => Operator::I32Sub,
        ConstOp::I32Mul => Operator::I32Mul,
        ConstOp::I64Add => Operator::I64Add,
        ConstOp::I64Sub => Operator::I64Sub,
        ConstOp::I64Mul => Operator::I64Mul,
        ConstOp::StructNew { struct_type_index } => Operator::StructNew {
            struct_type_index: struct_type_index.as_u32(),
        },
        ConstOp::StructNewDefault { struct_type_index } => Operator::StructNewDefault {
            struct_type_index: struct_type_index.as_u32(),
        },
        ConstOp::ArrayNew { array_type_index } => Operator::ArrayNew {
            array_type_index: array_type_index.as_u32(),
        },
        ConstOp::ArrayNewDefault { array_type_index } => Operator::ArrayNewDefault {
            array_type_index: array_type_index.as_u32(),
        },
        ConstOp::ArrayNewFixed {
            array_type_index,
            array_size,
        } => Operator::ArrayNewFixed {
            array_type_index: array_type_index.as_u32(),
            array_size: *array_size,
        },
        ConstOp::ExternConvertAny => Operator::ExternConvertAny,
        ConstOp::AnyConvertExtern => Operator::AnyConvertExtern,
    }
}

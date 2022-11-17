/// Helper macro to iterate over all builtin functions and their signatures.
#[macro_export]
macro_rules! foreach_builtin_function {
    ($mac:ident) => {
        $mac! {
            /// Returns an index for wasm's `memory.grow` builtin function.
            memory32_grow(vmctx: vmctx, delta: i64, index: i32) -> pointer;
            /// Returns an index for wasm's `table.copy` when both tables are locally
            /// defined.
            table_copy(vmctx: vmctx, dst_index: i32, src_index: i32, dst: i32, src: i32, len: i32);
            /// Returns an index for wasm's `table.init`.
            table_init(vmctx: vmctx, table: i32, elem: i32, dst: i32, src: i32, len: i32);
            /// Returns an index for wasm's `elem.drop`.
            elem_drop(vmctx: vmctx, elem: i32);
            /// Returns an index for wasm's `memory.copy`
            memory_copy(vmctx: vmctx, dst_index: i32, dst: i64, src_index: i32, src: i64, len: i64);
            /// Returns an index for wasm's `memory.fill` instruction.
            memory_fill(vmctx: vmctx, memory: i32, dst: i64, val: i32, len: i64);
            /// Returns an index for wasm's `memory.init` instruction.
            memory_init(vmctx: vmctx, memory: i32, data: i32, dst: i64, src: i32, len: i32);
            /// Returns a value for wasm's `ref.func` instruction.
            ref_func(vmctx: vmctx, func: i32) -> pointer;
            /// Returns an index for wasm's `data.drop` instruction.
            data_drop(vmctx: vmctx, data: i32);
            /// Returns a table entry after lazily initializing it.
            table_get_lazy_init_funcref(vmctx: vmctx, table: i32, index: i32) -> pointer;
            /// Returns an index for Wasm's `table.grow` instruction for `funcref`s.
            table_grow_funcref(vmctx: vmctx, table: i32, delta: i32, init: pointer) -> i32;
            /// Returns an index for Wasm's `table.grow` instruction for `externref`s.
            table_grow_externref(vmctx: vmctx, table: i32, delta: i32, init: reference) -> i32;
            /// Returns an index for Wasm's `table.fill` instruction for `externref`s.
            table_fill_externref(vmctx: vmctx, table: i32, dst: i32, val: reference, len: i32);
            /// Returns an index for Wasm's `table.fill` instruction for `funcref`s.
            table_fill_funcref(vmctx: vmctx, table: i32, dst: i32, val: pointer, len: i32);
            /// Returns an index to drop a `VMExternRef`.
            drop_externref(vmctx: vmctx, val: pointer);
            /// Returns an index to do a GC and then insert a `VMExternRef` into the
            /// `VMExternRefActivationsTable`.
            activations_table_insert_with_gc(vmctx: vmctx, val: reference);
            /// Returns an index for Wasm's `global.get` instruction for `externref`s.
            externref_global_get(vmctx: vmctx, global: i32) -> reference;
            /// Returns an index for Wasm's `global.get` instruction for `externref`s.
            externref_global_set(vmctx: vmctx, global: i32, val: reference);
            /// Returns an index for wasm's `memory.atomic.notify` instruction.
            memory_atomic_notify(vmctx: vmctx, memory: i32, addr: i64, count: i32) -> i32;
            /// Returns an index for wasm's `memory.atomic.wait32` instruction.
            memory_atomic_wait32(vmctx: vmctx, memory: i32, addr: i64, expected: i32, timeout: i64) -> i32;
            /// Returns an index for wasm's `memory.atomic.wait64` instruction.
            memory_atomic_wait64(vmctx: vmctx, memory: i32, addr: i64, expected: i64, timeout: i64) -> i32;
            /// Invoked when fuel has run out while executing a function.
            out_of_gas(vmctx: vmctx);
            /// Invoked when we reach a new epoch.
            new_epoch(vmctx: vmctx) -> i64;
        }
    };
}

/// An index type for builtin functions.
#[derive(Copy, Clone, Debug)]
pub struct BuiltinFunctionIndex(u32);

impl BuiltinFunctionIndex {
    /// Create a new `BuiltinFunctionIndex` from its index
    pub const fn from_u32(i: u32) -> Self {
        Self(i)
    }

    /// Return the index as an u32 number.
    pub const fn index(&self) -> u32 {
        self.0
    }
}

macro_rules! declare_indexes {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
        )*
    ) => {
        impl BuiltinFunctionIndex {
            declare_indexes!(
                @indices;
                0;
                $( $( #[$attr] )* $name; )*
            );
        }
    };

    // Base case: no more indices to declare, so define the total number of
    // function indices.
    (
        @indices;
        $len:expr;
    ) => {
        /// Returns the total number of builtin functions.
        pub const fn builtin_functions_total_number() -> u32 {
            $len
        }
    };

    // Recursive case: declare the next index, and then keep declaring the rest of
    // the indices.
    (
         @indices;
         $index:expr;
         $( #[$this_attr:meta] )*
         $this_name:ident;
         $(
             $( #[$rest_attr:meta] )*
             $rest_name:ident;
         )*
    ) => {
        $( #[$this_attr] )*
        pub const fn $this_name() -> Self {
            Self($index)
        }

        declare_indexes!(
            @indices;
            ($index + 1);
            $( $( #[$rest_attr] )* $rest_name; )*
        );
    }
}

foreach_builtin_function!(declare_indexes);

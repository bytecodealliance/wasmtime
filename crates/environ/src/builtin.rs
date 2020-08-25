/// Helper macro to iterate over all builtin functions and their signatures.
#[macro_export]
macro_rules! foreach_builtin_function {
    ($mac:ident) => {
        $mac! {
            /// Returns an index for wasm's `memory.grow` builtin function.
            memory32_grow(vmctx, i32, i32) -> (i32);
            /// Returns an index for wasm's imported `memory.grow` builtin function.
            imported_memory32_grow(vmctx, i32, i32) -> (i32);
            /// Returns an index for wasm's `memory.size` builtin function.
            memory32_size(vmctx, i32) -> (i32);
            /// Returns an index for wasm's imported `memory.size` builtin function.
            imported_memory32_size(vmctx, i32) -> (i32);
            /// Returns an index for wasm's `table.copy` when both tables are locally
            /// defined.
            table_copy(vmctx, i32, i32, i32, i32, i32) -> ();
            /// Returns an index for wasm's `table.init`.
            table_init(vmctx, i32, i32, i32, i32, i32) -> ();
            /// Returns an index for wasm's `elem.drop`.
            elem_drop(vmctx, i32) -> ();
            /// Returns an index for wasm's `memory.copy` for locally defined memories.
            defined_memory_copy(vmctx, i32, i32, i32, i32) -> ();
            /// Returns an index for wasm's `memory.copy` for imported memories.
            imported_memory_copy(vmctx, i32, i32, i32, i32) -> ();
            /// Returns an index for wasm's `memory.fill` for locally defined memories.
            memory_fill(vmctx, i32, i32, i32, i32) -> ();
            /// Returns an index for wasm's `memory.fill` for imported memories.
            imported_memory_fill(vmctx, i32, i32, i32, i32) -> ();
            /// Returns an index for wasm's `memory.init` instruction.
            memory_init(vmctx, i32, i32, i32, i32, i32) -> ();
            /// Returns an index for wasm's `data.drop` instruction.
            data_drop(vmctx, i32) -> ();
            /// Returns an index for Wasm's `table.grow` instruction for `funcref`s.
            table_grow_funcref(vmctx, i32, i32, pointer) -> (i32);
            /// Returns an index for Wasm's `table.grow` instruction for `externref`s.
            table_grow_externref(vmctx, i32, i32, reference) -> (i32);
            /// Returns an index for Wasm's `table.fill` instruction for `externref`s.
            table_fill_externref(vmctx, i32, i32, reference, i32) -> ();
            /// Returns an index for Wasm's `table.fill` instruction for `funcref`s.
            table_fill_funcref(vmctx, i32, i32, pointer, i32) -> ();
            /// Returns an index to drop a `VMExternRef`.
            drop_externref(pointer) -> ();
            /// Returns an index to do a GC and then insert a `VMExternRef` into the
            /// `VMExternRefActivationsTable`.
            activations_table_insert_with_gc(vmctx, reference) -> ();
            /// Returns an index for Wasm's `global.get` instruction for `externref`s.
            externref_global_get(vmctx, i32) -> (reference);
            /// Returns an index for Wasm's `global.get` instruction for `externref`s.
            externref_global_set(vmctx, i32, reference) -> ();
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
            $name:ident( $( $param:ident ),* ) -> ( $( $result:ident ),* );
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

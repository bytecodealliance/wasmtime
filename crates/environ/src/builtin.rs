/// Helper macro to iterate over all builtin functions and their signatures.
#[macro_export]
macro_rules! foreach_builtin_function {
    ($mac:ident) => {
        $mac! {
            // Returns an index for wasm's `memory.grow` builtin function.
            memory32_grow(vmctx: vmctx, delta: i64, index: i32) -> pointer;
            // Returns an index for wasm's `table.copy` when both tables are locally
            // defined.
            table_copy(vmctx: vmctx, dst_index: i32, src_index: i32, dst: i64, src: i64, len: i64) -> bool;
            // Returns an index for wasm's `table.init`.
            table_init(vmctx: vmctx, table: i32, elem: i32, dst: i64, src: i64, len: i64) -> bool;
            // Returns an index for wasm's `elem.drop`.
            elem_drop(vmctx: vmctx, elem: i32);
            // Returns an index for wasm's `memory.copy`
            memory_copy(vmctx: vmctx, dst_index: i32, dst: i64, src_index: i32, src: i64, len: i64) -> bool;
            // Returns an index for wasm's `memory.fill` instruction.
            memory_fill(vmctx: vmctx, memory: i32, dst: i64, val: i32, len: i64) -> bool;
            // Returns an index for wasm's `memory.init` instruction.
            memory_init(vmctx: vmctx, memory: i32, data: i32, dst: i64, src: i32, len: i32) -> bool;
            // Returns a value for wasm's `ref.func` instruction.
            ref_func(vmctx: vmctx, func: i32) -> pointer;
            // Returns an index for wasm's `data.drop` instruction.
            data_drop(vmctx: vmctx, data: i32);
            // Returns a table entry after lazily initializing it.
            table_get_lazy_init_func_ref(vmctx: vmctx, table: i32, index: i64) -> pointer;
            // Returns an index for Wasm's `table.grow` instruction for `funcref`s.
            table_grow_func_ref(vmctx: vmctx, table: i32, delta: i64, init: pointer) -> pointer;
            // Returns an index for Wasm's `table.fill` instruction for `funcref`s.
            table_fill_func_ref(vmctx: vmctx, table: i32, dst: i64, val: pointer, len: i64) -> bool;
            // Returns an index for wasm's `memory.atomic.notify` instruction.
            #[cfg(feature = "threads")]
            memory_atomic_notify(vmctx: vmctx, memory: i32, addr: i64, count: i32) -> i64;
            // Returns an index for wasm's `memory.atomic.wait32` instruction.
            #[cfg(feature = "threads")]
            memory_atomic_wait32(vmctx: vmctx, memory: i32, addr: i64, expected: i32, timeout: i64) -> i64;
            // Returns an index for wasm's `memory.atomic.wait64` instruction.
            #[cfg(feature = "threads")]
            memory_atomic_wait64(vmctx: vmctx, memory: i32, addr: i64, expected: i64, timeout: i64) -> i64;
            // Invoked when fuel has run out while executing a function.
            out_of_gas(vmctx: vmctx) -> bool;
            // Invoked when we reach a new epoch.
            new_epoch(vmctx: vmctx) -> i64;
            // Invoked before malloc returns.
            #[cfg(feature = "wmemcheck")]
            check_malloc(vmctx: vmctx, addr: i32, len: i32) -> bool;
            // Invoked before the free returns.
            #[cfg(feature = "wmemcheck")]
            check_free(vmctx: vmctx, addr: i32) -> bool;
            // Invoked before a load is executed.
            #[cfg(feature = "wmemcheck")]
            check_load(vmctx: vmctx, num_bytes: i32, addr: i32, offset: i32) -> bool;
            // Invoked before a store is executed.
            #[cfg(feature = "wmemcheck")]
            check_store(vmctx: vmctx, num_bytes: i32, addr: i32, offset: i32) -> bool;
            // Invoked after malloc is called.
            #[cfg(feature = "wmemcheck")]
            malloc_start(vmctx: vmctx);
            // Invoked after free is called.
            #[cfg(feature = "wmemcheck")]
            free_start(vmctx: vmctx);
            // Invoked when wasm stack pointer is updated.
            #[cfg(feature = "wmemcheck")]
            update_stack_pointer(vmctx: vmctx, value: i32);
            // Invoked before memory.grow is called.
            #[cfg(feature = "wmemcheck")]
            update_mem_size(vmctx: vmctx, num_bytes: i32);

            // Drop a non-stack GC reference (eg an overwritten table entry)
            // once it will no longer be used again. (Note: `val` is not of type
            // `reference` because it needn't appear in any stack maps, as it
            // must not be live after this call.)
            #[cfg(feature = "gc-drc")]
            drop_gc_ref(vmctx: vmctx, val: i32);

            // Do a GC, treating the optional `root` as a GC root and returning
            // the updated `root` (so that, in the case of moving collectors,
            // callers have a valid version of `root` again).
            #[cfg(feature = "gc-drc")]
            gc(vmctx: vmctx, root: i32) -> i64;

            // Allocate a new, uninitialized GC object and return a reference to
            // it.
            #[cfg(feature = "gc-drc")]
            gc_alloc_raw(
                vmctx: vmctx,
                kind: i32,
                module_interned_type_index: i32,
                size: i32,
                align: i32
            ) -> i64;

            // Intern a `funcref` into the GC heap, returning its
            // `FuncRefTableId`.
            //
            // This libcall may not GC.
            #[cfg(feature = "gc")]
            intern_func_ref_for_gc_heap(
                vmctx: vmctx,
                func_ref: pointer
            ) -> i64;

            // Get the raw `VMFuncRef` pointer associated with a
            // `FuncRefTableId` from an earlier `intern_func_ref_for_gc_heap`
            // call.
            //
            // This libcall may not GC.
            //
            // Passes in the `ModuleInternedTypeIndex` of the funcref's expected
            // type, or `ModuleInternedTypeIndex::reserved_value()` if we are
            // getting the function reference as an untyped `funcref` rather
            // than a typed `(ref $ty)`.
            //
            // TODO: We will want to eventually expose the table directly to
            // Wasm code, so that it doesn't need to make a libcall to go from
            // id to `VMFuncRef`. That will be a little tricky: it will also
            // require updating the pointer to the slab in the `VMContext` (or
            // `VMRuntimeLimits` or wherever we put it) when the slab is
            // resized.
            #[cfg(feature = "gc")]
            get_interned_func_ref(
                vmctx: vmctx,
                func_ref_id: i32,
                module_interned_type_index: i32
            ) -> pointer;

            // Builtin implementation of the `array.new_data` instruction.
            #[cfg(feature = "gc")]
            array_new_data(
                vmctx: vmctx,
                array_interned_type_index: i32,
                data_index: i32,
                data_offset: i32,
                len: i32
            ) -> i64;

            // Builtin implementation of the `array.new_elem` instruction.
            #[cfg(feature = "gc")]
            array_new_elem(
                vmctx: vmctx,
                array_interned_type_index: i32,
                elem_index: i32,
                elem_offset: i32,
                len: i32
            ) -> i64;

            // Builtin implementation of the `array.copy` instruction.
            #[cfg(feature = "gc")]
            array_copy(
                vmctx: vmctx,
                dst_array: i32,
                dst_index: i32,
                src_array: i32,
                src_index: i32,
                len: i32
            ) -> bool;

            // Builtin implementation of the `array.init_data` instruction.
            #[cfg(feature = "gc")]
            array_init_data(
                vmctx: vmctx,
                array_interned_type_index: i32,
                array: i32,
                dst_index: i32,
                data_index: i32,
                data_offset: i32,
                len: i32
            ) -> bool;

            // Builtin implementation of the `array.init_elem` instruction.
            #[cfg(feature = "gc")]
            array_init_elem(
                vmctx: vmctx,
                array_interned_type_index: i32,
                array: i32,
                dst: i32,
                elem_index: i32,
                src: i32,
                len: i32
            ) -> bool;

            // Returns whether `actual_engine_type` is a subtype of
            // `expected_engine_type`.
            #[cfg(feature = "gc")]
            is_subtype(
                vmctx: vmctx,
                actual_engine_type: i32,
                expected_engine_type: i32
            ) -> i32;

            // Returns an index for Wasm's `table.grow` instruction for GC references.
            #[cfg(feature = "gc")]
            table_grow_gc_ref(vmctx: vmctx, table: i32, delta: i64, init: i32) -> pointer;

            // Returns an index for Wasm's `table.fill` instruction for GC references.
            #[cfg(feature = "gc")]
            table_fill_gc_ref(vmctx: vmctx, table: i32, dst: i64, val: i32, len: i64) -> bool;

            // Raises an unconditional trap with the specified code.
            //
            // This is used when signals-based-traps are disabled for backends
            // when an illegal instruction can't be executed for example.
            trap(vmctx: vmctx, code: u8);

            // Raises an unconditional trap where the trap information must have
            // been previously filled in.
            raise(vmctx: vmctx);
        }
    };
}

/// Helper macro to define a builtin type such as `BuiltinFunctionIndex` and
/// `ComponentBuiltinFunctionIndex` using the iterator macro, e.g.
/// `foreach_builtin_function`, as the way to generate accessor methods.
macro_rules! declare_builtin_index {
    ($index_name:ident, $iter:ident) => {
        /// An index type for builtin functions.
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $index_name(u32);

        impl $index_name {
            /// Create a new builtin from its raw index
            pub const fn from_u32(i: u32) -> Self {
                assert!(i < Self::len());
                Self(i)
            }

            /// Return the index as an u32 number.
            pub const fn index(&self) -> u32 {
                self.0
            }

            $iter!(declare_builtin_index_constructors);
        }
    };
}

/// Helper macro used by the above macro.
macro_rules! declare_builtin_index_constructors {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
        )*
    ) => {
        declare_builtin_index_constructors!(
            @indices;
            0;
            $( $( #[$attr] )* $name; )*
        );

        /// Returns a symbol name for this builtin.
        pub fn name(&self) -> &'static str {
            $(
                $( #[$attr] )*
                if *self == Self::$name() {
                    return stringify!($name);
                }
            )*
            unreachable!()
        }
    };

    // Base case: no more indices to declare, so define the total number of
    // function indices.
    (
        @indices;
        $len:expr;
    ) => {
        /// Returns the total number of builtin functions.
        pub const fn len() -> u32 {
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
        #[allow(missing_docs, reason = "macro-generated")]
        pub const fn $this_name() -> Self {
            Self($index)
        }

        declare_builtin_index_constructors!(
            @indices;
            ($index + 1);
            $( $( #[$rest_attr] )* $rest_name; )*
        );
    }
}

// Define `struct BuiltinFunctionIndex`
declare_builtin_index!(BuiltinFunctionIndex, foreach_builtin_function);

/// Return value of [`BuiltinFunctionIndex::trap_sentinel`].
pub enum TrapSentinel {
    /// A falsy or zero value indicates a trap.
    Falsy,
    /// The value `-2` indicates a trap (used for growth-related builtins).
    NegativeTwo,
    /// The value `-1` indicates a trap .
    NegativeOne,
    /// Any negative value indicates a trap.
    Negative,
}

impl BuiltinFunctionIndex {
    /// Describes the return value of this builtin and what represents a trap.
    ///
    /// Libcalls don't raise traps themselves and instead delegate to compilers
    /// to do so. This means that some return values of libcalls indicate a trap
    /// is happening and this is represented with sentinel values. This function
    /// returns the description of the sentinel value which indicates a trap, if
    /// any. If `None` is returned from this function then this builtin cannot
    /// generate a trap.
    #[allow(unreachable_code, unused_macro_rules, reason = "macro-generated code")]
    pub fn trap_sentinel(&self) -> Option<TrapSentinel> {
        macro_rules! trap_sentinel {
            (
                $(
                    $( #[$attr:meta] )*
                    $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
                )*
            ) => {{
                $(
                    $(#[$attr])*
                    if *self == BuiltinFunctionIndex::$name() {
                        let mut _ret = None;
                        $(_ret = Some(trap_sentinel!(@get $name $result));)?
                        return _ret;
                    }
                )*

                None
            }};

            // Growth-related functions return -2 as a sentinel.
            (@get memory32_grow pointer) => (TrapSentinel::NegativeTwo);
            (@get table_grow_func_ref pointer) => (TrapSentinel::NegativeTwo);
            (@get table_grow_gc_ref pointer) => (TrapSentinel::NegativeTwo);

            // Atomics-related functions return a negative value indicating trap
            // indicate a trap.
            (@get memory_atomic_notify i64) => (TrapSentinel::Negative);
            (@get memory_atomic_wait32 i64) => (TrapSentinel::Negative);
            (@get memory_atomic_wait64 i64) => (TrapSentinel::Negative);

            // GC-related functions return a 64-bit value which is negative to
            // indicate a trap.
            (@get gc i64) => (TrapSentinel::Negative);
            (@get gc_alloc_raw i64) => (TrapSentinel::Negative);
            (@get array_new_data i64) => (TrapSentinel::Negative);
            (@get array_new_elem i64) => (TrapSentinel::Negative);

            // The final epoch represents a trap
            (@get new_epoch i64) => (TrapSentinel::NegativeOne);

            // These libcalls can't trap
            (@get ref_func pointer) => (return None);
            (@get table_get_lazy_init_func_ref pointer) => (return None);
            (@get get_interned_func_ref pointer) => (return None);
            (@get intern_func_ref_for_gc_heap i64) => (return None);
            (@get is_subtype i32) => (return None);

            // Bool-returning functions use `false` as an indicator of a trap.
            (@get $name:ident bool) => (TrapSentinel::Falsy);

            (@get $name:ident $ret:ident) => (
                compile_error!(concat!("no trap sentinel registered for ", stringify!($name)))
            )
        }

        foreach_builtin_function!(trap_sentinel)
    }
}

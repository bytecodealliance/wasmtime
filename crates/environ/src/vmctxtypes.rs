//! Centralized definitions of the layouts of Wasmtime's two "vmctx" types:
//! `VMContext`, the runtime context for a core Wasm instance, and
//! `VMComponentContext`, the runtime context for a component.
//!
//! Unlike the `VM*` types defined by `for_each_vm_type!`, neither of these has
//! a corresponding `#[repr(C)]` Rust `struct`: their sizes depend on the module
//! or component being instantiated, so they are dynamically laid out and
//! accessed exclusively through the computed offsets in `VMOffsets` and
//! `VMComponentOffsets`. That layout is defined exactly once here, via the
//! higher-order `for_each_vmctx_type!` macro, and each consumer generates its
//! view of it from that single source of truth.

/// Invoke the given macro `$mac` once, passing it the layout of each of
/// Wasmtime's "vmctx" types.
///
/// This is a higher-order macro: callers define a `macro_rules!` macro that
/// matches the grammar described below and pass its name as an argument to this
/// macro's invocation, e.g. `for_each_vmctx_type!(define_vmctx_offsets)`.
///
/// # Grammar
///
/// The layout below is written in exactly the same grammar that it is handed to
/// `$mac` in: this macro has a single rule, and that rule does nothing but
/// forward those tokens along. Writing the layout in the grammar that consumers
/// match makes it more verbose than a bespoke input syntax would be, but in
/// exchange there is no normalization pass in between, so what a consumer matches
/// is exactly what a reader of the layout sees.
///
/// `$mac` receives one brace-delimited group per vmctx type:
///
/// ```ignore
/// {
///     VMContext vmctx
///     static { ...entries... }
///     dynamic { ...entries... }
/// }
/// ```
///
/// where `vmctx` is the name used for accessor methods of that type, `static`
/// holds the fixed-width prefix whose offsets depend only on the target pointer
/// size, and `dynamic` holds the rest, whose offsets additionally depend on the
/// module or component being compiled.
///
/// Each entry within a section is a keyword naming the entry's shape followed by
/// a single delimited group, so that a consumer can iterate over a section with
/// `$( ...$kind:ident $entry:tt... )*` and dispatch on one whole entry at a time
/// instead of munching the section token by token. An entry is one of:
///
/// * `align { ptr }` or `align { N }`: round the running offset up to the target
///   pointer size, or to `N` bytes. Alignment is *always* explicit: it is never
///   derived from a field's type, because the layout being described here does
///   not necessarily align every field to its natural alignment, and inserting
///   padding that the layout does not actually have would silently corrupt every
///   subsequent offset.
///
/// * `field { <attrs> <name>: <ty> }`: a single field.
///
/// * `array { <attrs> <name>[<count>; <IndexType>]: <ty> }` (`dynamic` only): an
///   array of `self.<count>` elements, indexed by `<IndexType>`.
///
/// * `optional { <attrs> <name>[if <flag>]: <ty> }` (`dynamic` only): a field that
///   is present when `self.<flag>` is true and absent (zero-sized) otherwise.
///
/// A field's type is always the last thing in its entry. Consumers capture it
/// as a trailing `$($fty:tt)*` and re-dispatch on its tokens only where they
/// actually need to classify it into a size or a Cranelift type; a `:ty`
/// capture would be opaque forever, and so could never be classified at
/// all.
///
/// `<attrs>` is a possibly-empty sequence of marker attributes, which consumers
/// match with `$(# $fattr:tt)*`. They do not affect the layout, but they do affect
/// the accesses generated for the field:
///
/// * `#[aggregate]`: this field is a composite (a nested struct, or an array of
///   them) rather than a single scalar. Compiled code accesses such a field's
///   interior piecewise, so there is no one Cranelift type for the field as a
///   whole and no alias-region accessor is generated for it. Its offset is still
///   generated, since that is what interior accesses are computed relative to.
///
/// * `#[readonly]` and/or `#[can_move]`: describe how Cranelift may treat loads
///   and stores of this field.
///
/// * `#[access_as = Type]`: this field is *declared* with one type (because that
///   is what determines its size and stride) but *accessed* as another. For
///   example, the component context's `may_leave` flags are each stored in a
///   whole `VMGlobalDefinition` but only ever accessed as a `u32`.
///
/// * `#[ptr_size_offset]` (`dynamic` only): this field's offset is a function
///   of the target pointer size alone, even though it lives in the `dynamic`
///   section, because it and everything before it have sizes that do not depend
///   on the vmctx's shape. These fields get generated accessors that do not
///   need to be parameterized over `VMOffsets`, only `GetPtrSize`.
///
/// * `#[pointee(<attrs> <Region> as <name> $([<IndexType>])? : <ty>)]`: this
///   field is a pointer to memory that lives *outside* the vmctx and so is not
///   part of its layout at all, but that compiled code reaches only by loading
///   this field first. `<Region>` names the alias region that memory belongs
///   to, `<name>` is the accessor generated for it, and `<ty>` is the type it is
///   accessed as. With an `[<IndexType>]`, the pointee is an array of `<ty>`
///   indexed by `<IndexType>` and each element gets its own alias region;
///   without one, it is a single `<ty>`. The pointee's `<attrs>` are its own:
///   how compiled code may treat a load of the pointee is independent of how it
///   may treat a load of the pointer.
///
/// Doc comments are deliberately *not* accepted on fields; use `//` comments for
/// prose about the layout. Accessor documentation is synthesized from the field
/// names instead, so that there is only one place a field can be described.
///
/// A consumer that only cares about one of the two types can filter with a
/// literal-name arm followed by a catch-all, e.g.
///
/// ```ignore
/// (@one VMContext $snake:ident dynamic { $($dyn:tt)* }) => { ...generate... };
/// (@one $other:ident $snake:ident dynamic { $($dyn:tt)* }) => {};
/// ```
#[macro_export]
macro_rules! for_each_vmctx_type {
    ($mac:ident) => {
        $mac! {
            {
                VMContext vmctx

                // Fixed-width data comes first so that the calculation of these
                // fields' offsets is a compile-time constant when using
                // `HostPtr`.
                static {
                    field { #[readonly] #[can_move] magic: u32 }

                    // NB: this is where the four bytes of padding after `magic`
                    // live on targets with eight-byte pointers.
                    align { ptr }

                    field { #[readonly] #[can_move] store_context: VmPtr<VMStoreContext> }

                    // NB: `VMBuiltinFunctionsArray` is a `repr(C)` struct of
                    // `unsafe extern "C" fn` fields rather than a true array,
                    // so its elements are pointer-width and pointer-aligned.
                    field {
                        #[readonly]
                        #[can_move]
                        #[pointee(
                            #[readonly]
                            #[can_move]
                            BuiltinFunctionsArray as builtin_functions_array[BuiltinFunctionIndex]:
                                unsafe extern "C" fn
                        )]
                        builtin_functions: VmPtr<VMBuiltinFunctionsArray>
                    }

                    field {
                        #[pointee(EpochCounter as epoch_counter: AtomicU64)]
                        epoch_ptr: VmPtr<AtomicU64>
                    }

                    // A pointer that different collectors use however they see
                    // fit.
                    field { #[readonly] #[can_move] gc_heap_data: VmPtr<u8> }

                    field {
                        #[readonly]
                        #[can_move]
                        #[pointee(
                            #[readonly]
                            #[can_move]
                            TypeIdsArray as type_ids_array[ModuleInternedTypeIndex]:
                                VMSharedTypeIndex
                        )]
                        type_ids: VmPtr<VMSharedTypeIndex>
                    }
                }

                // Variable-width fields come after the fixed-width fields
                // above. Memory-related items are placed first as they are some
                // of the most frequently accessed items, and minimizing their
                // offset can shrink the size of load/store instruction offset
                // immediates on platforms like x64 and Pulley (e.g. fit in an
                // 8-bit offset instead of needing a 32-bit offset).
                dynamic {
                    array {
                        #[aggregate]
                        imported_memories[num_imported_memories; MemoryIndex]: VMMemoryImport
                    }

                    array {
                        #[readonly]
                        #[can_move]
                        memories[num_defined_memories; DefinedMemoryIndex]: VmPtr<VMMemoryDefinition>
                    }

                    array {
                        #[aggregate]
                        owned_memories[num_owned_memories; OwnedMemoryIndex]: VMMemoryDefinition
                    }

                    array {
                        #[aggregate]
                        imported_functions[num_imported_functions; FuncIndex]: VMFunctionImport
                    }

                    array {
                        #[aggregate]
                        imported_tables[num_imported_tables; TableIndex]: VMTableImport
                    }

                    array {
                        #[aggregate]
                        imported_globals[num_imported_globals; GlobalIndex]: VMGlobalImport
                    }

                    array {
                        #[aggregate]
                        imported_tags[num_imported_tags; TagIndex]: VMTagImport
                    }

                    array {
                        #[aggregate]
                        tables[num_defined_tables; DefinedTableIndex]: VMTableDefinition
                    }

                    align { 16 }

                    array {
                        #[aggregate]
                        globals[num_defined_globals; DefinedGlobalIndex]: VMGlobalDefinition
                    }

                    array {
                        #[aggregate]
                        tags[num_defined_tags; DefinedTagIndex]: VMTagDefinition
                    }

                    array {
                        #[aggregate]
                        func_refs[num_escaped_funcs; FuncRefIndex]: VMFuncRef
                    }

                    optional {
                        #[aggregate]
                        startup_func_ref[if has_startup_func]: VMFuncRef
                    }

                    array {
                        runtime_data_bases[num_runtime_data; RuntimeDataIndex]: VmPtr<u8>
                    }

                    array {
                        runtime_data_lengths[num_runtime_data; RuntimeDataIndex]: u32
                    }
                }
            }

            {
                VMComponentContext vmcomponent

                static {
                    // NB: `magic` must be at offset zero; this is relied upon by
                    // `VMComponentContext::from_opaque`.
                    field { #[readonly] #[can_move] magic: u32 }

                    align { ptr }

                    field {
                        #[readonly]
                        #[pointee(
                            #[readonly]
                            #[can_move]
                            ComponentBuiltinFunctionsArray as builtins_array[ComponentBuiltinFunctionIndex]:
                                unsafe extern "C" fn
                        )]
                        builtins: VmPtr<VMComponentBuiltins>
                    }

                    field { #[readonly] #[can_move] store_context: VmPtr<VMStoreContext> }
                }

                dynamic {
                    align { 16 }

                    // Each of these flags gets a whole `VMGlobalDefinition`'s
                    // worth of space, but only its first four bytes are ever
                    // accessed.
                    //
                    // NB: these flags come first, before any field whose offset
                    // depends on the component's shape, so that their offsets
                    // are a function of the target pointer size alone, as marked
                    // by `#[ptr_size_offset]`. Core Wasm compilation does not
                    // have the enclosing `VMComponentContext`'s offsets on hand,
                    // but must still be able to compute these flags' offsets to
                    // build the alias regions for accessing them.

                    array {
                        #[ptr_size_offset]
                        #[access_as = u32]
                        may_leave[num_runtime_component_instances; RuntimeComponentInstanceIndex]: VMGlobalDefinition
                    }

                    align { ptr }

                    array {
                        #[aggregate]
                        trampoline_func_refs[num_trampolines; TrampolineIndex]: VMFuncRef
                    }

                    array {
                        #[aggregate]
                        intrinsic_func_refs[num_unsafe_intrinsics; UnsafeIntrinsic]: VMFuncRef
                    }

                    array {
                        #[aggregate]
                        lowerings[num_lowerings; LoweredIndex]: VMLowering
                    }

                    array {
                        memories
                            [num_runtime_memories; RuntimeMemoryIndex]: VmPtr<VMMemoryDefinition>
                    }

                    array {
                        #[aggregate]
                        tables[num_runtime_tables; RuntimeTableIndex]: VMTableImport
                    }

                    array {
                        reallocs[num_runtime_reallocs; RuntimeReallocIndex]: VmPtr<VMFuncRef>
                    }

                    array {
                        callbacks[num_runtime_callbacks; RuntimeCallbackIndex]: VmPtr<VMFuncRef>
                    }

                    array {
                        post_returns[num_runtime_post_returns; RuntimePostReturnIndex]: VmPtr<VMFuncRef>
                    }

                    array {
                        #[readonly]
                        resource_destructors[num_resources; ResourceIndex]: VmPtr<VMFuncRef>
                    }
                }
            }
        }
    };
}

/// Round `offset` up to a multiple of `align`.
#[inline]
pub(crate) fn align_up(offset: u32, align: u32) -> u32 {
    debug_assert!(align.is_power_of_two());
    (offset + (align - 1)) & !(align - 1)
}

/// Add two offsets, panicking on overflow.
#[inline]
pub(crate) fn cadd(a: u32, b: u32) -> u32 {
    a.checked_add(b).unwrap()
}

/// Multiply an element count by an element size, panicking on overflow.
#[inline]
pub(crate) fn cmul(count: u32, size: u32) -> u32 {
    count.checked_mul(size).unwrap()
}

/// Classify one of the field types accepted by `for_each_vmctx_type!` to its
/// size in bytes as a `u32`, given a `PtrSize`.
#[allow(
    unused_macro_rules,
    reason = "some element types only appear in `VMComponentContext`, and so are \
              unused when the `component-model` feature is disabled"
)]
macro_rules! vmctx_field_size {
    (($p:expr) u32) => {
        4u32
    };
    (($p:expr) VmPtr < $g:ident >) => {
        u32::from($p)
    };
    // `VMLowering` is a pair of pointers, and is not itself defined by
    // `for_each_vm_type!`.
    (($p:expr) VMLowering) => {
        2u32 * u32::from($p)
    };
    // Anything else names one of the `VM*` types, and so has an
    // `offsets::VMFoo` generated for it by `for_each_vm_type!`. Deferring to
    // that keeps this macro from having to mirror the list of `VM*` types, and
    // a type that has no such entry fails to resolve here.
    (($p:expr) $Name:ident) => {
        u32::from(crate::vmoffsets::offsets::$Name($p).size())
    };
}

/// Classify a `for_each_vmctx_type!` alignment step to its alignment in bytes
/// as a `u32`, given a `PtrSize`.
macro_rules! vmctx_align_value {
    (($p:expr) ptr) => {
        u32::from($p)
    };
    (($p:expr) $n:literal) => {
        $n
    };
}

/// Generate the accessors for, and the layout computation of, the
/// dynamically-positioned fields of one of the vmctx types.
#[allow(
    unused_macro_rules,
    reason = "single dynamically-positioned fields only appear in \
              `VMComponentContext`, and so are unused when the `component-model` \
              feature is disabled"
)]
macro_rules! define_vmctx_dynamic_offsets {
    (@accessors ($s:ident) [ $($kind:ident $entry:tt)* ]) => {
        $( define_vmctx_dynamic_offsets!(@accessor ($s) $kind $entry); )*
    };

    (@accessor ($s:ident) align { $al:tt }) => {};

    (@accessor ($s:ident) field { $(# $fattr:tt)* $fname:ident : $($fty:tt)* }) => {
        #[doc = concat!("The offset of the `", stringify!($fname), "` field.")]
        #[inline]
        pub fn $fname(&$s) -> u32 {
            $s.$fname
        }
    };

    (@accessor ($s:ident) array {
        $(# $fattr:tt)* $fname:ident [ $count:ident ; $Index:ident ] : $($fty:tt)*
    }) => {
        #[doc = concat!("The offsets of the `", stringify!($fname), "` array.")]
        #[inline]
        pub fn $fname(&$s) -> $crate::ArrayOffsets<$Index> {
            $crate::ArrayOffsets::new(
                $s.$fname,
                vmctx_field_size!(($s.ptr.size()) $($fty)*),
                $s.$count,
            )
        }
    };

    (@accessor ($s:ident) optional {
        $(# $fattr:tt)* $fname:ident [ if $flag:ident ] : $($fty:tt)*
    }) => {
        #[doc = concat!(
            "The offset of the `", stringify!($fname), "` field.\n\n",
            "Panics if `", stringify!($flag), "` is false, in which case this \
             field is not present at all."
        )]
        #[inline]
        pub fn $fname(&$s) -> u32 {
            assert!($s.$flag);
            $s.$fname
        }
    };

    (@compute_fn ($s:ident, $next:ident) $snake:ident [ $($kind:ident $entry:tt)* ]) => {
        /// Compute the offset of each dynamically-positioned field, and this
        /// vmctx's total size.
        fn compute_field_offsets(&mut $s) {
            let mut $next = u32::from($s.ptr.$snake().end_of_static_fields());
            $( define_vmctx_dynamic_offsets!(@compute ($s, $next) $kind $entry); )*
            $s.size = $next;
        }
    };

    (@compute ($s:ident, $next:ident) align { $al:tt }) => {
        $next = crate::vmctxtypes::align_up($next, vmctx_align_value!(($s.ptr.size()) $al));
    };

    (@compute ($s:ident, $next:ident) field {
        $(# $fattr:tt)* $fname:ident : $($fty:tt)*
    }) => {
        $s.$fname = $next;
        $next = crate::vmctxtypes::cadd($next, vmctx_field_size!(($s.ptr.size()) $($fty)*));
    };

    (@compute ($s:ident, $next:ident) array {
        $(# $fattr:tt)* $fname:ident [ $count:ident ; $Index:ident ] : $($fty:tt)*
    }) => {
        $s.$fname = $next;
        $next = crate::vmctxtypes::cadd(
            $next,
            crate::vmctxtypes::cmul($s.$count, vmctx_field_size!(($s.ptr.size()) $($fty)*)),
        );
    };

    (@compute ($s:ident, $next:ident) optional {
        $(# $fattr:tt)* $fname:ident [ if $flag:ident ] : $($fty:tt)*
    }) => {
        $s.$fname = $next;
        $next = crate::vmctxtypes::cadd(
            $next,
            if $s.$flag {
                vmctx_field_size!(($s.ptr.size()) $($fty)*)
            } else {
                0
            },
        );
    };
}

/// The offsets of one array field within a vmctx.
#[derive(Debug, Clone, Copy)]
pub struct ArrayOffsets<I> {
    begin: u32,
    stride: u32,
    count: u32,
    _index: core::marker::PhantomData<I>,
}

impl<I> ArrayOffsets<I> {
    /// Create the offsets for an array of `count` elements which begins at
    /// `begin` within its vmctx and whose elements are `stride` bytes apart.
    #[inline]
    pub fn new(begin: u32, stride: u32, count: u32) -> Self {
        ArrayOffsets {
            begin,
            stride,
            count,
            _index: core::marker::PhantomData,
        }
    }

    /// The offset of the start of this array within its vmctx.
    #[inline]
    pub fn begin(&self) -> u32 {
        self.begin
    }

    /// The number of bytes between the start of consecutive elements of this
    /// array.
    #[inline]
    pub fn stride(&self) -> u32 {
        self.stride
    }

    /// The number of elements in this array.
    #[inline]
    pub fn count(&self) -> u32 {
        self.count
    }
}

impl<I: VmctxArrayIndex> ArrayOffsets<I> {
    /// The offset of the given element within this array's vmctx.
    ///
    /// Panics if `index` is out of bounds for this array.
    #[inline]
    pub fn at(&self, index: I) -> u32 {
        let index = index.vmctx_array_index();
        assert!(index < self.count);
        self.begin + index * self.stride
    }
}

/// An index type that can be used to index one of a vmctx's arrays.
pub trait VmctxArrayIndex: Copy {
    /// This index's position within its array.
    fn vmctx_array_index(self) -> u32;
}

/// Implement `VmctxArrayIndex` for entity references.
macro_rules! impl_vmctx_array_index {
    ($($ty:ty),* $(,)?) => {
        $(
            impl $crate::VmctxArrayIndex for $ty {
                #[inline]
                fn vmctx_array_index(self) -> u32 {
                    self.as_u32()
                }
            }
        )*
    };
}

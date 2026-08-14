//! Offsets and sizes of various structs in `wasmtime::runtime::vm::*` that are
//! accessed directly by compiled Wasm code.

// The `VMContext` layout is not defined here: it is defined once, alongside
// `VMComponentContext`'s, in `for_each_vmctx_type!`. Everything in this module
// is either generated from that definition or is an offset that is not simply
// the offset of one of the layout's fields.

use crate::{
    DefinedGlobalIndex, DefinedMemoryIndex, DefinedTableIndex, DefinedTagIndex, FuncIndex,
    FuncRefIndex, GlobalIndex, MemoryIndex, Module, ModuleInternedTypeIndex, OwnedMemoryIndex,
    RuntimeDataIndex, TableIndex, TagIndex,
};

/// Number of slots in for `component_context` in the `VMStoreContext`. This is
/// defined by the component model's `context.{get,set}` intrinsics.
pub const NUM_COMPONENT_CONTEXT_SLOTS: usize = 2;

#[cfg(target_pointer_width = "32")]
fn cast_to_u32(sz: usize) -> u32 {
    u32::try_from(sz).unwrap()
}
#[cfg(target_pointer_width = "64")]
fn cast_to_u32(sz: usize) -> u32 {
    u32::try_from(sz).expect("overflow in cast from usize to u32")
}

/// Align an offset used in this module to a specific byte-width by rounding up
#[inline]
fn align(offset: u32, width: u32) -> u32 {
    (offset + (width - 1)) / width * width
}

/// Generate the [`offsets`] module: a `struct VMFoo<P: PtrSize>(P)` for each
/// `VM*` type, with a method per field returning that field's offset, plus
/// `size` and `align` methods.
macro_rules! define_vm_type_offsets {
    // `UnsafeCell<T>` is `repr(transparent)`, so it has exactly `T`'s layout;
    // delegate to the inner type.
    (@size ($p:expr) UnsafeCell < $inner:tt >) => { define_vm_type_offsets!(@size ($p) $inner) };

    // Classify a field type to its size in bytes as a `u32`, given `$p` (the
    // target pointer size as a `u8`). All `VmPtr<_>` and `Option<VmPtr<_>>`
    // fields are pointer-sized; the `Defined*Index` types are `u32` entity
    // references; and `VMGlobalKind` is a `repr(C, u32)` enum with a `u32`
    // payload.
    (@size ($p:expr) VmPtr < $g:ty >) => { u32::from($p) };
    (@size ($p:expr) Option < VmPtr < $g:ty >>) => { u32::from($p) };
    (@size ($p:expr) AtomicUsize) => { u32::from($p) };
    (@size ($p:expr) usize) => { u32::from($p) };
    (@size ($p:expr) i64) => { 8u32 };
    (@size ($p:expr) u64) => { 8u32 };
    (@size ($p:expr) u32) => { 4u32 };
    // `VMGcRef` is a `NonZeroU32` newtype, so `Option<VMGcRef>` is niche-packed
    // back down into four bytes.
    (@size ($p:expr) NonZeroU32) => { 4u32 };
    (@size ($p:expr) Option < VMGcRef >) => { 4u32 };
    (@size ($p:expr) [u8; 16]) => { 16u32 };
    (@size ($p:expr) [u32; $n:expr]) => { 4u32 * u32::try_from($n).unwrap() };
    (@size ($p:expr) VMSharedTypeIndex) => { u32::from(($p).size_of_vmshared_type_index()) };
    (@size ($p:expr) DefinedTableIndex) => { 4u32 };
    (@size ($p:expr) DefinedMemoryIndex) => { 4u32 };
    (@size ($p:expr) DefinedTagIndex) => { 4u32 };
    (@size ($p:expr) VMGlobalKind) => { 8u32 };
    // `Range<T>` is a two-field `{ start: T, end: T }` struct.
    (@size ($p:expr) Range < *mut u8 >) => { 2u32 * u32::from($p) };
    // Nested `VM*` types recurse through their own generated offsets, keeping
    // this macro the single source of truth for their layout too.
    (@size ($p:expr) VMMemoryDefinition) => { u32::from(($p).vm_memory_definition().size()) };
    (@size ($p:expr) VMLazyThread) => { u32::from(($p).vm_lazy_thread().size()) };
    // `VMStackChain` is a `repr(usize, C)` enum, and is not itself defined by
    // `for_each_vm_type!`.
    (@size ($p:expr) VMStackChain) => { u32::from(($p).size_of_vmstack_chain()) };

    // As with `@size` above, `UnsafeCell<T>` has exactly `T`'s alignment.
    (@align ($p:expr) UnsafeCell < $inner:tt >) => { define_vm_type_offsets!(@align ($p) $inner) };

    // Classify a field type to its alignment in bytes as a `u32`, given `$p`
    // (the target pointer size as a `u8`).
    //
    // NB: 64-bit integers are assumed to be 8-aligned, which holds everywhere except
    // `i686-unknown-linux-gnu`, and the pointer size alone can't tell those apart. Types
    // with 64-bit fields must therefore put them first and force their own alignment with
    // `#[repr(C, align(8))]`, as `VMStoreContext` does.
    (@align ($p:expr) VmPtr < $g:ty >) => { u32::from($p) };
    (@align ($p:expr) Option < VmPtr < $g:ty >>) => { u32::from($p) };
    (@align ($p:expr) AtomicUsize) => { u32::from($p) };
    (@align ($p:expr) usize) => { u32::from($p) };
    (@align ($p:expr) i64) => { 8u32 };
    (@align ($p:expr) u64) => { 8u32 };
    (@align ($p:expr) u32) => { 4u32 };
    (@align ($p:expr) NonZeroU32) => { 4u32 };
    (@align ($p:expr) Option < VMGcRef >) => { 4u32 };
    (@align ($p:expr) [u8; 16]) => { 16u32 };
    (@align ($p:expr) [u32; $n:expr]) => { 4u32 };
    (@align ($p:expr) VMSharedTypeIndex) => { u32::from(($p).align_of_vmshared_type_index()) };
    (@align ($p:expr) DefinedTableIndex) => { 4u32 };
    (@align ($p:expr) DefinedMemoryIndex) => { 4u32 };
    (@align ($p:expr) DefinedTagIndex) => { 4u32 };
    (@align ($p:expr) VMGlobalKind) => { 4u32 };
    (@align ($p:expr) Range < *mut u8 >) => { u32::from($p) };
    (@align ($p:expr) VMMemoryDefinition) => { u32::from(($p).vm_memory_definition().align()) };
    (@align ($p:expr) VMLazyThread) => { u32::from(($p).vm_lazy_thread().align()) };
    (@align ($p:expr) VMStackChain) => { u32::from($p) };

    // Classify a `#[repr(...)]` to the minimum alignment it forces, as a `u32`.
    (@repr_align C) => { 1u32 };
    (@repr_align transparent) => { 1u32 };
    (@repr_align C, align($n:literal)) => {{ let align: u32 = $n; align }};

    // Emit a `pub fn` per field returning that field's offset, computed by
    // accumulating the aligned size of each preceding field. `$p`/`$o` are
    // caller-minted identifiers (for the pointer size and running offset) that
    // are threaded through the recursion so their hygiene stays consistent
    // across the emitted `let` bindings.
    //
    // Fields arrive pre-split into `[ $fname : $fty... ]` groups (see `@impl`
    // below), so each field's type is a raw token sequence that the `@size`
    // and `@align` classifiers can match against structurally.
    (@fields $Name:ident ($p:ident, $o:ident) prefix( $($prefix:tt)* )) => {};
    (@fields $Name:ident ($p:ident, $o:ident) prefix( $($prefix:tt)* )
        [ $fname:ident : $($fty:tt)* ]
        $($rest:tt)*
    ) => {
        #[doc = concat!(
            "The offset of the `", stringify!($fname),
            "` field of `", stringify!($Name), "`."
        )]
        #[inline]
        pub fn $fname(&self) -> u8 {
            let $p = self.0.size();
            let $o: u32 = 0;
            $($prefix)*
            let $o = align($o, define_vm_type_offsets!(@align ($p) $($fty)*));
            let _ = $p;
            u8::try_from($o).unwrap()
        }
        define_vm_type_offsets!(@fields $Name ($p, $o)
            prefix(
                $($prefix)*
                let $o = align($o, define_vm_type_offsets!(@align ($p) $($fty)*));
                let $o = $o + define_vm_type_offsets!(@size ($p) $($fty)*);
            )
            $($rest)*
        );
    };

    // Emit an `offsets::VMFoo` type's inherent `impl`. Fields are peeled off
    // the raw struct body one at a time into `[ $fname : $fty... ]` groups
    // accumulated in the `{ ... }` list; once the body is exhausted, the
    // terminal arm emits the per-field offset methods plus `align`/`size`.
    //
    // Splitting the body by hand (rather than matching `$fty:tt $(< $fgen:ty
    // >)?` within a repetition) is what lets each field's type reach the
    // `@size`/`@align` classifiers as raw tokens, so those classifiers can
    // require `Option`s to specifically be `Option<VmPtr<_>>`.
    (@impl $Name:ident [$($repr:tt)*] { $( [ $fname:ident : $($fty:tt)* ] )* }) => {
        impl<P: PtrSize> $Name<P> {
            define_vm_type_offsets!(@fields $Name (p, o) prefix()
                $( [ $fname : $($fty)* ] )*
            );

            #[doc = concat!("The alignment of the `", stringify!($Name), "` type.")]
            #[inline]
            pub fn align(&self) -> u8 {
                let p = self.0.size();
                let a: u32 = define_vm_type_offsets!(@repr_align $($repr)*);
                $(
                    let a = core::cmp::max(
                        a,
                        define_vm_type_offsets!(@align (p) $($fty)*),
                    );
                )*
                let _ = p;
                u8::try_from(a).unwrap()
            }

            #[doc = concat!("The size of the `", stringify!($Name), "` type.")]
            #[inline]
            pub fn size(&self) -> u8 {
                let p = self.0.size();
                let o: u32 = 0;
                $(
                    let o = align(o, define_vm_type_offsets!(@align (p) $($fty)*));
                    let o = o + define_vm_type_offsets!(@size (p) $($fty)*);
                )*
                let o = align(o, u32::from(self.align()));
                let _ = p;
                u8::try_from(o).unwrap()
            }
        }
    };
    // Consume one field's attributes, visibility, and name, then collect its
    // type tokens. None of the field attributes (doc comments and the
    // `#[aggregate]`/`#[readonly]`/`#[can_move]` markers) affect layout, so they
    // are all discarded here.
    (@impl $Name:ident $repr:tt { $($groups:tt)* }
        $(#[$($attr:tt)*])* $fvis:vis $fname:ident : $($rest:tt)*
    ) => {
        define_vm_type_offsets!(@impl_ty $Name $repr { $($groups)* } $fname [] $($rest)*);
    };
    // Accumulate one field's type tokens up to its terminating comma, then
    // append the completed `[ $fname : $fty... ]` group and resume `@impl`.
    (@impl_ty $Name:ident $repr:tt { $($groups:tt)* } $fname:ident [ $($fty:tt)* ] , $($rest:tt)*) => {
        define_vm_type_offsets!(@impl $Name $repr { $($groups)* [ $fname : $($fty)* ] } $($rest)*);
    };
    (@impl_ty $Name:ident $repr:tt { $($groups:tt)* } $fname:ident [ $($fty:tt)* ] $tok:tt $($rest:tt)*) => {
        define_vm_type_offsets!(@impl_ty $Name $repr { $($groups)* } $fname [ $($fty)* $tok ] $($rest)*);
    };

    // Top-level entry: the list of `VM*` type definitions.
    ( $(
        $(#[doc = $sdoc:literal])*
        $(#[cfg($($scfg:tt)*)])?
        $(#[derive($($d:ident),*)])?
        #[repr($($repr:tt)*)]
        #[snake_name = $snake:ident]
        $svis:vis struct $Name:ident {
            $($body:tt)*
        }
    )* ) => {
        $(
            #[doc = concat!("Offsets of fields within the `", stringify!($Name), "` type.")]
            pub struct $Name<P: PtrSize>(pub P);

            define_vm_type_offsets!(@impl $Name [$($repr)*] {} $($body)*);
        )*
    };
}

/// Generate a `struct VMContext<P: PtrSize>(P)`-style wrapper for each vmctx
/// type, with a method per statically-positioned field returning that field's
/// offset.
#[allow(
    unused_macro_rules,
    reason = "only `VMComponentContext` has a `#[ptr_size_offset]` prefix in its \
              `dynamic` section, so those arms go unused for `VMContext`"
)]
macro_rules! define_vmctx_static_offsets {
    // Munch the `static` section, threading a closed-form expression for the
    // running offset. Once it is exhausted, keep going into the `dynamic`
    // section's `#[ptr_size_offset]` prefix, whose offsets are closed-form too.
    (@chain $p:ident [ $($prev:tt)* ] [] [ $($dyn:tt)* ]) => {
        /// The offset just past this type's last statically-positioned field.
        ///
        /// Everything after this point is dynamically sized.
        #[inline]
        pub fn end_of_static_fields(&self) -> u8 {
            let $p = self.0.size();
            let _ = $p;
            u8::try_from($($prev)*).unwrap()
        }

        define_vmctx_static_offsets!(@dyn_chain $p [ $($prev)* ] [ $($dyn)* ]);
    };
    (@chain $p:ident [ $($prev:tt)* ] [ align { $al:tt } $($rest:tt)* ] $dyn:tt) => {
        define_vmctx_static_offsets!(
            @chain $p
            [ crate::vmctxtypes::align_up($($prev)*, vmctx_align_value!(($p) $al)) ]
            [ $($rest)* ]
            $dyn
        );
    };
    (@chain $p:ident [ $($prev:tt)* ] [
        field { $(# $fattr:tt)* $fname:ident : $($fty:tt)* } $($rest:tt)*
    ] $dyn:tt) => {
        #[doc = concat!("The offset of the `", stringify!($fname), "` field.")]
        #[inline]
        pub fn $fname(&self) -> u8 {
            let $p = self.0.size();
            let _ = $p;
            u8::try_from($($prev)*).unwrap()
        }
        define_vmctx_static_offsets!(
            @chain $p
            [ $($prev)* + vmctx_field_size!(($p) $($fty)*) ]
            [ $($rest)* ]
            $dyn
        );
    };

    // Munch the `dynamic` section's leading run of `#[ptr_size_offset]` entries,
    // continuing to thread the closed-form running offset.
    (@dyn_chain $p:ident [ $($prev:tt)* ] [ align { $al:tt } $($rest:tt)* ]) => {
        define_vmctx_static_offsets!(
            @dyn_chain $p
            [ crate::vmctxtypes::align_up($($prev)*, vmctx_align_value!(($p) $al)) ]
            [ $($rest)* ]
        );
    };
    (@dyn_chain $p:ident [ $($prev:tt)* ] [
        field { #[ptr_size_offset] $(# $fattr:tt)* $fname:ident : $($fty:tt)* } $($rest:tt)*
    ]) => {
        #[doc = concat!("The offset of the `", stringify!($fname), "` field.")]
        #[inline]
        pub fn $fname(&self) -> u32 {
            let $p = self.0.size();
            let _ = $p;
            $($prev)*
        }
        define_vmctx_static_offsets!(
            @dyn_chain $p
            [ $($prev)* + vmctx_field_size!(($p) $($fty)*) ]
            [ $($rest)* ]
        );
    };
    (@dyn_chain $p:ident [ $($prev:tt)* ] [
        array {
            #[ptr_size_offset] $(# $fattr:tt)*
            $fname:ident [ $count:ident ; $Index:ident ] : $($fty:tt)*
        } $($rest:tt)*
    ]) => {
        // NB: this takes `impl VmctxArrayIndex` rather than the declared
        // `$Index` because these wrappers are compiled even when the
        // `component-model` feature is off, and some of the index types are not.
        #[doc = concat!(
            "The offset of the `index`th element of the `", stringify!($fname),
            "` array.\n\nThis is not bounds checked: the array's length is a \
             property of the particular module or component being compiled, which \
             is exactly what this wrapper does not know."
        )]
        #[inline]
        pub fn $fname(&self, index: impl crate::VmctxArrayIndex) -> u32 {
            let $p = self.0.size();
            let _ = $p;
            let index = index.vmctx_array_index();
            ($($prev)*) + vmctx_field_size!(($p) $($fty)*) * index
        }
        // An array's size depends on how many elements this particular module or
        // component needs, so nothing after it has a closed-form offset and the
        // chain necessarily stops here.
    };
    // Anything else ends the closed-form prefix.
    (@dyn_chain $p:ident [ $($prev:tt)* ] [ $($rest:tt)* ]) => {};

    ( $(
        {
            $Name:ident $snake:ident
            static { $($stat:tt)* }
            dynamic { $($dyn:tt)* }
        }
    )* ) => {
        $(
            #[doc = concat!("Offsets of the statically-positioned fields within the `",
                            stringify!($Name), "` type.")]
            pub struct $Name<P: PtrSize>(pub P);

            impl<P: PtrSize> $Name<P> {
                define_vmctx_static_offsets!(@chain ptr [ 0u32 ] [ $($stat)* ] [ $($dyn)* ]);
            }
        )*
    };
}

/// Offsets of fields within the various `VM*` types and within the two vmctx
/// types, parameterized over a target `PtrSize` so that they can be computed
/// during cross compilation.
///
/// These types are namespaced within their own module so that they never collide
/// with the real definitions of the `VM*` types themselves.
pub mod offsets {
    use super::{NUM_COMPONENT_CONTEXT_SLOTS, PtrSize, align};

    for_each_vm_type!(define_vm_type_offsets);
    for_each_vmctx_type!(define_vmctx_static_offsets);
}

/// Offsets within a `VMStoreContext` that are not simply the offset of one of
/// its fields, and so are not generated by `for_each_vm_type!`.
impl<P: PtrSize> offsets::VMStoreContext<P> {
    /// The offset of the `gc_heap.base` field within a `VMStoreContext`.
    pub fn gc_heap_base(&self) -> u8 {
        let offset = self.gc_heap() + self.0.vm_memory_definition().base();
        debug_assert!(offset < self.last_wasm_exit_trampoline_fp());
        offset
    }

    /// The offset of the `gc_heap.current_length` field within a
    /// `VMStoreContext`.
    pub fn gc_heap_current_length(&self) -> u8 {
        let offset = self.gc_heap() + self.0.vm_memory_definition().current_length();
        debug_assert!(offset < self.last_wasm_exit_trampoline_fp());
        offset
    }
}

/// Add a `fn vm_foo(&self) -> offsets::VMFoo<&Self>` accessor to `PtrSize` for
/// each `VM*` type.
macro_rules! define_ptr_size_vm_type_accessors {
    ( $(
        $(#[doc = $sdoc:literal])*
        $(#[cfg($($scfg:tt)*)])?
        $(#[derive($($d:ident),*)])?
        #[repr($($repr:tt)*)]
        #[snake_name = $snake:ident]
        $svis:vis struct $Name:ident {
            $($body:tt)*
        }
    )* ) => {
        $(
            #[doc = concat!("Get the [`offsets::", stringify!($Name), "`] offsets for this pointer size.")]
            #[inline]
            fn $snake(&self) -> offsets::$Name<&Self> {
                offsets::$Name(self)
            }
        )*
    };
}

/// Add a `fn vmctx(&self) -> offsets::VMContext<&Self>` accessor to `PtrSize`
/// for each vmctx type.
macro_rules! define_ptr_size_vmctx_type_accessors {
    ( $(
        {
            $Name:ident $snake:ident
            static { $($stat:tt)* }
            dynamic { $($dyn:tt)* }
        }
    )* ) => {
        $(
            #[doc = concat!("Get the [`offsets::", stringify!($Name),
                            "`] offsets for this pointer size.")]
            #[inline]
            fn $snake(&self) -> offsets::$Name<&Self> {
                offsets::$Name(self)
            }
        )*
    };
}

/// This class computes offsets to fields within `VMContext` and other
/// related structs that JIT code accesses directly.
#[derive(Debug, Clone, Copy)]
pub struct VMOffsets<P> {
    /// The size in bytes of a pointer on the target.
    pub ptr: P,
    /// The number of imported functions in the module.
    pub num_imported_functions: u32,
    /// The number of imported tables in the module.
    pub num_imported_tables: u32,
    /// The number of imported memories in the module.
    pub num_imported_memories: u32,
    /// The number of imported globals in the module.
    pub num_imported_globals: u32,
    /// The number of imported tags in the module.
    pub num_imported_tags: u32,
    /// The number of defined tables in the module.
    pub num_defined_tables: u32,
    /// The number of defined memories in the module.
    pub num_defined_memories: u32,
    /// The number of memories owned by the module instance.
    pub num_owned_memories: u32,
    /// The number of defined globals in the module.
    pub num_defined_globals: u32,
    /// The number of defined tags in the module.
    pub num_defined_tags: u32,
    /// The number of escaped functions in the module, the size of the func_refs
    /// array.
    pub num_escaped_funcs: u32,
    /// The number of runtime data segments in the module.
    pub num_runtime_data: u32,
    /// Whether or not the module has a start function.
    pub has_startup_func: bool,

    // Precalculated offsets of the dynamically-positioned fields.
    imported_memories: u32,
    memories: u32,
    owned_memories: u32,
    imported_functions: u32,
    imported_tables: u32,
    imported_globals: u32,
    imported_tags: u32,
    tables: u32,
    globals: u32,
    tags: u32,
    func_refs: u32,
    startup_func_ref: u32,
    runtime_data_bases: u32,
    runtime_data_lengths: u32,
    size: u32,
}

/// Trait used for the `ptr` representation of the field of `VMOffsets`
pub trait PtrSize {
    /// Returns the pointer size, in bytes, for the target.
    fn size(&self) -> u8;

    // Generate a `fn vm_foo(&self) -> offsets::VMFoo<&Self>` accessor for each
    // `VM*` type.
    for_each_vm_type!(define_ptr_size_vm_type_accessors);

    // Generate a `fn vmctx(&self) -> offsets::VMContext<&Self>` accessor for
    // each vmctx type.
    for_each_vmctx_type!(define_ptr_size_vmctx_type_accessors);

    /// Return the size of `VMSharedTypeIndex`.
    #[inline]
    fn size_of_vmshared_type_index(&self) -> u8 {
        4
    }

    /// Return the alignment of `VMSharedTypeIndex`.
    #[inline]
    fn align_of_vmshared_type_index(&self) -> u8 {
        4
    }

    /// This is the size of the largest value type (i.e. a V128).
    #[inline]
    fn maximum_value_size(&self) -> u8 {
        self.vm_global_definition().size()
    }

    /// Return the size of `*mut VMMemoryDefinition`.
    #[inline]
    fn size_of_vmmemory_pointer(&self) -> u8 {
        self.size()
    }

    // Offsets within `VMArrayCallHostFuncContext`.

    /// Return the offset of `VMArrayCallHostFuncContext::func_ref`.
    fn vmarray_call_host_func_context_func_ref(&self) -> u8 {
        u8::try_from(align(
            u32::try_from(core::mem::size_of::<u32>()).unwrap(),
            u32::from(self.size()),
        ))
        .unwrap()
    }

    /// Return the size of `VMStackChain`.
    fn size_of_vmstack_chain(&self) -> u8 {
        2 * self.size()
    }

    // Offsets within `VMStackLimits`

    /// Return the offset of `VMStackLimits::stack_limit`.
    fn vmstack_limits_stack_limit(&self) -> u8 {
        0
    }

    /// Return the offset of `VMStackLimits::last_wasm_entry_fp`.
    fn vmstack_limits_last_wasm_entry_fp(&self) -> u8 {
        self.size()
    }

    /// Return the offset of `VMStackLimits::last_wasm_entry_sp`.
    fn vmstack_limits_last_wasm_entry_sp(&self) -> u8 {
        self.vmstack_limits_last_wasm_entry_fp() + self.size()
    }

    /// Return the offset of `VMStackLimits::last_wasm_entry_trap_handler`.
    fn vmstack_limits_last_wasm_entry_trap_handler(&self) -> u8 {
        self.vmstack_limits_last_wasm_entry_sp() + self.size()
    }

    // Offsets within `VMHostArray`

    /// Return the offset of `VMHostArray::length`.
    fn vmhostarray_length(&self) -> u8 {
        0
    }

    /// Return the offset of `VMHostArray::capacity`.
    fn vmhostarray_capacity(&self) -> u8 {
        4
    }

    /// Return the offset of `VMHostArray::data`.
    fn vmhostarray_data(&self) -> u8 {
        8
    }

    /// Return the size of `VMHostArray`.
    fn size_of_vmhostarray(&self) -> u8 {
        8 + self.size()
    }

    // Offsets within `VMCommonStackInformation`

    /// Return the offset of `VMCommonStackInformation::limits`.
    fn vmcommon_stack_information_limits(&self) -> u8 {
        0 * self.size()
    }

    /// Return the offset of `VMCommonStackInformation::state`.
    fn vmcommon_stack_information_state(&self) -> u8 {
        4 * self.size()
    }

    /// Return the offset of `VMCommonStackInformation::handlers`.
    fn vmcommon_stack_information_handlers(&self) -> u8 {
        u8::try_from(align(
            self.vmcommon_stack_information_state() as u32 + 4,
            u32::from(self.size()),
        ))
        .unwrap()
    }

    /// Return the offset of `VMCommonStackInformation::first_switch_handler_index`.
    fn vmcommon_stack_information_first_switch_handler_index(&self) -> u8 {
        self.vmcommon_stack_information_handlers() + self.size_of_vmhostarray()
    }

    /// Return the size of `VMCommonStackInformation`.
    fn size_of_vmcommon_stack_information(&self) -> u8 {
        u8::try_from(align(
            self.vmcommon_stack_information_first_switch_handler_index() as u32 + 4,
            u32::from(self.size()),
        ))
        .unwrap()
    }

    // Offsets within `VMContObj`

    /// Return the offset of `VMContObj::contref`
    fn vmcontobj_contref(&self) -> u8 {
        0
    }

    /// Return the offset of `VMContObj::revision`
    fn vmcontobj_revision(&self) -> u8 {
        self.size()
    }

    /// Return the size of `VMContObj`.
    fn size_of_vmcontobj(&self) -> u8 {
        u8::try_from(align(
            u32::from(self.vmcontobj_revision())
                + u32::try_from(core::mem::size_of::<usize>()).unwrap(),
            u32::from(self.size()),
        ))
        .unwrap()
    }

    // Offsets within `VMContRef`

    /// Return the offset of `VMContRef::common_stack_information`.
    fn vmcontref_common_stack_information(&self) -> u8 {
        0 * self.size()
    }

    /// Return the offset of `VMContRef::parent_chain`.
    fn vmcontref_parent_chain(&self) -> u8 {
        u8::try_from(align(
            (self.vmcontref_common_stack_information() + self.size_of_vmcommon_stack_information())
                as u32,
            u32::from(self.size()),
        ))
        .unwrap()
    }

    /// Return the offset of `VMContRef::last_ancestor`.
    fn vmcontref_last_ancestor(&self) -> u8 {
        self.vmcontref_parent_chain() + 2 * self.size()
    }

    /// Return the offset of `VMContRef::revision`.
    fn vmcontref_revision(&self) -> u8 {
        self.vmcontref_last_ancestor() + self.size()
    }

    /// Return the offset of `VMContRef::stack`.
    fn vmcontref_stack(&self) -> u8 {
        self.vmcontref_revision() + self.size()
    }

    /// Return the offset of `VMContRef::args`.
    fn vmcontref_args(&self) -> u8 {
        self.vmcontref_stack() + 3 * self.size()
    }

    /// Return the offset of `VMContRef::values`.
    fn vmcontref_values(&self) -> u8 {
        self.vmcontref_args() + self.size_of_vmhostarray()
    }
}

/// A trait to abstract over various types that contain a `P: PtrSize`.
pub trait GetPtrSize {
    /// The type that implements `PtrSize`.
    type Ptr: PtrSize;

    /// Get a `&P` where `P: PtrSize`.
    fn get_ptr_size(&self) -> &Self::Ptr;
}

impl<P> GetPtrSize for P
where
    P: PtrSize,
{
    type Ptr = Self;

    #[inline]
    fn get_ptr_size(&self) -> &Self::Ptr {
        self
    }
}

/// Type representing the size of a pointer for the current compilation host
#[derive(Clone, Copy)]
pub struct HostPtr;

impl PtrSize for HostPtr {
    #[inline]
    fn size(&self) -> u8 {
        core::mem::size_of::<usize>() as u8
    }
}

impl PtrSize for u8 {
    #[inline]
    fn size(&self) -> u8 {
        *self
    }
}

impl<P> PtrSize for &'_ P
where
    P: PtrSize + ?Sized,
{
    #[inline]
    fn size(&self) -> u8 {
        (**self).size()
    }
}

/// Used to construct a `VMOffsets`
#[derive(Debug, Clone, Copy)]
pub struct VMOffsetsFields<P> {
    /// The size in bytes of a pointer on the target.
    pub ptr: P,
    /// The number of imported functions in the module.
    pub num_imported_functions: u32,
    /// The number of imported tables in the module.
    pub num_imported_tables: u32,
    /// The number of imported memories in the module.
    pub num_imported_memories: u32,
    /// The number of imported globals in the module.
    pub num_imported_globals: u32,
    /// The number of imported tags in the module.
    pub num_imported_tags: u32,
    /// The number of defined tables in the module.
    pub num_defined_tables: u32,
    /// The number of defined memories in the module.
    pub num_defined_memories: u32,
    /// The number of memories owned by the module instance.
    pub num_owned_memories: u32,
    /// The number of defined globals in the module.
    pub num_defined_globals: u32,
    /// The number of defined tags in the module.
    pub num_defined_tags: u32,
    /// The number of escaped functions in the module, the size of the function
    /// references array.
    pub num_escaped_funcs: u32,
    /// The number of runtime data segments in the module.
    pub num_runtime_data: u32,
    /// Whether or not the module has a start function.
    pub has_startup_func: bool,
}

impl<P: PtrSize> VMOffsets<P> {
    /// Return a new `VMOffsets` instance, for a given pointer size.
    pub fn new(ptr: P, module: &Module) -> Self {
        let num_owned_memories = module
            .memories
            .iter()
            .skip(module.num_imported_memories)
            .filter(|p| !p.1.shared)
            .count()
            .try_into()
            .unwrap();
        VMOffsets::from(VMOffsetsFields {
            ptr,
            num_imported_functions: cast_to_u32(module.num_imported_funcs),
            num_imported_tables: cast_to_u32(module.num_imported_tables),
            num_imported_memories: cast_to_u32(module.num_imported_memories),
            num_imported_globals: cast_to_u32(module.num_imported_globals),
            num_imported_tags: cast_to_u32(module.num_imported_tags),
            num_defined_tables: cast_to_u32(module.num_defined_tables()),
            num_defined_memories: cast_to_u32(module.num_defined_memories()),
            num_owned_memories,
            num_defined_globals: cast_to_u32(module.globals.len() - module.num_imported_globals),
            num_defined_tags: cast_to_u32(module.tags.len() - module.num_imported_tags),
            num_escaped_funcs: cast_to_u32(module.num_escaped_funcs),
            num_runtime_data: cast_to_u32(module.runtime_data.len()),
            has_startup_func: !module.startup.is_none(),
        })
    }

    /// Returns the size, in bytes, of the target
    #[inline]
    pub fn pointer_size(&self) -> u8 {
        self.ptr.size()
    }

    /// Returns an iterator which provides a human readable description and a
    /// byte size. The iterator returned will iterate over the bytes allocated
    /// to the entire `VMOffsets` structure to explain where each byte size is
    /// coming from.
    pub fn region_sizes(&self) -> impl Iterator<Item = (&str, u32)> {
        macro_rules! calculate_sizes {
            ($($name:ident: $desc:tt,)*) => {{
                let VMOffsets {
                    // These fields are metadata not talking about specific
                    // offsets of specific fields.
                    ptr: _,
                    num_imported_functions: _,
                    num_imported_tables: _,
                    num_imported_memories: _,
                    num_imported_globals: _,
                    num_imported_tags: _,
                    num_defined_tables: _,
                    num_defined_globals: _,
                    num_defined_memories: _,
                    num_defined_tags: _,
                    num_owned_memories: _,
                    num_escaped_funcs: _,
                    num_runtime_data: _,
                    has_startup_func: _,

                    // used as the initial size below
                    size,

                    // exhaustively match the rest of the fields with input from
                    // the macro
                    $($name,)*
                } = *self;

                // calculate the size of each field by relying on the inputs to
                // the macro being in reverse order and determining the size of
                // the field as the offset from the field to the last field.
                let mut last = size;
                $(
                    assert!($name <= last);
                    let tmp = $name;
                    let $name = last - $name;
                    last = tmp;
                )*
                assert_ne!(last, 0);
                IntoIterator::into_iter([
                    $(($desc, $name),)*
                    ("static vmctx data", last),
                ])
            }};
        }

        calculate_sizes! {
            runtime_data_lengths: "runtime data lengths",
            runtime_data_bases: "runtime data base pointers",
            startup_func_ref: "startup funcref",
            func_refs: "module functions",
            tags: "defined tags",
            globals: "defined globals",
            tables: "defined tables",
            imported_tags: "imported tags",
            imported_globals: "imported globals",
            imported_tables: "imported tables",
            imported_functions: "imported functions",
            owned_memories: "owned memories",
            memories: "defined memories",
            imported_memories: "imported memories",
        }
    }
}

impl<P: PtrSize> GetPtrSize for VMOffsets<P> {
    type Ptr = P;

    #[inline]
    fn get_ptr_size(&self) -> &Self::Ptr {
        &self.ptr
    }
}

impl<P: PtrSize> From<VMOffsetsFields<P>> for VMOffsets<P> {
    fn from(fields: VMOffsetsFields<P>) -> VMOffsets<P> {
        let mut ret = Self {
            ptr: fields.ptr,
            num_imported_functions: fields.num_imported_functions,
            num_imported_tables: fields.num_imported_tables,
            num_imported_memories: fields.num_imported_memories,
            num_imported_globals: fields.num_imported_globals,
            num_imported_tags: fields.num_imported_tags,
            num_defined_tables: fields.num_defined_tables,
            num_defined_memories: fields.num_defined_memories,
            num_owned_memories: fields.num_owned_memories,
            num_defined_globals: fields.num_defined_globals,
            num_defined_tags: fields.num_defined_tags,
            num_escaped_funcs: fields.num_escaped_funcs,
            num_runtime_data: fields.num_runtime_data,
            has_startup_func: fields.has_startup_func,
            imported_memories: 0,
            memories: 0,
            owned_memories: 0,
            imported_functions: 0,
            imported_tables: 0,
            imported_globals: 0,
            imported_tags: 0,
            tables: 0,
            globals: 0,
            tags: 0,
            func_refs: 0,
            startup_func_ref: 0,
            runtime_data_bases: 0,
            runtime_data_lengths: 0,
            size: 0,
        };
        ret.compute_field_offsets();
        ret
    }
}

/// Offsets for `*const VMFunctionBody`.
impl<P: PtrSize> VMOffsets<P> {
    /// The size of the `current_elements` field.
    pub fn size_of_vmfunction_body_ptr(&self) -> u8 {
        1 * self.pointer_size()
    }
}

/// Offsets for `VMTableDefinition`.
impl<P: PtrSize> VMOffsets<P> {
    /// The size of the `current_elements` field.
    #[inline]
    pub fn size_of_vmtable_definition_current_elements(&self) -> u8 {
        self.pointer_size()
    }
}

/// Offsets for `VMSharedTypeIndex`.
impl<P: PtrSize> VMOffsets<P> {
    /// Return the size of `VMSharedTypeIndex`.
    #[inline]
    pub fn size_of_vmshared_type_index(&self) -> u8 {
        self.ptr.size_of_vmshared_type_index()
    }
}

impl_vmctx_array_index! {
    MemoryIndex,
    DefinedMemoryIndex,
    OwnedMemoryIndex,
    FuncIndex,
    TableIndex,
    DefinedTableIndex,
    GlobalIndex,
    DefinedGlobalIndex,
    TagIndex,
    DefinedTagIndex,
    FuncRefIndex,
    RuntimeDataIndex,
    ModuleInternedTypeIndex,
}

/// Generate the accessors for the offsets of `VMContext`'s
/// dynamically-positioned fields.
macro_rules! define_vmoffsets_dynamic_offsets {
    (@one VMContext $snake:ident { $($dyn:tt)* }) => {
        /// Offsets of the dynamically-positioned fields of `VMContext`.
        impl<P: PtrSize> VMOffsets<P> {
            define_vmctx_dynamic_offsets!(@accessors (self) [ $($dyn)* ]);
            define_vmctx_dynamic_offsets!(@compute_fn (self, next) $snake [ $($dyn)* ]);

            /// Return the size of the `VMContext` allocation.
            #[inline]
            pub fn size_of_vmctx(&self) -> u32 {
                self.size
            }
        }
    };
    (@one $other:ident $snake:ident { $($dyn:tt)* }) => {};

    ( $(
        {
            $Name:ident $snake:ident
            static { $($stat:tt)* }
            dynamic { $($dyn:tt)* }
        }
    )* ) => {
        $( define_vmoffsets_dynamic_offsets!(@one $Name $snake { $($dyn)* }); )*
    };
}
for_each_vmctx_type!(define_vmoffsets_dynamic_offsets);

/// Offsets for `VMGcHeader`.
impl<P: PtrSize> VMOffsets<P> {
    /// Return the offset for the `VMGcHeader::kind` field.
    #[inline]
    pub fn vm_gc_header_kind(&self) -> u32 {
        0
    }

    /// Return the offset for the `VMGcHeader`'s reserved bits.
    #[inline]
    pub fn vm_gc_header_reserved_bits(&self) -> u32 {
        // NB: The reserved bits are the unused `VMGcKind` bits.
        self.vm_gc_header_kind()
    }

    /// Return the offset for the `VMGcHeader::ty` field.
    #[inline]
    pub fn vm_gc_header_ty(&self) -> u32 {
        self.vm_gc_header_kind() + 4
    }
}

/// Offsets for `VMDrcHeader`.
///
/// Should only be used when the DRC collector is enabled.
impl<P: PtrSize> VMOffsets<P> {
    /// Return the offset for `VMDrcHeader::ref_count`.
    #[inline]
    pub fn vm_drc_header_ref_count(&self) -> u32 {
        8
    }

    /// Return the offset for `VMDrcHeader::next_over_approximated_stack_root`.
    #[inline]
    pub fn vm_drc_header_next_over_approximated_stack_root(&self) -> u32 {
        self.vm_drc_header_ref_count() + 8
    }
}

/// Magic value for core Wasm VM contexts.
///
/// This is stored at the start of all `VMContext` structures.
pub const VMCONTEXT_MAGIC: u32 = u32::from_le_bytes(*b"core");

/// Equivalent of `VMCONTEXT_MAGIC` except for array-call host functions.
///
/// This is stored at the start of all `VMArrayCallHostFuncContext` structures
/// and double-checked on `VMArrayCallHostFuncContext::from_opaque`.
pub const VM_ARRAY_CALL_HOST_FUNC_MAGIC: u32 = u32::from_le_bytes(*b"ACHF");

#[cfg(test)]
mod tests {
    use crate::vmoffsets::align;

    #[test]
    fn alignment() {
        fn is_aligned(x: u32) -> bool {
            x % 16 == 0
        }
        assert!(is_aligned(align(0, 16)));
        assert!(is_aligned(align(32, 16)));
        assert!(is_aligned(align(33, 16)));
        assert!(is_aligned(align(31, 16)));
    }
}

/// The bit pattern of `VMLazyThread::forced()`.
pub const VM_LAZY_THREAD_FORCED: u64 = 1;

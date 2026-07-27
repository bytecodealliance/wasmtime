//! Centralized definitions of the various `VM*` types whose layout is shared
//! between the runtime (which uses the actual structures) and the compiler
//! (which uses the types' offsets and has per-type alias regions).
//!
//! To keep these in sync, the shape of each type is defined exactly once here,
//! via the higher-order [`for_each_vm_type!`] macro, and each consumer
//! generates its view of the type from that single source of truth.

/// Invoke the given macro `$mac` once, passing it the definitions of each of the
/// `VM*` types whose layout is shared between the runtime, compilation offsets,
/// and Cranelift alias regions.
///
/// This is a higher-order macro: callers define a `macro_rules!` macro that
/// matches the grammar defined below and pass its name as an argument to this
/// macro's invocation, e.g. `for_each_vm_type!(define_vm_types)`.
///
/// # Grammar
///
/// Each type is emitted as a struct definition preceded by:
///
/// * Doc-comment attributes (`#[doc = "..."]`).
///
/// * An optional `#[derive(...)]` attribute.
///
/// * A `#[repr(...)]` attribute.
///
/// * A `#[snake_name = <ident>]` attribute giving the type's name in
///   `snake_case`, used to generate accessor method names.
///
/// Each field may be preceded by doc-comment attributes and, optionally, the
/// marker attributes `#[readonly]` and/or `#[can_move]` (in that order) which
/// describe how Cranelift may treat loads and stores of that field.
#[macro_export]
macro_rules! for_each_vm_type {
    ($mac:ident) => {
        $mac! {
            /// The fields compiled code needs to access to utilize a WebAssembly linear
            /// memory defined within the instance, namely the start address and the
            /// size in bytes.
            #[derive(Debug)]
            #[repr(C)]
            #[snake_name = vm_memory_definition]
            pub struct VMMemoryDefinition {
                /// The start address.
                pub base: VmPtr<u8>,

                /// The current logical size of this linear memory in bytes.
                ///
                /// This is atomic because shared memories must be able to grow their length
                /// atomically. For relaxed access, see
                /// [`VMMemoryDefinition::current_length()`].
                pub current_length: AtomicUsize,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly table
            /// defined within the instance.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_table_definition]
            pub struct VMTableDefinition {
                /// Pointer to the table data.
                pub base: VmPtr<u8>,

                /// The current number of elements in the table.
                pub current_elements: usize,
            }

            /// The storage for a WebAssembly global defined within the instance.
            ///
            /// TODO: Pack the globals more densely, rather than using the same size
            /// for every type.
            #[derive(Debug)]
            #[repr(C, align(16))]
            #[snake_name = vm_global_definition]
            pub struct VMGlobalDefinition {
                /// The raw storage for a global's value.
                storage: [u8; 16],
            }

            /// A WebAssembly tag defined within the instance.
            #[derive(Debug)]
            #[repr(C)]
            #[snake_name = vm_tag_definition]
            pub struct VMTagDefinition {
                /// Function signature's type id.
                pub type_index: VMSharedTypeIndex,
            }

            /// The VM caller-checked "funcref" record, for caller-side signature checking.
            ///
            /// It consists of function pointer(s), a type id to be checked by the
            /// caller, and the vmctx closure associated with this function.
            #[derive(Debug, Clone)]
            #[repr(C)]
            #[snake_name = vm_func_ref]
            pub struct VMFuncRef {
                /// Function pointer for this funcref if being called via the "array"
                /// calling convention that `Func::new` et al use.
                pub array_call: VmPtr<VMArrayCallFunction>,

                /// Function pointer for this funcref if being called via the calling
                /// convention we use when compiling Wasm.
                ///
                /// Most functions come with a function pointer that we can use when they
                /// are called from Wasm. The notable exception is when we `Func::wrap` a
                /// host function, and we don't have a Wasm compiler on hand to compile a
                /// Wasm-to-native trampoline for the function. In this case, we leave
                /// `wasm_call` empty until the function is passed as an import to Wasm (or
                /// otherwise exposed to Wasm via tables/globals). At this point, we look up
                /// a Wasm-to-native trampoline for the function in the Wasm's compiled
                /// module and use that fill in `VMFunctionImport::wasm_call`. **However**
                /// there is no guarantee that the Wasm module has a trampoline for this
                /// function's signature. The Wasm module only has trampolines for its
                /// types, and if this function isn't of one of those types, then the Wasm
                /// module will not have a trampoline for it. This is actually okay, because
                /// it means that the Wasm cannot actually call this function. But it does
                /// mean that this field needs to be an `Option` even though it is non-null
                /// the vast vast vast majority of the time.
                pub wasm_call: Option<VmPtr<VMWasmCallFunction>>,

                /// Function signature's type id.
                pub type_index: VMSharedTypeIndex,

                /// The VM state associated with this function.
                ///
                /// The actual definition of what this pointer points to depends on the
                /// function being referenced: for core Wasm functions, this is a `*mut
                /// VMContext`, for host functions it is a `*mut VMHostFuncContext`, and for
                /// component functions it is a `*mut VMComponentContext`.
                pub vmctx: VmPtr<VMOpaqueContext>,
            }

            /// An imported function.
            ///
            /// Basically the same as `VMFuncRef`, except that `wasm_call` is not optional.
            #[derive(Debug, Clone)]
            #[repr(C)]
            #[snake_name = vm_function_import]
            pub struct VMFunctionImport {
                /// Same as `VMFuncRef::array_call`.
                pub array_call: VmPtr<VMArrayCallFunction>,

                /// Same as `VMFuncRef::wasm_call`, except always non-null. Must be filled
                /// in by the time Wasm is importing this function!
                pub wasm_call: VmPtr<VMWasmCallFunction>,

                /// Function signature's _actual_ type id.
                ///
                /// This is the type that the function was defined with, not the type that
                /// it was imported as. These two can be different in the face of subtyping
                /// and we need the former for to correctly implement dynamic downcasts.
                pub type_index: VMSharedTypeIndex,

                /// Same as `VMFuncRef::vmctx`.
                pub vmctx: VmPtr<VMOpaqueContext>,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly table
            /// imported from another instance.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_table_import]
            pub struct VMTableImport {
                /// A pointer to the imported table description.
                pub from: VmPtr<VMTableDefinition>,

                /// A pointer to the `VMContext` that owns the table description.
                pub vmctx: VmPtr<VMContext>,

                /// The table index, within `vmctx`, this definition resides at.
                pub index: DefinedTableIndex,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly linear
            /// memory imported from another instance.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_memory_import]
            pub struct VMMemoryImport {
                /// A pointer to the imported memory description.
                pub from: VmPtr<VMMemoryDefinition>,

                /// A pointer to the `VMContext` that owns the memory description.
                pub vmctx: VmPtr<VMContext>,

                /// The index of the memory in the containing `vmctx`.
                pub index: DefinedMemoryIndex,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly global
            /// variable imported from another instance.
            ///
            /// Note that unlike with functions, tables, and memories, `VMGlobalImport`
            /// doesn't include a `vmctx` pointer. Globals are never resized, and don't
            /// require a `vmctx` pointer to access.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_global_import]
            pub struct VMGlobalImport {
                /// A pointer to the imported global variable description.
                pub from: VmPtr<VMGlobalDefinition>,

                /// A pointer to the context that owns the global.
                ///
                /// Exactly what's stored here is dictated by `kind` below. This is `None`
                /// for `VMGlobalKind::Host`, it's a `VMContext` for
                /// `VMGlobalKind::Instance`, and it's `VMComponentContext` for
                /// `VMGlobalKind::ComponentFlags`.
                pub vmctx: Option<VmPtr<VMOpaqueContext>>,

                /// The kind of global, and extra location information in addition to
                /// `vmctx` above.
                pub kind: VMGlobalKind,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly
            /// tag imported from another instance.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_tag_import]
            pub struct VMTagImport {
                /// A pointer to the imported tag description.
                pub from: VmPtr<VMTagDefinition>,

                /// The instance that owns this tag.
                pub vmctx: VmPtr<VMContext>,

                /// The index of the tag in the containing `vmctx`.
                pub index: DefinedTagIndex,
            }
        }
    };
}

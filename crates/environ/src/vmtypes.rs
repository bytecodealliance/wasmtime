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
/// * An optional `#[cfg(...)]` attribute, gating the type on the Cargo features
///   whose code actually uses it.
///
///   Only the runtime's `struct` definition (and its layout test) is gated by
///   it. A type's offsets and alias regions are always generated because the
///   `wasmtime-environ` and `wasmtime-internal-cranelift` crates do not have
///   Cargo features for all the various Wasm features.
///
/// * An optional `#[derive(...)]` attribute.
///
/// * A `#[repr(...)]` attribute.
///
/// * A `#[snake_name = <ident>]` attribute giving the type's name in
///   `snake_case`, used to generate accessor method names.
///
/// Each field may be preceded by doc-comment attributes and, optionally, these
/// marker attributes, in this order:
///
/// * `#[aggregate]`: this field is a composite (a nested struct or array)
///   rather than a single scalar. Compiled Wasm code accesses such a field's
///   interior piecewise, so there is no one Cranelift type for the field as a
///   whole and no alias-region accessor is generated for it. The field's offset
///   is still generated, since that is what interior accesses are computed
///   relative to.
///
/// * `#[indexed]`: this field is a fixed-size array of a scalar type, whose
///   elements compiled Wasm code accesses individually by a compile-time
///   constant index. Like `#[aggregate]`, the field has no single Cranelift
///   type; unlike `#[aggregate]`, its elements all do, so an alias-region
///   accessor taking that index is generated for it and each element gets its
///   own alias region.
///
/// * `#[readonly]` and/or `#[can_move]`: describe how Cranelift may treat loads
///   and stores of that field.
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
                ///
                /// Once a `VMFuncRef` is exposed to compiled code this field
                /// never changes again, so accesses of it are `readonly`. It is
                /// not `can_move`, however: the load may be the one that traps
                /// on a null funcref, and moving it would move that trap.
                #[readonly]
                pub wasm_call: Option<VmPtr<VMWasmCallFunction>>,

                /// Function signature's type id.
                ///
                /// See the note about `readonly` and not `can_move` on
                /// `wasm_call`.
                #[readonly]
                pub type_index: VMSharedTypeIndex,

                /// The VM state associated with this function.
                ///
                /// The actual definition of what this pointer points to depends on the
                /// function being referenced: for core Wasm functions, this is a `*mut
                /// VMContext`, for host functions it is a `*mut VMHostFuncContext`, and for
                /// component functions it is a `*mut VMComponentContext`.
                ///
                /// See the note about `readonly` and not `can_move` on
                /// `wasm_call`.
                #[readonly]
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
                #[readonly]
                #[can_move]
                pub array_call: VmPtr<VMArrayCallFunction>,

                /// Same as `VMFuncRef::wasm_call`, except always non-null. Must be filled
                /// in by the time Wasm is importing this function!
                #[readonly]
                #[can_move]
                pub wasm_call: VmPtr<VMWasmCallFunction>,

                /// Function signature's _actual_ type id.
                ///
                /// This is the type that the function was defined with, not the type that
                /// it was imported as. These two can be different in the face of subtyping
                /// and we need the former for to correctly implement dynamic downcasts.
                #[readonly]
                #[can_move]
                pub type_index: VMSharedTypeIndex,

                /// Same as `VMFuncRef::vmctx`.
                #[readonly]
                #[can_move]
                pub vmctx: VmPtr<VMOpaqueContext>,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly table
            /// imported from another instance.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_table_import]
            pub struct VMTableImport {
                /// A pointer to the imported table description.
                #[readonly]
                #[can_move]
                pub from: VmPtr<VMTableDefinition>,

                /// A pointer to the `VMContext` that owns the table description.
                #[readonly]
                #[can_move]
                pub vmctx: VmPtr<VMContext>,

                /// The table index, within `vmctx`, this definition resides at.
                #[readonly]
                #[can_move]
                pub index: DefinedTableIndex,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly linear
            /// memory imported from another instance.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_memory_import]
            pub struct VMMemoryImport {
                /// A pointer to the imported memory description.
                #[readonly]
                #[can_move]
                pub from: VmPtr<VMMemoryDefinition>,

                /// A pointer to the `VMContext` that owns the memory description.
                #[readonly]
                #[can_move]
                pub vmctx: VmPtr<VMContext>,

                /// The index of the memory in the containing `vmctx`.
                #[readonly]
                #[can_move]
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
                #[readonly]
                #[can_move]
                pub from: VmPtr<VMGlobalDefinition>,

                /// A pointer to the context that owns the global.
                ///
                /// Exactly what's stored here is dictated by `kind` below. This is `None`
                /// for `VMGlobalKind::Host`, it's a `VMContext` for
                /// `VMGlobalKind::Instance`, and it's `VMComponentContext` for
                /// `VMGlobalKind::ComponentFlags`.
                #[readonly]
                #[can_move]
                pub vmctx: Option<VmPtr<VMOpaqueContext>>,

                /// The kind of global, and extra location information in addition to
                /// `vmctx` above.
                #[readonly]
                #[can_move]
                pub kind: VMGlobalKind,
            }

            /// The fields compiled code needs to access to utilize a WebAssembly
            /// tag imported from another instance.
            #[derive(Debug, Copy, Clone)]
            #[repr(C)]
            #[snake_name = vm_tag_import]
            pub struct VMTagImport {
                /// A pointer to the imported tag description.
                #[readonly]
                #[can_move]
                pub from: VmPtr<VMTagDefinition>,

                /// The instance that owns this tag.
                #[readonly]
                #[can_move]
                pub vmctx: VmPtr<VMContext>,

                /// The index of the tag in the containing `vmctx`.
                #[readonly]
                #[can_move]
                pub index: DefinedTagIndex,
            }

            /// Structure that holds all mutable context that is shared across all instances
            /// in a store, for example data related to fuel or epochs.
            ///
            /// `VMStoreContext`s are one-to-one with `wasmtime::Store`s, the same way that
            /// `VMContext`s are one-to-one with `wasmtime::Instance`s. And the same way
            /// that multiple `wasmtime::Instance`s may be associated with the same
            /// `wasmtime::Store`, multiple `VMContext`s hold a pointer to the same
            /// `VMStoreContext` when they are associated with the same `wasmtime::Store`.
            #[derive(Debug)]
            // NB: `align(8)` is forced rather than inferred because the i386
            // System V ABI aligns 64-bit integers to 4 bytes, and `VMOffsets`
            // can't tell that target apart from the ones that align them to 8,
            // since it only knows the target's pointer width.
            #[repr(C, align(8))]
            #[snake_name = vm_store_context]
            pub struct VMStoreContext {
                // NB: 64-bit integer fields are located first with pointer-sized fields
                // trailing afterwards. That makes the offsets in this structure easier to
                // calculate on 32-bit platforms as we don't have to worry about the
                // alignment of 64-bit integers.
                //
                /// Indicator of how much fuel has been consumed and is remaining to
                /// WebAssembly.
                ///
                /// This field is typically negative and increments towards positive. Upon
                /// turning positive a wasm trap will be generated. This field is only
                /// modified if wasm is configured to consume fuel.
                pub fuel_consumed: UnsafeCell<i64>,

                /// Deadline epoch for interruption: if epoch-based interruption
                /// is enabled and the global (per engine) epoch counter is
                /// observed to reach or exceed this value, the guest code will
                /// yield if running asynchronously.
                pub epoch_deadline: UnsafeCell<u64>,

                /// The "store version".
                ///
                /// This is used to test whether stack-frame handles referring to
                /// suspended stack frames remain valid.
                ///
                /// The invariant that this upward-counting number must satisfy
                /// is: the number must be incremented whenever execution starts
                /// or resumes in the `Store` or when any stack is
                /// dropped/freed. That way, if we take a reference to some
                /// suspended stack frame and track the "version" at the time we
                /// took that reference, if the version still matches, we can be
                /// sure that nothing could have unwound the referenced Wasm
                /// frame.
                ///
                /// This version number is incremented in exactly one place: the
                /// Wasm-to-host trampolines, after return from host code. Note
                /// that this captures both the normal "return into Wasm" case
                /// (where Wasm frames can subsequently return normally and thus
                /// invalidate frames), and the "trap/exception unwinds Wasm
                /// frames" case, which is done internally via the `raise` libcall
                /// invoked after the main hostcall returns an error, and after we
                /// increment this version number.
                ///
                /// Note that this also handles the fiber/future-drop case because
                /// because we *always* return into the trampoline to clean up;
                /// that trampoline immediately raises an error and uses the
                /// longjmp-like unwind within Cranelift frames to skip over all
                /// the guest Wasm frames, but not before it increments the
                /// store's execution version number.
                ///
                /// This field is in use only if guest debugging is enabled.
                pub execution_version: u64,

                /// Current stack limit of the wasm module.
                ///
                /// For more information see `crates/cranelift/src/lib.rs`.
                pub stack_limit: UnsafeCell<usize>,

                /// The `VMMemoryDefinition` for this store's GC heap.
                #[aggregate]
                pub gc_heap: UnsafeCell<VMMemoryDefinition>,

                /// The value of the frame pointer register in the trampoline used
                /// to call from Wasm to the host.
                ///
                /// Maintained by our Wasm-to-host trampoline, and cleared just
                /// before calling into Wasm in `catch_traps`.
                ///
                /// This member is `0` when Wasm is actively running and has not called out
                /// to the host.
                ///
                /// Used to find the start of a contiguous sequence of Wasm frames
                /// when walking the stack. Note that we record the FP of the
                /// *trampoline*'s frame, not the last Wasm frame, because we need
                /// to know the SP (bottom of frame) of the last Wasm frame as
                /// well in case we need to resume to an exception handler in that
                /// frame. The FP of the last Wasm frame can be recovered by
                /// loading the saved FP value at this FP address.
                pub last_wasm_exit_trampoline_fp: UnsafeCell<usize>,

                /// The last Wasm program counter before we called from Wasm to the host.
                ///
                /// Maintained by our Wasm-to-host trampoline, and cleared just before
                /// calling into Wasm in `catch_traps`.
                ///
                /// This member is `0` when Wasm is actively running and has not called out
                /// to the host.
                ///
                /// Used when walking a contiguous sequence of Wasm frames.
                pub last_wasm_exit_pc: UnsafeCell<usize>,

                /// The last host stack pointer before we called into Wasm from the host.
                ///
                /// Maintained by our host-to-Wasm trampoline. This member is `0` when Wasm
                /// is not running, and it's set to nonzero once a host-to-wasm trampoline
                /// is executed.
                ///
                /// When a host function is wrapped into a `wasmtime::Func`, and is then
                /// called from the host, then this member is not changed meaning that the
                /// previous activation in pointed to by `last_wasm_exit_trampoline_fp` is
                /// still the last wasm set of frames on the stack.
                ///
                /// This field is saved/restored during fiber suspension/resumption
                /// resumption as part of `CallThreadState::swap`.
                ///
                /// This field is used to find the end of a contiguous sequence of Wasm
                /// frames when walking the stack. Additionally it's used when a trap is
                /// raised as part of the set of parameters used to resume in the entry
                /// trampoline's "catch" block.
                pub last_wasm_entry_sp: UnsafeCell<usize>,

                /// Same as `last_wasm_entry_sp`, but for the `fp` of the trampoline.
                pub last_wasm_entry_fp: UnsafeCell<usize>,

                /// The last trap handler from a host-to-wasm entry trampoline on the stack.
                ///
                /// This field is configured when the host calls into wasm by the trampoline
                /// itself. It stores the `pc` of an exception handler suitable to handle
                /// all traps (or uncaught exceptions).
                pub last_wasm_entry_trap_handler: UnsafeCell<usize>,

                /// Stack information used by stack switching instructions. See documentation
                /// on `VMStackChain` for details.
                #[aggregate]
                pub stack_chain: UnsafeCell<VMStackChain>,

                /// A pointer to the embedder's `T` inside a `Store<T>`, for use with the
                /// `store-data-address` unsafe intrinsic.
                ///
                /// This pointer is fixed for the lifetime of the store, so loads
                /// of it are `readonly` and `can_move`.
                #[readonly]
                #[can_move]
                pub store_data: VmPtr<()>,

                /// The range, in addresses, of the guard page that is currently in use.
                ///
                /// This field is used when signal handlers are run to determine whether a
                /// faulting address lies within the guard page of an async stack for
                /// example. If this happens then the signal handler aborts with a stack
                /// overflow message similar to what would happen had the stack overflow
                /// happened on the main thread. This field is, by default a null..null
                /// range indicating that no async guard is in use (aka no fiber). In such a
                /// situation while this field is read it'll never classify a fault as an
                /// guard page fault.
                #[aggregate]
                pub async_guard_range: Range<*mut u8>,

                /// The `context.{get,set}` values for the current thread in the component
                /// model. This is only used for `component-model-async` and slot[1] is only
                /// used for `component-model-threading`. Despite the conditional use nature
                /// this is unconditionally present as it avoids the need to make logic in
                /// `VMOffsets` conditional.
                ///
                /// This is saved/restored when threads are swapped in the component model.
                ///
                /// NB: `UnsafeCell` because JIT code writes to the slots.
                #[indexed]
                pub component_context: UnsafeCell<[u32; NUM_COMPONENT_CONTEXT_SLOTS]>,

                /// JIT-visible current thread for the component model's sync-to-sync
                /// adapter fast path.
                ///
                /// Like `component_context`, this is unconditionally present to keep
                /// `VMOffsets` logic unconditional even though it is only used when
                /// `component-model-async` is enabled.
                ///
                /// NB: `UnsafeCell` because JIT code writes to this field.
                pub current_thread: UnsafeCell<VMLazyThread>,
            }

            /// JIT-visible representation of the store's current thread for the component
            /// model, encoded as a single pointer-sized integer so that generated JIT code
            /// can load, store, and compare it with a handful of instructions.
            ///
            /// This is the inline fast-path counterpart to the host-side `CurrentThread`: a
            /// fused sync-to-sync adapter records a lazy deferred thread here (a pointer to
            /// a `VMDeferredThread` on its own stack frame) instead of eagerly allocating a
            /// `GuestTask`/`GuestThread` in the host. Host code promotes the deferred
            /// thread into a real one only when it actually needs it; see
            /// `StoreOpaque::force_current_thread`.
            ///
            /// This type is a bitpacked equivalent of the following logical `enum`:
            ///
            /// ```ignore
            /// enum VMLazyThread {
            ///     /// No thread.
            ///     None,
            ///
            ///     /// The lazy thread was promoted and materialized; get it from
            ///     /// `ConcurrentState::current_thread`.
            ///     Forced,
            ///
            ///     /// The lazy thread has not been materialized, here is a pointer to the
            ///     /// stack-allocated data needed to do force that promotion.
            ///     Deferred(*mut VMDeferredThread),
            /// }
            /// ```
            ///
            /// Bitpacking details:
            ///
            /// * `None`: `0`
            ///
            /// * `Forced`: A non-zero value with its low-bit set.
            ///
            /// * `Deferred`: A non-zero value with its low-bit clear.
            #[derive(Debug, Copy, Clone, PartialEq, Eq)]
            #[repr(transparent)]
            #[snake_name = vm_lazy_thread]
            pub struct VMLazyThread {
                /// The bitpacked thread representation described above.
                ///
                /// Private: use the `VMLazyThread::{none,forced,deferred}` constructors
                /// and the `is_*`/`as_deferred` accessors instead of touching this
                /// directly.
                thread: Option<VmPtr<VMDeferredThread>>,
            }

            /// A deferred component-model thread.
            ///
            /// This is an on-stack record pushed by a fused sync-to-sync adapter's fast
            /// path to defer the work that the `enter_sync_call` libcall would otherwise do
            /// eagerly.
            ///
            /// The adapter allocates one of these in its own stack frame, links the
            /// previous current-thread value to it via `parent`, and finally points
            /// `VMStoreContext::current_thread` at it. When host code actually needs the
            /// real thread, it walks the `parent` chain to materialize thread state (see
            /// `StoreOpaque::force_current_thread`).
            #[derive(Debug)]
            #[repr(C)]
            #[snake_name = vm_deferred_thread]
            pub struct VMDeferredThread {
                /// The previous value of `VMStoreContext::current_thread`.
                pub parent: VMLazyThread,
                /// The caller component instance (a deferred `enter_sync_call` argument).
                pub caller_instance: u32,
                /// Whether the callee is async-lifted (a deferred `enter_sync_call` arg).
                pub callee_async: u32,
                /// The callee component instance (a deferred `enter_sync_call` argument).
                pub callee_instance: u32,
                /// The caller thread's `context.{get,set}` slots, saved on entry and
                /// restored on the fast-path exit (or recovered while forcing).
                #[indexed]
                pub saved_context: [u32; NUM_COMPONENT_CONTEXT_SLOTS],
            }

            /// The deferred-reference-counting collector's JIT-accessible heap
            /// data.
            ///
            /// This is a separate allocation, reached through the
            /// `VMContext::gc_heap_data` pointer. Its fields are not GC heap
            /// locations, so they get this type's own alias regions rather than
            /// the GC heap's.
            ///
            /// `wasmtime::runtime::vm::gc::enabled::drc` owns one of these,
            /// wrapped in a cell because compiled Wasm writes to it, and
            /// accesses it only through that wrapper's methods.
            #[cfg(feature = "gc-drc")]
            #[derive(Default)]
            #[repr(C)]
            #[snake_name = vm_drc_heap_data]
            pub struct VMDrcHeapData {
                /// The head of the over-approximated-stack-roots list.
                pub over_approximated_stack_roots: Option<VMGcRef>,

                /// The current size of the over-approximated-stack-roots list.
                pub current_over_approximated_stack_roots_len: u32,

                /// The size of the over-approximated-stack-roots list
                /// immediately after the last GC.
                pub over_approximated_stack_roots_len_after_last_gc: u32,
            }

            /// The copying collector's JIT-accessible bump-allocation state.
            ///
            /// Like [`VMDrcHeapData`], this is a separate allocation reached
            /// through `VMContext::gc_heap_data`, and is owned, wrapped in a
            /// cell, by `wasmtime::runtime::vm::gc::enabled::copying`.
            #[cfg(feature = "gc-copying")]
            #[derive(Default)]
            #[repr(C)]
            #[snake_name = vm_copying_heap_data]
            pub struct VMCopyingHeapData {
                /// Current bump pointer (an index into the GC heap).
                pub bump_ptr: u32,

                /// End of the active semi-space.
                pub active_space_end: u32,
            }

            /// The null collector's JIT-accessible bump-allocation state.
            ///
            /// Unlike the two above, this is not a separate allocation: it is
            /// the first field of `NullHeap` (in
            /// `wasmtime::runtime::vm::gc::enabled::null`), again wrapped in a
            /// cell, and compiled Wasm reaches it through a pointer to that
            /// field.
            #[cfg(feature = "gc-null")]
            #[repr(C)]
            #[snake_name = vm_null_heap_data]
            pub struct VMNullHeapData {
                /// The bump-allocation finger, an index into the GC heap.
                pub next: NonZeroU32,
            }
        }
    };
}

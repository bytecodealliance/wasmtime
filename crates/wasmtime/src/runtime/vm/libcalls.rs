//! Runtime library calls.
//!
//! Note that Wasm compilers may sometimes perform these inline rather than
//! calling them, particularly when CPUs have special instructions which compute
//! them directly.
//!
//! These functions are called by compiled Wasm code, and therefore must take
//! certain care about some things:
//!
//! * They must only contain basic, raw i32/i64/f32/f64/pointer parameters that
//!   are safe to pass across the system ABI.
//!
//! * If any nested function propagates an `Err(trap)` out to the library
//!   function frame, we need to raise it. This involves some nasty and quite
//!   unsafe code under the covers! Notably, after raising the trap, drops
//!   **will not** be run for local variables! This can lead to things like
//!   leaking `InstanceHandle`s which leads to never deallocating JIT code,
//!   instances, and modules if we are not careful!
//!
//! * The libcall must be entered via a Wasm-to-libcall trampoline that saves
//!   the last Wasm FP and PC for stack walking purposes. (For more details, see
//!   `crates/wasmtime/src/runtime/vm/backtrace.rs`.)
//!
//! To make it easier to correctly handle all these things, **all** libcalls
//! must be defined via the `libcall!` helper macro! See its doc comments below
//! for an example, or just look at the rest of the file.
//!
//! ## Dealing with `externref`s
//!
//! When receiving a raw `*mut u8` that is actually a `VMExternRef` reference,
//! convert it into a proper `VMExternRef` with `VMExternRef::clone_from_raw` as
//! soon as apossible. Any GC before raw pointer is converted into a reference
//! can potentially collect the referenced object, which could lead to use after
//! free.
//!
//! Avoid this by eagerly converting into a proper `VMExternRef`! (Unfortunately
//! there is no macro to help us automatically get this correct, so stay
//! vigilant!)
//!
//! ```ignore
//! pub unsafe extern "C" my_libcall_takes_ref(raw_extern_ref: *mut u8) {
//!     // Before `clone_from_raw`, `raw_extern_ref` is potentially unrooted,
//!     // and doing GC here could lead to use after free!
//!
//!     let my_extern_ref = if raw_extern_ref.is_null() {
//!         None
//!     } else {
//!         Some(VMExternRef::clone_from_raw(raw_extern_ref))
//!     };
//!
//!     // Now that we did `clone_from_raw`, it is safe to do a GC (or do
//!     // anything else that might transitively GC, like call back into
//!     // Wasm!)
//! }
//! ```

use crate::bail_bug;
use crate::prelude::*;
use crate::runtime::store::{Asyncness, InstanceId, StoreOpaque};
#[cfg(feature = "gc")]
use crate::runtime::vm::VMGcRef;
use crate::runtime::vm::{self, HostResultHasUnwindSentinel, VMStore, f32x4, f64x2, i8x16};
use core::convert::Infallible;
use core::ptr::NonNull;
#[cfg(feature = "threads")]
use core::time::Duration;
use wasmtime_core::math::WasmFloat;
use wasmtime_environ::{
    CompiledTrap, DefinedMemoryIndex, DefinedTableIndex, FuncIndex, PassiveElemIndex, TableIndex,
    Trap,
};
#[cfg(feature = "wmemcheck")]
use wasmtime_wmemcheck::AccessError::{
    DoubleMalloc, InvalidFree, InvalidRead, InvalidWrite, OutOfBounds,
};

/// Raw functions which are actually called from compiled code.
///
/// Invocation of a builtin currently looks like:
///
/// * A wasm function calls a cranelift-compiled trampoline that's generated
///   once-per-builtin.
/// * The cranelift-compiled trampoline performs any necessary actions to exit
///   wasm, such as dealing with fp/pc/etc.
/// * The cranelift-compiled trampoline loads a function pointer from an array
///   stored in `VMContext` That function pointer is defined in this module.
/// * This module runs, handling things like `catch_unwind` and `Result` and
///   such.
/// * This module delegates to the outer module (this file) which has the actual
///   implementation.
///
/// For more information on converting from host-defined values to Cranelift ABI
/// values see the `catch_unwind_and_record_trap` function.
pub mod raw {
    use crate::runtime::vm::{Instance, VMContext, f32x4, f64x2, i8x16};
    use core::ptr::NonNull;

    macro_rules! libcall {
        (
            $(
                $( #[cfg($attr:meta)] )?
                $name:ident( vmctx: vmctx $(, $pname:ident: $param:ident )* ) $(-> $result:ident)?;
            )*
        ) => {
            $(
                // This is the direct entrypoint from the compiled module which
                // still has the raw signature.
                //
                // This will delegate to the outer module to the actual
                // implementation and automatically perform `catch_unwind` along
                // with conversion of the return value in the face of traps.
                #[allow(improper_ctypes_definitions, reason = "__m128i known not FFI-safe")]
                #[allow(unused_variables, reason = "macro-generated")]
                #[allow(unreachable_code, reason = "some types uninhabited on some platforms")]
                pub unsafe extern "C" fn $name(
                    vmctx: NonNull<VMContext>,
                    $( $pname : libcall!(@ty $param), )*
                ) $(-> libcall!(@ty $result))? {
                    $(#[cfg($attr)])?
                    unsafe {
                        Instance::enter_host_from_wasm(vmctx, |store, instance| {
                            super::$name(store, instance, $($pname),*)
                        })
                    }
                    $(
                        #[cfg(not($attr))]
                        {
                            let _ = vmctx;
                            unreachable!();
                        }
                    )?
                }

                // This works around a `rustc` bug where compiling with LTO
                // will sometimes strip out some of these symbols resulting
                // in a linking failure.
                #[allow(improper_ctypes_definitions, reason = "__m128i known not FFI-safe")]
                const _: () = {
                    #[used]
                    static I_AM_USED: unsafe extern "C" fn(
                        NonNull<VMContext>,
                        $( $pname : libcall!(@ty $param), )*
                    ) $( -> libcall!(@ty $result))? = $name;
                };
            )*
        };

        (@ty u32) => (u32);
        (@ty u64) => (u64);
        (@ty f32) => (f32);
        (@ty f64) => (f64);
        (@ty u8) => (u8);
        (@ty i8x16) => (i8x16);
        (@ty f32x4) => (f32x4);
        (@ty f64x2) => (f64x2);
        (@ty bool) => (bool);
        (@ty pointer) => (*mut u8);
        (@ty size) => (usize);
    }

    wasmtime_environ::foreach_builtin_function!(libcall);
}

/// Uses the `$store` provided to invoke the async closure `$f` and block on the
/// result.
///
/// This will internally multiplex on `$store.with_blocking(...)` vs simply
/// asserting the closure is ready depending on whether a store's
/// `can_block` flag is set or not.
///
/// FIXME: ideally this would be a function, not a macro. If this is a function
/// though it would require placing a bound on the async closure $f where the
/// returned future is itself `Send`. That's not possible in Rust right now,
/// unfortunately.
///
/// As a workaround this takes advantage of the fact that we can assume that the
/// compiler can infer that the future returned by `$f` is indeed `Send` so long
/// as we don't try to name the type or place it behind a generic. In the future
/// when we can bound the return future of async functions with `Send` this
/// macro should be replaced with an equivalent function.
macro_rules! block_on {
    ($store:expr, $f:expr) => {{
        let store: &mut StoreOpaque = $store;
        let closure = assert_async_fn_closure($f);

        if store.can_block() {
            // If the store can block then that means it's on a fiber. We can
            // forward to `block_on` and everything should be fine and dandy.
            #[cfg(feature = "async")]
            {
                store.with_blocking(|store, cx| cx.block_on(closure(store, Asyncness::Yes)))
            }
            #[cfg(not(feature = "async"))]
            {
                unreachable!()
            }
        } else {
            // If the store cannot block it's not on a fiber. That means that we get
            // at most one poll of `closure(store)` here. In the typical case
            // what this means is that nothing async is configured in the store
            // and one poll should be all we need. There are niche cases where
            // one poll is not sufficient though, for example:
            //
            // * Store is created.
            // * Wasm is called.
            // * Wasm calls host.
            // * Host configures an async resource limiter, returns back to
            //   wasm.
            // * Wasm grows memory.
            // * Limiter wants to block asynchronously.
            //
            // Technically there's nothing wrong with this, but it means that
            // we're in wasm and one poll is not enough here. Given the niche
            // nature of this scenario and how it's not really expected to work
            // this translates failures in `closure` to a trap. This trap is
            // only expected to show up in niche-ish scenarios, not for actual
            // blocking work, as that would otherwise be too surprising.
            vm::one_poll(closure(store, Asyncness::No)).ok_or_else(|| {
                crate::format_err!(
                    "

A synchronously called wasm function invoked an async-defined libcall which
failed to complete synchronously and is thus raising a trap. It's expected
that this indicates that the store was configured to do async things after the
original synchronous entrypoint to wasm was called. That's generally not
supported in Wasmtime and async entrypoint should be used instead. If you're
seeing this message in error please file an issue on Wasmtime.

"
                )
            })
        }
    }};
}

fn assert_async_fn_closure<F, R>(f: F) -> F
where
    F: AsyncFnOnce(&mut StoreOpaque, Asyncness) -> R,
{
    f
}

fn memory_grow(
    store: &mut dyn VMStore,
    instance: InstanceId,
    delta: u64,
    memory_index: u32,
) -> Result<Option<AllocationSize>> {
    let memory_index = DefinedMemoryIndex::from_u32(memory_index);
    let (mut limiter, store) = store.resource_limiter_and_store_opaque();
    let limiter = limiter.as_mut();
    block_on!(store, async |store, _| {
        let instance = store.instance_mut(instance);
        let module = instance.env_module();
        let page_size_log2 = module.memories[module.memory_index(memory_index)].page_size_log2;

        let result = instance
            .memory_grow(limiter, memory_index, delta)
            .await?
            .map(|size_in_bytes| AllocationSize(size_in_bytes >> page_size_log2));

        Ok(result)
    })?
}

/// A helper structure to represent the return value of a memory or table growth
/// call.
///
/// This represents a byte or element-based count of the size of an item on the
/// host. For example a memory is how many bytes large the memory is, or a table
/// is how many elements large it is. It's assumed that the value here is never
/// -1 or -2 as that would mean the entire host address space is allocated which
/// is not possible.
struct AllocationSize(usize);

/// Special implementation for growth-related libcalls.
///
/// Here the optional return value means:
///
/// * `Some(val)` - the growth succeeded and the previous size of the item was
///   `val`.
/// * `None` - the growth failed.
///
/// The failure case returns -1 (or `usize::MAX` as an unsigned integer) and the
/// successful case returns the `val` itself. Note that -2 (`usize::MAX - 1`
/// when unsigned) is unwind as a sentinel to indicate an unwind as no valid
/// allocation can be that large.
unsafe impl HostResultHasUnwindSentinel for Option<AllocationSize> {
    type Abi = *mut u8;
    const SENTINEL: *mut u8 = (usize::MAX - 1) as *mut u8;

    fn into_abi(self) -> *mut u8 {
        match self {
            Some(size) => {
                debug_assert!(size.0 < (usize::MAX - 1));
                size.0 as *mut u8
            }
            None => usize::MAX as *mut u8,
        }
    }
}

/// Implementation of `table.grow`.
unsafe fn table_grow(
    store: &mut dyn VMStore,
    instance: InstanceId,
    defined_table_index: u32,
    delta: u64,
) -> Result<Option<AllocationSize>> {
    let defined_table_index = DefinedTableIndex::from_u32(defined_table_index);
    let (mut limiter, store) = store.resource_limiter_and_store_opaque();
    let limiter = limiter.as_mut();
    block_on!(store, async |store, _| unsafe {
        let result = store
            .instance_mut(instance)
            .defined_table_grow(defined_table_index, limiter, delta)
            .await?
            .map(AllocationSize);
        Ok(result)
    })?
}

fn passive_elem_segment_len(
    store: &mut dyn VMStore,
    instance: InstanceId,
    elem_index: u32,
) -> usize {
    let elem_index = PassiveElemIndex::from_u32(elem_index);
    store
        .instance_mut(instance)
        .passive_element_segment(elem_index)
        .len()
}

fn passive_elem_segment_base(
    store: &mut dyn VMStore,
    instance: InstanceId,
    elem_index: u32,
) -> *mut u8 {
    let elem_index = PassiveElemIndex::from_u32(elem_index);
    store
        .instance_mut(instance)
        .passive_element_segment(elem_index)
        .as_mut_ptr()
        .cast()
}

// Implementation of `elem.drop`.
fn passive_elem_segment_drop(
    store: &mut dyn VMStore,
    instance: InstanceId,
    elem_index: u32,
) -> Result<()> {
    let elem_index = PassiveElemIndex::from_u32(elem_index);
    let (gc_store, instance) = store.optional_gc_store_and_instance_mut(instance);
    instance.passive_elem_drop(gc_store, elem_index)?;
    Ok(())
}

// Implementation of `memory.copy`.
unsafe fn memory_copy(
    _store: &mut dyn VMStore,
    _instance: InstanceId,
    dst: *mut u8,
    src: *mut u8,
    len: usize,
) {
    let src = src.cast_const();
    // FIXME(#4203): this is known to not be sound in the presence of shared
    // memories.
    unsafe { src.copy_to(dst, len) }
}

unsafe fn memory_fill(
    _store: &mut dyn VMStore,
    _instance: InstanceId,
    dst: *mut u8,
    val: u32,
    len: usize,
) {
    // FIXME(#4203): this is known to not be sound in the presence of shared
    // memories.
    unsafe {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the libcall intentionally takes the raw 32-bit value, \
                      and semantically that's intentionally truncated"
        )]
        dst.write_bytes(val as u8, len);
    }
}

// Implementation of `ref.func`.
fn ref_func(store: &mut dyn VMStore, instance: InstanceId, func_index: u32) -> NonNull<u8> {
    let (instance, registry) = store.instance_and_module_registry_mut(instance);
    instance
        .get_func_ref(registry, FuncIndex::from_u32(func_index))
        .expect("ref_func: funcref should always be available for given func index")
        .cast()
}

// Returns a table entry after lazily initializing it.
fn table_get_lazy_init_func_ref(
    store: &mut dyn VMStore,
    instance: InstanceId,
    table_index: u32,
    index: u64,
) -> *mut u8 {
    let table_index = TableIndex::from_u32(table_index);
    let (instance, registry) = store.instance_and_module_registry_mut(instance);
    let table = instance.get_table_with_lazy_init(registry, table_index, core::iter::once(index));
    let elem = table
        .get_func(index)
        .expect("table access already bounds-checked");

    match elem {
        Some(ptr) => ptr.as_ptr().cast(),
        None => core::ptr::null_mut(),
    }
}

/// Drop a GC reference.
#[cfg(feature = "gc-drc")]
fn drop_gc_ref(store: &mut dyn VMStore, _instance: InstanceId, gc_ref: u32) {
    log::trace!("libcalls::drop_gc_ref({gc_ref:#x})");
    let gc_ref = VMGcRef::from_raw_u32(gc_ref).expect("non-null VMGcRef");
    store
        .store_opaque_mut()
        .unwrap_gc_store_mut()
        .drop_gc_ref(gc_ref);
}

/// Force a DRC GC cycle.
#[cfg(feature = "gc-drc")]
fn force_gc(store: &mut dyn VMStore, _instance: InstanceId) -> Result<()> {
    let store = store.store_opaque_mut();
    block_on!(store, async |store, asyncness| {
        store.gc(None, None, None, asyncness).await?;
        Ok::<(), Error>(())
    })??;
    Ok(())
}

/// Grow the GC heap.
#[cfg(feature = "gc-null")]
fn grow_gc_heap(store: &mut dyn VMStore, _instance: InstanceId, bytes_needed: u64) -> Result<()> {
    let orig_len = u64::try_from(
        store
            .require_gc_store()?
            .gc_heap
            .vmmemory()
            .current_length(),
    )
    .unwrap();

    let (mut limiter, store) = store.resource_limiter_and_store_opaque();
    block_on!(store, async |store, asyncness| {
        // We error below if there's still not enough space; swallow
        // any growth failures here.
        let _ = store
            .grow_gc_heap(limiter.as_mut(), bytes_needed, asyncness)
            .await;
    })?;

    // JIT code relies on the memory having grown by `bytes_needed` bytes if
    // this libcall returns successfully, so trap if we didn't grow that much.
    let new_len = u64::try_from(
        store
            .require_gc_store()?
            .gc_heap
            .vmmemory()
            .current_length(),
    )
    .unwrap();
    if orig_len
        .checked_add(bytes_needed)
        .is_none_or(|expected_len| new_len < expected_len)
    {
        return Err(crate::Trap::AllocationTooLarge.into());
    }

    Ok(())
}

/// Allocate a raw, unininitialized GC object for Wasm code.
///
/// The Wasm code is responsible for initializing the object.
#[cfg(any(feature = "gc-drc", feature = "gc-copying"))]
fn gc_alloc_raw(
    store: &mut dyn VMStore,
    _instance: InstanceId,
    kind_and_reserved: u32,
    shared_type_index: u32,
    size: u32,
    align: u32,
) -> Result<core::num::NonZeroU32> {
    use crate::vm::VMGcHeader;
    use core::alloc::Layout;
    use wasmtime_environ::{VMGcKind, VMSharedTypeIndex};

    let kind = VMGcKind::from_high_bits_of_u32(kind_and_reserved);
    log::trace!("gc_alloc_raw(kind={kind:?}, size={size}, align={align})");

    let shared_type_index = VMSharedTypeIndex::from_u32(shared_type_index);
    let mut header = VMGcHeader::from_kind_and_index(kind, shared_type_index);
    header.set_reserved_u26(kind_and_reserved & VMGcKind::UNUSED_MASK);

    let size = usize::try_from(size).unwrap();
    let align = usize::try_from(align).unwrap();
    assert!(align.is_power_of_two());
    let layout = Layout::from_size_align(size, align).map_err(|e| {
        let err = Error::from(crate::Trap::AllocationTooLarge);
        err.context(e)
    })?;

    // Fast path: when the GC store already exists, try to allocate directly to
    // skip the async/fiber machinery.
    let opaque = store.store_opaque_mut();
    if let Some(gc_store) = opaque.try_gc_store_mut() {
        if let Ok(gc_ref) = gc_store.alloc_raw(header, layout)? {
            let raw = gc_store.expose_gc_ref_to_wasm(gc_ref)?;
            return Ok(raw);
        }
    }

    let (mut limiter, store) = store.resource_limiter_and_store_opaque();
    block_on!(store, async |store, asyncness| {
        let gc_ref = store
            .retry_after_gc_async(limiter.as_mut(), (), asyncness, |store, ()| {
                store
                    .unwrap_gc_store_mut()
                    .alloc_raw(header, layout)?
                    .map_err(|bytes_needed| crate::GcHeapOutOfMemory::new((), bytes_needed).into())
            })
            .await?;

        store.unwrap_gc_store_mut().expose_gc_ref_to_wasm(gc_ref)
    })?
}

// Intern a `funcref` into the GC heap, returning its `FuncRefTableId`.
//
// This libcall may not GC.
#[cfg(feature = "gc")]
unsafe fn intern_func_ref_for_gc_heap(
    store: &mut dyn VMStore,
    _instance: InstanceId,
    func_ref: *mut u8,
) -> Result<u32> {
    use crate::runtime::vm::vmcontext::VMFuncRef;
    use crate::{store::AutoAssertNoGc, vm::SendSyncPtr};
    use core::ptr::NonNull;

    let mut store = AutoAssertNoGc::new(store.store_opaque_mut());

    let func_ref = func_ref.cast::<VMFuncRef>();
    let func_ref = NonNull::new(func_ref).map(SendSyncPtr::new);

    let func_ref_id = unsafe {
        store
            .require_gc_store_mut()?
            .func_ref_table
            .intern(func_ref)
    };
    Ok(func_ref_id.into_raw())
}

// Get the raw `VMFuncRef` pointer associated with a `FuncRefTableId` from an
// earlier `intern_func_ref_for_gc_heap` call.
//
// This libcall may not GC.
#[cfg(feature = "gc")]
fn get_interned_func_ref(
    store: &mut dyn VMStore,
    instance: InstanceId,
    func_ref_id: u32,
    module_interned_type_index: u32,
) -> Result<*mut u8> {
    use super::FuncRefTableId;
    use crate::store::AutoAssertNoGc;
    use wasmtime_environ::{ModuleInternedTypeIndex, packed_option::ReservedValue};

    let store = AutoAssertNoGc::new(store.store_opaque_mut());

    let func_ref_id = FuncRefTableId::from_raw(func_ref_id);
    let module_interned_type_index = ModuleInternedTypeIndex::from_bits(module_interned_type_index);

    let func_ref = if module_interned_type_index.is_reserved_value() {
        store
            .unwrap_gc_store()
            .func_ref_table
            .get_untyped(func_ref_id)?
    } else {
        let types = store.engine().signatures();
        let engine_ty = store
            .instance(instance)
            .engine_type_index(module_interned_type_index);
        store
            .unwrap_gc_store()
            .func_ref_table
            .get_typed(types, func_ref_id, engine_ty)?
    };

    Ok(func_ref.map_or(core::ptr::null_mut(), |f| f.as_ptr().cast()))
}

#[cfg(feature = "gc")]
fn is_subtype(
    store: &mut dyn VMStore,
    _instance: InstanceId,
    actual_engine_type: u32,
    expected_engine_type: u32,
) -> u32 {
    use wasmtime_environ::VMSharedTypeIndex;

    let actual = VMSharedTypeIndex::from_u32(actual_engine_type);
    let expected = VMSharedTypeIndex::from_u32(expected_engine_type);

    let is_subtype: bool = store.engine().signatures().is_subtype(actual, expected);

    log::trace!("is_subtype(actual={actual:?}, expected={expected:?}) -> {is_subtype}",);
    is_subtype as u32
}

// Implementation of `memory.atomic.notify` for locally defined memories.
#[cfg(feature = "threads")]
fn memory_atomic_notify(
    store: &mut dyn VMStore,
    instance: InstanceId,
    memory_index: u32,
    addr_index: u64,
    count: u32,
) -> Result<u32, Trap> {
    let memory = DefinedMemoryIndex::from_u32(memory_index);
    store
        .instance_mut(instance)
        .get_defined_memory_mut(memory)
        .atomic_notify(addr_index, count)
}

// Implementation of `memory.atomic.wait32` for locally defined memories.
#[cfg(feature = "threads")]
fn memory_atomic_wait32(
    store: &mut dyn VMStore,
    instance: InstanceId,
    memory_index: u32,
    addr_index: u64,
    expected: u32,
    timeout: u64,
) -> Result<u32, Trap> {
    let timeout = (timeout as i64 >= 0).then(|| Duration::from_nanos(timeout));
    let memory = DefinedMemoryIndex::from_u32(memory_index);
    Ok(store
        .instance_mut(instance)
        .get_defined_memory_mut(memory)
        .atomic_wait32(addr_index, expected, timeout)? as u32)
}

// Implementation of `memory.atomic.wait64` for locally defined memories.
#[cfg(feature = "threads")]
fn memory_atomic_wait64(
    store: &mut dyn VMStore,
    instance: InstanceId,
    memory_index: u32,
    addr_index: u64,
    expected: u64,
    timeout: u64,
) -> Result<u32, Trap> {
    let timeout = (timeout as i64 >= 0).then(|| Duration::from_nanos(timeout));
    let memory = DefinedMemoryIndex::from_u32(memory_index);
    Ok(store
        .instance_mut(instance)
        .get_defined_memory_mut(memory)
        .atomic_wait64(addr_index, expected, timeout)? as u32)
}

// Hook for when an instance runs out of fuel.
fn out_of_gas(store: &mut dyn VMStore, _instance: InstanceId) -> Result<()> {
    block_on!(store, async |store, _| {
        if !store.refuel() {
            return Err(Trap::OutOfFuel.into());
        }
        #[cfg(feature = "async")]
        if store.fuel_yield_interval.is_some() {
            store.yield_now().await;
        }
        Ok(())
    })?
}

// Hook for when an instance observes that the epoch has changed.
#[cfg(target_has_atomic = "64")]
fn new_epoch(store: &mut dyn VMStore, _instance: InstanceId) -> Result<NextEpoch> {
    use crate::UpdateDeadline;

    #[cfg(feature = "debug")]
    {
        store.block_on_debug_handler(crate::DebugEvent::EpochYield)?;
    }

    let update_deadline = store.new_epoch_updated_deadline()?;
    block_on!(store, async move |store, asyncness| {
        #[cfg(not(feature = "async"))]
        let _ = asyncness;

        let delta = match update_deadline {
            UpdateDeadline::Interrupt => return Err(Trap::Interrupt.into()),
            UpdateDeadline::Continue(delta) => delta,

            // Note that custom errors are used here to avoid tripping up on the
            // `block_on!` message that otherwise assumes
            // async-configuration-after-the-fact.
            #[cfg(feature = "async")]
            UpdateDeadline::Yield(delta) => {
                if asyncness != Asyncness::Yes {
                    bail!(
                        "cannot use `UpdateDeadline::Yield` without using \
                         an async wasm entrypoint",
                    );
                }
                store.yield_now().await;
                delta
            }
            #[cfg(feature = "async")]
            UpdateDeadline::YieldCustom(delta, future) => {
                if asyncness != Asyncness::Yes {
                    bail!(
                        "cannot use `UpdateDeadline::YieldCustom` without using \
                         an async wasm entrypoint",
                    );
                }
                future.await;
                delta
            }
        };

        // Set a new deadline and return the new epoch deadline so
        // the Wasm code doesn't have to reload it.
        store.set_epoch_deadline(delta);
        Ok(NextEpoch(store.get_epoch_deadline()))
    })?
}

struct NextEpoch(u64);

unsafe impl HostResultHasUnwindSentinel for NextEpoch {
    type Abi = u64;
    const SENTINEL: u64 = u64::MAX;
    fn into_abi(self) -> u64 {
        self.0
    }
}

// Hook for validating malloc using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
fn check_malloc(store: &mut dyn VMStore, instance: InstanceId, addr: u32, len: u32) -> Result<()> {
    let instance = store.instance_mut(instance);
    if let Some(wmemcheck_state) = instance.wmemcheck_state_mut() {
        let result = wmemcheck_state.malloc(addr as usize, len as usize);
        wmemcheck_state.memcheck_on();
        match result {
            Ok(()) => {}
            Err(DoubleMalloc { addr, len }) => {
                bail!("Double malloc at addr {:#x} of size {}", addr, len)
            }
            Err(OutOfBounds { addr, len }) => {
                bail!("Malloc out of bounds at addr {:#x} of size {}", addr, len);
            }
            _ => {
                panic!("unreachable")
            }
        }
    }
    Ok(())
}

// Hook for validating free using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
fn check_free(store: &mut dyn VMStore, instance: InstanceId, addr: u32) -> Result<()> {
    let instance = store.instance_mut(instance);
    if let Some(wmemcheck_state) = instance.wmemcheck_state_mut() {
        let result = wmemcheck_state.free(addr as usize);
        wmemcheck_state.memcheck_on();
        match result {
            Ok(()) => {}
            Err(InvalidFree { addr }) => {
                bail!("Invalid free at addr {:#x}", addr)
            }
            _ => {
                panic!("unreachable")
            }
        }
    }
    Ok(())
}

// Hook for validating load using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
fn check_load(
    store: &mut dyn VMStore,
    instance: InstanceId,
    num_bytes: u32,
    addr: u32,
    offset: u32,
) -> Result<()> {
    let instance = store.instance_mut(instance);
    if let Some(wmemcheck_state) = instance.wmemcheck_state_mut() {
        let result = wmemcheck_state.read(addr as usize + offset as usize, num_bytes as usize);
        match result {
            Ok(()) => {}
            Err(InvalidRead { addr, len }) => {
                bail!("Invalid load at addr {:#x} of size {}", addr, len);
            }
            Err(OutOfBounds { addr, len }) => {
                bail!("Load out of bounds at addr {:#x} of size {}", addr, len);
            }
            _ => {
                panic!("unreachable")
            }
        }
    }
    Ok(())
}

// Hook for validating store using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
fn check_store(
    store: &mut dyn VMStore,
    instance: InstanceId,
    num_bytes: u32,
    addr: u32,
    offset: u32,
) -> Result<()> {
    let instance = store.instance_mut(instance);
    if let Some(wmemcheck_state) = instance.wmemcheck_state_mut() {
        let result = wmemcheck_state.write(addr as usize + offset as usize, num_bytes as usize);
        match result {
            Ok(()) => {}
            Err(InvalidWrite { addr, len }) => {
                bail!("Invalid store at addr {:#x} of size {}", addr, len)
            }
            Err(OutOfBounds { addr, len }) => {
                bail!("Store out of bounds at addr {:#x} of size {}", addr, len)
            }
            _ => {
                panic!("unreachable")
            }
        }
    }
    Ok(())
}

// Hook for turning wmemcheck load/store validation off when entering a malloc function.
#[cfg(feature = "wmemcheck")]
fn malloc_start(store: &mut dyn VMStore, instance: InstanceId) {
    let instance = store.instance_mut(instance);
    if let Some(wmemcheck_state) = instance.wmemcheck_state_mut() {
        wmemcheck_state.memcheck_off();
    }
}

// Hook for turning wmemcheck load/store validation off when entering a free function.
#[cfg(feature = "wmemcheck")]
fn free_start(store: &mut dyn VMStore, instance: InstanceId) {
    let instance = store.instance_mut(instance);
    if let Some(wmemcheck_state) = instance.wmemcheck_state_mut() {
        wmemcheck_state.memcheck_off();
    }
}

// Hook for tracking wasm stack updates using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
fn update_stack_pointer(_store: &mut dyn VMStore, _instance: InstanceId, _value: u32) {
    // TODO: stack-tracing has yet to be finalized. All memory below
    // the address of the top of the stack is marked as valid for
    // loads and stores.
    // if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
    //     instance.wmemcheck_state.update_stack_pointer(value as usize);
    // }
}

// Hook updating wmemcheck_state memory state vector every time memory.grow is called.
#[cfg(feature = "wmemcheck")]
fn update_mem_size(store: &mut dyn VMStore, instance: InstanceId, num_pages: u32) {
    let instance = store.instance_mut(instance);
    if let Some(wmemcheck_state) = instance.wmemcheck_state_mut() {
        const KIB: usize = 1024;
        let num_bytes = num_pages as usize * 64 * KIB;
        wmemcheck_state.update_mem_size(num_bytes);
    }
}

fn floor_f32(_store: &mut dyn VMStore, _instance: InstanceId, val: f32) -> f32 {
    val.wasm_floor()
}

fn floor_f64(_store: &mut dyn VMStore, _instance: InstanceId, val: f64) -> f64 {
    val.wasm_floor()
}

fn ceil_f32(_store: &mut dyn VMStore, _instance: InstanceId, val: f32) -> f32 {
    val.wasm_ceil()
}

fn ceil_f64(_store: &mut dyn VMStore, _instance: InstanceId, val: f64) -> f64 {
    val.wasm_ceil()
}

fn trunc_f32(_store: &mut dyn VMStore, _instance: InstanceId, val: f32) -> f32 {
    val.wasm_trunc()
}

fn trunc_f64(_store: &mut dyn VMStore, _instance: InstanceId, val: f64) -> f64 {
    val.wasm_trunc()
}

fn nearest_f32(_store: &mut dyn VMStore, _instance: InstanceId, val: f32) -> f32 {
    val.wasm_nearest()
}

fn nearest_f64(_store: &mut dyn VMStore, _instance: InstanceId, val: f64) -> f64 {
    val.wasm_nearest()
}

// This intrinsic is only used on x86_64 platforms as an implementation of
// the `i8x16.swizzle` instruction when `pshufb` in SSSE3 is not available.
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn i8x16_swizzle(_store: &mut dyn VMStore, _instance: InstanceId, a: i8x16, b: i8x16) -> i8x16 {
    union U {
        reg: i8x16,
        mem: [u8; 16],
    }

    unsafe {
        let a = U { reg: a }.mem;
        let b = U { reg: b }.mem;

        // Use the `swizzle` semantics of returning 0 on any out-of-bounds
        // index, rather than the x86 pshufb semantics, since Wasmtime uses
        // this to implement `i8x16.swizzle`.
        let select = |arr: &[u8; 16], byte: u8| {
            if byte >= 16 { 0x00 } else { arr[byte as usize] }
        };

        U {
            mem: [
                select(&a, b[0]),
                select(&a, b[1]),
                select(&a, b[2]),
                select(&a, b[3]),
                select(&a, b[4]),
                select(&a, b[5]),
                select(&a, b[6]),
                select(&a, b[7]),
                select(&a, b[8]),
                select(&a, b[9]),
                select(&a, b[10]),
                select(&a, b[11]),
                select(&a, b[12]),
                select(&a, b[13]),
                select(&a, b[14]),
                select(&a, b[15]),
            ],
        }
        .reg
    }
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
fn i8x16_swizzle(_store: &mut dyn VMStore, _instance: InstanceId, _a: i8x16, _b: i8x16) -> i8x16 {
    unreachable!()
}

// This intrinsic is only used on x86_64 platforms as an implementation of
// the `i8x16.shuffle` instruction when `pshufb` in SSSE3 is not available.
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn i8x16_shuffle(
    _store: &mut dyn VMStore,
    _instance: InstanceId,
    a: i8x16,
    b: i8x16,
    c: i8x16,
) -> i8x16 {
    union U {
        reg: i8x16,
        mem: [u8; 16],
    }

    unsafe {
        let ab = [U { reg: a }.mem, U { reg: b }.mem];
        let c = U { reg: c }.mem;

        // Use the `shuffle` semantics of returning 0 on any out-of-bounds
        // index, rather than the x86 pshufb semantics, since Wasmtime uses
        // this to implement `i8x16.shuffle`.
        let select = |arr: &[[u8; 16]; 2], byte: u8| {
            if byte >= 32 {
                0x00
            } else if byte >= 16 {
                arr[1][byte as usize - 16]
            } else {
                arr[0][byte as usize]
            }
        };

        U {
            mem: [
                select(&ab, c[0]),
                select(&ab, c[1]),
                select(&ab, c[2]),
                select(&ab, c[3]),
                select(&ab, c[4]),
                select(&ab, c[5]),
                select(&ab, c[6]),
                select(&ab, c[7]),
                select(&ab, c[8]),
                select(&ab, c[9]),
                select(&ab, c[10]),
                select(&ab, c[11]),
                select(&ab, c[12]),
                select(&ab, c[13]),
                select(&ab, c[14]),
                select(&ab, c[15]),
            ],
        }
        .reg
    }
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
fn i8x16_shuffle(
    _store: &mut dyn VMStore,
    _instance: InstanceId,
    _a: i8x16,
    _b: i8x16,
    _c: i8x16,
) -> i8x16 {
    unreachable!()
}

fn fma_f32x4(
    _store: &mut dyn VMStore,
    _instance: InstanceId,
    x: f32x4,
    y: f32x4,
    z: f32x4,
) -> f32x4 {
    union U {
        reg: f32x4,
        mem: [f32; 4],
    }

    unsafe {
        let x = U { reg: x }.mem;
        let y = U { reg: y }.mem;
        let z = U { reg: z }.mem;

        U {
            mem: [
                x[0].wasm_mul_add(y[0], z[0]),
                x[1].wasm_mul_add(y[1], z[1]),
                x[2].wasm_mul_add(y[2], z[2]),
                x[3].wasm_mul_add(y[3], z[3]),
            ],
        }
        .reg
    }
}

fn fma_f64x2(
    _store: &mut dyn VMStore,
    _instance: InstanceId,
    x: f64x2,
    y: f64x2,
    z: f64x2,
) -> f64x2 {
    union U {
        reg: f64x2,
        mem: [f64; 2],
    }

    unsafe {
        let x = U { reg: x }.mem;
        let y = U { reg: y }.mem;
        let z = U { reg: z }.mem;

        U {
            mem: [x[0].wasm_mul_add(y[0], z[0]), x[1].wasm_mul_add(y[1], z[1])],
        }
        .reg
    }
}

/// This intrinsic is just used to record trap information.
///
/// The `Infallible` "ok" type here means that this never returns success, it
/// only ever returns an error, and this hooks into the machinery to handle
/// `Result` values to record such trap information.
fn trap(_store: &mut dyn VMStore, _instance: InstanceId, code: u8) -> Result<Infallible> {
    match CompiledTrap::from_u8(code).unwrap() {
        CompiledTrap::Normal(trap) => Err(trap.into()),
        CompiledTrap::InternalAssert => bail_bug!("internal assert hit in wasm"),
        CompiledTrap::GcHeapCorrupt => bail_bug!("GC heap corruption detected"),
    }
}

fn raise(store: &mut dyn VMStore, _instance: InstanceId) {
    // SAFETY: this is only called from compiled wasm so we know that wasm has
    // already been entered. It's a dynamic safety precondition that the trap
    // information has already been arranged to be present.
    unsafe { crate::runtime::vm::traphandlers::raise_preexisting_trap(store) }
}

// Builtins for continuations. These are thin wrappers around the
// respective definitions in stack_switching.rs.
#[cfg(feature = "stack-switching")]
fn cont_new(
    store: &mut dyn VMStore,
    instance: InstanceId,
    func: *mut u8,
    param_count: u32,
    result_count: u32,
) -> Result<Option<AllocationSize>> {
    let ans =
        crate::vm::stack_switching::cont_new(store, instance, func, param_count, result_count)?;
    Ok(Some(AllocationSize(ans.cast::<u8>() as usize)))
}

#[cfg(feature = "gc")]
fn get_instance_id(_store: &mut dyn VMStore, instance: InstanceId) -> u32 {
    instance.as_u32()
}

#[cfg(feature = "gc")]
fn throw_ref(store: &mut dyn VMStore, _instance: InstanceId, exnref: u32) -> Result<()> {
    let exnref = VMGcRef::from_raw_u32(exnref).ok_or_else(|| Trap::NullReference)?;
    Err(store.set_pending_exception(&exnref))
}

fn breakpoint(store: &mut dyn VMStore, _instance: InstanceId) -> Result<()> {
    #[cfg(feature = "debug")]
    {
        store.block_on_debug_handler(crate::DebugEvent::Breakpoint)?;
    }
    // Avoid unused-argument warning in no-debugger builds.
    let _ = store;
    Ok(())
}

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

use crate::prelude::*;
use crate::runtime::vm::table::{Table, TableElementType};
use crate::runtime::vm::vmcontext::VMFuncRef;
use crate::runtime::vm::{Instance, TrapReason, VMGcRef};
#[cfg(feature = "threads")]
use core::time::Duration;
use wasmtime_environ::Unsigned;
use wasmtime_environ::{DataIndex, ElemIndex, FuncIndex, MemoryIndex, TableIndex, Trap};
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
pub mod raw {
    // Allow these things because of the macro and how we can't differentiate
    // between doc comments and `cfg`s.
    #![allow(unused_doc_comments, unused_attributes)]

    use crate::runtime::vm::{Instance, TrapReason, VMContext};

    macro_rules! libcall {
        (
            $(
                $( #[cfg($attr:meta)] )?
                $name:ident( vmctx: vmctx $(, $pname:ident: $param:ident )* ) $( -> $result:ident )?;
            )*
        ) => {
            $(
                // This is the direct entrypoint from the compiled module which
                // still has the raw signature.
                //
                // This will delegate to the outer module to the actual
                // implementation and automatically perform `catch_unwind` along
                // with conversion of the return value in the face of traps.
                #[allow(unused_variables, missing_docs)]
                pub unsafe extern "C" fn $name(
                    vmctx: *mut VMContext,
                    $( $pname : libcall!(@ty $param), )*
                ) $( -> libcall!(@ty $result))? {
                    $(#[cfg($attr)])?
                    {
                        let ret = crate::runtime::vm::traphandlers::catch_unwind_and_longjmp(|| {
                            Instance::from_vmctx(vmctx, |instance| {
                                {
                                    super::$name(instance, $($pname),*)
                                }
                            })
                        });
                        LibcallResult::convert(ret)
                    }
                    $(
                        #[cfg(not($attr))]
                        unreachable!();
                    )?
                }

                // This works around a `rustc` bug where compiling with LTO
                // will sometimes strip out some of these symbols resulting
                // in a linking failure.
                #[allow(non_upper_case_globals)]
                const _: () = {
                    #[used]
                    static I_AM_USED: unsafe extern "C" fn(
                        *mut VMContext,
                        $( $pname : libcall!(@ty $param), )*
                    ) $( -> libcall!(@ty $result))? = $name;
                };
            )*
        };

        (@ty i32) => (u32);
        (@ty i64) => (u64);
        (@ty f64) => (f64);
        (@ty u8) => (u8);
        (@ty reference) => (u32);
        (@ty pointer) => (*mut u8);
        (@ty vmctx) => (*mut VMContext);
    }

    wasmtime_environ::foreach_builtin_function!(libcall);

    // Helper trait to convert results of libcalls below into the ABI of what
    // the libcall expects.
    //
    // This basically entirely exists for the `Result` implementation which
    // "unwraps" via a throwing of a trap.
    trait LibcallResult {
        type Abi;
        unsafe fn convert(self) -> Self::Abi;
    }

    impl LibcallResult for () {
        type Abi = ();
        unsafe fn convert(self) {}
    }

    impl<T, E> LibcallResult for Result<T, E>
    where
        E: Into<TrapReason>,
    {
        type Abi = T;
        unsafe fn convert(self) -> T {
            match self {
                Ok(t) => t,
                Err(e) => crate::runtime::vm::traphandlers::raise_trap(e.into()),
            }
        }
    }

    impl LibcallResult for *mut u8 {
        type Abi = *mut u8;
        unsafe fn convert(self) -> *mut u8 {
            self
        }
    }
}

fn memory32_grow(
    instance: &mut Instance,
    delta: u64,
    memory_index: u32,
) -> Result<*mut u8, TrapReason> {
    let memory_index = MemoryIndex::from_u32(memory_index);
    let result =
        match instance
            .memory_grow(memory_index, delta)
            .map_err(|error| TrapReason::User {
                error,
                needs_backtrace: true,
            })? {
            Some(size_in_bytes) => size_in_bytes / instance.memory_page_size(memory_index),
            None => usize::max_value(),
        };
    Ok(result as *mut _)
}

/// Implementation of `table.grow` for `funcref` tables.
unsafe fn table_grow_func_ref(
    instance: &mut Instance,
    table_index: u32,
    delta: u64,
    init_value: *mut u8,
) -> Result<*mut u8> {
    let table_index = TableIndex::from_u32(table_index);

    let element = match instance.table_element_type(table_index) {
        TableElementType::Func => (init_value as *mut VMFuncRef).into(),
        TableElementType::GcRef => unreachable!(),
    };

    let result = match instance.table_grow(table_index, delta, element)? {
        Some(r) => r,
        None => usize::MAX,
    };
    Ok(result as *mut _)
}

/// Implementation of `table.grow` for GC-reference tables.
#[cfg(feature = "gc")]
unsafe fn table_grow_gc_ref(
    instance: &mut Instance,
    table_index: u32,
    delta: u64,
    init_value: u32,
) -> Result<*mut u8> {
    let table_index = TableIndex::from_u32(table_index);

    let element = match instance.table_element_type(table_index) {
        TableElementType::Func => unreachable!(),
        TableElementType::GcRef => VMGcRef::from_raw_u32(init_value)
            .map(|r| (*instance.store()).unwrap_gc_store_mut().clone_gc_ref(&r))
            .into(),
    };

    let result = match instance.table_grow(table_index, delta, element)? {
        Some(r) => r,
        None => usize::MAX,
    };
    Ok(result as *mut _)
}

/// Implementation of `table.fill` for `funcref`s.
unsafe fn table_fill_func_ref(
    instance: &mut Instance,
    table_index: u32,
    dst: u64,
    val: *mut u8,
    len: u64,
) -> Result<(), Trap> {
    let table_index = TableIndex::from_u32(table_index);
    let table = &mut *instance.get_table(table_index);
    match table.element_type() {
        TableElementType::Func => {
            let val = val.cast::<VMFuncRef>();
            table.fill((*instance.store()).unwrap_gc_store_mut(), dst, val.into(), len)
        }
        TableElementType::GcRef => unreachable!(),
    }
}

#[cfg(feature = "gc")]
unsafe fn table_fill_gc_ref(
    instance: &mut Instance,
    table_index: u32,
    dst: u64,
    val: u32,
    len: u64,
) -> Result<(), Trap> {
    let table_index = TableIndex::from_u32(table_index);
    let table = &mut *instance.get_table(table_index);
    match table.element_type() {
        TableElementType::Func => unreachable!(),
        TableElementType::GcRef => {
            let gc_store = (*instance.store()).unwrap_gc_store_mut();
            let gc_ref = VMGcRef::from_raw_u32(val);
            let gc_ref = gc_ref.map(|r| gc_store.clone_gc_ref(&r));
            table.fill(gc_store, dst, gc_ref.into(), len)
        }
    }
}

// Implementation of `table.copy`.
unsafe fn table_copy(
    instance: &mut Instance,
    dst_table_index: u32,
    src_table_index: u32,
    dst: u64,
    src: u64,
    len: u64,
) -> Result<(), Trap> {
    let dst_table_index = TableIndex::from_u32(dst_table_index);
    let src_table_index = TableIndex::from_u32(src_table_index);
    let dst_table = instance.get_table(dst_table_index);
    // Lazy-initialize the whole range in the source table first.
    let src_range = src..(src.checked_add(len).unwrap_or(u64::MAX));
    let src_table = instance.get_table_with_lazy_init(src_table_index, src_range);
    let gc_store = (*instance.store()).unwrap_gc_store_mut();
    Table::copy(gc_store, dst_table, src_table, dst, src, len)
}

// Implementation of `table.init`.
fn table_init(
    instance: &mut Instance,
    table_index: u32,
    elem_index: u32,
    dst: u64,
    src: u64,
    len: u64,
) -> Result<(), Trap> {
    let table_index = TableIndex::from_u32(table_index);
    let elem_index = ElemIndex::from_u32(elem_index);
    instance.table_init(table_index, elem_index, dst, src, len)
}

// Implementation of `elem.drop`.
fn elem_drop(instance: &mut Instance, elem_index: u32) {
    let elem_index = ElemIndex::from_u32(elem_index);
    instance.elem_drop(elem_index)
}

// Implementation of `memory.copy`.
fn memory_copy(
    instance: &mut Instance,
    dst_index: u32,
    dst: u64,
    src_index: u32,
    src: u64,
    len: u64,
) -> Result<(), Trap> {
    let src_index = MemoryIndex::from_u32(src_index);
    let dst_index = MemoryIndex::from_u32(dst_index);
    instance.memory_copy(dst_index, dst, src_index, src, len)
}

// Implementation of `memory.fill` for locally defined memories.
fn memory_fill(
    instance: &mut Instance,
    memory_index: u32,
    dst: u64,
    val: u32,
    len: u64,
) -> Result<(), Trap> {
    let memory_index = MemoryIndex::from_u32(memory_index);
    #[allow(clippy::cast_possible_truncation)]
    instance.memory_fill(memory_index, dst, val as u8, len)
}

// Implementation of `memory.init`.
fn memory_init(
    instance: &mut Instance,
    memory_index: u32,
    data_index: u32,
    dst: u64,
    src: u32,
    len: u32,
) -> Result<(), Trap> {
    let memory_index = MemoryIndex::from_u32(memory_index);
    let data_index = DataIndex::from_u32(data_index);
    instance.memory_init(memory_index, data_index, dst, src, len)
}

// Implementation of `ref.func`.
fn ref_func(instance: &mut Instance, func_index: u32) -> *mut u8 {
    instance
        .get_func_ref(FuncIndex::from_u32(func_index))
        .expect("ref_func: funcref should always be available for given func index")
        .cast()
}

// Implementation of `data.drop`.
fn data_drop(instance: &mut Instance, data_index: u32) {
    let data_index = DataIndex::from_u32(data_index);
    instance.data_drop(data_index)
}

// Returns a table entry after lazily initializing it.
unsafe fn table_get_lazy_init_func_ref(
    instance: &mut Instance,
    table_index: u32,
    index: u64,
) -> *mut u8 {
    let table_index = TableIndex::from_u32(table_index);
    let table = instance.get_table_with_lazy_init(table_index, core::iter::once(index));
    let gc_store = (*instance.store()).unwrap_gc_store_mut();
    let elem = (*table)
        .get(gc_store, index)
        .expect("table access already bounds-checked");

    elem.into_func_ref_asserting_initialized().cast()
}

/// Drop a GC reference.
#[cfg(feature = "gc")]
unsafe fn drop_gc_ref(instance: &mut Instance, gc_ref: u32) {
    log::trace!("libcalls::drop_gc_ref({gc_ref:#x})");
    let gc_ref = VMGcRef::from_raw_u32(gc_ref).expect("non-null VMGcRef");
    (*instance.store()).unwrap_gc_store_mut().drop_gc_ref(gc_ref);
}

/// Do a GC, keeping `gc_ref` rooted and returning the updated `gc_ref`
/// reference.
#[cfg(feature = "gc")]
unsafe fn gc(instance: &mut Instance, gc_ref: u32) -> Result<u32> {
    let gc_ref = VMGcRef::from_raw_u32(gc_ref);
    let gc_ref = gc_ref.map(|r| (*instance.store()).unwrap_gc_store_mut().clone_gc_ref(&r));

    if let Some(gc_ref) = &gc_ref {
        // It is possible that we are GC'ing because the DRC's activation
        // table's bump region is full, and we failed to insert `gc_ref` into
        // the bump region. But it is an invariant for DRC collection that all
        // GC references on the stack are in the DRC's activations table at the
        // time of a GC. So make sure to "expose" this GC reference to Wasm (aka
        // insert it into the DRC's activation table) before we do the actual
        // GC.
        let gc_store = (*instance.store()).unwrap_gc_store_mut();
        let gc_ref = gc_store.clone_gc_ref(gc_ref);
        gc_store.expose_gc_ref_to_wasm(gc_ref);
    }

    match (*instance.store()).gc(gc_ref)? {
        None => Ok(0),
        Some(r) => {
            let raw = r.as_raw_u32();
            (*instance.store()).unwrap_gc_store_mut().expose_gc_ref_to_wasm(r);
            Ok(raw)
        }
    }
}

/// Allocate a raw, unininitialized GC object for Wasm code.
///
/// The Wasm code is responsible for initializing the object.
#[cfg(feature = "gc")]
unsafe fn gc_alloc_raw(
    instance: &mut Instance,
    kind: u32,
    module_interned_type_index: u32,
    size: u32,
    align: u32,
) -> Result<u32> {
    use crate::{vm::VMGcHeader, GcHeapOutOfMemory};
    use core::alloc::Layout;
    use wasmtime_environ::{ModuleInternedTypeIndex, VMGcKind};

    debug_assert_eq!(VMGcKind::UNUSED_MASK & kind, 0);
    let kind = VMGcKind::from_high_bits_of_u32(kind);

    let module = instance
        .runtime_module()
        .expect("should never allocate GC types defined in a dummy module");

    let module_interned_type_index = ModuleInternedTypeIndex::from_u32(module_interned_type_index);
    let shared_type_index = module
        .signatures()
        .shared_type(module_interned_type_index)
        .expect("should have engine type index for module type index");

    let header = VMGcHeader::from_kind_and_index(kind, shared_type_index);

    let size = usize::try_from(size).unwrap();
    let align = usize::try_from(align).unwrap();
    let layout = Layout::from_size_align(size, align).unwrap();

    let gc_ref = match (*instance.store()).unwrap_gc_store_mut().alloc_raw(header, layout)? {
        Some(r) => r,
        None => {
            // If the allocation failed, do a GC to hopefully clean up space.
            (*instance.store()).gc(None)?;

            // And then try again.
            (*instance.store())
                .unwrap_gc_store_mut()
                .alloc_raw(header, layout)?
                .ok_or_else(|| GcHeapOutOfMemory::new(()))
                .err2anyhow()?
        }
    };

    Ok(gc_ref.as_raw_u32())
}

// Implementation of `memory.atomic.notify` for locally defined memories.
#[cfg(feature = "threads")]
fn memory_atomic_notify(
    instance: &mut Instance,
    memory_index: u32,
    addr_index: u64,
    count: u32,
) -> Result<u32, Trap> {
    let memory = MemoryIndex::from_u32(memory_index);
    instance
        .get_runtime_memory(memory)
        .atomic_notify(addr_index, count)
}

// Implementation of `memory.atomic.wait32` for locally defined memories.
#[cfg(feature = "threads")]
fn memory_atomic_wait32(
    instance: &mut Instance,
    memory_index: u32,
    addr_index: u64,
    expected: u32,
    timeout: u64,
) -> Result<u32, Trap> {
    let timeout = (timeout as i64 >= 0).then(|| Duration::from_nanos(timeout));
    let memory = MemoryIndex::from_u32(memory_index);
    Ok(instance
        .get_runtime_memory(memory)
        .atomic_wait32(addr_index, expected, timeout)? as u32)
}

// Implementation of `memory.atomic.wait64` for locally defined memories.
#[cfg(feature = "threads")]
fn memory_atomic_wait64(
    instance: &mut Instance,
    memory_index: u32,
    addr_index: u64,
    expected: u64,
    timeout: u64,
) -> Result<u32, Trap> {
    let timeout = (timeout as i64 >= 0).then(|| Duration::from_nanos(timeout));
    let memory = MemoryIndex::from_u32(memory_index);
    Ok(instance
        .get_runtime_memory(memory)
        .atomic_wait64(addr_index, expected, timeout)? as u32)
}

// Hook for when an instance runs out of fuel.
unsafe fn out_of_gas(instance: &mut Instance) -> Result<()> {
    (*instance.store()).out_of_gas()
}

// Hook for when an instance observes that the epoch has changed.
unsafe fn new_epoch(instance: &mut Instance) -> Result<u64> {
    (*instance.store()).new_epoch()
}

// Hook for validating malloc using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
unsafe fn check_malloc(instance: &mut Instance, addr: u32, len: u32) -> Result<u32> {
    if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
        let result = wmemcheck_state.malloc(addr as usize, len as usize);
        wmemcheck_state.memcheck_on();
        match result {
            Ok(()) => {
                return Ok(0);
            }
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
    Ok(0)
}

// Hook for validating free using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
unsafe fn check_free(instance: &mut Instance, addr: u32) -> Result<u32> {
    if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
        let result = wmemcheck_state.free(addr as usize);
        wmemcheck_state.memcheck_on();
        match result {
            Ok(()) => {
                return Ok(0);
            }
            Err(InvalidFree { addr }) => {
                bail!("Invalid free at addr {:#x}", addr)
            }
            _ => {
                panic!("unreachable")
            }
        }
    }
    Ok(0)
}

// Hook for validating load using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
fn check_load(instance: &mut Instance, num_bytes: u32, addr: u32, offset: u32) -> Result<u32> {
    if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
        let result = wmemcheck_state.read(addr as usize + offset as usize, num_bytes as usize);
        match result {
            Ok(()) => {
                return Ok(0);
            }
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
    Ok(0)
}

// Hook for validating store using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
fn check_store(instance: &mut Instance, num_bytes: u32, addr: u32, offset: u32) -> Result<u32> {
    if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
        let result = wmemcheck_state.write(addr as usize + offset as usize, num_bytes as usize);
        match result {
            Ok(()) => {
                return Ok(0);
            }
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
    Ok(0)
}

// Hook for turning wmemcheck load/store validation off when entering a malloc function.
#[cfg(feature = "wmemcheck")]
fn malloc_start(instance: &mut Instance) {
    if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
        wmemcheck_state.memcheck_off();
    }
}

// Hook for turning wmemcheck load/store validation off when entering a free function.
#[cfg(feature = "wmemcheck")]
fn free_start(instance: &mut Instance) {
    if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
        wmemcheck_state.memcheck_off();
    }
}

// Hook for tracking wasm stack updates using wmemcheck_state.
#[cfg(feature = "wmemcheck")]
fn update_stack_pointer(_instance: &mut Instance, _value: u32) {
    // TODO: stack-tracing has yet to be finalized. All memory below
    // the address of the top of the stack is marked as valid for
    // loads and stores.
    // if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
    //     instance.wmemcheck_state.update_stack_pointer(value as usize);
    // }
}

// Hook updating wmemcheck_state memory state vector every time memory.grow is called.
#[cfg(feature = "wmemcheck")]
fn update_mem_size(instance: &mut Instance, num_pages: u32) {
    if let Some(wmemcheck_state) = &mut instance.wmemcheck_state {
        const KIB: usize = 1024;
        let num_bytes = num_pages as usize * 64 * KIB;
        wmemcheck_state.update_mem_size(num_bytes);
    }
}

fn trap(_instance: &mut Instance, code: u8) -> Result<(), TrapReason> {
    Err(TrapReason::Wasm(
        wasmtime_environ::Trap::from_u8(code).unwrap(),
    ))
}

fn f64_to_i64(_instance: &mut Instance, val: f64) -> Result<u64, TrapReason> {
    if val.is_nan() {
        return Err(TrapReason::Wasm(Trap::BadConversionToInteger));
    }
    let val = relocs::truncf64(val);
    if val <= -9223372036854777856.0 || val >= 9223372036854775808.0 {
        return Err(TrapReason::Wasm(Trap::IntegerOverflow));
    }
    #[allow(clippy::cast_possible_truncation)]
    return Ok((val as i64).unsigned());
}

fn f64_to_u64(_instance: &mut Instance, val: f64) -> Result<u64, TrapReason> {
    if val.is_nan() {
        return Err(TrapReason::Wasm(Trap::BadConversionToInteger));
    }
    let val = relocs::truncf64(val);
    if val <= -1.0 || val >= 18446744073709551616.0 {
        return Err(TrapReason::Wasm(Trap::IntegerOverflow));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    return Ok(val as u64);
}

fn f64_to_i32(_instance: &mut Instance, val: f64) -> Result<u32, TrapReason> {
    if val.is_nan() {
        return Err(TrapReason::Wasm(Trap::BadConversionToInteger));
    }
    let val = relocs::truncf64(val);
    if val <= -2147483649.0 || val >= 2147483648.0 {
        return Err(TrapReason::Wasm(Trap::IntegerOverflow));
    }
    #[allow(clippy::cast_possible_truncation)]
    return Ok((val as i32).unsigned());
}

fn f64_to_u32(_instance: &mut Instance, val: f64) -> Result<u32, TrapReason> {
    if val.is_nan() {
        return Err(TrapReason::Wasm(Trap::BadConversionToInteger));
    }
    let val = relocs::truncf64(val);
    if val <= -1.0 || val >= 4294967296.0 {
        return Err(TrapReason::Wasm(Trap::IntegerOverflow));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    return Ok(val as u32);
}

/// This module contains functions which are used for resolving relocations at
/// runtime if necessary.
///
/// These functions are not used by default and currently the only platform
/// they're used for is on x86_64 when SIMD is disabled and then SSE features
/// are further disabled. In these configurations Cranelift isn't allowed to use
/// native CPU instructions so it falls back to libcalls and we rely on the Rust
/// standard library generally for implementing these.
#[allow(missing_docs)]
pub mod relocs {
    macro_rules! float_function {
        (std: $std:path, core: $core:path,) => {{
            #[cfg(feature = "std")]
            let func = $std;
            #[cfg(not(feature = "std"))]
            let func = $core;
            func
        }};
    }
    pub extern "C" fn floorf32(f: f32) -> f32 {
        let func = float_function! {
            std: f32::floor,
            core: libm::floorf,
        };
        func(f)
    }

    pub extern "C" fn floorf64(f: f64) -> f64 {
        let func = float_function! {
            std: f64::floor,
            core: libm::floor,
        };
        func(f)
    }

    pub extern "C" fn ceilf32(f: f32) -> f32 {
        let func = float_function! {
            std: f32::ceil,
            core: libm::ceilf,
        };
        func(f)
    }

    pub extern "C" fn ceilf64(f: f64) -> f64 {
        let func = float_function! {
            std: f64::ceil,
            core: libm::ceil,
        };
        func(f)
    }

    pub extern "C" fn truncf32(f: f32) -> f32 {
        let func = float_function! {
            std: f32::trunc,
            core: libm::truncf,
        };
        func(f)
    }

    pub extern "C" fn truncf64(f: f64) -> f64 {
        let func = float_function! {
            std: f64::trunc,
            core: libm::trunc,
        };
        func(f)
    }

    const TOINT_32: f32 = 1.0 / f32::EPSILON;
    const TOINT_64: f64 = 1.0 / f64::EPSILON;

    // NB: replace with `round_ties_even` from libstd when it's stable as
    // tracked by rust-lang/rust#96710
    pub extern "C" fn nearestf32(x: f32) -> f32 {
        // Rust doesn't have a nearest function; there's nearbyint, but it's not
        // stabilized, so do it manually.
        // Nearest is either ceil or floor depending on which is nearest or even.
        // This approach exploited round half to even default mode.
        let i = x.to_bits();
        let e = i >> 23 & 0xff;
        if e >= 0x7f_u32 + 23 {
            // Check for NaNs.
            if e == 0xff {
                // Read the 23-bits significand.
                if i & 0x7fffff != 0 {
                    // Ensure it's arithmetic by setting the significand's most
                    // significant bit to 1; it also works for canonical NaNs.
                    return f32::from_bits(i | (1 << 22));
                }
            }
            x
        } else {
            let abs = float_function! {
                std: f32::abs,
                core: libm::fabsf,
            };
            let copysign = float_function! {
                std: f32::copysign,
                core: libm::copysignf,
            };

            copysign(abs(x) + TOINT_32 - TOINT_32, x)
        }
    }

    pub extern "C" fn nearestf64(x: f64) -> f64 {
        let i = x.to_bits();
        let e = i >> 52 & 0x7ff;
        if e >= 0x3ff_u64 + 52 {
            // Check for NaNs.
            if e == 0x7ff {
                // Read the 52-bits significand.
                if i & 0xfffffffffffff != 0 {
                    // Ensure it's arithmetic by setting the significand's most
                    // significant bit to 1; it also works for canonical NaNs.
                    return f64::from_bits(i | (1 << 51));
                }
            }
            x
        } else {
            let abs = float_function! {
                std: f64::abs,
                core: libm::fabs,
            };
            let copysign = float_function! {
                std: f64::copysign,
                core: libm::copysign,
            };

            copysign(abs(x) + TOINT_64 - TOINT_64, x)
        }
    }

    pub extern "C" fn fmaf32(a: f32, b: f32, c: f32) -> f32 {
        let func = float_function! {
            std: f32::mul_add,
            core: libm::fmaf,
        };
        func(a, b, c)
    }

    pub extern "C" fn fmaf64(a: f64, b: f64, c: f64) -> f64 {
        let func = float_function! {
            std: f64::mul_add,
            core: libm::fma,
        };
        func(a, b, c)
    }

    // This intrinsic is only used on x86_64 platforms as an implementation of
    // the `pshufb` instruction when SSSE3 is not available.
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::__m128i;
    #[cfg(target_arch = "x86_64")]
    #[allow(improper_ctypes_definitions)]
    pub extern "C" fn x86_pshufb(a: __m128i, b: __m128i) -> __m128i {
        union U {
            reg: __m128i,
            mem: [u8; 16],
        }

        unsafe {
            let a = U { reg: a }.mem;
            let b = U { reg: b }.mem;

            let select = |arr: &[u8; 16], byte: u8| {
                if byte & 0x80 != 0 {
                    0x00
                } else {
                    arr[(byte & 0xf) as usize]
                }
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
}

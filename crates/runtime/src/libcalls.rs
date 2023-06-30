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
//!   `crates/runtime/src/backtrace.rs`.)
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

use crate::externref::VMExternRef;
use crate::table::{Table, TableElementType};
use crate::vmcontext::VMFuncRef;
use crate::{Instance, TrapReason};
use anyhow::Result;
use std::mem;
use std::ptr::{self, NonNull};
use std::time::{Duration, Instant};
use wasmtime_environ::{
    DataIndex, ElemIndex, FuncIndex, GlobalIndex, MemoryIndex, TableIndex, Trap,
};

/// Actually public trampolines which are used by the runtime as the entrypoint
/// for libcalls.
///
/// Note that the trampolines here are actually defined in inline assembly right
/// now to ensure that the fp/sp on exit are recorded for backtraces to work
/// properly.
pub mod trampolines {
    use crate::{Instance, TrapReason, VMContext};

    macro_rules! libcall {
        (
            $(
                $( #[$attr:meta] )*
                $name:ident( vmctx: vmctx $(, $pname:ident: $param:ident )* ) $( -> $result:ident )?;
            )*
        ) => {paste::paste! {
            $(
                // The actual libcall itself, which has the `pub` name here, is
                // defined via the `wasm_to_libcall_trampoline!` macro on
                // supported platforms or otherwise in inline assembly for
                // platforms like s390x which don't have stable `global_asm!`
                // yet.
                extern "C" {
                    #[allow(missing_docs)]
                    #[allow(improper_ctypes)]
                    #[wasmtime_versioned_export_macros::versioned_link]
                    pub fn $name(
                        vmctx: *mut VMContext,
                        $( $pname: libcall!(@ty $param), )*
                    ) $(-> libcall!(@ty $result))?;
                }

                wasm_to_libcall_trampoline!($name ; [<impl_ $name>]);

                // This is the direct entrypoint from the inline assembly which
                // still has the same raw signature as the trampoline itself.
                // This will delegate to the outer module to the actual
                // implementation and automatically perform `catch_unwind` along
                // with conversion of the return value in the face of traps.
                //
                // Note that rust targets which support `global_asm!` can use
                // the `sym` operator to get the symbol here, but other targets
                // like s390x need to use outlined assembly files which requires
                // `no_mangle`.
                #[cfg_attr(target_arch = "s390x", wasmtime_versioned_export_macros::versioned_export)]
                unsafe extern "C" fn [<impl_ $name>](
                    vmctx: *mut VMContext,
                    $( $pname : libcall!(@ty $param), )*
                ) $( -> libcall!(@ty $result))? {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        Instance::from_vmctx(vmctx, |instance| {
                            super::$name(instance, $($pname),*)
                        })
                    }));
                    match result {
                        Ok(ret) => LibcallResult::convert(ret),
                        Err(panic) => crate::traphandlers::resume_panic(panic),
                    }
                }

                // This works around a `rustc` bug where compiling with LTO
                // will sometimes strip out some of these symbols resulting
                // in a linking failure.
                #[allow(non_upper_case_globals)]
                #[used]
                static [<impl_ $name _ref>]: unsafe extern "C" fn(
                    *mut VMContext,
                    $( $pname : libcall!(@ty $param), )*
                ) $( -> libcall!(@ty $result))? = [<impl_ $name>];

            )*
        }};

        (@ty i32) => (u32);
        (@ty i64) => (u64);
        (@ty reference) => (*mut u8);
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
                Err(e) => crate::traphandlers::raise_trap(e.into()),
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
            Some(size_in_bytes) => size_in_bytes / (wasmtime_environ::WASM_PAGE_SIZE as usize),
            None => usize::max_value(),
        };
    Ok(result as *mut _)
}

// Implementation of `table.grow`.
unsafe fn table_grow(
    instance: &mut Instance,
    table_index: u32,
    delta: u32,
    // NB: we don't know whether this is a pointer to a `VMFuncRef`
    // or is a `VMExternRef` until we look at the table type.
    init_value: *mut u8,
) -> Result<u32> {
    let table_index = TableIndex::from_u32(table_index);
    let element = match instance.table_element_type(table_index) {
        TableElementType::Func => (init_value as *mut VMFuncRef).into(),
        TableElementType::Extern => {
            let init_value = if init_value.is_null() {
                None
            } else {
                Some(VMExternRef::clone_from_raw(init_value))
            };
            init_value.into()
        }
    };
    Ok(match instance.table_grow(table_index, delta, element)? {
        Some(r) => r,
        None => -1_i32 as u32,
    })
}

use table_grow as table_grow_func_ref;
use table_grow as table_grow_externref;

// Implementation of `table.fill`.
unsafe fn table_fill(
    instance: &mut Instance,
    table_index: u32,
    dst: u32,
    // NB: we don't know whether this is a `VMExternRef` or a pointer to a
    // `VMFuncRef` until we look at the table's element type.
    val: *mut u8,
    len: u32,
) -> Result<(), Trap> {
    let table_index = TableIndex::from_u32(table_index);
    let table = &mut *instance.get_table(table_index);
    match table.element_type() {
        TableElementType::Func => {
            let val = val as *mut VMFuncRef;
            table.fill(dst, val.into(), len)
        }
        TableElementType::Extern => {
            let val = if val.is_null() {
                None
            } else {
                Some(VMExternRef::clone_from_raw(val))
            };
            table.fill(dst, val.into(), len)
        }
    }
}

use table_fill as table_fill_func_ref;
use table_fill as table_fill_externref;

// Implementation of `table.copy`.
unsafe fn table_copy(
    instance: &mut Instance,
    dst_table_index: u32,
    src_table_index: u32,
    dst: u32,
    src: u32,
    len: u32,
) -> Result<(), Trap> {
    let dst_table_index = TableIndex::from_u32(dst_table_index);
    let src_table_index = TableIndex::from_u32(src_table_index);
    let dst_table = instance.get_table(dst_table_index);
    // Lazy-initialize the whole range in the source table first.
    let src_range = src..(src.checked_add(len).unwrap_or(u32::MAX));
    let src_table = instance.get_table_with_lazy_init(src_table_index, src_range);
    Table::copy(dst_table, src_table, dst, src, len)
}

// Implementation of `table.init`.
fn table_init(
    instance: &mut Instance,
    table_index: u32,
    elem_index: u32,
    dst: u32,
    src: u32,
    len: u32,
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
    index: u32,
) -> *mut u8 {
    let table_index = TableIndex::from_u32(table_index);
    let table = instance.get_table_with_lazy_init(table_index, std::iter::once(index));
    let elem = (*table)
        .get(index)
        .expect("table access already bounds-checked");

    elem.into_ref_asserting_initialized()
}

// Drop a `VMExternRef`.
unsafe fn drop_externref(_instance: &mut Instance, externref: *mut u8) {
    let externref = externref as *mut crate::externref::VMExternData;
    let externref = NonNull::new(externref).unwrap().into();
    crate::externref::VMExternData::drop_and_dealloc(externref);
}

// Do a GC and insert the given `externref` into the
// `VMExternRefActivationsTable`.
unsafe fn activations_table_insert_with_gc(instance: &mut Instance, externref: *mut u8) {
    let externref = VMExternRef::clone_from_raw(externref);
    let limits = *instance.runtime_limits();
    let (activations_table, module_info_lookup) = (*instance.store()).externref_activations_table();

    // Invariant: all `externref`s on the stack have an entry in the activations
    // table. So we need to ensure that this `externref` is in the table
    // *before* we GC, even though `insert_with_gc` will ensure that it is in
    // the table *after* the GC. This technically results in one more hash table
    // look up than is strictly necessary -- which we could avoid by having an
    // additional GC method that is aware of these GC-triggering references --
    // but it isn't really a concern because this is already a slow path.
    activations_table.insert_without_gc(externref.clone());

    activations_table.insert_with_gc(limits, externref, module_info_lookup);
}

// Perform a Wasm `global.get` for `externref` globals.
unsafe fn externref_global_get(instance: &mut Instance, index: u32) -> *mut u8 {
    let index = GlobalIndex::from_u32(index);
    let limits = *instance.runtime_limits();
    let global = instance.defined_or_imported_global_ptr(index);
    match (*global).as_externref().clone() {
        None => ptr::null_mut(),
        Some(externref) => {
            let raw = externref.as_raw();
            let (activations_table, module_info_lookup) =
                (*instance.store()).externref_activations_table();
            activations_table.insert_with_gc(limits, externref, module_info_lookup);
            raw
        }
    }
}

// Perform a Wasm `global.set` for `externref` globals.
unsafe fn externref_global_set(instance: &mut Instance, index: u32, externref: *mut u8) {
    let externref = if externref.is_null() {
        None
    } else {
        Some(VMExternRef::clone_from_raw(externref))
    };

    let index = GlobalIndex::from_u32(index);
    let global = instance.defined_or_imported_global_ptr(index);

    // Swap the new `externref` value into the global before we drop the old
    // value. This protects against an `externref` with a `Drop` implementation
    // that calls back into Wasm and touches this global again (we want to avoid
    // it observing a halfway-deinitialized value).
    let old = mem::replace((*global).as_externref_mut(), externref);
    drop(old);
}

// Implementation of `memory.atomic.notify` for locally defined memories.
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
fn memory_atomic_wait32(
    instance: &mut Instance,
    memory_index: u32,
    addr_index: u64,
    expected: u32,
    timeout: u64,
) -> Result<u32, Trap> {
    // convert timeout to Instant, before any wait happens on locking
    let timeout = (timeout as i64 >= 0).then(|| Instant::now() + Duration::from_nanos(timeout));
    let memory = MemoryIndex::from_u32(memory_index);
    Ok(instance
        .get_runtime_memory(memory)
        .atomic_wait32(addr_index, expected, timeout)? as u32)
}

// Implementation of `memory.atomic.wait64` for locally defined memories.
fn memory_atomic_wait64(
    instance: &mut Instance,
    memory_index: u32,
    addr_index: u64,
    expected: u64,
    timeout: u64,
) -> Result<u32, Trap> {
    // convert timeout to Instant, before any wait happens on locking
    let timeout = (timeout as i64 >= 0).then(|| Instant::now() + Duration::from_nanos(timeout));
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
    pub extern "C" fn floorf32(f: f32) -> f32 {
        f.floor()
    }

    pub extern "C" fn floorf64(f: f64) -> f64 {
        f.floor()
    }

    pub extern "C" fn ceilf32(f: f32) -> f32 {
        f.ceil()
    }

    pub extern "C" fn ceilf64(f: f64) -> f64 {
        f.ceil()
    }

    pub extern "C" fn truncf32(f: f32) -> f32 {
        f.trunc()
    }

    pub extern "C" fn truncf64(f: f64) -> f64 {
        f.trunc()
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
            (x.abs() + TOINT_32 - TOINT_32).copysign(x)
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
            (x.abs() + TOINT_64 - TOINT_64).copysign(x)
        }
    }

    pub extern "C" fn fmaf32(a: f32, b: f32, c: f32) -> f32 {
        a.mul_add(b, c)
    }

    pub extern "C" fn fmaf64(a: f64, b: f64, c: f64) -> f64 {
        a.mul_add(b, c)
    }

    // This intrinsic is only used on x86_64 platforms as an implementation of
    // the `pshufb` instruction when SSSE3 is not available.
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::__m128i;
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

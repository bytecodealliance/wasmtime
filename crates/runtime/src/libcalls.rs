//! Runtime library calls.
//!
//! Note that Wasm compilers may sometimes perform these inline rather than
//! calling them, particularly when CPUs have special instructions which compute
//! them directly.
//!
//! These functions are called by compiled Wasm code, and therefore must take
//! certain care about some things:
//!
//! * They must always be `pub extern "C"` and should only contain basic, raw
//!   i32/i64/f32/f64/pointer parameters that are safe to pass across the system
//!   ABI!
//!
//! * If any nested function propagates an `Err(trap)` out to the library
//!   function frame, we need to raise it. This involves some nasty and quite
//!   unsafe code under the covers! Notable, after raising the trap, drops
//!   **will not** be run for local variables! This can lead to things like
//!   leaking `InstanceHandle`s which leads to never deallocating JIT code,
//!   instances, and modules! Therefore, always use the `fallible_lib_call`
//!   macro to define such functions.
//!
//! * Any libcall function that can trap must also be defined with the
//!   `fallible_lib_call` macro!
//!
//! * When receiving a raw `*mut u8` that is actually a `VMExternRef` reference,
//!   convert it into a proper `VMExternRef` with `VMExternRef::clone_from_raw`
//!   as soon as apossible. Any GC before raw pointer is converted into a
//!   reference can potentially collect the referenced object, which could lead
//!   to use after free. Avoid this by eagerly converting into a proper
//!   `VMExternRef`!
//!
//!   ```ignore
//!   pub unsafe extern "C" my_lib_takes_ref(raw_extern_ref: *mut u8) {
//!       // Before `clone_from_raw`, `raw_extern_ref` is potentially unrooted,
//!       // and doing GC here could lead to use after free!
//!
//!       let my_extern_ref = if raw_extern_ref.is_null() {
//!           None
//!       } else {
//!           Some(VMExternRef::clone_from_raw(raw_extern_ref))
//!       };
//!
//!       // Now that we did `clone_from_raw`, it is safe to do a GC (or do
//!       // anything else that might transitively GC, like call back into
//!       // Wasm!)
//!   }
//!   ```

use crate::externref::VMExternRef;
use crate::instance::Instance;
use crate::table::{Table, TableElementType};
use crate::traphandlers::resume_panic;
use crate::vmcontext::{VMCallerCheckedAnyfunc, VMContext};
use std::mem;
use std::ptr::{self, NonNull};
use wasmtime_environ::{
    DataIndex, ElemIndex, FuncIndex, GlobalIndex, MemoryIndex, TableIndex, TrapCode,
};

/// Define a fallible (trap raising and/or can panic) libcall function.
///
/// This macro takes care of handling panics and raising traps for you, without
/// you having to worry about leaking resources due to `longjmp`ing and skipping
/// destructors. Additionally, it handles defining the Wasm-to-host trampoline
/// for maintaining Wasm frame and stack pointers, used when capturing
/// backtraces.
///
/// If your libcall can raise a user trap, use `-> Result<T>` for returns
/// (i.e. the error variant is the default `anyhow::Error), but if it can only
/// raise a lib trap, then use `-> Result<T, TrapCode>` for your return type
/// (use the `wasmtime_environ::TrapCode` type for the error variant). If your
/// libcall can both raise user and lib traps, then use `-> Result<T,
/// TrapReason>`.
///
/// # Example
///
/// ```ignore
/// fallible_lib_call! {
///     my_libcall => unsafe fn __wasmtime_my_libcall(vmctx: *mut VMContext) -> Result<u64> {
///         // do stuff here
///     }
/// }
/// ```
macro_rules! fallible_lib_call {
    // Variant for `TrapReason` error types.
    (
        $libcall:ident => unsafe fn $impl_name:ident (
            $vmctx:ident: *mut VMContext $( $arg:tt )*
        ) -> Result<$ret:ty, TrapReason> {
            $( $body:tt )*
        }
    ) => {
        fallible_lib_call!(@extern_decl $libcall ; ( $vmctx: *mut VMContext $( $arg )* ) ; $ret);
        fallible_lib_call!(@global_asm $libcall ; $impl_name);

        #[no_mangle]
        unsafe extern "C" fn $impl_name( $vmctx: *mut VMContext $( $arg )* ) -> $ret {
            let result = std::panic::catch_unwind(|| { $( $body )* });
            match result {
                Ok(Ok(x)) => x,
                Ok(Err(reason)) => {
                    let reason: crate::traphandlers::TrapReason = reason;
                    debug_assert!(!reason.is_jit());
                    crate::traphandlers::raise_trap(reason)
                },
                Err(panic) => resume_panic(panic),
            }
        }
    };

    // Variant for `TrapCode` error types.
    (
        $libcall:ident => unsafe fn $impl_name:ident (
            $vmctx:ident: *mut VMContext $( $arg:tt )*
        ) -> Result<$ret:ty, TrapCode> {
            $( $body:tt )*
        }
    ) => {
        fallible_lib_call!(@extern_decl $libcall ; ( $vmctx: *mut VMContext $( $arg )* ) ; $ret);
        fallible_lib_call!(@global_asm $libcall ; $impl_name);

        #[no_mangle]
        unsafe extern "C" fn $impl_name( $vmctx: *mut VMContext $( $arg )* ) -> $ret {
            let result = std::panic::catch_unwind(|| { $( $body )* });
            match result {
                Ok(Ok(x)) => x,
                Ok(Err(trap)) => crate::traphandlers::raise_lib_trap(trap),
                Err(panic) => resume_panic(panic),
            }
        }
    };

    // Variant for (implicit) `anyhow::Error` error types.
    (
        $libcall:ident => unsafe fn $impl_name:ident (
            $vmctx:ident: *mut VMContext $( $arg:tt )*
        ) -> Result<$ret:ty> {
            $( $body:tt )*
        }
    ) => {
        fallible_lib_call!(@extern_decl $libcall ; ( $vmctx: *mut VMContext $( $arg )* ) ; $ret);
        fallible_lib_call!(@global_asm $libcall ; $impl_name);

        #[no_mangle]
        unsafe extern "C" fn $impl_name( $vmctx: *mut VMContext $( $arg )* ) -> $ret {
            let result = std::panic::catch_unwind(|| { $( $body )* });
            match result {
                Ok(Ok(x)) => x,
                Ok(Err(trap)) => crate::traphandlers::raise_user_trap(trap),
                Err(panic) => resume_panic(panic),
            }
        }
    };

    // Declare the `extern` libcall function (that is actually the Wasm-to-host
    // trampoline).
    (@extern_decl $libcall:ident ; ( $( $arg: tt)* ) ; $ret:ty) => {
        extern "C" {
            #[allow(missing_docs)]
            #[allow(improper_ctypes)]
            pub fn $libcall ( $( $arg )* ) -> $ret;
        }
    };

    // Define the Wasm-to-host exit trampoline for this libcall.
    (@global_asm $libcall:ident ; $libcall_impl:ident) => {
        wasm_to_libcall_trampoline!($libcall ; $libcall_impl);
    }
}

fallible_lib_call! {
    memory32_grow => unsafe fn __wasmtime_memory32_grow_impl(
        vmctx: *mut VMContext,
        delta: u64,
        memory_index: u32,
    ) -> Result<*mut u8> {
        let instance = (*vmctx).instance_mut();
        let memory_index = MemoryIndex::from_u32(memory_index);
        let result = match instance.memory_grow(memory_index, delta)? {
            Some(size_in_bytes) => size_in_bytes / (wasmtime_environ::WASM_PAGE_SIZE as usize),
            None => usize::max_value(),
        };
        Ok(result as *mut _)
    }
}

// Implementation of `table.grow`.
//
// Table grow can invoke user code provided in a ResourceLimiter{,Async}, so we
// need to catch a possible panic.
fallible_lib_call! {
    table_grow => unsafe fn __wasmtime_table_grow(
        vmctx: *mut VMContext,
        table_index: u32,
        delta: u32,
        // NB: we don't know whether this is a pointer to a `VMCallerCheckedAnyfunc`
        // or is a `VMExternRef` until we look at the table type.
        init_value: *mut u8,
    ) -> Result<u32> {
        let instance = (*vmctx).instance_mut();
        let table_index = TableIndex::from_u32(table_index);
        let element = match instance.table_element_type(table_index) {
            TableElementType::Func => (init_value as *mut VMCallerCheckedAnyfunc).into(),
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
}

pub use table_grow as table_grow_funcref;
pub use table_grow as table_grow_externref;

// Implementation of `table.fill`.
fallible_lib_call! {
    table_fill => unsafe fn __wasmtime_table_fill(
        vmctx: *mut VMContext,
        table_index: u32,
        dst: u32,
        // NB: we don't know whether this is a `VMExternRef` or a pointer to a
        // `VMCallerCheckedAnyfunc` until we look at the table's element type.
        val: *mut u8,
        len: u32,
    ) -> Result<(), TrapCode> {
        let instance = (*vmctx).instance_mut();
        let table_index = TableIndex::from_u32(table_index);
        let table = &mut *instance.get_table(table_index);
        match table.element_type() {
            TableElementType::Func => {
                let val = val as *mut VMCallerCheckedAnyfunc;
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
}

pub use table_fill as table_fill_funcref;
pub use table_fill as table_fill_externref;

// Implementation of `table.copy`.
fallible_lib_call! {
    table_copy => unsafe fn __wasmtime_table_copy(
        vmctx: *mut VMContext,
        dst_table_index: u32,
        src_table_index: u32,
        dst: u32,
        src: u32,
        len: u32,
    ) -> Result<(), TrapCode> {
        let dst_table_index = TableIndex::from_u32(dst_table_index);
        let src_table_index = TableIndex::from_u32(src_table_index);
        let instance = (*vmctx).instance_mut();
        let dst_table = instance.get_table(dst_table_index);
        // Lazy-initialize the whole range in the source table first.
        let src_range = src..(src.checked_add(len).unwrap_or(u32::MAX));
        let src_table = instance.get_table_with_lazy_init(src_table_index, src_range);
        Table::copy(dst_table, src_table, dst, src, len)
    }
}

// Implementation of `table.init`.
fallible_lib_call! {
    table_init => unsafe fn __wasmtime_table_init(
        vmctx: *mut VMContext,
        table_index: u32,
        elem_index: u32,
        dst: u32,
        src: u32,
        len: u32,
    ) -> Result<(), TrapCode> {
        let table_index = TableIndex::from_u32(table_index);
        let elem_index = ElemIndex::from_u32(elem_index);
        let instance = (*vmctx).instance_mut();
        instance.table_init(table_index, elem_index, dst, src, len)
    }
}

/// Implementation of `elem.drop`.
pub unsafe extern "C" fn elem_drop(vmctx: *mut VMContext, elem_index: u32) {
    let elem_index = ElemIndex::from_u32(elem_index);
    let instance = (*vmctx).instance_mut();
    instance.elem_drop(elem_index);
}

// Implementation of `memory.copy` for locally defined memories.
fallible_lib_call! {
    memory_copy => unsafe fn __wasmtime_memory_copy(
        vmctx: *mut VMContext,
        dst_index: u32,
        dst: u64,
        src_index: u32,
        src: u64,
        len: u64,
    ) -> Result<(), TrapCode> {
        let src_index = MemoryIndex::from_u32(src_index);
        let dst_index = MemoryIndex::from_u32(dst_index);
        let instance = (*vmctx).instance_mut();
        instance.memory_copy(dst_index, dst, src_index, src, len)
    }
}

// Implementation of `memory.fill` for locally defined memories.
fallible_lib_call! {
    memory_fill => unsafe fn __wasmtime_memory_fill(
        vmctx: *mut VMContext,
        memory_index: u32,
        dst: u64,
        val: u32,
        len: u64,
    ) -> Result<(), TrapCode> {
        let memory_index = MemoryIndex::from_u32(memory_index);
        let instance = (*vmctx).instance_mut();
        instance.memory_fill(memory_index, dst, val as u8, len)
    }
}

// Implementation of `memory.init`.
fallible_lib_call! {
    memory_init => unsafe fn __wasmtime_memory_init(
        vmctx: *mut VMContext,
        memory_index: u32,
        data_index: u32,
        dst: u64,
        src: u32,
        len: u32,
    ) -> Result<(), TrapCode> {
        let memory_index = MemoryIndex::from_u32(memory_index);
        let data_index = DataIndex::from_u32(data_index);
        let instance = (*vmctx).instance_mut();
        instance.memory_init(memory_index, data_index, dst, src, len)
    }
}

/// Implementation of `ref.func`.
pub unsafe extern "C" fn ref_func(vmctx: *mut VMContext, func_index: u32) -> *mut u8 {
    let instance = (*vmctx).instance_mut();
    let anyfunc = instance
        .get_caller_checked_anyfunc(FuncIndex::from_u32(func_index))
        .expect("ref_func: caller_checked_anyfunc should always be available for given func index");
    anyfunc as *mut _
}

/// Implementation of `data.drop`.
pub unsafe extern "C" fn data_drop(vmctx: *mut VMContext, data_index: u32) {
    let data_index = DataIndex::from_u32(data_index);
    let instance = (*vmctx).instance_mut();
    instance.data_drop(data_index)
}

/// Returns a table entry after lazily initializing it.
pub unsafe extern "C" fn table_get_lazy_init_funcref(
    vmctx: *mut VMContext,
    table_index: u32,
    index: u32,
) -> *mut u8 {
    let instance = (*vmctx).instance_mut();
    let table_index = TableIndex::from_u32(table_index);
    let table = instance.get_table_with_lazy_init(table_index, std::iter::once(index));
    let elem = (*table)
        .get(index)
        .expect("table access already bounds-checked");

    elem.into_ref_asserting_initialized() as *mut _
}

/// Drop a `VMExternRef`.
pub unsafe extern "C" fn drop_externref(externref: *mut u8) {
    let externref = externref as *mut crate::externref::VMExternData;
    let externref = NonNull::new(externref).unwrap();
    crate::externref::VMExternData::drop_and_dealloc(externref);
}

/// Do a GC and insert the given `externref` into the
/// `VMExternRefActivationsTable`.
pub unsafe extern "C" fn activations_table_insert_with_gc(
    vmctx: *mut VMContext,
    externref: *mut u8,
) {
    let externref = VMExternRef::clone_from_raw(externref);
    let instance = (*vmctx).instance();
    let (activations_table, module_info_lookup) = (*instance.store()).externref_activations_table();

    // Invariant: all `externref`s on the stack have an entry in the activations
    // table. So we need to ensure that this `externref` is in the table
    // *before* we GC, even though `insert_with_gc` will ensure that it is in
    // the table *after* the GC. This technically results in one more hash table
    // look up than is strictly necessary -- which we could avoid by having an
    // additional GC method that is aware of these GC-triggering references --
    // but it isn't really a concern because this is already a slow path.
    activations_table.insert_without_gc(externref.clone());

    activations_table.insert_with_gc(externref, module_info_lookup);
}

/// Perform a Wasm `global.get` for `externref` globals.
pub unsafe extern "C" fn externref_global_get(vmctx: *mut VMContext, index: u32) -> *mut u8 {
    let index = GlobalIndex::from_u32(index);
    let instance = (*vmctx).instance();
    let global = instance.defined_or_imported_global_ptr(index);
    match (*global).as_externref().clone() {
        None => ptr::null_mut(),
        Some(externref) => {
            let raw = externref.as_raw();
            let (activations_table, module_info_lookup) =
                (*instance.store()).externref_activations_table();
            activations_table.insert_with_gc(externref, module_info_lookup);
            raw
        }
    }
}

/// Perform a Wasm `global.set` for `externref` globals.
pub unsafe extern "C" fn externref_global_set(
    vmctx: *mut VMContext,
    index: u32,
    externref: *mut u8,
) {
    let externref = if externref.is_null() {
        None
    } else {
        Some(VMExternRef::clone_from_raw(externref))
    };

    let index = GlobalIndex::from_u32(index);
    let instance = (*vmctx).instance();
    let global = instance.defined_or_imported_global_ptr(index);

    // Swap the new `externref` value into the global before we drop the old
    // value. This protects against an `externref` with a `Drop` implementation
    // that calls back into Wasm and touches this global again (we want to avoid
    // it observing a halfway-deinitialized value).
    let old = mem::replace((*global).as_externref_mut(), externref);
    drop(old);
}

// Implementation of `memory.atomic.notify` for locally defined memories.
fallible_lib_call! {
    memory_atomic_notify => unsafe fn __wasmtime_memory_atomic_notify(
        vmctx: *mut VMContext,
        memory_index: u32,
        addr: *mut u8,
        _count: u32,
    ) -> Result<u32, TrapReason> {
        let addr = addr as usize;
        let memory = MemoryIndex::from_u32(memory_index);
        let instance = (*vmctx).instance();
        // this should never overflow since addr + 4 either hits a guard page
        // or it's been validated to be in-bounds already. Double-check for now
        // just to be sure.
        let addr_to_check = addr.checked_add(4).unwrap();
        validate_atomic_addr(instance, memory, addr_to_check)?;
        Err(anyhow::anyhow!(
            "unimplemented: wasm atomics (fn memory_atomic_notify) unsupported",
        ).into())
    }
}

// Implementation of `memory.atomic.wait32` for locally defined memories.
fallible_lib_call! {
    memory_atomic_wait32 => unsafe fn __wasmtime_memory_atomic_wait32(
        vmctx: *mut VMContext,
        memory_index: u32,
        addr: *mut u8,
        _expected: u32,
        _timeout: u64,
    ) -> Result<u32, TrapReason> {
        let addr = addr as usize;
        let memory = MemoryIndex::from_u32(memory_index);
        let instance = (*vmctx).instance();
        // see wasmtime_memory_atomic_notify for why this shouldn't overflow
        // but we still double-check
        let addr_to_check = addr.checked_add(4).unwrap();
        validate_atomic_addr(instance, memory, addr_to_check)?;
        Err(anyhow::anyhow!(
            "unimplemented: wasm atomics (fn memory_atomic_wait32) unsupported",
        ).into())
    }
}

// Implementation of `memory.atomic.wait64` for locally defined memories.
fallible_lib_call! {
    memory_atomic_wait64 => unsafe fn __wasmtime_memory_atomic_wait64(
        vmctx: *mut VMContext,
        memory_index: u32,
        addr: *mut u8,
        _expected: u64,
        _timeout: u64,
    ) -> Result<u32, TrapReason> {
        let addr = addr as usize;
        let memory = MemoryIndex::from_u32(memory_index);
        let instance = (*vmctx).instance();
        // see wasmtime_memory_atomic_notify for why this shouldn't overflow
        // but we still double-check
        let addr_to_check = addr.checked_add(8).unwrap();
        validate_atomic_addr(instance, memory, addr_to_check)?;
        Err(anyhow::anyhow!(
            "unimplemented: wasm atomics (fn memory_atomic_wait64) unsupported",
        ).into())
    }
}

/// For atomic operations we still check the actual address despite this also
/// being checked via the `heap_addr` instruction in cranelift. The reason for
/// that is because the `heap_addr` instruction can defer to a later segfault to
/// actually recognize the out-of-bounds whereas once we're running Rust code
/// here we don't want to segfault.
///
/// In the situations where bounds checks were elided in JIT code (because oob
/// would then be later guaranteed to segfault) this manual check is here
/// so we don't segfault from Rust.
unsafe fn validate_atomic_addr(
    instance: &Instance,
    memory: MemoryIndex,
    addr: usize,
) -> Result<(), TrapCode> {
    if addr > instance.get_memory(memory).current_length() {
        return Err(TrapCode::HeapOutOfBounds);
    }
    Ok(())
}

// Hook for when an instance runs out of fuel.
fallible_lib_call! {
    out_of_gas => unsafe fn __wasmtime_out_of_gas(vmctx: *mut VMContext) -> Result<()> {
         (*(*vmctx).instance().store()).out_of_gas()
    }
}

// Hook for when an instance observes that the epoch has changed.
fallible_lib_call! {
    new_epoch => unsafe fn __wasmtime_new_epoch(vmctx: *mut VMContext) -> Result<u64> {
        (*(*vmctx).instance().store()).new_epoch()
    }
}

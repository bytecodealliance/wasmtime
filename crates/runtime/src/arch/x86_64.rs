//! x86_64-specific definitions of architecture-specific functions in Wasmtime.

/// x86 vectors are represented with XMM registers which are represented
/// with the `__m128i` type. This type is considered a vector type for
/// ABI purposes which is implemented by Cranelift.
pub type V128Abi = std::arch::x86_64::__m128i;

#[inline]
#[allow(missing_docs)]
pub fn get_stack_pointer() -> usize {
    let stack_pointer: usize;
    unsafe {
        std::arch::asm!(
            "mov {}, rsp",
            out(reg) stack_pointer,
            options(nostack,nomem),
        );
    }
    stack_pointer
}

pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    // The calling convention always pushes the return pointer (aka the PC of
    // the next older frame) just before this frame.
    *(fp as *mut usize).offset(1)
}

// And the current frame pointer points to the next older frame pointer.
pub const NEXT_OLDER_FP_FROM_FP_OFFSET: usize = 0;

pub fn reached_entry_sp(fp: usize, entry_sp: usize) -> bool {
    fp >= entry_sp
}

pub fn assert_entry_sp_is_aligned(sp: usize) {
    assert_eq!(sp % 16, 0, "stack should always be aligned to 16");
}

pub fn assert_fp_is_aligned(fp: usize) {
    assert_eq!(fp % 16, 0, "stack should always be aligned to 16");
}

// Helper macros for getting the first and second arguments according to the
// system calling convention, as well as some callee-saved scratch registers we
// can safely use in the trampolines.
cfg_if::cfg_if! {
    if #[cfg(windows)] {
        macro_rules! callee_vmctx { () => ("rcx") }
        macro_rules! scratch0 { () => ("r10") }
        macro_rules! scratch1 { () => ("r11") }
    } else if #[cfg(unix)] {
        macro_rules! callee_vmctx { () => ("rdi") }
        macro_rules! scratch0 { () => ("r10") }
        macro_rules! scratch1 { () => ("r11") }
    } else {
        compile_error!("default calling convention for this platform is not known");
    }
}
pub(crate) use {callee_vmctx, scratch0, scratch1};

#[rustfmt::skip]
macro_rules! wasm_to_libcall_trampoline {
    ($libcall:ident ; $libcall_impl:ident) => {
        wasmtime_asm_macros::asm_func!(
            wasmtime_versioned_export_macros::versioned_stringify_ident!($libcall),
            concat!(
                "
                   .cfi_startproc simple
                   .cfi_def_cfa_offset 0

                    // Load the pointer to `VMRuntimeLimits` in `scratch0!()`.
                    mov ", crate::arch::scratch0!(), ", 8[", crate::arch::callee_vmctx!(), "]

                    // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
                    mov 24[", crate::arch::scratch0!(), "], rbp

                    // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
                    mov ", crate::arch::scratch1!(), ", [rsp]
                    mov 32[", crate::arch::scratch0!(), "], ", crate::arch::scratch1!(), "

                    // Tail call to the actual implementation of this libcall.
                    jmp {}

                    .cfi_endproc
                ",
            ),
            sym $libcall_impl
        );
    };
}
pub(crate) use wasm_to_libcall_trampoline;

#[cfg(test)]
mod wasm_to_libcall_trampoline_offsets_tests {
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn test() {
        let module = Module::new();
        let offsets = VMOffsets::new(std::mem::size_of::<*mut u8>() as u8, &module);

        assert_eq!(8, offsets.vmctx_runtime_limits());
        assert_eq!(24, offsets.ptr.vmruntime_limits_last_wasm_exit_fp());
        assert_eq!(32, offsets.ptr.vmruntime_limits_last_wasm_exit_pc());
    }
}

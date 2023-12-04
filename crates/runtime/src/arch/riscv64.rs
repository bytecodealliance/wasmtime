//! Riscv64-specific definitions of architecture-specific functions in Wasmtime.

/// RISC-V currently always passes all vector arguments indirectly in the
/// ABI. Currently Rust has no stable means of representing this meaning
/// that a 128-bit representation is chosen here but it can't be passed
/// directly to WebAssembly, for example, and must instead be passed
/// through an array-call trampoline.
pub type V128Abi = u128;

#[inline]
#[allow(missing_docs)]
pub fn get_stack_pointer() -> usize {
    let stack_pointer: usize;
    unsafe {
        std::arch::asm!(
            "mv {}, sp",
            out(reg) stack_pointer,
            options(nostack,nomem),
        );
    }
    stack_pointer
}

pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
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

#[rustfmt::skip]
macro_rules! wasm_to_libcall_trampoline {
    ($libcall:ident ; $libcall_impl:ident) => {
        wasmtime_asm_macros::asm_func!(
            wasmtime_versioned_export_macros::versioned_stringify_ident!($libcall),
            concat!(
                "
                    .cfi_startproc

                    // Load the pointer to `VMRuntimeLimits` in `t0`.
                    ld t0, 8(a0)

                    // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
                    sd fp, 24(t0)

                    // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
                    sd ra, 32(t0)

                    // Tail call to the actual implementation of this libcall.
                .Lhi_{0}:
                    auipc t0, %pcrel_hi({0})
                    jalr x0, %pcrel_lo(.Lhi_{0})(t0)

                    .cfi_endproc
                ",
            ),
            sym $libcall_impl,
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

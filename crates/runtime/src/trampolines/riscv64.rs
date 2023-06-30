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
                    j {}

                    .cfi_endproc
                ",
            ),
            sym $libcall_impl,
        );
    };
}

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

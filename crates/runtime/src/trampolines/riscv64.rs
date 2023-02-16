use wasmtime_asm_macros::asm_func;

#[rustfmt::skip]
asm_func!(
    "host_to_wasm_trampoline",
    r#"
        .cfi_startproc

        // Load the pointer to `VMRuntimeLimits` in `t0`.
        ld t0, 8(a1)

        // Check to see if callee is a core `VMContext` (MAGIC == "core"). NB:
        // we do not support big-endian riscv64 so the magic value is always
        // little-endian encoded.
        li t1,0x65726f63
        lwu t3,0(a0)
        bne t3,t1,ne
          mv t1,sp
          j over
        ne:
          li t1,-1
        over:
        // Store the last Wasm SP into the `last_wasm_entry_sp` in the limits, if this
        // was core Wasm, otherwise store an invalid sentinal value.
        sd t1,40(t0)

        ld t0,16(a1)
        jr t0

        .cfi_endproc
    "#
);

#[cfg(test)]
mod host_to_wasm_trampoline_offsets_tests {
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn test() {
        let module = Module::new();
        let offsets = VMOffsets::new(std::mem::size_of::<*mut u8>() as u8, &module);

        assert_eq!(8, offsets.vmctx_runtime_limits());
        assert_eq!(40, offsets.ptr.vmruntime_limits_last_wasm_entry_sp());
        assert_eq!(16, offsets.vmctx_callee());
        assert_eq!(0x65726f63, u32::from_le_bytes(*b"core"));
    }
}

#[rustfmt::skip]
asm_func!(
    "wasm_to_host_trampoline",
    "
        .cfi_startproc simple

        // Load the pointer to `VMRuntimeLimits` in `t0`.
        ld t0,8(a1)

        // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
        sd fp,24(t0)

        // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
        sd ra,32(t0)

        // Tail call to the actual host function.
        //
        // This *must* be a tail call so that we do not push to the stack and mess
        // up the offsets of stack arguments (if any).
        ld t0, 8(a0)
        jr t0
        .cfi_endproc
    ",
);

#[cfg(test)]
mod wasm_to_host_trampoline_offsets_tests {
    use crate::VMHostFuncContext;
    use memoffset::offset_of;
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn test() {
        let module = Module::new();
        let offsets = VMOffsets::new(std::mem::size_of::<*mut u8>() as u8, &module);

        assert_eq!(8, offsets.vmctx_runtime_limits());
        assert_eq!(24, offsets.ptr.vmruntime_limits_last_wasm_exit_fp());
        assert_eq!(32, offsets.ptr.vmruntime_limits_last_wasm_exit_pc());
        assert_eq!(8, offset_of!(VMHostFuncContext, host_func));
    }
}

#[rustfmt::skip]
macro_rules! wasm_to_libcall_trampoline {
    ($libcall:ident ; $libcall_impl:ident) => {
        wasmtime_asm_macros::asm_func!(
            stringify!($libcall),
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

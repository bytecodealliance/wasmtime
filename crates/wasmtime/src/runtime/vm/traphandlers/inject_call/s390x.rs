//! s390x injected-call trampoline.
//!
//! see [`super::x86_64`] for documentation on how the
//! architecture-specific trampolines are used.

use core::arch::naked_asm;

#[unsafe(naked)]
pub(crate) unsafe extern "C" fn injected_call_trampoline(_hostcall: usize, _store: usize) {
    naked_asm!(
        "
        // Unlike an ordinary function in the s390x calling convention,
        // we do not enter here with a stack frame already allocated for
        // us; rather, we have to assume everything at SP (%r15) and up is
        // user-controlled, and we must allocate our own frame. We also save
        // all registers, not only volatiles.

        aghi %r15, -128
        stmg %r0, %r15, 0(%r15)

        aghi %r15, -512
        // We want to save vector registers, but the default Rust target
        // s390x-unknown-linux-gnu does not have the vector extension
        // enabled by default, even though we generate vector code with
        // Cranelift just fine. So we're going to manually encode instructions
        // below with byte literals (eek!). Manually verified with
        // `s390x-linux-gnu-as` + `s390x-linux-gnu-objdump`.

        // vst %v0, 0(%r15)
        .word 0xe700, 0xf000, 0x000e
        // vst %v1, 16(%r15)
        .word 0xe710, 0xf010, 0x000e
        // vst %v2, 32(%r15)
        .word 0xe720, 0xf020, 0x000e
        // vst %v3, 48(%r15)
        .word 0xe730, 0xf030, 0x000e
        // vst %v4, 64(%r15)
        .word 0xe740, 0xf040, 0x000e
        // vst %v5, 80(%r15)
        .word 0xe750, 0xf050, 0x000e
        // vst %v6, 96(%r15)
        .word 0xe760, 0xf060, 0x000e
        // vst %v7, 112(%r15)
        .word 0xe770, 0xf070, 0x000e
        // vst %v8, 128(%r15)
        .word 0xe780, 0xf080, 0x000e
        // vst %v9, 144(%r15)
        .word 0xe790, 0xf090, 0x000e
        // vst %v10, 160(%r15)
        .word 0xe7a0, 0xf0a0, 0x000e
        // vst %v11, 176(%r15)
        .word 0xe7b0, 0xf0b0, 0x000e
        // vst %v12, 192(%r15)
        .word 0xe7c0, 0xf0c0, 0x000e
        // vst %v13, 208(%r15)
        .word 0xe7d0, 0xf0d0, 0x000e
        // vst %v14, 224(%r15)
        .word 0xe7e0, 0xf0e0, 0x000e
        // vst %v15, 240(%r15)
        .word 0xe7f0, 0xf0f0, 0x000e
        // vst %v16, 256(%r15)
        .word 0xe700, 0xf100, 0x080e
        // vst %v17, 272(%r15)
        .word 0xe710, 0xf110, 0x080e
        // vst %v18, 288(%r15)
        .word 0xe720, 0xf120, 0x080e
        // vst %v19, 304(%r15)
        .word 0xe730, 0xf130, 0x080e
        // vst %v20, 320(%r15)
        .word 0xe740, 0xf140, 0x080e
        // vst %v21, 336(%r15)
        .word 0xe750, 0xf150, 0x080e
        // vst %v22, 352(%r15)
        .word 0xe760, 0xf160, 0x080e
        // vst %v23, 368(%r15)
        .word 0xe770, 0xf170, 0x080e
        // vst %v24, 384(%r15)
        .word 0xe780, 0xf180, 0x080e
        // vst %v25, 400(%r15)
        .word 0xe790, 0xf190, 0x080e
        // vst %v26, 416(%r15)
        .word 0xe7a0, 0xf1a0, 0x080e
        // vst %v27, 432(%r15)
        .word 0xe7b0, 0xf1b0, 0x080e
        // vst %v28, 448(%r15)
        .word 0xe7c0, 0xf1c0, 0x080e
        // vst %v29, 464(%r15)
        .word 0xe7d0, 0xf1d0, 0x080e
        // vst %v30, 480(%r15)
        .word 0xe7e0, 0xf1e0, 0x080e
        // vst %v31, 496(%r15)
        .word 0xe7f0, 0xf1f0, 0x080e

        // Create frame for callee.
        aghi %r15, -160
        lgr %r1, %r2 // Arg 0: next callee.
        lgr %r2, %r3 // Arg 1: callee's first arg.
        lgr %r3, %r15
        // Address of saved return address. We use r1: it is the spilltmp
        // in Cranelift-compiled code and will not be relied upon to be
        // saved across any trapping instruction.
        aghi %r3, 160 + 512 + 1*8
        lgr %r4, %r15
        aghi %r4, 160 + 512 + 2*8 // Address of saved r2 (arg 0).
        lgr %r5, %r15
        aghi %r5, 160 + 512 + 3*8 // Address of saved r3 (arg 1).
        basr %r14, %r1
        aghi %r15, 160

        // vl %v0, 0(%r15)
        .word 0xe700, 0xf000, 0x0006
        // vl %v1, 16(%r15)
        .word 0xe710, 0xf010, 0x0006
        // vl %v2, 32(%r15)
        .word 0xe720, 0xf020, 0x0006
        // vl %v3, 48(%r15)
        .word 0xe730, 0xf030, 0x0006
        // vl %v4, 64(%r15)
        .word 0xe740, 0xf040, 0x0006
        // vl %v5, 80(%r15)
        .word 0xe750, 0xf050, 0x0006
        // vl %v6, 96(%r15)
        .word 0xe760, 0xf060, 0x0006
        // vl %v7, 112(%r15)
        .word 0xe770, 0xf070, 0x0006
        // vl %v8, 128(%r15)
        .word 0xe780, 0xf080, 0x0006
        // vl %v9, 144(%r15)
        .word 0xe790, 0xf090, 0x0006
        // vl %v10, 160(%r15)
        .word 0xe7a0, 0xf0a0, 0x0006
        // vl %v11, 176(%r15)
        .word 0xe7b0, 0xf0b0, 0x0006
        // vl %v12, 192(%r15)
        .word 0xe7c0, 0xf0c0, 0x0006
        // vl %v13, 208(%r15)
        .word 0xe7d0, 0xf0d0, 0x0006
        // vl %v14, 224(%r15)
        .word 0xe7e0, 0xf0e0, 0x0006
        // vl %v15, 240(%r15)
        .word 0xe7f0, 0xf0f0, 0x0006
        // vl %v16, 256(%r15)
        .word 0xe700, 0xf100, 0x0806
        // vl %v17, 272(%r15)
        .word 0xe710, 0xf110, 0x0806
        // vl %v18, 288(%r15)
        .word 0xe720, 0xf120, 0x0806
        // vl %v19, 304(%r15)
        .word 0xe730, 0xf130, 0x0806
        // vl %v20, 320(%r15)
        .word 0xe740, 0xf140, 0x0806
        // vl %v21, 336(%r15)
        .word 0xe750, 0xf150, 0x0806
        // vl %v22, 352(%r15)
        .word 0xe760, 0xf160, 0x0806
        // vl %v23, 368(%r15)
        .word 0xe770, 0xf170, 0x0806
        // vl %v24, 384(%r15)
        .word 0xe780, 0xf180, 0x0806
        // vl %v25, 400(%r15)
        .word 0xe790, 0xf190, 0x0806
        // vl %v26, 416(%r15)
        .word 0xe7a0, 0xf1a0, 0x0806
        // vl %v27, 432(%r15)
        .word 0xe7b0, 0xf1b0, 0x0806
        // vl %v28, 448(%r15)
        .word 0xe7c0, 0xf1c0, 0x0806
        // vl %v29, 464(%r15)
        .word 0xe7d0, 0xf1d0, 0x0806
        // vl %v30, 480(%r15)
        .word 0xe7e0, 0xf1e0, 0x0806
        // vl %v31, 496(%r15)
        .word 0xe7f0, 0xf1f0, 0x0806

        aghi %r15, 512

        lmg %r0, %r15, 0(%r15)
        // No need to add 128 to SP (%r15); we restored it
        // with the load-multiple above.

        br %r1
    ",
    );
}

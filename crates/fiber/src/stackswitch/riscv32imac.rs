// A WORD OF CAUTION
//
// This entire file basically needs to be kept in sync with itself. It's not
// really possible to modify just one bit of this file without understanding
// all the other bits. Documentation tries to reference various bits here and
// there but try to make sure to read over everything before tweaking things!
//
// This file is modelled after riscv64.rs. For reference be sure to review the
// other file.

use core::arch::naked_asm;

#[inline(never)] // FIXME(rust-lang/rust#148307)
pub(crate) unsafe extern "C" fn wasmtime_fiber_switch(top_of_stack: *mut u8) {
    unsafe { wasmtime_fiber_switch_(top_of_stack) }
}

#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_switch_(top_of_stack: *mut u8 /* a0 */) {
    naked_asm!(
        "
      // See https://github.com/rust-lang/rust/issues/80608.
      .attribute arch, \"rv32i\" // This implementation should work for any
      // architecture with the same registers as riscv32i, e.g. riscv32imac,
      // but not riscv32gc.

      // We're switching to arbitrary code somewhere else, so pessimistically
      // assume that all callee-save register are clobbered. This means we need
      // to save/restore all of them.
      //
      // Note that this order for saving is important since we use CFI directives
      // below to point to where all the saved registers are.
      sw ra,-0x4(sp)
      sw fp,-0x8(sp) // fp is s0
      sw s1,-0xc(sp)
      sw s2,-0x10(sp)
      sw s3,-0x14(sp)
      sw s4,-0x18(sp)
      sw s5,-0x1c(sp)
      sw s6,-0x20(sp)
      sw s7,-0x24(sp)
      sw s8,-0x28(sp)
      sw s9,-0x2c(sp)
      sw s10,-0x30(sp)
      sw s11,-0x34(sp)
      addi sp , sp , -0x40 // Choose 0x40 to be 16-byte aligned

      lw t0 ,-0x8(a0)
      sw sp ,-0x8(a0)

      // Swap stacks and restore all our callee-saved registers
      mv sp,t0

      lw s11,0xc(sp)
      lw s10,0x10(sp)
      lw s9,0x14(sp)
      lw s8,0x18(sp)
      lw s7,0x1c(sp)
      lw s6,0x20(sp)
      lw s5,0x24(sp)
      lw s4,0x28(sp)
      lw s3,0x2c(sp)
      lw s2,0x30(sp)
      lw s1,0x34(sp)
      lw fp,0x38(sp)
      lw ra,0x3c(sp)
      addi sp , sp , 0x40
      jr ra
        ",
    );
}

pub(crate) unsafe fn wasmtime_fiber_init(
    top_of_stack: *mut u8,
    entry_point: extern "C" fn(*mut u8, *mut u8),
    entry_arg0: *mut u8,
) {
    #[repr(C)]
    #[derive(Default)]
    struct InitialStack {
        padding: [u8; 12], // 12 bytes of padding for 16-byte alignment

        s11: *mut u8,
        s10: *mut u8,
        s9: *mut u8,
        s8: *mut u8,
        s7: *mut u8,
        s6: *mut u8,
        s5: *mut u8,
        s4: *mut u8,
        s3: *mut u8,
        s2: *mut u8,
        s1: *mut u8,
        fp: *mut u8,

        ra: *mut u8,

        // unix.rs reserved space
        last_sp: *mut u8,
        run_result: *mut u8,
    }

    unsafe {
        let initial_stack = top_of_stack.cast::<InitialStack>().sub(1);
        initial_stack.write(InitialStack {
            s1: entry_point as *mut u8,
            s2: entry_arg0,
            fp: top_of_stack,
            ra: wasmtime_fiber_start as *mut u8,
            last_sp: initial_stack.cast(),
            ..InitialStack::default()
        });
    }
}

#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_start() -> ! {
    naked_asm!(
        "
    .cfi_startproc simple
    .cfi_def_cfa_offset 0


    .cfi_escape 0x0f, /* DW_CFA_def_cfa_expression */ \
      5,             /* the byte length of this expression */ \
      0x52,          /* DW_OP_reg2 (sp) */ \
      0x06,          /* DW_OP_deref */ \
      0x08, 0x40 ,   /* DW_OP_const1u 0x40 */ \
      0x22           /* DW_OP_plus */


      .cfi_rel_offset ra,-0x4
      .cfi_rel_offset fp,-0x8
      .cfi_rel_offset s1,-0xc
      .cfi_rel_offset s2,-0x10
      .cfi_rel_offset s3,-0x14
      .cfi_rel_offset s4,-0x18
      .cfi_rel_offset s5,-0x1c
      .cfi_rel_offset s6,-0x20
      .cfi_rel_offset s7,-0x24
      .cfi_rel_offset s8,-0x28
      .cfi_rel_offset s9,-0x2c
      .cfi_rel_offset s10,-0x30
      .cfi_rel_offset s11,-0x34

      mv a0,s2
      mv a1,fp
      jalr s1
      // .4byte 0 will cause panic.
      // for safety just like x86_64.rs and riscv64.rs.
      .4byte 0
      .cfi_endproc
  ",
    );
}

// A WORD OF CAUTION
//
// This entire file basically needs to be kept in sync with itself. It's not
// really possible to modify just one bit of this file without understanding
// all the other bits. Documentation tries to reference various bits here and
// there but try to make sure to read over everything before tweaking things!

use core::arch::naked_asm;

#[inline(never)] // FIXME(rust-lang/rust#148307)
pub(crate) unsafe extern "C" fn wasmtime_fiber_switch(top_of_stack: *mut u8) {
    unsafe { wasmtime_fiber_switch_(top_of_stack) }
}

#[unsafe(naked)]
unsafe extern "C" fn wasmtime_fiber_switch_(top_of_stack: *mut u8 /* a0 */) {
    naked_asm!(
        "
      // Save all callee-saved registers on the stack since we're
      // assuming they're clobbered as a result of the stack switch.
      st.d $ra, $sp, -0x08
      st.d $fp, $sp, -0x10
      st.d $s0, $sp, -0x18
      st.d $s1, $sp, -0x20
      st.d $s2, $sp, -0x28
      st.d $s3, $sp, -0x30
      st.d $s4, $sp, -0x38
      st.d $s5, $sp, -0x40
      st.d $s6, $sp, -0x48
      st.d $s7, $sp, -0x50
      st.d $s8, $sp, -0x58
      fst.d $fs0, $sp, -0x60
      fst.d $fs1, $sp, -0x68
      fst.d $fs2, $sp, -0x70
      fst.d $fs3, $sp, -0x78
      fst.d $fs4, $sp, -0x80
      fst.d $fs5, $sp, -0x88
      fst.d $fs6, $sp, -0x90
      fst.d $fs7, $sp, -0x98
      addi.d $sp, $sp, -0xa0

      // Load our previously saved stack pointer to resume to, and save
      // off our current stack pointer on where to come back to
      // eventually.
      ld.d $t0, $a0, -0x10
      st.d $sp, $a0, -0x10

      // Switch to the new stack and restore all our callee-saved
      // registers after the switch and return to our new stack.
      move $sp, $t0

      fld.d $fs7, $sp, 0x08
      fld.d $fs6, $sp, 0x10
      fld.d $fs5, $sp, 0x18
      fld.d $fs4, $sp, 0x20
      fld.d $fs3, $sp, 0x28
      fld.d $fs2, $sp, 0x30
      fld.d $fs1, $sp, 0x38
      fld.d $fs0, $sp, 0x40
      ld.d $s8, $sp, 0x48
      ld.d $s7, $sp, 0x50
      ld.d $s6, $sp, 0x58
      ld.d $s5, $sp, 0x60
      ld.d $s4, $sp, 0x68
      ld.d $s3, $sp, 0x70
      ld.d $s2, $sp, 0x78
      ld.d $s1, $sp, 0x80
      ld.d $s0, $sp, 0x88
      ld.d $fp, $sp, 0x90
      ld.d $ra, $sp, 0x98
      addi.d $sp, $sp, 0xa0
      ret
        ",
    );
}

pub(crate) unsafe fn wasmtime_fiber_init(
    top_of_stack: *mut u8,
    entry_point: extern "C" fn(*mut u8, *mut u8) -> *mut u8,
    entry_arg0: *mut u8,
) {
    #[repr(C)]
    #[derive(Default)]
    struct InitialStack {
        align_to_16_byte_size: u64,

        fs: [f64; 8],

        s8: *mut u8,
        s7: *mut u8,
        s6: *mut u8,
        s5: *mut u8,
        s4: *mut u8,
        s3: *mut u8,
        s2: *mut u8,
        s1: *mut u8,
        s0: *mut u8,

        fp: *mut u8,
        ra: *mut u8,

        // unix.rs reserved space
        last_sp: *mut u8,
        run_result: *mut u8,
    }

    unsafe {
        let initial_stack = top_of_stack.cast::<InitialStack>().sub(1);
        initial_stack.write(InitialStack {
            s0: entry_point as *mut u8,
            s1: entry_arg0,
            s2: wasmtime_fiber_switch_ as *mut u8,
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
        0x53,          /* DW_OP_reg3 (sp) */ \
        0x06,          /* DW_OP_deref */ \
        0x08, 0xa0,    /* DW_OP_const1u 0x98 */ \
        0x22           /* DW_OP_plus */


      .cfi_rel_offset ra, -0x8
      .cfi_rel_offset fp, -0x10
      .cfi_rel_offset s0, -0x18
      .cfi_rel_offset s1, -0x20
      .cfi_rel_offset s2, -0x28
      .cfi_rel_offset s3, -0x30
      .cfi_rel_offset s4, -0x38
      .cfi_rel_offset s5, -0x40
      .cfi_rel_offset s6, -0x48
      .cfi_rel_offset s7, -0x50
      .cfi_rel_offset s8, -0x58
      .cfi_rel_offset fs0, -0x60
      .cfi_rel_offset fs1, -0x68
      .cfi_rel_offset fs2, -0x70
      .cfi_rel_offset fs3, -0x78
      .cfi_rel_offset fs4, -0x80
      .cfi_rel_offset fs5, -0x88
      .cfi_rel_offset fs6, -0x90
      .cfi_rel_offset fs7, -0x98

      move $a0, $s1
      move $a1, $fp
      jirl $ra, $s0, 0 // entry_point
      jirl $ra, $s2, 0 // wasmtime_fiber_switch_

      // .4byte 0 will cause panic.
      // for safety just like x86_64.rs.
      .4byte 0
      .cfi_endproc
  ",
    );
}

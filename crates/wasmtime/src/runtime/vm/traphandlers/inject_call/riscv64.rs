//! riscv64 injected-call trampoline.
//!
//! see [`super::x86_64`] for documentation on how the
//! architecture-specific trampolines are used.

use core::arch::naked_asm;

#[unsafe(naked)]
pub(crate) unsafe extern "C" fn injected_call_trampoline(_hostcall: usize, _store: usize) {
    naked_asm!(
        "
       // Note that we assume the V extension on platforms that
       // support debugging. To do otherwise, we'd need two
       // versions of this trampoline and pick the right one at
       // runtime depending on the engine configuration.
       .attribute arch, \"rv64gc\"
       .option push
       .option arch, +zve32x

        addi sp, sp, -16
        sd zero, 8(sp) // Fake return address.
        sd fp, 0(sp)
        mv fp, sp

        // 28 (other) X regs: 8 * 28 = 224
        addi sp, sp, -224

        sd gp, 216(sp)
        sd tp, 208(sp)
        sd t0, 200(sp)
        sd t1, 192(sp)
        sd t2, 184(sp)
        // s0 is fp (saved above).
        sd s1, 176(sp)
        sd a0, 168(sp)
        sd a1, 160(sp)
        sd a2, 152(sp)
        sd a3, 144(sp)
        sd a4, 136(sp)
        sd a5, 128(sp)
        sd a6, 120(sp)
        sd a7, 112(sp)
        sd s2, 104(sp)
        sd s3, 96(sp)
        sd s4, 88(sp)
        sd s5, 80(sp)
        sd s6, 72(sp)
        sd s7, 64(sp)
        sd s8, 56(sp)
        sd s9, 48(sp)
        sd s10, 40(sp)
        sd s11, 32(sp)
        sd t3, 24(sp)
        sd t4, 16(sp)
        sd t5, 8(sp)
        sd t6, 0(sp)

        // 32 F regs: 8 * 32 = 256
        addi sp, sp, -256
        fsd f0, 248(sp)
        fsd f1, 240(sp)
        fsd f2, 232(sp)
        fsd f3, 224(sp)
        fsd f4, 216(sp)
        fsd f5, 208(sp)
        fsd f6, 200(sp)
        fsd f7, 192(sp)
        fsd f8, 184(sp)
        fsd f9, 176(sp)
        fsd f10, 168(sp)
        fsd f11, 160(sp)
        fsd f12, 152(sp)
        fsd f13, 144(sp)
        fsd f14, 136(sp)
        fsd f15, 128(sp)
        fsd f16, 120(sp)
        fsd f17, 112(sp)
        fsd f18, 104(sp)
        fsd f19, 96(sp)
        fsd f20, 88(sp)
        fsd f21, 80(sp)
        fsd f22, 72(sp)
        fsd f23, 64(sp)
        fsd f24, 56(sp)
        fsd f25, 48(sp)
        fsd f26, 40(sp)
        fsd f27, 32(sp)
        fsd f28, 24(sp)
        fsd f29, 16(sp)
        fsd f30, 8(sp)
        fsd f31, 0(sp)

        // V extension state save/restore.
        // See the Linux kernel context-switching implementation at
        // https://github.com/torvalds/linux/blob/98906f9d850e4882004749eccb8920649dc98456/arch/riscv/include/asm/vector.h
        // for a good source on all of this.

        // First save the VSTART, VTYPE, VL, VCSR control registers.
        addi sp, sp, -32
        csrr t0, 0x8   // VSTART
        csrr t1, 0xc21 // VTYPE
        csrr t2, 0xc20 // VL
        csrr t3, 0xf   // VCSR
        sd t0, 24(sp)
        sd t1, 16(sp)
        sd t2, 8(sp)
        sd t3, 0(sp)

        // 32 V regs, saving 128 bits because that is
        // all we use in Cranelift-compiled code.

        // Set the VLEN and VTYPE registers so that we have grouping
        // by 8 in the V register bank.
        vsetivli zero, 16, e8, m8, ta, ma // 16 elems, 8-bit elems, 8-reg groups,
                                          // tail-agnostic, mask-agnostic
        addi sp, sp, -128
        vse8.v v0, 0(sp)
        addi sp, sp, -128
        vse8.v v8, 0(sp)
        addi sp, sp, -128
        vse8.v v16, 0(sp)
        addi sp, sp, -128
        vse8.v v24, 0(sp)

        mv t0, a0
        mv a0, a1
        addi a1, sp, (512 + 32 + 256 + 224 + 8) // saved RA.
        addi a2, sp, (512 + 32 + 256 + 168)     // saved A0.
        addi a3, sp, (512 + 32 + 256 + 160)     // saved A1.
        jalr t0

        // Set up V state again for our restore.
        vsetivli zero, 16, e8, m8, ta, ma
        vle8.v v24, 0(sp)
        addi sp, sp, 128
        vle8.v v16, 0(sp)
        addi sp, sp, 128
        vle8.v v8, 0(sp)
        addi sp, sp, 128
        vle8.v v0, 0(sp)
        addi sp, sp, 128

        // Restore VSTART, VTYPE, VL, VCSR.
        ld t0, 24(sp)
        ld t1, 16(sp)
        ld t2, 8(sp)
        ld t3, 0(sp)
        csrw 0x8, t0   // VSTART
        vsetvl zero, t2, t1
        csrw 0xf, t3   // VCSR
        addi sp, sp, 32

        // Restore F regs.
        fld f0, 248(sp)
        fld f1, 240(sp)
        fld f2, 232(sp)
        fld f3, 224(sp)
        fld f4, 216(sp)
        fld f5, 208(sp)
        fld f6, 200(sp)
        fld f7, 192(sp)
        fld f8, 184(sp)
        fld f9, 176(sp)
        fld f10, 168(sp)
        fld f11, 160(sp)
        fld f12, 152(sp)
        fld f13, 144(sp)
        fld f14, 136(sp)
        fld f15, 128(sp)
        fld f16, 120(sp)
        fld f17, 112(sp)
        fld f18, 104(sp)
        fld f19, 96(sp)
        fld f20, 88(sp)
        fld f21, 80(sp)
        fld f22, 72(sp)
        fld f23, 64(sp)
        fld f24, 56(sp)
        fld f25, 48(sp)
        fld f26, 40(sp)
        fld f27, 32(sp)
        fld f28, 24(sp)
        fld f29, 16(sp)
        fld f30, 8(sp)
        fld f31, 0(sp)
        addi sp, sp, 256

        // Restore X regs.
        ld gp, 216(sp)
        ld tp, 208(sp)
        ld t0, 200(sp)
        ld t1, 192(sp)
        ld t2, 184(sp)
        // s0 is fp (restored below).
        ld s1, 176(sp)
        ld a0, 168(sp)
        ld a1, 160(sp)
        ld a2, 152(sp)
        ld a3, 144(sp)
        ld a4, 136(sp)
        ld a5, 128(sp)
        ld a6, 120(sp)
        ld a7, 112(sp)
        ld s2, 104(sp)
        ld s3, 96(sp)
        ld s4, 88(sp)
        ld s5, 80(sp)
        ld s6, 72(sp)
        ld s7, 64(sp)
        ld s8, 56(sp)
        ld s9, 48(sp)
        ld s10, 40(sp)
        ld s11, 32(sp)
        ld t3, 24(sp)
        ld t4, 16(sp)
        ld t5, 8(sp)
        ld t6, 0(sp)
        addi sp, sp, 224

        // Restore FP and RA.
        ld fp, 0(sp)
        ld ra, 8(sp)
        addi sp, sp, 16

        // N.B.: this leaves RA clobbered; but Cranelift
        // code interrupted by a signal will already have
        // unconditionally saved LR in its frame.
        ret

        .option pop
    ",
    );
}

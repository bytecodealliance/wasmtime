//! Tests for the emitter
//!
//! See comments at the top of `fn x64_emit` for advice on how to create reliable test cases.
//!
//! to see stdout: cargo test -- --nocapture
//!
//! for this specific case, as of 24 Aug 2020:
//!
//! cd to the top of your wasmtime tree, then:
//!
//! RUST_BACKTRACE=1 cargo test --features test-programs/test_programs \
//!   --all --exclude wasmtime-wasi-nn \
//!   -- isa::x64::inst::emit_tests::test_x64_emit

use super::*;
use crate::ir::{MemFlags, UserExternalNameRef};
use crate::isa::x64;
use crate::isa::x64::lower::isle::generated_code::{Atomic128RmwSeqOp, AtomicRmwSeqOp};
use alloc::vec::Vec;
use cranelift_entity::EntityRef as _;

impl Inst {
    fn xmm_unary_rm_r_evex(op: Avx512Opcode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmUnaryRmREvex {
            op,
            src: XmmMem::unwrap_new(src),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    fn xmm_rm_r_evex(op: Avx512Opcode, src1: Reg, src2: RegMem, dst: Writable<Reg>) -> Self {
        debug_assert_ne!(op, Avx512Opcode::Vpermi2b);
        src2.assert_regclass_is(RegClass::Float);
        debug_assert!(src1.class() == RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmRmREvex {
            op,
            src1: Xmm::unwrap_new(src1),
            src2: XmmMem::unwrap_new(src2),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    fn xmm_rm_r_evex3(
        op: Avx512Opcode,
        src1: Reg,
        src2: Reg,
        src3: RegMem,
        dst: Writable<Reg>,
    ) -> Self {
        debug_assert_eq!(op, Avx512Opcode::Vpermi2b);
        src3.assert_regclass_is(RegClass::Float);
        debug_assert!(src1.class() == RegClass::Float);
        debug_assert!(src2.class() == RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmRmREvex3 {
            op,
            src1: Xmm::unwrap_new(src1),
            src2: Xmm::unwrap_new(src2),
            src3: XmmMem::unwrap_new(src3),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    fn setcc(cc: CC, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        let dst = WritableGpr::from_writable_reg(dst).unwrap();
        Inst::Setcc { cc, dst }
    }

    fn xmm_rm_r_imm(
        op: SseOpcode,
        src: RegMem,
        dst: Writable<Reg>,
        imm: u8,
        size: OperandSize,
    ) -> Inst {
        debug_assert!(size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        Inst::XmmRmRImm {
            op,
            src1: dst.to_reg(),
            src2: src,
            dst,
            imm,
            size,
        }
    }

    fn xmm_rm_r_blend(op: SseOpcode, src2: RegMem, dst: Writable<Reg>) -> Inst {
        Inst::XmmRmRBlend {
            op,
            src1: Xmm::unwrap_new(dst.to_reg()),
            src2: XmmMemAligned::unwrap_new(src2),
            mask: Xmm::unwrap_new(regs::xmm0()),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }
}

#[test]
fn test_x64_emit() {
    let rax = regs::rax();
    let rbx = regs::rbx();
    let rcx = regs::rcx();
    let rdx = regs::rdx();
    let rsi = regs::rsi();
    let rdi = regs::rdi();
    let rsp = regs::rsp();
    let rbp = regs::rbp();
    let r8 = regs::r8();
    let r9 = regs::r9();
    let r10 = regs::r10();
    let r11 = regs::r11();
    let r12 = regs::r12();
    let r13 = regs::r13();
    let r14 = regs::r14();
    let r15 = regs::r15();

    let xmm0 = regs::xmm0();
    let xmm1 = regs::xmm1();
    let xmm2 = regs::xmm2();
    let xmm3 = regs::xmm3();
    let xmm4 = regs::xmm4();
    let xmm5 = regs::xmm5();
    let xmm6 = regs::xmm6();
    let xmm7 = regs::xmm7();
    let xmm8 = regs::xmm8();
    let xmm9 = regs::xmm9();
    let xmm10 = regs::xmm10();
    let xmm11 = regs::xmm11();
    let xmm12 = regs::xmm12();
    let xmm13 = regs::xmm13();
    let xmm14 = regs::xmm14();
    let xmm15 = regs::xmm15();

    // And Writable<> versions of the same:
    let w_rax = Writable::<Reg>::from_reg(rax);
    let w_rbx = Writable::<Reg>::from_reg(rbx);
    let w_rcx = Writable::<Reg>::from_reg(rcx);
    let w_rdx = Writable::<Reg>::from_reg(rdx);
    let w_rsi = Writable::<Reg>::from_reg(rsi);
    let w_rdi = Writable::<Reg>::from_reg(rdi);
    let _w_rsp = Writable::<Reg>::from_reg(rsp);
    let _w_rbp = Writable::<Reg>::from_reg(rbp);
    let w_r8 = Writable::<Reg>::from_reg(r8);
    let w_r9 = Writable::<Reg>::from_reg(r9);
    let w_r10 = Writable::<Reg>::from_reg(r10);
    let w_r11 = Writable::<Reg>::from_reg(r11);
    let w_r12 = Writable::<Reg>::from_reg(r12);
    let w_r13 = Writable::<Reg>::from_reg(r13);
    let w_r14 = Writable::<Reg>::from_reg(r14);
    let w_r15 = Writable::<Reg>::from_reg(r15);

    let w_xmm0 = Writable::<Reg>::from_reg(xmm0);
    let w_xmm1 = Writable::<Reg>::from_reg(xmm1);
    let w_xmm2 = Writable::<Reg>::from_reg(xmm2);
    let w_xmm3 = Writable::<Reg>::from_reg(xmm3);
    let w_xmm4 = Writable::<Reg>::from_reg(xmm4);
    let w_xmm6 = Writable::<Reg>::from_reg(xmm6);
    let w_xmm7 = Writable::<Reg>::from_reg(xmm7);
    let w_xmm8 = Writable::<Reg>::from_reg(xmm8);
    let w_xmm9 = Writable::<Reg>::from_reg(xmm9);
    let w_xmm10 = Writable::<Reg>::from_reg(xmm10);
    let w_xmm11 = Writable::<Reg>::from_reg(xmm11);
    let w_xmm12 = Writable::<Reg>::from_reg(xmm12);
    let w_xmm13 = Writable::<Reg>::from_reg(xmm13);

    let mut insns = Vec::<(Inst, &str, &str)>::new();

    // End of test cases for Addr
    // ========================================================

    // ========================================================
    // General tests for each insn.  Don't forget to follow the
    // guidelines commented just prior to `fn x64_emit`.
    //

    // ========================================================
    // Imm_R
    //
    insns.push((
        Inst::imm(OperandSize::Size32, 1234567, w_r14),
        "41BE87D61200",
        "movl    $1234567, %r14d",
    ));
    insns.push((
        Inst::imm(OperandSize::Size32, -126i64 as u64, w_r14),
        "41BE82FFFFFF",
        "movl    $-126, %r14d",
    ));
    insns.push((
        Inst::imm(OperandSize::Size64, 1234567898765, w_r14),
        "49BE8D26FB711F010000",
        "movabsq $1234567898765, %r14",
    ));
    insns.push((
        Inst::imm(OperandSize::Size64, -126i64 as u64, w_r14),
        "49C7C682FFFFFF",
        "movabsq $-126, %r14",
    ));
    insns.push((
        Inst::imm(OperandSize::Size32, 1234567, w_rcx),
        "B987D61200",
        "movl    $1234567, %ecx",
    ));
    insns.push((
        Inst::imm(OperandSize::Size32, -126i64 as u64, w_rcx),
        "B982FFFFFF",
        "movl    $-126, %ecx",
    ));
    insns.push((
        Inst::imm(OperandSize::Size64, 1234567898765, w_rsi),
        "48BE8D26FB711F010000",
        "movabsq $1234567898765, %rsi",
    ));
    insns.push((
        Inst::imm(OperandSize::Size64, -126i64 as u64, w_rbx),
        "48C7C382FFFFFF",
        "movabsq $-126, %rbx",
    ));

    // ========================================================
    // Mov_R_R
    insns.push((
        Inst::mov_r_r(OperandSize::Size32, rbx, w_rsi),
        "89DE",
        "movl    %ebx, %esi",
    ));
    insns.push((
        Inst::mov_r_r(OperandSize::Size32, rbx, w_r9),
        "4189D9",
        "movl    %ebx, %r9d",
    ));
    insns.push((
        Inst::mov_r_r(OperandSize::Size32, r11, w_rsi),
        "4489DE",
        "movl    %r11d, %esi",
    ));
    insns.push((
        Inst::mov_r_r(OperandSize::Size32, r12, w_r9),
        "4589E1",
        "movl    %r12d, %r9d",
    ));
    insns.push((
        Inst::mov_r_r(OperandSize::Size64, rbx, w_rsi),
        "4889DE",
        "movq    %rbx, %rsi",
    ));
    insns.push((
        Inst::mov_r_r(OperandSize::Size64, rbx, w_r9),
        "4989D9",
        "movq    %rbx, %r9",
    ));
    insns.push((
        Inst::mov_r_r(OperandSize::Size64, r11, w_rsi),
        "4C89DE",
        "movq    %r11, %rsi",
    ));
    insns.push((
        Inst::mov_r_r(OperandSize::Size64, r12, w_r9),
        "4D89E1",
        "movq    %r12, %r9",
    ));

    // ========================================================
    // LoadEffectiveAddress
    insns.push((
        Inst::lea(Amode::imm_reg(42, r10), w_r8),
        "4D8D422A",
        "lea     42(%r10), %r8",
    ));
    insns.push((
        Inst::lea(Amode::imm_reg(42, r10), w_r15),
        "4D8D7A2A",
        "lea     42(%r10), %r15",
    ));
    insns.push((
        Inst::lea(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(r10), Gpr::unwrap_new(r9), 0),
            w_r8,
        ),
        "4F8D840AB3000000",
        "lea     179(%r10,%r9,1), %r8",
    ));
    insns.push((
        Inst::lea(
            Amode::rip_relative(MachLabel::from_block(BlockIndex::new(0))),
            w_rdi,
        ),
        "488D3D00000000",
        "lea     label0(%rip), %rdi",
    ));

    // Mov_Imm_M.

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size8,
            simm32: i8::MIN as i32,
            dst: Amode::imm_reg(99, rax).into(),
        },
        "C6406380",
        "movb    $-128, 99(%rax)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size8,
            simm32: i8::MAX as i32,
            dst: Amode::imm_reg(99, r8).into(),
        },
        "41C640637F",
        "movb    $127, 99(%r8)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size16,
            simm32: i16::MIN as i32,
            dst: Amode::imm_reg(99, rcx).into(),
        },
        "66C741630080",
        "movw    $-32768, 99(%rcx)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size16,
            simm32: i16::MAX as i32,
            dst: Amode::imm_reg(99, r9).into(),
        },
        "6641C74163FF7F",
        "movw    $32767, 99(%r9)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size32,
            simm32: i32::MIN,
            dst: Amode::imm_reg(99, rdx).into(),
        },
        "C7426300000080",
        "movl    $-2147483648, 99(%rdx)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size32,
            simm32: i32::MAX,
            dst: Amode::imm_reg(99, r10).into(),
        },
        "41C74263FFFFFF7F",
        "movl    $2147483647, 99(%r10)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size64,
            simm32: i32::MIN,
            dst: Amode::imm_reg(99, rbx).into(),
        },
        "48C7436300000080",
        "movq    $-2147483648, 99(%rbx)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size64,
            simm32: i32::MAX,
            dst: Amode::imm_reg(99, r11).into(),
        },
        "49C74363FFFFFF7F",
        "movq    $2147483647, 99(%r11)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size8,
            simm32: 0i32,
            dst: Amode::imm_reg(99, rsp).into(),
        },
        "C644246300",
        "movb    $0, 99(%rsp)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size16,
            simm32: 0i32,
            dst: Amode::imm_reg(99, r12).into(),
        },
        "6641C74424630000",
        "movw    $0, 99(%r12)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size32,
            simm32: 0i32,
            dst: Amode::imm_reg(99, rbp).into(),
        },
        "C7456300000000",
        "movl    $0, 99(%rbp)",
    ));

    insns.push((
        Inst::MovImmM {
            size: OperandSize::Size64,
            simm32: 0i32,
            dst: Amode::imm_reg(99, r13).into(),
        },
        "49C7456300000000",
        "movq    $0, 99(%r13)",
    ));

    // ========================================================
    // Mov_R_M.  Byte stores are tricky.  Check everything carefully.
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, rax, Amode::imm_reg(99, rdi)),
        "48894763",
        "movq    %rax, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, rbx, Amode::imm_reg(99, r8)),
        "49895863",
        "movq    %rbx, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, rcx, Amode::imm_reg(99, rsi)),
        "48894E63",
        "movq    %rcx, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, rdx, Amode::imm_reg(99, r9)),
        "49895163",
        "movq    %rdx, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, rsi, Amode::imm_reg(99, rax)),
        "48897063",
        "movq    %rsi, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, rdi, Amode::imm_reg(99, r15)),
        "49897F63",
        "movq    %rdi, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, rsp, Amode::imm_reg(99, rcx)),
        "48896163",
        "movq    %rsp, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, rbp, Amode::imm_reg(99, r14)),
        "49896E63",
        "movq    %rbp, 99(%r14)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, r8, Amode::imm_reg(99, rdi)),
        "4C894763",
        "movq    %r8, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, r9, Amode::imm_reg(99, r8)),
        "4D894863",
        "movq    %r9, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, r10, Amode::imm_reg(99, rsi)),
        "4C895663",
        "movq    %r10, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, r11, Amode::imm_reg(99, r9)),
        "4D895963",
        "movq    %r11, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, r12, Amode::imm_reg(99, rax)),
        "4C896063",
        "movq    %r12, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, r13, Amode::imm_reg(99, r15)),
        "4D896F63",
        "movq    %r13, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, r14, Amode::imm_reg(99, rcx)),
        "4C897163",
        "movq    %r14, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size64, r15, Amode::imm_reg(99, r14)),
        "4D897E63",
        "movq    %r15, 99(%r14)",
    ));
    //
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, rax, Amode::imm_reg(99, rdi)),
        "894763",
        "movl    %eax, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, rbx, Amode::imm_reg(99, r8)),
        "41895863",
        "movl    %ebx, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, rcx, Amode::imm_reg(99, rsi)),
        "894E63",
        "movl    %ecx, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, rdx, Amode::imm_reg(99, r9)),
        "41895163",
        "movl    %edx, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, rsi, Amode::imm_reg(99, rax)),
        "897063",
        "movl    %esi, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, rdi, Amode::imm_reg(99, r15)),
        "41897F63",
        "movl    %edi, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, rsp, Amode::imm_reg(99, rcx)),
        "896163",
        "movl    %esp, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, rbp, Amode::imm_reg(99, r14)),
        "41896E63",
        "movl    %ebp, 99(%r14)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, r8, Amode::imm_reg(99, rdi)),
        "44894763",
        "movl    %r8d, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, r9, Amode::imm_reg(99, r8)),
        "45894863",
        "movl    %r9d, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, r10, Amode::imm_reg(99, rsi)),
        "44895663",
        "movl    %r10d, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, r11, Amode::imm_reg(99, r9)),
        "45895963",
        "movl    %r11d, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, r12, Amode::imm_reg(99, rax)),
        "44896063",
        "movl    %r12d, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, r13, Amode::imm_reg(99, r15)),
        "45896F63",
        "movl    %r13d, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, r14, Amode::imm_reg(99, rcx)),
        "44897163",
        "movl    %r14d, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size32, r15, Amode::imm_reg(99, r14)),
        "45897E63",
        "movl    %r15d, 99(%r14)",
    ));
    //
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, rax, Amode::imm_reg(99, rdi)),
        "66894763",
        "movw    %ax, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, rbx, Amode::imm_reg(99, r8)),
        "6641895863",
        "movw    %bx, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, rcx, Amode::imm_reg(99, rsi)),
        "66894E63",
        "movw    %cx, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, rdx, Amode::imm_reg(99, r9)),
        "6641895163",
        "movw    %dx, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, rsi, Amode::imm_reg(99, rax)),
        "66897063",
        "movw    %si, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, rdi, Amode::imm_reg(99, r15)),
        "6641897F63",
        "movw    %di, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, rsp, Amode::imm_reg(99, rcx)),
        "66896163",
        "movw    %sp, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, rbp, Amode::imm_reg(99, r14)),
        "6641896E63",
        "movw    %bp, 99(%r14)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, r8, Amode::imm_reg(99, rdi)),
        "6644894763",
        "movw    %r8w, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, r9, Amode::imm_reg(99, r8)),
        "6645894863",
        "movw    %r9w, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, r10, Amode::imm_reg(99, rsi)),
        "6644895663",
        "movw    %r10w, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, r11, Amode::imm_reg(99, r9)),
        "6645895963",
        "movw    %r11w, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, r12, Amode::imm_reg(99, rax)),
        "6644896063",
        "movw    %r12w, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, r13, Amode::imm_reg(99, r15)),
        "6645896F63",
        "movw    %r13w, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, r14, Amode::imm_reg(99, rcx)),
        "6644897163",
        "movw    %r14w, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size16, r15, Amode::imm_reg(99, r14)),
        "6645897E63",
        "movw    %r15w, 99(%r14)",
    ));
    //
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, rax, Amode::imm_reg(99, rdi)),
        "884763",
        "movb    %al, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, rbx, Amode::imm_reg(99, r8)),
        "41885863",
        "movb    %bl, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, rcx, Amode::imm_reg(99, rsi)),
        "884E63",
        "movb    %cl, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, rdx, Amode::imm_reg(99, r9)),
        "41885163",
        "movb    %dl, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, rsi, Amode::imm_reg(99, rax)),
        "40887063",
        "movb    %sil, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, rdi, Amode::imm_reg(99, r15)),
        "41887F63",
        "movb    %dil, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, rsp, Amode::imm_reg(99, rcx)),
        "40886163",
        "movb    %spl, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, rbp, Amode::imm_reg(99, r14)),
        "41886E63",
        "movb    %bpl, 99(%r14)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, r8, Amode::imm_reg(99, rdi)),
        "44884763",
        "movb    %r8b, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, r9, Amode::imm_reg(99, r8)),
        "45884863",
        "movb    %r9b, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, r10, Amode::imm_reg(99, rsi)),
        "44885663",
        "movb    %r10b, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, r11, Amode::imm_reg(99, r9)),
        "45885963",
        "movb    %r11b, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, r12, Amode::imm_reg(99, rax)),
        "44886063",
        "movb    %r12b, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, r13, Amode::imm_reg(99, r15)),
        "45886F63",
        "movb    %r13b, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, r14, Amode::imm_reg(99, rcx)),
        "44887163",
        "movb    %r14b, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(OperandSize::Size8, r15, Amode::imm_reg(99, r14)),
        "45887E63",
        "movb    %r15b, 99(%r14)",
    ));

    // ========================================================
    // CmpRMIR
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size64, rdx, RegMemImm::reg(r15)),
        "4C39FA",
        "cmpq    %r15, %rdx",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size64, r8, RegMemImm::reg(rcx)),
        "4939C8",
        "cmpq    %rcx, %r8",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size64, rsi, RegMemImm::reg(rcx)),
        "4839CE",
        "cmpq    %rcx, %rsi",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size64,
            rdx,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "483B5763",
        "cmpq    99(%rdi), %rdx",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size64,
            r8,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "4C3B4763",
        "cmpq    99(%rdi), %r8",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size64,
            rsi,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "483B7763",
        "cmpq    99(%rdi), %rsi",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size64, rdx, RegMemImm::imm(76543210)),
        "4881FAEAF48F04",
        "cmpq    $76543210, %rdx",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size64, r8, RegMemImm::imm(-76543210i32 as u32)),
        "4981F8160B70FB",
        "cmpq    $-76543210, %r8",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size64, rsi, RegMemImm::imm(76543210)),
        "4881FEEAF48F04",
        "cmpq    $76543210, %rsi",
    ));
    //
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size32, rdx, RegMemImm::reg(r15)),
        "4439FA",
        "cmpl    %r15d, %edx",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size32, r8, RegMemImm::reg(rcx)),
        "4139C8",
        "cmpl    %ecx, %r8d",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size32, rsi, RegMemImm::reg(rcx)),
        "39CE",
        "cmpl    %ecx, %esi",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size32,
            rdx,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "3B5763",
        "cmpl    99(%rdi), %edx",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size32,
            r8,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "443B4763",
        "cmpl    99(%rdi), %r8d",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size32,
            rsi,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "3B7763",
        "cmpl    99(%rdi), %esi",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size32, rdx, RegMemImm::imm(76543210)),
        "81FAEAF48F04",
        "cmpl    $76543210, %edx",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size32, r8, RegMemImm::imm(-76543210i32 as u32)),
        "4181F8160B70FB",
        "cmpl    $-76543210, %r8d",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size32, rsi, RegMemImm::imm(76543210)),
        "81FEEAF48F04",
        "cmpl    $76543210, %esi",
    ));
    //
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size16, rdx, RegMemImm::reg(r15)),
        "664439FA",
        "cmpw    %r15w, %dx",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size16, r8, RegMemImm::reg(rcx)),
        "664139C8",
        "cmpw    %cx, %r8w",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size16, rsi, RegMemImm::reg(rcx)),
        "6639CE",
        "cmpw    %cx, %si",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size16,
            rdx,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "663B5763",
        "cmpw    99(%rdi), %dx",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size16,
            r8,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "66443B4763",
        "cmpw    99(%rdi), %r8w",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size16,
            rsi,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "663B7763",
        "cmpw    99(%rdi), %si",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size16, rdx, RegMemImm::imm(23210)),
        "6681FAAA5A",
        "cmpw    $23210, %dx",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size16, r8, RegMemImm::imm(-7654i32 as u32)),
        "664181F81AE2",
        "cmpw    $-7654, %r8w",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size16, rsi, RegMemImm::imm(7654)),
        "6681FEE61D",
        "cmpw    $7654, %si",
    ));
    //
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rdx, RegMemImm::reg(r15)),
        "4438FA",
        "cmpb    %r15b, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r8, RegMemImm::reg(rcx)),
        "4138C8",
        "cmpb    %cl, %r8b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rsi, RegMemImm::reg(rcx)),
        "4038CE",
        "cmpb    %cl, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size8,
            rdx,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "3A5763",
        "cmpb    99(%rdi), %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size8,
            r8,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "443A4763",
        "cmpb    99(%rdi), %r8b",
    ));
    insns.push((
        Inst::cmp_rmi_r(
            OperandSize::Size8,
            rsi,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
        ),
        "403A7763",
        "cmpb    99(%rdi), %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rdx, RegMemImm::imm(70)),
        "80FA46",
        "cmpb    $70, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r8, RegMemImm::imm(-76i32 as u32)),
        "4180F8B4",
        "cmpb    $-76, %r8b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rsi, RegMemImm::imm(76)),
        "4080FE4C",
        "cmpb    $76, %sil",
    ));
    // Extra byte-cases (paranoia!) for cmp_rmi_r for first operand = R
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rbx, RegMemImm::reg(rax)),
        "38C3",
        "cmpb    %al, %bl",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rax, RegMemImm::reg(rbx)),
        "38D8",
        "cmpb    %bl, %al",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rdx, RegMemImm::reg(rcx)),
        "38CA",
        "cmpb    %cl, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rsi, RegMemImm::reg(rcx)),
        "4038CE",
        "cmpb    %cl, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r10, RegMemImm::reg(rcx)),
        "4138CA",
        "cmpb    %cl, %r10b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r14, RegMemImm::reg(rcx)),
        "4138CE",
        "cmpb    %cl, %r14b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rdx, RegMemImm::reg(rbp)),
        "4038EA",
        "cmpb    %bpl, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rsi, RegMemImm::reg(rbp)),
        "4038EE",
        "cmpb    %bpl, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r10, RegMemImm::reg(rbp)),
        "4138EA",
        "cmpb    %bpl, %r10b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r14, RegMemImm::reg(rbp)),
        "4138EE",
        "cmpb    %bpl, %r14b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rdx, RegMemImm::reg(r9)),
        "4438CA",
        "cmpb    %r9b, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rsi, RegMemImm::reg(r9)),
        "4438CE",
        "cmpb    %r9b, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r10, RegMemImm::reg(r9)),
        "4538CA",
        "cmpb    %r9b, %r10b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r14, RegMemImm::reg(r9)),
        "4538CE",
        "cmpb    %r9b, %r14b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rdx, RegMemImm::reg(r13)),
        "4438EA",
        "cmpb    %r13b, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, rsi, RegMemImm::reg(r13)),
        "4438EE",
        "cmpb    %r13b, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r10, RegMemImm::reg(r13)),
        "4538EA",
        "cmpb    %r13b, %r10b",
    ));
    insns.push((
        Inst::cmp_rmi_r(OperandSize::Size8, r14, RegMemImm::reg(r13)),
        "4538EE",
        "cmpb    %r13b, %r14b",
    ));

    // ========================================================
    // SetCC
    insns.push((Inst::setcc(CC::O, w_rsi), "400F90C6", "seto    %sil"));
    insns.push((Inst::setcc(CC::NLE, w_rsi), "400F9FC6", "setnle  %sil"));
    insns.push((Inst::setcc(CC::Z, w_r14), "410F94C6", "setz    %r14b"));
    insns.push((Inst::setcc(CC::LE, w_r14), "410F9EC6", "setle   %r14b"));
    insns.push((Inst::setcc(CC::P, w_r9), "410F9AC1", "setp    %r9b"));
    insns.push((Inst::setcc(CC::NP, w_r8), "410F9BC0", "setnp   %r8b"));

    // ========================================================
    // Cmove
    insns.push((
        Inst::cmove(OperandSize::Size16, CC::O, RegMem::reg(rdi), w_rsi),
        "660F40F7",
        "cmovow  %di, %si, %si",
    ));
    insns.push((
        Inst::cmove(
            OperandSize::Size16,
            CC::NO,
            RegMem::mem(Amode::imm_reg_reg_shift(
                37,
                Gpr::unwrap_new(rdi),
                Gpr::unwrap_new(rsi),
                2,
            )),
            w_r15,
        ),
        "66440F417CB725",
        "cmovnow 37(%rdi,%rsi,4), %r15w, %r15w",
    ));
    insns.push((
        Inst::cmove(OperandSize::Size32, CC::LE, RegMem::reg(rdi), w_rsi),
        "0F4EF7",
        "cmovlel %edi, %esi, %esi",
    ));
    insns.push((
        Inst::cmove(
            OperandSize::Size32,
            CC::NLE,
            RegMem::mem(Amode::imm_reg(0, r15)),
            w_rsi,
        ),
        "410F4F37",
        "cmovnlel 0(%r15), %esi, %esi",
    ));
    insns.push((
        Inst::cmove(OperandSize::Size64, CC::Z, RegMem::reg(rdi), w_r14),
        "4C0F44F7",
        "cmovzq  %rdi, %r14, %r14",
    ));
    insns.push((
        Inst::cmove(
            OperandSize::Size64,
            CC::NZ,
            RegMem::mem(Amode::imm_reg(13, rdi)),
            w_r14,
        ),
        "4C0F45770D",
        "cmovnzq 13(%rdi), %r14, %r14",
    ));

    // ========================================================
    // Push64
    insns.push((Inst::push64(RegMemImm::reg(rdi)), "57", "pushq   %rdi"));
    insns.push((Inst::push64(RegMemImm::reg(r8)), "4150", "pushq   %r8"));
    insns.push((
        Inst::push64(RegMemImm::mem(Amode::imm_reg_reg_shift(
            321,
            Gpr::unwrap_new(rsi),
            Gpr::unwrap_new(rcx),
            3,
        ))),
        "FFB4CE41010000",
        "pushq   321(%rsi,%rcx,8)",
    ));
    insns.push((
        Inst::push64(RegMemImm::mem(Amode::imm_reg_reg_shift(
            321,
            Gpr::unwrap_new(r9),
            Gpr::unwrap_new(rbx),
            2,
        ))),
        "41FFB49941010000",
        "pushq   321(%r9,%rbx,4)",
    ));
    insns.push((Inst::push64(RegMemImm::imm(0)), "6A00", "pushq   $0"));
    insns.push((Inst::push64(RegMemImm::imm(127)), "6A7F", "pushq   $127"));
    insns.push((
        Inst::push64(RegMemImm::imm(128)),
        "6880000000",
        "pushq   $128",
    ));
    insns.push((
        Inst::push64(RegMemImm::imm(0x31415927)),
        "6827594131",
        "pushq   $826366247",
    ));
    insns.push((
        Inst::push64(RegMemImm::imm(-128i32 as u32)),
        "6A80",
        "pushq   $-128",
    ));
    insns.push((
        Inst::push64(RegMemImm::imm(-129i32 as u32)),
        "687FFFFFFF",
        "pushq   $-129",
    ));
    insns.push((
        Inst::push64(RegMemImm::imm(-0x75c4e8a1i32 as u32)),
        "685F173B8A",
        "pushq   $-1975838881",
    ));

    // ========================================================
    // Pop64
    insns.push((Inst::pop64(w_rax), "58", "popq    %rax"));
    insns.push((Inst::pop64(w_rdi), "5F", "popq    %rdi"));
    insns.push((Inst::pop64(w_r8), "4158", "popq    %r8"));
    insns.push((Inst::pop64(w_r15), "415F", "popq    %r15"));

    // ========================================================
    // CallKnown
    insns.push((
        Inst::call_known(Box::new(CallInfo::empty(
            ExternalName::User(UserExternalNameRef::new(0)),
            CallConv::SystemV,
        ))),
        "E800000000",
        "call    User(userextname0)",
    ));

    // ========================================================
    // CallUnknown
    fn call_unknown(rm: RegMem) -> Inst {
        Inst::call_unknown(Box::new(CallInfo::empty(rm, CallConv::SystemV)))
    }

    insns.push((call_unknown(RegMem::reg(rbp)), "FFD5", "call    *%rbp"));
    insns.push((call_unknown(RegMem::reg(r11)), "41FFD3", "call    *%r11"));
    insns.push((
        call_unknown(RegMem::mem(Amode::imm_reg_reg_shift(
            321,
            Gpr::unwrap_new(rsi),
            Gpr::unwrap_new(rcx),
            3,
        ))),
        "FF94CE41010000",
        "call    *321(%rsi,%rcx,8)",
    ));
    insns.push((
        call_unknown(RegMem::mem(Amode::imm_reg_reg_shift(
            321,
            Gpr::unwrap_new(r10),
            Gpr::unwrap_new(rdx),
            2,
        ))),
        "41FF949241010000",
        "call    *321(%r10,%rdx,4)",
    ));

    // ========================================================
    // LoadExtName
    // N.B.: test harness below sets is_pic.
    insns.push((
        Inst::LoadExtName {
            dst: Writable::from_reg(r11),
            name: Box::new(ExternalName::User(UserExternalNameRef::new(0))),
            offset: 0,
            distance: RelocDistance::Far,
        },
        "4C8B1D00000000",
        "load_ext_name userextname0+0, %r11",
    ));
    insns.push((
        Inst::LoadExtName {
            dst: Writable::from_reg(r11),
            name: Box::new(ExternalName::User(UserExternalNameRef::new(0))),
            offset: 0x12345678,
            distance: RelocDistance::Far,
        },
        "4C8B1D000000004981C378563412",
        "load_ext_name userextname0+305419896, %r11",
    ));
    insns.push((
        Inst::LoadExtName {
            dst: Writable::from_reg(r11),
            name: Box::new(ExternalName::User(UserExternalNameRef::new(0))),
            offset: -0x12345678,
            distance: RelocDistance::Far,
        },
        "4C8B1D000000004981EB78563412",
        "load_ext_name userextname0+-305419896, %r11",
    ));

    // ========================================================
    // Ret
    insns.push((Inst::ret(0), "C3", "ret"));
    insns.push((Inst::ret(8), "C20800", "ret 8"));

    // ========================================================
    // JmpKnown skipped for now

    // ========================================================
    // JmpCondSymm isn't a real instruction

    // ========================================================
    // JmpCond skipped for now

    // ========================================================
    // JmpCondCompound isn't a real instruction

    // ========================================================
    // JmpUnknown
    insns.push((Inst::jmp_unknown(RegMem::reg(rbp)), "FFE5", "jmp     *%rbp"));
    insns.push((
        Inst::jmp_unknown(RegMem::reg(r11)),
        "41FFE3",
        "jmp     *%r11",
    ));
    insns.push((
        Inst::jmp_unknown(RegMem::mem(Amode::imm_reg_reg_shift(
            321,
            Gpr::unwrap_new(rsi),
            Gpr::unwrap_new(rcx),
            3,
        ))),
        "FFA4CE41010000",
        "jmp     *321(%rsi,%rcx,8)",
    ));
    insns.push((
        Inst::jmp_unknown(RegMem::mem(Amode::imm_reg_reg_shift(
            321,
            Gpr::unwrap_new(r10),
            Gpr::unwrap_new(rdx),
            2,
        ))),
        "41FFA49241010000",
        "jmp     *321(%r10,%rdx,4)",
    ));

    // ========================================================
    // XMM FMA

    insns.push((
        Inst::xmm_rmr_vex3(AvxOpcode::Vfmadd213ss, RegMem::reg(xmm2), xmm1, w_xmm0),
        "C4E271A9C2",
        "vfmadd213ss %xmm0, %xmm1, %xmm2, %xmm0",
    ));

    insns.push((
        Inst::xmm_rmr_vex3(AvxOpcode::Vfmadd213sd, RegMem::reg(xmm5), xmm4, w_xmm3),
        "C4E2D9A9DD",
        "vfmadd213sd %xmm3, %xmm4, %xmm5, %xmm3",
    ));

    insns.push((
        Inst::xmm_rmr_vex3(AvxOpcode::Vfmadd213ps, RegMem::reg(xmm2), xmm1, w_xmm0),
        "C4E271A8C2",
        "vfmadd213ps %xmm0, %xmm1, %xmm2, %xmm0",
    ));

    insns.push((
        Inst::xmm_rmr_vex3(AvxOpcode::Vfmadd213pd, RegMem::reg(xmm5), xmm4, w_xmm3),
        "C4E2D9A8DD",
        "vfmadd213pd %xmm3, %xmm4, %xmm5, %xmm3",
    ));

    // ========================================================
    // XMM_CMP_RM_R

    insns.push((
        Inst::xmm_cmp_rm_r(SseOpcode::Ucomiss, xmm2, RegMem::reg(xmm1)),
        "0F2ED1",
        "ucomiss %xmm1, %xmm2",
    ));

    insns.push((
        Inst::xmm_cmp_rm_r(SseOpcode::Ucomiss, xmm9, RegMem::reg(xmm0)),
        "440F2EC8",
        "ucomiss %xmm0, %xmm9",
    ));

    insns.push((
        Inst::xmm_cmp_rm_r(SseOpcode::Ucomisd, xmm4, RegMem::reg(xmm13)),
        "66410F2EE5",
        "ucomisd %xmm13, %xmm4",
    ));

    insns.push((
        Inst::xmm_cmp_rm_r(SseOpcode::Ucomisd, xmm12, RegMem::reg(xmm11)),
        "66450F2EE3",
        "ucomisd %xmm11, %xmm12",
    ));

    // ========================================================
    // XMM_RM_R: float binary ops

    insns.push((
        Inst::xmm_rm_r_blend(SseOpcode::Blendvpd, RegMem::reg(xmm15), w_xmm4),
        "66410F3815E7",
        "blendvpd %xmm4, %xmm15, %xmm4",
    ));

    insns.push((
        Inst::xmm_rm_r_blend(SseOpcode::Blendvps, RegMem::reg(xmm2), w_xmm3),
        "660F3814DA",
        "blendvps %xmm3, %xmm2, %xmm3",
    ));

    insns.push((
        Inst::xmm_rm_r_blend(SseOpcode::Pblendvb, RegMem::reg(xmm12), w_xmm13),
        "66450F3810EC",
        "pblendvb %xmm13, %xmm12, %xmm13",
    ));

    // ========================================================
    // XMM_RM_R: Integer Packed

    insns.push((
        Inst::xmm_rm_r_evex(Avx512Opcode::Vpmullq, xmm10, RegMem::reg(xmm14), w_xmm1),
        "62D2AD0840CE",
        "vpmullq %xmm14, %xmm10, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r_evex(Avx512Opcode::Vpsraq, xmm10, RegMem::reg(xmm14), w_xmm1),
        "62D1AD08E2CE",
        "vpsraq  %xmm14, %xmm10, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r_evex3(
            Avx512Opcode::Vpermi2b,
            xmm1,
            xmm10,
            RegMem::reg(xmm14),
            w_xmm1,
        ),
        "62D22D0875CE",
        "vpermi2b %xmm14, %xmm10, %xmm1, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r_evex3(
            Avx512Opcode::Vpermi2b,
            xmm2,
            xmm0,
            RegMem::reg(xmm1),
            w_xmm2,
        ),
        "62F27D0875D1",
        "vpermi2b %xmm1, %xmm0, %xmm2, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmaddwd, RegMem::reg(xmm8), w_xmm1),
        "66410FF5C8",
        "pmaddwd %xmm1, %xmm8, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pshufb, RegMem::reg(xmm11), w_xmm2),
        "66410F3800D3",
        "pshufb  %xmm2, %xmm11, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Packssdw, RegMem::reg(xmm11), w_xmm12),
        "66450F6BE3",
        "packssdw %xmm12, %xmm11, %xmm12",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Packsswb, RegMem::reg(xmm11), w_xmm2),
        "66410F63D3",
        "packsswb %xmm2, %xmm11, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Packusdw, RegMem::reg(xmm13), w_xmm6),
        "66410F382BF5",
        "packusdw %xmm6, %xmm13, %xmm6",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Packuswb, RegMem::reg(xmm9), w_xmm4),
        "66410F67E1",
        "packuswb %xmm4, %xmm9, %xmm4",
    ));

    // ========================================================
    // XMM_MOV: Packed Move

    // XmmUnary: moves and unary float ops

    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Pabsb, RegMem::reg(xmm2), w_xmm1),
        "660F381CCA",
        "pabsb   %xmm2, %xmm1",
    ));
    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Pabsw, RegMem::reg(xmm0), w_xmm0),
        "660F381DC0",
        "pabsw   %xmm0, %xmm0",
    ));
    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Pabsd, RegMem::reg(xmm10), w_xmm11),
        "66450F381EDA",
        "pabsd   %xmm10, %xmm11",
    ));

    insns.push((
        Inst::xmm_unary_rm_r_evex(Avx512Opcode::Vpabsq, RegMem::reg(xmm2), w_xmm8),
        "6272FD081FC2",
        "vpabsq  %xmm2, %xmm8",
    ));

    insns.push((
        Inst::xmm_unary_rm_r_evex(Avx512Opcode::Vcvtudq2ps, RegMem::reg(xmm2), w_xmm8),
        "62717F087AC2",
        "vcvtudq2ps %xmm2, %xmm8",
    ));

    insns.push((
        Inst::xmm_unary_rm_r_evex(Avx512Opcode::Vpopcntb, RegMem::reg(xmm2), w_xmm8),
        "62727D0854C2",
        "vpopcntb %xmm2, %xmm8",
    ));

    // ========================================================
    // XmmRmRImm
    insns.push((
        Inst::xmm_rm_r_imm(
            SseOpcode::Cmppd,
            RegMem::reg(xmm5),
            w_xmm1,
            2,
            OperandSize::Size32,
        ),
        "660FC2CD02",
        "cmppd   $2, %xmm1, %xmm5, %xmm1",
    ));
    insns.push((
        Inst::xmm_rm_r_imm(
            SseOpcode::Cmpps,
            RegMem::reg(xmm15),
            w_xmm7,
            0,
            OperandSize::Size32,
        ),
        "410FC2FF00",
        "cmpps   $0, %xmm7, %xmm15, %xmm7",
    ));
    insns.push((
        Inst::xmm_rm_r_imm(
            SseOpcode::Palignr,
            RegMem::reg(xmm1),
            w_xmm9,
            3,
            OperandSize::Size32,
        ),
        "66440F3A0FC903",
        "palignr $3, %xmm9, %xmm1, %xmm9",
    ));

    insns.push((
        Inst::xmm_rm_r_imm(
            SseOpcode::Shufps,
            RegMem::reg(xmm1),
            w_xmm10,
            136,
            OperandSize::Size32,
        ),
        "440FC6D188",
        "shufps  $136, %xmm10, %xmm1, %xmm10",
    ));

    // ========================================================
    // XmmRmiRVex

    // Standard instruction w/ XmmMemImm::Reg operand.
    insns.push((
        Inst::XmmRmiRVex {
            op: AvxOpcode::Vpmaxub,
            dst: Writable::from_reg(Xmm::unwrap_new(xmm13)),
            src1: Xmm::unwrap_new(xmm1),
            src2: XmmMemImm::unwrap_new(xmm12.into()),
        },
        "C44171DEEC",
        "vpmaxub %xmm1, %xmm12, %xmm13",
    ));

    // Standard instruction w/ XmmMemImm::Mem operand.
    insns.push((
        Inst::XmmRmiRVex {
            op: AvxOpcode::Vpmaxub,
            dst: Writable::from_reg(Xmm::unwrap_new(xmm13)),
            src1: Xmm::unwrap_new(xmm1),
            src2: XmmMemImm::unwrap_new(RegMemImm::Mem {
                addr: Amode::ImmReg {
                    simm32: 10,
                    base: rax,
                    flags: MemFlags::trusted(),
                }
                .into(),
            }),
        },
        "C571DE680A",
        "vpmaxub %xmm1, 10(%rax), %xmm13",
    ));

    // When there's an immediate.
    insns.push((
        Inst::XmmRmiRVex {
            op: AvxOpcode::Vpsrlw,
            dst: Writable::from_reg(Xmm::unwrap_new(xmm13)),
            src1: Xmm::unwrap_new(xmm1),
            src2: XmmMemImm::unwrap_new(RegMemImm::Imm { simm32: 36 }),
        },
        "C59171D124",
        "vpsrlw  %xmm1, $36, %xmm13",
    ));

    // Certain commutative ops get their operands swapped to avoid relying on an
    // extra prefix byte, when possible. Note that these two instructions encode
    // to the same bytes, and are 4-byte encodings rather than 5-byte encodings.
    insns.push((
        Inst::XmmRmiRVex {
            op: AvxOpcode::Vmulsd,
            dst: Writable::from_reg(Xmm::unwrap_new(xmm13)),
            src1: Xmm::unwrap_new(xmm1),
            src2: XmmMemImm::unwrap_new(xmm12.into()),
        },
        "C51B59E9",
        "vmulsd  %xmm1, %xmm12, %xmm13",
    ));
    insns.push((
        Inst::XmmRmiRVex {
            op: AvxOpcode::Vmulsd,
            dst: Writable::from_reg(Xmm::unwrap_new(xmm13)),
            src1: Xmm::unwrap_new(xmm12),
            src2: XmmMemImm::unwrap_new(xmm1.into()),
        },
        "C51B59E9",
        "vmulsd  %xmm12, %xmm1, %xmm13",
    ));

    // ========================================================
    // XmmRmRImmVex
    insns.push((
        Inst::XmmVexPinsr {
            op: AvxOpcode::Vpinsrb,
            dst: Writable::from_reg(Xmm::unwrap_new(xmm13)),
            src1: Xmm::unwrap_new(xmm14),
            src2: GprMem::unwrap_new(RegMem::reg(r15)),
            imm: 2,
        },
        "C4430920EF02",
        "vpinsrb $2, %xmm14, %r15, %xmm13",
    ));

    // ========================================================
    // Pertaining to atomics.
    let am1: SyntheticAmode =
        Amode::imm_reg_reg_shift(321, Gpr::unwrap_new(r10), Gpr::unwrap_new(rdx), 2).into();
    // `am2` doesn't contribute any 1 bits to the rex prefix, so we must use it when testing
    // for retention of the apparently-redundant rex prefix in the 8-bit case.
    let am2: SyntheticAmode =
        Amode::imm_reg_reg_shift(-12345i32, Gpr::unwrap_new(rcx), Gpr::unwrap_new(rsi), 3).into();
    // Use `r9` with a 0 offset.
    let am3: SyntheticAmode = Amode::imm_reg(0, r9).into();

    // A general 8-bit case.
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I8,
            mem: am1,
            replacement: rbx,
            expected: rax,
            dst_old: w_rax,
        },
        "F0410FB09C9241010000",
        "lock cmpxchgb %bl, 321(%r10,%rdx,4), expected=%al, dst_old=%al",
    ));
    // Check redundant rex retention in 8-bit cases.
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I8,
            mem: am2.clone(),
            replacement: rdx,
            expected: rax,
            dst_old: w_rax,
        },
        "F00FB094F1C7CFFFFF",
        "lock cmpxchgb %dl, -12345(%rcx,%rsi,8), expected=%al, dst_old=%al",
    ));
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I8,
            mem: am2.clone(),
            replacement: rsi,
            expected: rax,
            dst_old: w_rax,
        },
        "F0400FB0B4F1C7CFFFFF",
        "lock cmpxchgb %sil, -12345(%rcx,%rsi,8), expected=%al, dst_old=%al",
    ));
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I8,
            mem: am2.clone(),
            replacement: r10,
            expected: rax,
            dst_old: w_rax,
        },
        "F0440FB094F1C7CFFFFF",
        "lock cmpxchgb %r10b, -12345(%rcx,%rsi,8), expected=%al, dst_old=%al",
    ));
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I8,
            mem: am2.clone(),
            replacement: r15,
            expected: rax,
            dst_old: w_rax,
        },
        "F0440FB0BCF1C7CFFFFF",
        "lock cmpxchgb %r15b, -12345(%rcx,%rsi,8), expected=%al, dst_old=%al",
    ));
    // 16 bit cases
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I16,
            mem: am2.clone(),
            replacement: rsi,
            expected: rax,
            dst_old: w_rax,
        },
        "66F00FB1B4F1C7CFFFFF",
        "lock cmpxchgw %si, -12345(%rcx,%rsi,8), expected=%ax, dst_old=%ax",
    ));
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I16,
            mem: am2.clone(),
            replacement: r10,
            expected: rax,
            dst_old: w_rax,
        },
        "66F0440FB194F1C7CFFFFF",
        "lock cmpxchgw %r10w, -12345(%rcx,%rsi,8), expected=%ax, dst_old=%ax",
    ));
    // 32 bit cases
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I32,
            mem: am2.clone(),
            replacement: rsi,
            expected: rax,
            dst_old: w_rax,
        },
        "F00FB1B4F1C7CFFFFF",
        "lock cmpxchgl %esi, -12345(%rcx,%rsi,8), expected=%eax, dst_old=%eax",
    ));
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I32,
            mem: am2.clone(),
            replacement: r10,
            expected: rax,
            dst_old: w_rax,
        },
        "F0440FB194F1C7CFFFFF",
        "lock cmpxchgl %r10d, -12345(%rcx,%rsi,8), expected=%eax, dst_old=%eax",
    ));
    // 64 bit cases
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I64,
            mem: am2.clone(),
            replacement: rsi,
            expected: rax,
            dst_old: w_rax,
        },
        "F0480FB1B4F1C7CFFFFF",
        "lock cmpxchgq %rsi, -12345(%rcx,%rsi,8), expected=%rax, dst_old=%rax",
    ));
    insns.push((
        Inst::LockCmpxchg {
            ty: types::I64,
            mem: am2.clone(),
            replacement: r10,
            expected: rax,
            dst_old: w_rax,
        },
        "F04C0FB194F1C7CFFFFF",
        "lock cmpxchgq %r10, -12345(%rcx,%rsi,8), expected=%rax, dst_old=%rax",
    ));

    insns.push((
        Inst::LockCmpxchg16b {
            mem: Box::new(am2.clone()),
            replacement_low: rbx,
            replacement_high: rcx,
            expected_low: rax,
            expected_high: rdx,
            dst_old_low: w_rax,
            dst_old_high: w_rdx,
        },
        "F0480FC78CF1C7CFFFFF",
        "lock cmpxchg16b -12345(%rcx,%rsi,8), replacement=%rcx:%rbx, expected=%rdx:%rax, dst_old=%rdx:%rax",
    ));

    // LockXadd
    insns.push((
        Inst::LockXadd {
            size: OperandSize::Size64,
            operand: r10,
            mem: am3.clone(),
            dst_old: w_r10,
        },
        "F04D0FC111",
        "lock xaddq %r10, 0(%r9), dst_old=%r10",
    ));
    insns.push((
        Inst::LockXadd {
            size: OperandSize::Size32,
            operand: r11,
            mem: am3.clone(),
            dst_old: w_r11,
        },
        "F0450FC119",
        "lock xaddl %r11d, 0(%r9), dst_old=%r11d",
    ));
    insns.push((
        Inst::LockXadd {
            size: OperandSize::Size16,
            operand: r12,
            mem: am3.clone(),
            dst_old: w_r12,
        },
        "66F0450FC121",
        "lock xaddw %r12w, 0(%r9), dst_old=%r12w",
    ));
    insns.push((
        Inst::LockXadd {
            size: OperandSize::Size8,
            operand: r13,
            mem: am3.clone(),
            dst_old: w_r13,
        },
        "F0450FC029",
        "lock xaddb %r13b, 0(%r9), dst_old=%r13b",
    ));

    // Xchg
    insns.push((
        Inst::Xchg {
            size: OperandSize::Size64,
            operand: r10,
            mem: am3.clone(),
            dst_old: w_r10,
        },
        "4D8711",
        "xchgq %r10, 0(%r9), dst_old=%r10",
    ));
    insns.push((
        Inst::Xchg {
            size: OperandSize::Size32,
            operand: r11,
            mem: am3.clone(),
            dst_old: w_r11,
        },
        "458719",
        "xchgl %r11d, 0(%r9), dst_old=%r11d",
    ));
    insns.push((
        Inst::Xchg {
            size: OperandSize::Size16,
            operand: r12,
            mem: am3.clone(),
            dst_old: w_r12,
        },
        "66458721",
        "xchgw %r12w, 0(%r9), dst_old=%r12w",
    ));
    insns.push((
        Inst::Xchg {
            size: OperandSize::Size8,
            operand: r13,
            mem: am3.clone(),
            dst_old: w_r13,
        },
        "458629",
        "xchgb %r13b, 0(%r9), dst_old=%r13b",
    ));

    // AtomicRmwSeq
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I8,
            op: AtomicRmwSeqOp::Or,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax,
        },
        "490FB6014989C34D0BDAF0450FB0190F85EFFFFFFF",
        "atomically { 8_bits_at_[%r9] Or= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }",
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I16,
            op: AtomicRmwSeqOp::And,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "490FB7014989C34D23DA66F0450FB1190F85EEFFFFFF",
        "atomically { 16_bits_at_[%r9] And= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I32,
            op: AtomicRmwSeqOp::Nand,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "418B014989C34D23DA49F7D3F0450FB1190F85ECFFFFFF",
        "atomically { 32_bits_at_[%r9] Nand= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I32,
            op: AtomicRmwSeqOp::Umin,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "418B014989C34539DA4D0F46DAF0450FB1190F85EBFFFFFF",
        "atomically { 32_bits_at_[%r9] Umin= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I64,
            op: AtomicRmwSeqOp::Smax,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "498B014989C34D39DA4D0F4DDAF04D0FB1190F85EBFFFFFF",
        "atomically { 64_bits_at_[%r9] Smax= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));

    // Atomic128RmwSeq
    insns.push((
        Inst::Atomic128RmwSeq {
            op: Atomic128RmwSeqOp::Or,
            mem: Box::new(am3.clone()),
            operand_low: r10,
            operand_high: r11,
            temp_low: w_rbx,
            temp_high: w_rcx,
            dst_old_low: w_rax,
            dst_old_high: w_rdx,
        },
        "498B01498B51084889C34889D1490BDA490BCBF0490FC7090F85E9FFFFFF",
        "atomically { %rdx:%rax = 0(%r9); %rcx:%rbx = %rdx:%rax Or %r11:%r10; 0(%r9) = %rcx:%rbx }",
    ));
    insns.push((
        Inst::Atomic128RmwSeq {
            op: Atomic128RmwSeqOp::And,
            mem: Box::new(am3.clone()),
            operand_low: r10,
            operand_high: r11,
            temp_low: w_rbx,
            temp_high: w_rcx,
            dst_old_low: w_rax,
            dst_old_high: w_rdx,
        },
        "498B01498B51084889C34889D14923DA4923CBF0490FC7090F85E9FFFFFF",
        "atomically { %rdx:%rax = 0(%r9); %rcx:%rbx = %rdx:%rax And %r11:%r10; 0(%r9) = %rcx:%rbx }"
    ));
    insns.push((
        Inst::Atomic128RmwSeq {
            op: Atomic128RmwSeqOp::Umin,
            mem: Box::new(am3.clone()),
            operand_low: r10,
            operand_high: r11,
            temp_low: w_rbx,
            temp_high: w_rcx,
            dst_old_low: w_rax,
            dst_old_high: w_rdx,
        },
        "498B01498B51084889C34889D14C39D3491BCB4889D1490F43DA490F43CBF0490FC7090F85DEFFFFFF",
        "atomically { %rdx:%rax = 0(%r9); %rcx:%rbx = %rdx:%rax Umin %r11:%r10; 0(%r9) = %rcx:%rbx }"
    ));
    insns.push((
        Inst::Atomic128RmwSeq {
            op: Atomic128RmwSeqOp::Add,
            mem: Box::new(am3.clone()),
            operand_low: r10,
            operand_high: r11,
            temp_low: w_rbx,
            temp_high: w_rcx,
            dst_old_low: w_rax,
            dst_old_high: w_rdx,
        },
        "498B01498B51084889C34889D14903DA4913CBF0490FC7090F85E9FFFFFF",
        "atomically { %rdx:%rax = 0(%r9); %rcx:%rbx = %rdx:%rax Add %r11:%r10; 0(%r9) = %rcx:%rbx }"
    ));
    insns.push((
        Inst::Atomic128XchgSeq {
            mem: am3.clone(),
            operand_low: rbx,
            operand_high: rcx,
            dst_old_low: w_rax,
            dst_old_high: w_rdx,
        },
        "498B01498B5108F0490FC7090F85F5FFFFFF",
        "atomically { %rdx:%rax = 0(%r9); 0(%r9) = %rcx:%rbx }",
    ));

    // Fence
    insns.push((
        Inst::Fence {
            kind: FenceKind::MFence,
        },
        "0FAEF0",
        "mfence",
    ));
    insns.push((
        Inst::Fence {
            kind: FenceKind::LFence,
        },
        "0FAEE8",
        "lfence",
    ));
    insns.push((
        Inst::Fence {
            kind: FenceKind::SFence,
        },
        "0FAEF8",
        "sfence",
    ));

    // ========================================================
    // Misc instructions.

    insns.push((Inst::Hlt, "CC", "hlt"));

    let trap_code = TrapCode::INTEGER_OVERFLOW;
    insns.push((Inst::Ud2 { trap_code }, "0F0B", "ud2 int_ovf"));

    insns.push((
        Inst::ElfTlsGetAddr {
            symbol: ExternalName::User(UserExternalNameRef::new(0)),
            dst: WritableGpr::from_writable_reg(w_rax).unwrap(),
        },
        "66488D3D00000000666648E800000000",
        "%rax = elf_tls_get_addr User(userextname0)",
    ));

    insns.push((
        Inst::MachOTlsGetAddr {
            symbol: ExternalName::User(UserExternalNameRef::new(0)),
            dst: WritableGpr::from_writable_reg(w_rax).unwrap(),
        },
        "488B3D00000000FF17",
        "%rax = macho_tls_get_addr User(userextname0)",
    ));

    insns.push((
        Inst::CoffTlsGetAddr {
            symbol: ExternalName::User(UserExternalNameRef::new(0)),
            dst: WritableGpr::from_writable_reg(w_rax).unwrap(),
            tmp: WritableGpr::from_writable_reg(w_rcx).unwrap(),
        },
        "8B050000000065488B0C2558000000488B04C1488D8000000000",
        "%rax = coff_tls_get_addr User(userextname0)",
    ));

    // ========================================================
    // Actually run the tests!
    let ctrl_plane = &mut Default::default();
    let constants = Default::default();
    let mut flag_builder = settings::builder();
    flag_builder.enable("is_pic").unwrap();
    let flags = settings::Flags::new(flag_builder);

    use crate::settings::Configurable;
    let mut isa_flag_builder = x64::settings::builder();
    isa_flag_builder.enable("has_cmpxchg16b").unwrap();
    isa_flag_builder.enable("has_ssse3").unwrap();
    isa_flag_builder.enable("has_sse41").unwrap();
    isa_flag_builder.enable("has_fma").unwrap();
    isa_flag_builder.enable("has_avx").unwrap();
    isa_flag_builder.enable("has_avx512bitalg").unwrap();
    isa_flag_builder.enable("has_avx512dq").unwrap();
    isa_flag_builder.enable("has_avx512f").unwrap();
    isa_flag_builder.enable("has_avx512vbmi").unwrap();
    isa_flag_builder.enable("has_avx512vl").unwrap();
    let isa_flags = x64::settings::Flags::new(&flags, &isa_flag_builder);

    let emit_info = EmitInfo::new(flags, isa_flags);
    for (insn, expected_encoding, expected_printing) in insns {
        // Check the printed text is as expected.
        let actual_printing = insn.pretty_print_inst(&mut Default::default());
        assert_eq!(expected_printing, actual_printing.trim());
        let mut buffer = MachBuffer::new();

        insn.emit(&mut buffer, &emit_info, &mut Default::default());

        // Allow one label just after the instruction (so the offset is 0).
        let label = buffer.get_label();
        buffer.bind_label(label, ctrl_plane);

        let buffer = buffer.finish(&constants, ctrl_plane);
        let actual_encoding = &buffer.stringify_code_bytes();
        assert_eq!(expected_encoding, actual_encoding, "{expected_printing}");
    }
}

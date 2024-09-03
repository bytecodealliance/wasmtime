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
use alloc::vec::Vec;
use cranelift_entity::EntityRef as _;

impl Inst {
    fn neg(size: OperandSize, src: Writable<Reg>) -> Inst {
        debug_assert_eq!(src.to_reg().class(), RegClass::Int);
        Inst::Neg {
            size,
            src: Gpr::unwrap_new(src.to_reg()),
            dst: WritableGpr::from_writable_reg(src).unwrap(),
        }
    }

    fn xmm_unary_rm_r_imm(op: SseOpcode, src: RegMem, dst: Writable<Reg>, imm: u8) -> Inst {
        src.assert_regclass_is(RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmUnaryRmRImm {
            op,
            src: XmmMemAligned::unwrap_new(src),
            imm,
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    fn xmm_unary_rm_r_evex(op: Avx512Opcode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmUnaryRmREvex {
            op,
            src: XmmMem::unwrap_new(src),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    fn xmm_rmi_reg(opcode: SseOpcode, src: RegMemImm, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmRmiReg {
            opcode,
            src1: Xmm::unwrap_new(dst.to_reg()),
            src2: XmmMemAlignedImm::unwrap_new(src),
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

    // TODO Can be replaced by `Inst::move` (high-level) and `Inst::unary_rm_r` (low-level)
    fn xmm_mov(op: SseOpcode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::Float);
        debug_assert!(dst.to_reg().class() == RegClass::Float);
        Inst::XmmUnaryRmR {
            op,
            src: XmmMemAligned::unwrap_new(src),
            dst: WritableXmm::from_writable_reg(dst).unwrap(),
        }
    }

    fn setcc(cc: CC, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        let dst = WritableGpr::from_writable_reg(dst).unwrap();
        Inst::Setcc { cc, dst }
    }

    fn bswap(size: OperandSize, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().class() == RegClass::Int);
        let src = Gpr::unwrap_new(dst.to_reg());
        let dst = WritableGpr::from_writable_reg(dst).unwrap();
        Inst::Bswap { size, src, dst }
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
    let _w_r10 = Writable::<Reg>::from_reg(r10);
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
    let w_xmm5 = Writable::<Reg>::from_reg(xmm5);
    let w_xmm6 = Writable::<Reg>::from_reg(xmm6);
    let w_xmm7 = Writable::<Reg>::from_reg(xmm7);
    let w_xmm8 = Writable::<Reg>::from_reg(xmm8);
    let w_xmm9 = Writable::<Reg>::from_reg(xmm9);
    let w_xmm10 = Writable::<Reg>::from_reg(xmm10);
    let w_xmm11 = Writable::<Reg>::from_reg(xmm11);
    let w_xmm12 = Writable::<Reg>::from_reg(xmm12);
    let w_xmm13 = Writable::<Reg>::from_reg(xmm13);
    let w_xmm14 = Writable::<Reg>::from_reg(xmm14);
    let w_xmm15 = Writable::<Reg>::from_reg(xmm15);

    let mut insns = Vec::<(Inst, &str, &str)>::new();

    // ========================================================
    // Cases aimed at checking Addr-esses: IR (Imm + Reg)
    //
    // These are just a bunch of loads with all supported (by the emitter)
    // permutations of address formats.
    //
    // Addr_IR, offset zero
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, rax), w_rdi),
        "488B38",
        "movq    0(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, rbx), w_rdi),
        "488B3B",
        "movq    0(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, rcx), w_rdi),
        "488B39",
        "movq    0(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, rdx), w_rdi),
        "488B3A",
        "movq    0(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, rbp), w_rdi),
        "488B7D00",
        "movq    0(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, rsp), w_rdi),
        "488B3C24",
        "movq    0(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, rsi), w_rdi),
        "488B3E",
        "movq    0(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, rdi), w_rdi),
        "488B3F",
        "movq    0(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, r8), w_rdi),
        "498B38",
        "movq    0(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, r9), w_rdi),
        "498B39",
        "movq    0(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, r10), w_rdi),
        "498B3A",
        "movq    0(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, r11), w_rdi),
        "498B3B",
        "movq    0(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, r12), w_rdi),
        "498B3C24",
        "movq    0(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, r13), w_rdi),
        "498B7D00",
        "movq    0(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, r14), w_rdi),
        "498B3E",
        "movq    0(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0, r15), w_rdi),
        "498B3F",
        "movq    0(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset max simm8
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, rax), w_rdi),
        "488B787F",
        "movq    127(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, rbx), w_rdi),
        "488B7B7F",
        "movq    127(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, rcx), w_rdi),
        "488B797F",
        "movq    127(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, rdx), w_rdi),
        "488B7A7F",
        "movq    127(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, rbp), w_rdi),
        "488B7D7F",
        "movq    127(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, rsp), w_rdi),
        "488B7C247F",
        "movq    127(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, rsi), w_rdi),
        "488B7E7F",
        "movq    127(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, rdi), w_rdi),
        "488B7F7F",
        "movq    127(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, r8), w_rdi),
        "498B787F",
        "movq    127(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, r9), w_rdi),
        "498B797F",
        "movq    127(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, r10), w_rdi),
        "498B7A7F",
        "movq    127(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, r11), w_rdi),
        "498B7B7F",
        "movq    127(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, r12), w_rdi),
        "498B7C247F",
        "movq    127(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, r13), w_rdi),
        "498B7D7F",
        "movq    127(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, r14), w_rdi),
        "498B7E7F",
        "movq    127(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(127, r15), w_rdi),
        "498B7F7F",
        "movq    127(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset min simm8
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, rax), w_rdi),
        "488B7880",
        "movq    -128(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, rbx), w_rdi),
        "488B7B80",
        "movq    -128(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, rcx), w_rdi),
        "488B7980",
        "movq    -128(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, rdx), w_rdi),
        "488B7A80",
        "movq    -128(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, rbp), w_rdi),
        "488B7D80",
        "movq    -128(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, rsp), w_rdi),
        "488B7C2480",
        "movq    -128(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, rsi), w_rdi),
        "488B7E80",
        "movq    -128(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, rdi), w_rdi),
        "488B7F80",
        "movq    -128(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, r8), w_rdi),
        "498B7880",
        "movq    -128(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, r9), w_rdi),
        "498B7980",
        "movq    -128(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, r10), w_rdi),
        "498B7A80",
        "movq    -128(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, r11), w_rdi),
        "498B7B80",
        "movq    -128(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, r12), w_rdi),
        "498B7C2480",
        "movq    -128(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, r13), w_rdi),
        "498B7D80",
        "movq    -128(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, r14), w_rdi),
        "498B7E80",
        "movq    -128(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-128, r15), w_rdi),
        "498B7F80",
        "movq    -128(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset smallest positive simm32
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, rax), w_rdi),
        "488BB880000000",
        "movq    128(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, rbx), w_rdi),
        "488BBB80000000",
        "movq    128(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, rcx), w_rdi),
        "488BB980000000",
        "movq    128(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, rdx), w_rdi),
        "488BBA80000000",
        "movq    128(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, rbp), w_rdi),
        "488BBD80000000",
        "movq    128(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, rsp), w_rdi),
        "488BBC2480000000",
        "movq    128(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, rsi), w_rdi),
        "488BBE80000000",
        "movq    128(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, rdi), w_rdi),
        "488BBF80000000",
        "movq    128(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, r8), w_rdi),
        "498BB880000000",
        "movq    128(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, r9), w_rdi),
        "498BB980000000",
        "movq    128(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, r10), w_rdi),
        "498BBA80000000",
        "movq    128(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, r11), w_rdi),
        "498BBB80000000",
        "movq    128(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, r12), w_rdi),
        "498BBC2480000000",
        "movq    128(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, r13), w_rdi),
        "498BBD80000000",
        "movq    128(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, r14), w_rdi),
        "498BBE80000000",
        "movq    128(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(128, r15), w_rdi),
        "498BBF80000000",
        "movq    128(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset smallest negative simm32
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, rax), w_rdi),
        "488BB87FFFFFFF",
        "movq    -129(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, rbx), w_rdi),
        "488BBB7FFFFFFF",
        "movq    -129(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, rcx), w_rdi),
        "488BB97FFFFFFF",
        "movq    -129(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, rdx), w_rdi),
        "488BBA7FFFFFFF",
        "movq    -129(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, rbp), w_rdi),
        "488BBD7FFFFFFF",
        "movq    -129(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, rsp), w_rdi),
        "488BBC247FFFFFFF",
        "movq    -129(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, rsi), w_rdi),
        "488BBE7FFFFFFF",
        "movq    -129(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, rdi), w_rdi),
        "488BBF7FFFFFFF",
        "movq    -129(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, r8), w_rdi),
        "498BB87FFFFFFF",
        "movq    -129(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, r9), w_rdi),
        "498BB97FFFFFFF",
        "movq    -129(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, r10), w_rdi),
        "498BBA7FFFFFFF",
        "movq    -129(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, r11), w_rdi),
        "498BBB7FFFFFFF",
        "movq    -129(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, r12), w_rdi),
        "498BBC247FFFFFFF",
        "movq    -129(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, r13), w_rdi),
        "498BBD7FFFFFFF",
        "movq    -129(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, r14), w_rdi),
        "498BBE7FFFFFFF",
        "movq    -129(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-129i32, r15), w_rdi),
        "498BBF7FFFFFFF",
        "movq    -129(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset large positive simm32
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, rax), w_rdi),
        "488BB877207317",
        "movq    393420919(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, rbx), w_rdi),
        "488BBB77207317",
        "movq    393420919(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, rcx), w_rdi),
        "488BB977207317",
        "movq    393420919(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, rdx), w_rdi),
        "488BBA77207317",
        "movq    393420919(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, rbp), w_rdi),
        "488BBD77207317",
        "movq    393420919(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, rsp), w_rdi),
        "488BBC2477207317",
        "movq    393420919(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, rsi), w_rdi),
        "488BBE77207317",
        "movq    393420919(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, rdi), w_rdi),
        "488BBF77207317",
        "movq    393420919(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, r8), w_rdi),
        "498BB877207317",
        "movq    393420919(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, r9), w_rdi),
        "498BB977207317",
        "movq    393420919(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, r10), w_rdi),
        "498BBA77207317",
        "movq    393420919(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, r11), w_rdi),
        "498BBB77207317",
        "movq    393420919(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, r12), w_rdi),
        "498BBC2477207317",
        "movq    393420919(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, r13), w_rdi),
        "498BBD77207317",
        "movq    393420919(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, r14), w_rdi),
        "498BBE77207317",
        "movq    393420919(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(0x17732077, r15), w_rdi),
        "498BBF77207317",
        "movq    393420919(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset large negative simm32
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, rax), w_rdi),
        "488BB8D9A6BECE",
        "movq    -826366247(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, rbx), w_rdi),
        "488BBBD9A6BECE",
        "movq    -826366247(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, rcx), w_rdi),
        "488BB9D9A6BECE",
        "movq    -826366247(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, rdx), w_rdi),
        "488BBAD9A6BECE",
        "movq    -826366247(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, rbp), w_rdi),
        "488BBDD9A6BECE",
        "movq    -826366247(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, rsp), w_rdi),
        "488BBC24D9A6BECE",
        "movq    -826366247(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, rsi), w_rdi),
        "488BBED9A6BECE",
        "movq    -826366247(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, rdi), w_rdi),
        "488BBFD9A6BECE",
        "movq    -826366247(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, r8), w_rdi),
        "498BB8D9A6BECE",
        "movq    -826366247(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, r9), w_rdi),
        "498BB9D9A6BECE",
        "movq    -826366247(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, r10), w_rdi),
        "498BBAD9A6BECE",
        "movq    -826366247(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, r11), w_rdi),
        "498BBBD9A6BECE",
        "movq    -826366247(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, r12), w_rdi),
        "498BBC24D9A6BECE",
        "movq    -826366247(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, r13), w_rdi),
        "498BBDD9A6BECE",
        "movq    -826366247(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, r14), w_rdi),
        "498BBED9A6BECE",
        "movq    -826366247(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Amode::imm_reg(-0x31415927i32, r15), w_rdi),
        "498BBFD9A6BECE",
        "movq    -826366247(%r15), %rdi",
    ));

    // ========================================================
    // Cases aimed at checking Addr-esses: IRRS (Imm + Reg + (Reg << Shift))
    // Note these don't check the case where the index reg is RSP, since we
    // don't encode any of those.
    //
    // Addr_IRRS, offset max simm8
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(127, Gpr::unwrap_new(rax), Gpr::unwrap_new(rax), 0),
            w_r11,
        ),
        "4C8B5C007F",
        "movq    127(%rax,%rax,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(127, Gpr::unwrap_new(rdi), Gpr::unwrap_new(rax), 1),
            w_r11,
        ),
        "4C8B5C477F",
        "movq    127(%rdi,%rax,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(127, Gpr::unwrap_new(r8), Gpr::unwrap_new(rax), 2),
            w_r11,
        ),
        "4D8B5C807F",
        "movq    127(%r8,%rax,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(127, Gpr::unwrap_new(r15), Gpr::unwrap_new(rax), 3),
            w_r11,
        ),
        "4D8B5CC77F",
        "movq    127(%r15,%rax,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(127, Gpr::unwrap_new(rax), Gpr::unwrap_new(rdi), 3),
            w_r11,
        ),
        "4C8B5CF87F",
        "movq    127(%rax,%rdi,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(127, Gpr::unwrap_new(rdi), Gpr::unwrap_new(rdi), 2),
            w_r11,
        ),
        "4C8B5CBF7F",
        "movq    127(%rdi,%rdi,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(127, Gpr::unwrap_new(r8), Gpr::unwrap_new(rdi), 1),
            w_r11,
        ),
        "4D8B5C787F",
        "movq    127(%r8,%rdi,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(127, Gpr::unwrap_new(r15), Gpr::unwrap_new(rdi), 0),
            w_r11,
        ),
        "4D8B5C3F7F",
        "movq    127(%r15,%rdi,1), %r11",
    ));

    // ========================================================
    // Addr_IRRS, offset min simm8
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-128i32, Gpr::unwrap_new(rax), Gpr::unwrap_new(r8), 2),
            w_r11,
        ),
        "4E8B5C8080",
        "movq    -128(%rax,%r8,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-128i32, Gpr::unwrap_new(rdi), Gpr::unwrap_new(r8), 3),
            w_r11,
        ),
        "4E8B5CC780",
        "movq    -128(%rdi,%r8,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-128i32, Gpr::unwrap_new(r8), Gpr::unwrap_new(r8), 0),
            w_r11,
        ),
        "4F8B5C0080",
        "movq    -128(%r8,%r8,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-128i32, Gpr::unwrap_new(r15), Gpr::unwrap_new(r8), 1),
            w_r11,
        ),
        "4F8B5C4780",
        "movq    -128(%r15,%r8,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-128i32, Gpr::unwrap_new(rax), Gpr::unwrap_new(r15), 1),
            w_r11,
        ),
        "4E8B5C7880",
        "movq    -128(%rax,%r15,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-128i32, Gpr::unwrap_new(rdi), Gpr::unwrap_new(r15), 0),
            w_r11,
        ),
        "4E8B5C3F80",
        "movq    -128(%rdi,%r15,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-128i32, Gpr::unwrap_new(r8), Gpr::unwrap_new(r15), 3),
            w_r11,
        ),
        "4F8B5CF880",
        "movq    -128(%r8,%r15,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-128i32, Gpr::unwrap_new(r15), Gpr::unwrap_new(r15), 2),
            w_r11,
        ),
        "4F8B5CBF80",
        "movq    -128(%r15,%r15,4), %r11",
    ));

    // ========================================================
    // Addr_IRRS, offset large positive simm32
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(0x4f6625be, Gpr::unwrap_new(rax), Gpr::unwrap_new(rax), 0),
            w_r11,
        ),
        "4C8B9C00BE25664F",
        "movq    1332094398(%rax,%rax,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(0x4f6625be, Gpr::unwrap_new(rdi), Gpr::unwrap_new(rax), 1),
            w_r11,
        ),
        "4C8B9C47BE25664F",
        "movq    1332094398(%rdi,%rax,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(0x4f6625be, Gpr::unwrap_new(r8), Gpr::unwrap_new(rax), 2),
            w_r11,
        ),
        "4D8B9C80BE25664F",
        "movq    1332094398(%r8,%rax,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(0x4f6625be, Gpr::unwrap_new(r15), Gpr::unwrap_new(rax), 3),
            w_r11,
        ),
        "4D8B9CC7BE25664F",
        "movq    1332094398(%r15,%rax,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(0x4f6625be, Gpr::unwrap_new(rax), Gpr::unwrap_new(rdi), 3),
            w_r11,
        ),
        "4C8B9CF8BE25664F",
        "movq    1332094398(%rax,%rdi,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(0x4f6625be, Gpr::unwrap_new(rdi), Gpr::unwrap_new(rdi), 2),
            w_r11,
        ),
        "4C8B9CBFBE25664F",
        "movq    1332094398(%rdi,%rdi,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(0x4f6625be, Gpr::unwrap_new(r8), Gpr::unwrap_new(rdi), 1),
            w_r11,
        ),
        "4D8B9C78BE25664F",
        "movq    1332094398(%r8,%rdi,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(0x4f6625be, Gpr::unwrap_new(r15), Gpr::unwrap_new(rdi), 0),
            w_r11,
        ),
        "4D8B9C3FBE25664F",
        "movq    1332094398(%r15,%rdi,1), %r11",
    ));

    // ========================================================
    // Addr_IRRS, offset large negative simm32
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-0x264d1690i32, Gpr::unwrap_new(rax), Gpr::unwrap_new(r8), 2),
            w_r11,
        ),
        "4E8B9C8070E9B2D9",
        "movq    -642586256(%rax,%r8,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-0x264d1690i32, Gpr::unwrap_new(rdi), Gpr::unwrap_new(r8), 3),
            w_r11,
        ),
        "4E8B9CC770E9B2D9",
        "movq    -642586256(%rdi,%r8,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-0x264d1690i32, Gpr::unwrap_new(r8), Gpr::unwrap_new(r8), 0),
            w_r11,
        ),
        "4F8B9C0070E9B2D9",
        "movq    -642586256(%r8,%r8,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-0x264d1690i32, Gpr::unwrap_new(r15), Gpr::unwrap_new(r8), 1),
            w_r11,
        ),
        "4F8B9C4770E9B2D9",
        "movq    -642586256(%r15,%r8,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(
                -0x264d1690i32,
                Gpr::unwrap_new(rax),
                Gpr::unwrap_new(r15),
                1,
            ),
            w_r11,
        ),
        "4E8B9C7870E9B2D9",
        "movq    -642586256(%rax,%r15,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(
                -0x264d1690i32,
                Gpr::unwrap_new(rdi),
                Gpr::unwrap_new(r15),
                0,
            ),
            w_r11,
        ),
        "4E8B9C3F70E9B2D9",
        "movq    -642586256(%rdi,%r15,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(-0x264d1690i32, Gpr::unwrap_new(r8), Gpr::unwrap_new(r15), 3),
            w_r11,
        ),
        "4F8B9CF870E9B2D9",
        "movq    -642586256(%r8,%r15,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(
                -0x264d1690i32,
                Gpr::unwrap_new(r15),
                Gpr::unwrap_new(r15),
                2,
            ),
            w_r11,
        ),
        "4F8B9CBF70E9B2D9",
        "movq    -642586256(%r15,%r15,4), %r11",
    ));

    // End of test cases for Addr
    // ========================================================

    // ========================================================
    // General tests for each insn.  Don't forget to follow the
    // guidelines commented just prior to `fn x64_emit`.
    //
    // Alu_RMI_R
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Add,
            RegMemImm::reg(r15),
            w_rdx,
        ),
        "4C01FA",
        "addq    %rdx, %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::reg(rcx),
            w_r8,
        ),
        "4101C8",
        "addl    %r8d, %ecx, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::reg(rcx),
            w_rsi,
        ),
        "01CE",
        "addl    %esi, %ecx, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Add,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
            w_rdx,
        ),
        "48035763",
        "addq    %rdx, 99(%rdi), %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
            w_r8,
        ),
        "44034763",
        "addl    %r8d, 99(%rdi), %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
            w_rsi,
        ),
        "037763",
        "addl    %esi, 99(%rdi), %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Add,
            RegMemImm::imm(-127i32 as u32),
            w_rdx,
        ),
        "4883C281",
        "addq    %rdx, $-127, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Add,
            RegMemImm::imm(-129i32 as u32),
            w_rdx,
        ),
        "4881C27FFFFFFF",
        "addq    %rdx, $-129, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Add,
            RegMemImm::imm(76543210),
            w_rdx,
        ),
        "4881C2EAF48F04",
        "addq    %rdx, $76543210, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::imm(-127i32 as u32),
            w_r8,
        ),
        "4183C081",
        "addl    %r8d, $-127, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::imm(-129i32 as u32),
            w_r8,
        ),
        "4181C07FFFFFFF",
        "addl    %r8d, $-129, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::imm(-76543210i32 as u32),
            w_r8,
        ),
        "4181C0160B70FB",
        "addl    %r8d, $-76543210, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::imm(-127i32 as u32),
            w_rsi,
        ),
        "83C681",
        "addl    %esi, $-127, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::imm(-129i32 as u32),
            w_rsi,
        ),
        "81C67FFFFFFF",
        "addl    %esi, $-129, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Add,
            RegMemImm::imm(76543210),
            w_rsi,
        ),
        "81C6EAF48F04",
        "addl    %esi, $76543210, %esi",
    ));
    // This is pretty feeble
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Sub,
            RegMemImm::reg(r15),
            w_rdx,
        ),
        "4C29FA",
        "subq    %rdx, %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::And,
            RegMemImm::reg(r15),
            w_rdx,
        ),
        "4C21FA",
        "andq    %rdx, %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Or,
            RegMemImm::reg(r15),
            w_rdx,
        ),
        "4C09FA",
        "orq     %rdx, %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Xor,
            RegMemImm::reg(r15),
            w_rdx,
        ),
        "4C31FA",
        "xorq    %rdx, %r15, %rdx",
    ));

    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size16,
            AluRmiROpcode::Add,
            RegMemImm::reg(rax),
            w_rdx,
        ),
        "6601C2",
        "addw    %dx, %ax, %dx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size16,
            AluRmiROpcode::Add,
            RegMemImm::imm(10),
            w_rdx,
        ),
        "6683C20A",
        "addw    %dx, $10, %dx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size16,
            AluRmiROpcode::Add,
            RegMemImm::imm(-512i32 as u32),
            w_rdx,
        ),
        "6681C200FE",
        "addw    %dx, $-512, %dx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size16,
            AluRmiROpcode::Sub,
            RegMemImm::reg(rax),
            w_r12,
        ),
        "664129C4",
        "subw    %r12w, %ax, %r12w",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size16,
            AluRmiROpcode::Xor,
            RegMemImm::reg(r10),
            w_rcx,
        ),
        "664431D1",
        "xorw    %cx, %r10w, %cx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size16,
            AluRmiROpcode::And,
            RegMemImm::reg(r10),
            w_r14,
        ),
        "664521D6",
        "andw    %r14w, %r10w, %r14w",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size16,
            AluRmiROpcode::And,
            RegMemImm::imm(10),
            w_r14,
        ),
        "664183E60A",
        "andw    %r14w, $10, %r14w",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size16,
            AluRmiROpcode::And,
            RegMemImm::imm(-512i32 as u32),
            w_r14,
        ),
        "664181E600FE",
        "andw    %r14w, $-512, %r14w",
    ));

    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Add,
            RegMemImm::imm(10),
            w_rax,
        ),
        "80C00A", // there is theoretically 040A as a valid encoding also
        "addb    %al, $10, %al",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Add,
            RegMemImm::reg(rcx),
            w_rax,
        ),
        "00C8",
        "addb    %al, %cl, %al",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Add,
            RegMemImm::reg(rsi),
            w_rax,
        ),
        "4000F0",
        "addb    %al, %sil, %al",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Add,
            RegMemImm::reg(r11),
            w_rax,
        ),
        "4400D8",
        "addb    %al, %r11b, %al",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Add,
            RegMemImm::reg(r15),
            w_rax,
        ),
        "4400F8",
        "addb    %al, %r15b, %al",
    ));

    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Sub,
            RegMemImm::imm(10),
            _w_rbp,
        ),
        "4080ED0A",
        "subb    %bpl, $10, %bpl",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Sub,
            RegMemImm::reg(rcx),
            _w_rbp,
        ),
        "4028CD",
        "subb    %bpl, %cl, %bpl",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Sub,
            RegMemImm::reg(rsi),
            _w_rbp,
        ),
        "4028F5",
        "subb    %bpl, %sil, %bpl",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Sub,
            RegMemImm::reg(r11),
            _w_rbp,
        ),
        "4428DD",
        "subb    %bpl, %r11b, %bpl",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Sub,
            RegMemImm::reg(r15),
            _w_rbp,
        ),
        "4428FD",
        "subb    %bpl, %r15b, %bpl",
    ));

    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Xor,
            RegMemImm::imm(10),
            _w_r10,
        ),
        "4180F20A",
        "xorb    %r10b, $10, %r10b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Xor,
            RegMemImm::reg(rcx),
            _w_r10,
        ),
        "4130CA",
        "xorb    %r10b, %cl, %r10b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Xor,
            RegMemImm::reg(rsi),
            _w_r10,
        ),
        "4130F2",
        "xorb    %r10b, %sil, %r10b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Xor,
            RegMemImm::reg(r11),
            _w_r10,
        ),
        "4530DA",
        "xorb    %r10b, %r11b, %r10b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::Xor,
            RegMemImm::reg(r15),
            _w_r10,
        ),
        "4530FA",
        "xorb    %r10b, %r15b, %r10b",
    ));

    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::And,
            RegMemImm::imm(10),
            w_r15,
        ),
        "4180E70A",
        "andb    %r15b, $10, %r15b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::And,
            RegMemImm::reg(rcx),
            w_r15,
        ),
        "4120CF",
        "andb    %r15b, %cl, %r15b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::And,
            RegMemImm::reg(rsi),
            w_r15,
        ),
        "4120F7",
        "andb    %r15b, %sil, %r15b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::And,
            RegMemImm::reg(r11),
            w_r15,
        ),
        "4520DF",
        "andb    %r15b, %r11b, %r15b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size8,
            AluRmiROpcode::And,
            RegMemImm::reg(r15),
            w_r15,
        ),
        "4520FF",
        "andb    %r15b, %r15b, %r15b",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size32,
            AluRmiROpcode::Sbb,
            RegMemImm::reg(r14),
            w_r15,
        ),
        "4519F7",
        "sbbl    %r15d, %r14d, %r15d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Sbb,
            RegMemImm::imm(0),
            w_r15,
        ),
        "4983DF00",
        "sbbq    %r15, $0, %r15",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Adc,
            RegMemImm::imm(0),
            w_r15,
        ),
        "4983D700",
        "adcq    %r15, $0, %r15",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Adc,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
            w_r15,
        ),
        "4C137F63",
        "adcq    %r15, 99(%rdi), %r15",
    ));
    insns.push((
        Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Sbb,
            RegMemImm::mem(Amode::imm_reg(99, rdi)),
            w_r15,
        ),
        "4C1B7F63",
        "sbbq    %r15, 99(%rdi), %r15",
    ));

    // ========================================================
    // AluRM

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size32,
            op: AluRmiROpcode::Add,
            src1_dst: Amode::imm_reg(99, rdi).into(),
            src2: Gpr::unwrap_new(r12),
        },
        "44016763",
        "addl    %r12d, 99(%rdi)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size64,
            op: AluRmiROpcode::Add,
            src1_dst: Amode::imm_reg_reg_shift(0, Gpr::unwrap_new(rbp), Gpr::unwrap_new(rax), 3)
                .into(),
            src2: Gpr::unwrap_new(rax),
        },
        "480144C500",
        "addq    %rax, 0(%rbp,%rax,8)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size32,
            op: AluRmiROpcode::Sub,
            src1_dst: Amode::imm_reg(0, rsp).into(),
            src2: Gpr::unwrap_new(rcx),
        },
        "290C24",
        "subl    %ecx, 0(%rsp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size64,
            op: AluRmiROpcode::Sub,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(rax),
        },
        "48294500",
        "subq    %rax, 0(%rbp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size32,
            op: AluRmiROpcode::And,
            src1_dst: Amode::imm_reg(0, rsp).into(),
            src2: Gpr::unwrap_new(rcx),
        },
        "210C24",
        "andl    %ecx, 0(%rsp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size64,
            op: AluRmiROpcode::And,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(rax),
        },
        "48214500",
        "andq    %rax, 0(%rbp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size32,
            op: AluRmiROpcode::Or,
            src1_dst: Amode::imm_reg(0, rsp).into(),
            src2: Gpr::unwrap_new(rcx),
        },
        "090C24",
        "orl     %ecx, 0(%rsp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size64,
            op: AluRmiROpcode::Or,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(rax),
        },
        "48094500",
        "orq     %rax, 0(%rbp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size32,
            op: AluRmiROpcode::Xor,
            src1_dst: Amode::imm_reg(0, rsp).into(),
            src2: Gpr::unwrap_new(rcx),
        },
        "310C24",
        "xorl    %ecx, 0(%rsp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size64,
            op: AluRmiROpcode::Xor,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(rax),
        },
        "48314500",
        "xorq    %rax, 0(%rbp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size16,
            op: AluRmiROpcode::Add,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(rax),
        },
        "66014500",
        "addw    %ax, 0(%rbp)",
    ));
    insns.push((
        Inst::AluRM {
            size: OperandSize::Size16,
            op: AluRmiROpcode::Sub,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(r12),
        },
        "6644296500",
        "subw    %r12w, 0(%rbp)",
    ));

    insns.push((
        Inst::AluRM {
            size: OperandSize::Size8,
            op: AluRmiROpcode::Add,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(rax),
        },
        "004500",
        "addb    %al, 0(%rbp)",
    ));
    insns.push((
        Inst::AluRM {
            size: OperandSize::Size8,
            op: AluRmiROpcode::Sub,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(rbp),
        },
        "40286D00",
        "subb    %bpl, 0(%rbp)",
    ));
    insns.push((
        Inst::AluRM {
            size: OperandSize::Size8,
            op: AluRmiROpcode::Xor,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(r10),
        },
        "44305500",
        "xorb    %r10b, 0(%rbp)",
    ));
    insns.push((
        Inst::AluRM {
            size: OperandSize::Size8,
            op: AluRmiROpcode::And,
            src1_dst: Amode::imm_reg(0, rbp).into(),
            src2: Gpr::unwrap_new(r15),
        },
        "44207D00",
        "andb    %r15b, 0(%rbp)",
    ));

    // ========================================================
    // UnaryRmR

    insns.push((
        Inst::unary_rm_r(
            OperandSize::Size32,
            UnaryRmROpcode::Bsr,
            RegMem::reg(rsi),
            w_rdi,
        ),
        "0FBDFE",
        "bsrl    %esi, %edi",
    ));
    insns.push((
        Inst::unary_rm_r(
            OperandSize::Size64,
            UnaryRmROpcode::Bsr,
            RegMem::reg(r15),
            w_rax,
        ),
        "490FBDC7",
        "bsrq    %r15, %rax",
    ));

    // ========================================================
    // Not
    insns.push((
        Inst::not(OperandSize::Size32, Writable::from_reg(regs::rsi())),
        "F7D6",
        "notl    %esi, %esi",
    ));
    insns.push((
        Inst::not(OperandSize::Size64, Writable::from_reg(regs::r15())),
        "49F7D7",
        "notq    %r15, %r15",
    ));
    insns.push((
        Inst::not(OperandSize::Size32, Writable::from_reg(regs::r14())),
        "41F7D6",
        "notl    %r14d, %r14d",
    ));
    insns.push((
        Inst::not(OperandSize::Size16, Writable::from_reg(regs::rdi())),
        "66F7D7",
        "notw    %di, %di",
    ));
    insns.push((
        Inst::not(OperandSize::Size8, Writable::from_reg(regs::rdi())),
        "40F6D7",
        "notb    %dil, %dil",
    ));
    insns.push((
        Inst::not(OperandSize::Size8, Writable::from_reg(regs::rax())),
        "F6D0",
        "notb    %al, %al",
    ));

    // ========================================================
    // Neg
    insns.push((
        Inst::neg(OperandSize::Size32, Writable::from_reg(regs::rsi())),
        "F7DE",
        "negl    %esi, %esi",
    ));
    insns.push((
        Inst::neg(OperandSize::Size64, Writable::from_reg(regs::r15())),
        "49F7DF",
        "negq    %r15, %r15",
    ));
    insns.push((
        Inst::neg(OperandSize::Size32, Writable::from_reg(regs::r14())),
        "41F7DE",
        "negl    %r14d, %r14d",
    ));
    insns.push((
        Inst::neg(OperandSize::Size16, Writable::from_reg(regs::rdi())),
        "66F7DF",
        "negw    %di, %di",
    ));
    insns.push((
        Inst::neg(OperandSize::Size8, Writable::from_reg(regs::rdi())),
        "40F6DF",
        "negb    %dil, %dil",
    ));
    insns.push((
        Inst::neg(OperandSize::Size8, Writable::from_reg(regs::rax())),
        "F6D8",
        "negb    %al, %al",
    ));

    // ========================================================
    // Div
    insns.push((
        Inst::div(
            OperandSize::Size32,
            DivSignedness::Signed,
            TrapCode::IntegerDivisionByZero,
            RegMem::reg(regs::rsi()),
            Gpr::unwrap_new(regs::rax()),
            Gpr::unwrap_new(regs::rdx()),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rax())),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rdx())),
        ),
        "F7FE",
        "idiv    %eax, %edx, %esi, %eax, %edx ; trap=int_divz",
    ));
    insns.push((
        Inst::div(
            OperandSize::Size64,
            DivSignedness::Signed,
            TrapCode::IntegerDivisionByZero,
            RegMem::reg(regs::r15()),
            Gpr::unwrap_new(regs::rax()),
            Gpr::unwrap_new(regs::rdx()),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rax())),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rdx())),
        ),
        "49F7FF",
        "idiv    %rax, %rdx, %r15, %rax, %rdx ; trap=int_divz",
    ));
    insns.push((
        Inst::div(
            OperandSize::Size32,
            DivSignedness::Unsigned,
            TrapCode::IntegerDivisionByZero,
            RegMem::reg(regs::r14()),
            Gpr::unwrap_new(regs::rax()),
            Gpr::unwrap_new(regs::rdx()),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rax())),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rdx())),
        ),
        "41F7F6",
        "div     %eax, %edx, %r14d, %eax, %edx ; trap=int_divz",
    ));
    insns.push((
        Inst::div(
            OperandSize::Size64,
            DivSignedness::Unsigned,
            TrapCode::IntegerDivisionByZero,
            RegMem::reg(regs::rdi()),
            Gpr::unwrap_new(regs::rax()),
            Gpr::unwrap_new(regs::rdx()),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rax())),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rdx())),
        ),
        "48F7F7",
        "div     %rax, %rdx, %rdi, %rax, %rdx ; trap=int_divz",
    ));
    insns.push((
        Inst::div8(
            DivSignedness::Unsigned,
            TrapCode::IntegerDivisionByZero,
            RegMem::reg(regs::rax()),
            Gpr::unwrap_new(regs::rax()),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rax())),
        ),
        "F6F0",
        "div     %al, %al, %al ; trap=int_divz",
    ));
    insns.push((
        Inst::div8(
            DivSignedness::Unsigned,
            TrapCode::IntegerDivisionByZero,
            RegMem::reg(regs::rsi()),
            Gpr::unwrap_new(regs::rax()),
            WritableGpr::from_reg(Gpr::unwrap_new(regs::rax())),
        ),
        "40F6F6",
        "div     %al, %sil, %al ; trap=int_divz",
    ));

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
    // MovZX_RM_R
    insns.push((
        Inst::movzx_rm_r(ExtMode::BL, RegMem::reg(rdi), w_rdi),
        "400FB6FF",
        "movzbl  %dil, %edi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BL, RegMem::reg(rax), w_rsi),
        "0FB6F0",
        "movzbl  %al, %esi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BL, RegMem::reg(r15), w_rsi),
        "410FB6F7",
        "movzbl  %r15b, %esi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BL, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "0FB671F9",
        "movzbl  -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BL, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "410FB658F9",
        "movzbl  -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BL, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "450FB64AF9",
        "movzbl  -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BL, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "410FB653F9",
        "movzbl  -7(%r11), %edx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BQ, RegMem::reg(rax), w_rsi),
        "480FB6F0",
        "movzbq  %al, %rsi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BQ, RegMem::reg(r10), w_rsi),
        "490FB6F2",
        "movzbq  %r10b, %rsi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BQ, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "480FB671F9",
        "movzbq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BQ, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "490FB658F9",
        "movzbq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BQ, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "4D0FB64AF9",
        "movzbq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::BQ, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "490FB653F9",
        "movzbq  -7(%r11), %rdx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WL, RegMem::reg(rcx), w_rsi),
        "0FB7F1",
        "movzwl  %cx, %esi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WL, RegMem::reg(r10), w_rsi),
        "410FB7F2",
        "movzwl  %r10w, %esi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WL, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "0FB771F9",
        "movzwl  -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WL, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "410FB758F9",
        "movzwl  -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WL, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "450FB74AF9",
        "movzwl  -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WL, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "410FB753F9",
        "movzwl  -7(%r11), %edx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WQ, RegMem::reg(rcx), w_rsi),
        "480FB7F1",
        "movzwq  %cx, %rsi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WQ, RegMem::reg(r11), w_rsi),
        "490FB7F3",
        "movzwq  %r11w, %rsi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WQ, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "480FB771F9",
        "movzwq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WQ, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "490FB758F9",
        "movzwq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WQ, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "4D0FB74AF9",
        "movzwq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::WQ, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "490FB753F9",
        "movzwq  -7(%r11), %rdx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::LQ, RegMem::reg(rcx), w_rsi),
        "8BF1",
        "movl    %ecx, %esi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::LQ, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "8B71F9",
        "movl    -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::LQ, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "418B58F9",
        "movl    -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::LQ, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "458B4AF9",
        "movl    -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movzx_rm_r(ExtMode::LQ, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "418B53F9",
        "movl    -7(%r11), %edx",
    ));

    // ========================================================
    // Mov64_M_R
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(rax), Gpr::unwrap_new(rbx), 0),
            w_rcx,
        ),
        "488B8C18B3000000",
        "movq    179(%rax,%rbx,1), %rcx",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(rax), Gpr::unwrap_new(rbx), 0),
            w_r8,
        ),
        "4C8B8418B3000000",
        "movq    179(%rax,%rbx,1), %r8",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(rax), Gpr::unwrap_new(r9), 0),
            w_rcx,
        ),
        "4A8B8C08B3000000",
        "movq    179(%rax,%r9,1), %rcx",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(rax), Gpr::unwrap_new(r9), 0),
            w_r8,
        ),
        "4E8B8408B3000000",
        "movq    179(%rax,%r9,1), %r8",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(r10), Gpr::unwrap_new(rbx), 0),
            w_rcx,
        ),
        "498B8C1AB3000000",
        "movq    179(%r10,%rbx,1), %rcx",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(r10), Gpr::unwrap_new(rbx), 0),
            w_r8,
        ),
        "4D8B841AB3000000",
        "movq    179(%r10,%rbx,1), %r8",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(r10), Gpr::unwrap_new(r9), 0),
            w_rcx,
        ),
        "4B8B8C0AB3000000",
        "movq    179(%r10,%r9,1), %rcx",
    ));
    insns.push((
        Inst::mov64_m_r(
            Amode::imm_reg_reg_shift(179, Gpr::unwrap_new(r10), Gpr::unwrap_new(r9), 0),
            w_r8,
        ),
        "4F8B840AB3000000",
        "movq    179(%r10,%r9,1), %r8",
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

    // ========================================================
    // MovSX_RM_R
    insns.push((
        Inst::movsx_rm_r(ExtMode::BL, RegMem::reg(rdi), w_rdi),
        "400FBEFF",
        "movsbl  %dil, %edi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BL, RegMem::reg(rcx), w_rsi),
        "0FBEF1",
        "movsbl  %cl, %esi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BL, RegMem::reg(r14), w_rsi),
        "410FBEF6",
        "movsbl  %r14b, %esi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BL, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "0FBE71F9",
        "movsbl  -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BL, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "410FBE58F9",
        "movsbl  -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BL, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "450FBE4AF9",
        "movsbl  -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BL, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "410FBE53F9",
        "movsbl  -7(%r11), %edx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BQ, RegMem::reg(rcx), w_rsi),
        "480FBEF1",
        "movsbq  %cl, %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BQ, RegMem::reg(r15), w_rsi),
        "490FBEF7",
        "movsbq  %r15b, %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BQ, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "480FBE71F9",
        "movsbq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BQ, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "490FBE58F9",
        "movsbq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BQ, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "4D0FBE4AF9",
        "movsbq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::BQ, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "490FBE53F9",
        "movsbq  -7(%r11), %rdx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WL, RegMem::reg(rcx), w_rsi),
        "0FBFF1",
        "movswl  %cx, %esi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WL, RegMem::reg(r14), w_rsi),
        "410FBFF6",
        "movswl  %r14w, %esi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WL, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "0FBF71F9",
        "movswl  -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WL, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "410FBF58F9",
        "movswl  -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WL, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "450FBF4AF9",
        "movswl  -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WL, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "410FBF53F9",
        "movswl  -7(%r11), %edx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WQ, RegMem::reg(rcx), w_rsi),
        "480FBFF1",
        "movswq  %cx, %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WQ, RegMem::reg(r13), w_rsi),
        "490FBFF5",
        "movswq  %r13w, %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WQ, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "480FBF71F9",
        "movswq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WQ, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "490FBF58F9",
        "movswq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WQ, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "4D0FBF4AF9",
        "movswq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::WQ, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "490FBF53F9",
        "movswq  -7(%r11), %rdx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::LQ, RegMem::reg(rcx), w_rsi),
        "4863F1",
        "movslq  %ecx, %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::LQ, RegMem::reg(r15), w_rsi),
        "4963F7",
        "movslq  %r15d, %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::LQ, RegMem::mem(Amode::imm_reg(-7i32, rcx)), w_rsi),
        "486371F9",
        "movslq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::LQ, RegMem::mem(Amode::imm_reg(-7i32, r8)), w_rbx),
        "496358F9",
        "movslq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::LQ, RegMem::mem(Amode::imm_reg(-7i32, r10)), w_r9),
        "4D634AF9",
        "movslq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movsx_rm_r(ExtMode::LQ, RegMem::mem(Amode::imm_reg(-7i32, r11)), w_rdx),
        "496353F9",
        "movslq  -7(%r11), %rdx",
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
    // Shift_R
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rdi,
            w_rdi,
        ),
        "D3E7",
        "shll    %cl, %edi, %edi",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            r12,
            w_r12,
        ),
        "41D3E4",
        "shll    %cl, %r12d, %r12d",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 2 }),
            r8,
            w_r8,
        ),
        "41C1E002",
        "shll    $2, %r8d, %r8d",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 31 }),
            r13,
            w_r13,
        ),
        "41C1E51F",
        "shll    $31, %r13d, %r13d",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            r13,
            w_r13,
        ),
        "49D3E5",
        "shlq    %cl, %r13, %r13",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rdi,
            w_rdi,
        ),
        "48D3E7",
        "shlq    %cl, %rdi, %rdi",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 2 }),
            r8,
            w_r8,
        ),
        "49C1E002",
        "shlq    $2, %r8, %r8",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 3 }),
            rbx,
            w_rbx,
        ),
        "48C1E303",
        "shlq    $3, %rbx, %rbx",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 63 }),
            r13,
            w_r13,
        ),
        "49C1E53F",
        "shlq    $63, %r13, %r13",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftRightLogical,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rdi,
            w_rdi,
        ),
        "D3EF",
        "shrl    %cl, %edi, %edi",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftRightLogical,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 2 }),
            r8,
            w_r8,
        ),
        "41C1E802",
        "shrl    $2, %r8d, %r8d",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftRightLogical,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 31 }),
            r13,
            w_r13,
        ),
        "41C1ED1F",
        "shrl    $31, %r13d, %r13d",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftRightLogical,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rdi,
            w_rdi,
        ),
        "48D3EF",
        "shrq    %cl, %rdi, %rdi",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftRightLogical,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 2 }),
            r8,
            w_r8,
        ),
        "49C1E802",
        "shrq    $2, %r8, %r8",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftRightLogical,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 63 }),
            r13,
            w_r13,
        ),
        "49C1ED3F",
        "shrq    $63, %r13, %r13",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftRightArithmetic,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rdi,
            w_rdi,
        ),
        "D3FF",
        "sarl    %cl, %edi, %edi",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftRightArithmetic,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 2 }),
            r8,
            w_r8,
        ),
        "41C1F802",
        "sarl    $2, %r8d, %r8d",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::ShiftRightArithmetic,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 31 }),
            r13,
            w_r13,
        ),
        "41C1FD1F",
        "sarl    $31, %r13d, %r13d",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftRightArithmetic,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rdi,
            w_rdi,
        ),
        "48D3FF",
        "sarq    %cl, %rdi, %rdi",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftRightArithmetic,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 2 }),
            r8,
            w_r8,
        ),
        "49C1F802",
        "sarq    $2, %r8, %r8",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::ShiftRightArithmetic,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 63 }),
            r13,
            w_r13,
        ),
        "49C1FD3F",
        "sarq    $63, %r13, %r13",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::RotateLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            r8,
            w_r8,
        ),
        "49D3C0",
        "rolq    %cl, %r8, %r8",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::RotateLeft,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 3 }),
            r9,
            w_r9,
        ),
        "41C1C103",
        "roll    $3, %r9d, %r9d",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size32,
            ShiftKind::RotateRight,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rsi,
            w_rsi,
        ),
        "D3CE",
        "rorl    %cl, %esi, %esi",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size64,
            ShiftKind::RotateRight,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 5 }),
            r15,
            w_r15,
        ),
        "49C1CF05",
        "rorq    $5, %r15, %r15",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size8,
            ShiftKind::RotateRight,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rsi,
            w_rsi,
        ),
        "40D2CE",
        "rorb    %cl, %sil, %sil",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size8,
            ShiftKind::RotateRight,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rax,
            w_rax,
        ),
        "D2C8",
        "rorb    %cl, %al, %al",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size8,
            ShiftKind::RotateRight,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 5 }),
            r15,
            w_r15,
        ),
        "41C0CF05",
        "rorb    $5, %r15b, %r15b",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size16,
            ShiftKind::RotateRight,
            Imm8Gpr::unwrap_new(Imm8Reg::Reg { reg: regs::rcx() }),
            rsi,
            w_rsi,
        ),
        "66D3CE",
        "rorw    %cl, %si, %si",
    ));
    insns.push((
        Inst::shift_r(
            OperandSize::Size16,
            ShiftKind::RotateRight,
            Imm8Gpr::unwrap_new(Imm8Reg::Imm8 { imm: 5 }),
            r15,
            w_r15,
        ),
        "6641C1CF05",
        "rorw    $5, %r15w, %r15w",
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
    // Bswap
    insns.push((
        Inst::bswap(OperandSize::Size64, w_rax),
        "480FC8",
        "bswapq  %rax, %rax",
    ));
    insns.push((
        Inst::bswap(OperandSize::Size64, w_r8),
        "490FC8",
        "bswapq  %r8, %r8",
    ));
    insns.push((
        Inst::bswap(OperandSize::Size32, w_rax),
        "0FC8",
        "bswapl  %eax, %eax",
    ));
    insns.push((
        Inst::bswap(OperandSize::Size64, w_rcx),
        "480FC9",
        "bswapq  %rcx, %rcx",
    ));
    insns.push((
        Inst::bswap(OperandSize::Size32, w_rcx),
        "0FC9",
        "bswapl  %ecx, %ecx",
    ));
    insns.push((
        Inst::bswap(OperandSize::Size64, w_r11),
        "490FCB",
        "bswapq  %r11, %r11",
    ));
    insns.push((
        Inst::bswap(OperandSize::Size32, w_r11),
        "410FCB",
        "bswapl  %r11d, %r11d",
    ));
    insns.push((
        Inst::bswap(OperandSize::Size64, w_r14),
        "490FCE",
        "bswapq  %r14, %r14",
    ));
    insns.push((
        Inst::bswap(OperandSize::Size32, w_r14),
        "410FCE",
        "bswapl  %r14d, %r14d",
    ));

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
        Inst::xmm_rm_r(SseOpcode::Addss, RegMem::reg(xmm1), w_xmm0),
        "F30F58C1",
        "addss   %xmm0, %xmm1, %xmm0",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Addss, RegMem::reg(xmm11), w_xmm13),
        "F3450F58EB",
        "addss   %xmm13, %xmm11, %xmm13",
    ));
    insns.push((
        Inst::xmm_rm_r(
            SseOpcode::Addss,
            RegMem::mem(Amode::imm_reg_reg_shift(
                123,
                Gpr::unwrap_new(r10),
                Gpr::unwrap_new(rdx),
                2,
            )),
            w_xmm0,
        ),
        "F3410F5844927B",
        "addss   %xmm0, 123(%r10,%rdx,4), %xmm0",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Addsd, RegMem::reg(xmm15), w_xmm4),
        "F2410F58E7",
        "addsd   %xmm4, %xmm15, %xmm4",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Subss, RegMem::reg(xmm0), w_xmm1),
        "F30F5CC8",
        "subss   %xmm1, %xmm0, %xmm1",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Subss, RegMem::reg(xmm12), w_xmm1),
        "F3410F5CCC",
        "subss   %xmm1, %xmm12, %xmm1",
    ));
    insns.push((
        Inst::xmm_rm_r(
            SseOpcode::Subss,
            RegMem::mem(Amode::imm_reg_reg_shift(
                321,
                Gpr::unwrap_new(r10),
                Gpr::unwrap_new(rax),
                3,
            )),
            w_xmm10,
        ),
        "F3450F5C94C241010000",
        "subss   %xmm10, 321(%r10,%rax,8), %xmm10",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Subsd, RegMem::reg(xmm5), w_xmm14),
        "F2440F5CF5",
        "subsd   %xmm14, %xmm5, %xmm14",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Mulss, RegMem::reg(xmm5), w_xmm4),
        "F30F59E5",
        "mulss   %xmm4, %xmm5, %xmm4",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Mulsd, RegMem::reg(xmm5), w_xmm4),
        "F20F59E5",
        "mulsd   %xmm4, %xmm5, %xmm4",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Divss, RegMem::reg(xmm8), w_xmm7),
        "F3410F5EF8",
        "divss   %xmm7, %xmm8, %xmm7",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Divsd, RegMem::reg(xmm5), w_xmm4),
        "F20F5EE5",
        "divsd   %xmm4, %xmm5, %xmm4",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Andps, RegMem::reg(xmm3), w_xmm12),
        "440F54E3",
        "andps   %xmm12, %xmm3, %xmm12",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Andnps, RegMem::reg(xmm4), w_xmm11),
        "440F55DC",
        "andnps  %xmm11, %xmm4, %xmm11",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Orps, RegMem::reg(xmm1), w_xmm15),
        "440F56F9",
        "orps    %xmm15, %xmm1, %xmm15",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Orps, RegMem::reg(xmm5), w_xmm4),
        "0F56E5",
        "orps    %xmm4, %xmm5, %xmm4",
    ));

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
        Inst::xmm_rm_r(SseOpcode::Paddb, RegMem::reg(xmm9), w_xmm5),
        "66410FFCE9",
        "paddb   %xmm5, %xmm9, %xmm5",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Paddw, RegMem::reg(xmm7), w_xmm6),
        "660FFDF7",
        "paddw   %xmm6, %xmm7, %xmm6",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Paddd, RegMem::reg(xmm12), w_xmm13),
        "66450FFEEC",
        "paddd   %xmm13, %xmm12, %xmm13",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Paddq, RegMem::reg(xmm1), w_xmm8),
        "66440FD4C1",
        "paddq   %xmm8, %xmm1, %xmm8",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Paddsb, RegMem::reg(xmm9), w_xmm5),
        "66410FECE9",
        "paddsb  %xmm5, %xmm9, %xmm5",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Paddsw, RegMem::reg(xmm7), w_xmm6),
        "660FEDF7",
        "paddsw  %xmm6, %xmm7, %xmm6",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Paddusb, RegMem::reg(xmm12), w_xmm13),
        "66450FDCEC",
        "paddusb %xmm13, %xmm12, %xmm13",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Paddusw, RegMem::reg(xmm1), w_xmm8),
        "66440FDDC1",
        "paddusw %xmm8, %xmm1, %xmm8",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Psubsb, RegMem::reg(xmm9), w_xmm5),
        "66410FE8E9",
        "psubsb  %xmm5, %xmm9, %xmm5",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Psubsw, RegMem::reg(xmm7), w_xmm6),
        "660FE9F7",
        "psubsw  %xmm6, %xmm7, %xmm6",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Psubusb, RegMem::reg(xmm12), w_xmm13),
        "66450FD8EC",
        "psubusb %xmm13, %xmm12, %xmm13",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Psubusw, RegMem::reg(xmm1), w_xmm8),
        "66440FD9C1",
        "psubusw %xmm8, %xmm1, %xmm8",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pavgb, RegMem::reg(xmm12), w_xmm13),
        "66450FE0EC",
        "pavgb   %xmm13, %xmm12, %xmm13",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pavgw, RegMem::reg(xmm1), w_xmm8),
        "66440FE3C1",
        "pavgw   %xmm8, %xmm1, %xmm8",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Psubb, RegMem::reg(xmm5), w_xmm9),
        "66440FF8CD",
        "psubb   %xmm9, %xmm5, %xmm9",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Psubw, RegMem::reg(xmm6), w_xmm7),
        "660FF9FE",
        "psubw   %xmm7, %xmm6, %xmm7",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Psubd, RegMem::reg(xmm13), w_xmm12),
        "66450FFAE5",
        "psubd   %xmm12, %xmm13, %xmm12",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Psubq, RegMem::reg(xmm8), w_xmm1),
        "66410FFBC8",
        "psubq   %xmm1, %xmm8, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmuldq, RegMem::reg(xmm4), w_xmm15),
        "66440F3828FC",
        "pmuldq  %xmm15, %xmm4, %xmm15",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmulhw, RegMem::reg(xmm9), w_xmm1),
        "66410FE5C9",
        "pmulhw  %xmm1, %xmm9, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmulhuw, RegMem::reg(xmm7), w_xmm9),
        "66440FE4CF",
        "pmulhuw %xmm9, %xmm7, %xmm9",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmulld, RegMem::reg(xmm15), w_xmm6),
        "66410F3840F7",
        "pmulld  %xmm6, %xmm15, %xmm6",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmullw, RegMem::reg(xmm14), w_xmm1),
        "66410FD5CE",
        "pmullw  %xmm1, %xmm14, %xmm1",
    ));

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
        Inst::xmm_rm_r(SseOpcode::Pmuludq, RegMem::reg(xmm8), w_xmm9),
        "66450FF4C8",
        "pmuludq %xmm9, %xmm8, %xmm9",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmaddwd, RegMem::reg(xmm8), w_xmm1),
        "66410FF5C8",
        "pmaddwd %xmm1, %xmm8, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmaxsb, RegMem::reg(xmm15), w_xmm6),
        "66410F383CF7",
        "pmaxsb  %xmm6, %xmm15, %xmm6",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmaxsw, RegMem::reg(xmm15), w_xmm6),
        "66410FEEF7",
        "pmaxsw  %xmm6, %xmm15, %xmm6",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmaxsd, RegMem::reg(xmm15), w_xmm6),
        "66410F383DF7",
        "pmaxsd  %xmm6, %xmm15, %xmm6",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmaxub, RegMem::reg(xmm14), w_xmm1),
        "66410FDECE",
        "pmaxub  %xmm1, %xmm14, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmaxuw, RegMem::reg(xmm14), w_xmm1),
        "66410F383ECE",
        "pmaxuw  %xmm1, %xmm14, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pmaxud, RegMem::reg(xmm14), w_xmm1),
        "66410F383FCE",
        "pmaxud  %xmm1, %xmm14, %xmm1",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pminsb, RegMem::reg(xmm8), w_xmm9),
        "66450F3838C8",
        "pminsb  %xmm9, %xmm8, %xmm9",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pminsw, RegMem::reg(xmm8), w_xmm9),
        "66450FEAC8",
        "pminsw  %xmm9, %xmm8, %xmm9",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pminsd, RegMem::reg(xmm8), w_xmm9),
        "66450F3839C8",
        "pminsd  %xmm9, %xmm8, %xmm9",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pminub, RegMem::reg(xmm3), w_xmm2),
        "660FDAD3",
        "pminub  %xmm2, %xmm3, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pminuw, RegMem::reg(xmm3), w_xmm2),
        "660F383AD3",
        "pminuw  %xmm2, %xmm3, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pminud, RegMem::reg(xmm3), w_xmm2),
        "660F383BD3",
        "pminud  %xmm2, %xmm3, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Pxor, RegMem::reg(xmm11), w_xmm2),
        "66410FEFD3",
        "pxor    %xmm2, %xmm11, %xmm2",
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

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Punpckhbw, RegMem::reg(xmm3), w_xmm2),
        "660F68D3",
        "punpckhbw %xmm2, %xmm3, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Punpckhwd, RegMem::reg(xmm13), w_xmm2),
        "66410F69D5",
        "punpckhwd %xmm2, %xmm13, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Punpcklbw, RegMem::reg(xmm1), w_xmm8),
        "66440F60C1",
        "punpcklbw %xmm8, %xmm1, %xmm8",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Punpcklwd, RegMem::reg(xmm11), w_xmm8),
        "66450F61C3",
        "punpcklwd %xmm8, %xmm11, %xmm8",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Unpcklps, RegMem::reg(xmm11), w_xmm2),
        "410F14D3",
        "unpcklps %xmm2, %xmm11, %xmm2",
    ));

    // ========================================================
    // XMM_RM_R: Integer Conversion
    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Cvtdq2ps, RegMem::reg(xmm1), w_xmm8),
        "440F5BC1",
        "cvtdq2ps %xmm1, %xmm8",
    ));

    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Cvttpd2dq, RegMem::reg(xmm15), w_xmm7),
        "66410FE6FF",
        "cvttpd2dq %xmm15, %xmm7",
    ));

    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Cvttps2dq, RegMem::reg(xmm9), w_xmm8),
        "F3450F5BC1",
        "cvttps2dq %xmm9, %xmm8",
    ));

    // XMM_Mov_R_M: float stores
    insns.push((
        Inst::xmm_mov_r_m(SseOpcode::Movss, xmm15, Amode::imm_reg(128, r12)),
        "F3450F11BC2480000000",
        "movss   %xmm15, 128(%r12)",
    ));
    insns.push((
        Inst::xmm_mov_r_m(SseOpcode::Movsd, xmm1, Amode::imm_reg(0, rsi)),
        "F20F110E",
        "movsd   %xmm1, 0(%rsi)",
    ));

    // ========================================================
    // XMM_MOV: Packed Move

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovsxbd, RegMem::reg(xmm6), w_xmm8),
        "66440F3821C6",
        "pmovsxbd %xmm6, %xmm8",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovsxbw, RegMem::reg(xmm9), w_xmm10),
        "66450F3820D1",
        "pmovsxbw %xmm9, %xmm10",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovsxbq, RegMem::reg(xmm1), w_xmm1),
        "660F3822C9",
        "pmovsxbq %xmm1, %xmm1",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovsxwd, RegMem::reg(xmm13), w_xmm10),
        "66450F3823D5",
        "pmovsxwd %xmm13, %xmm10",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovsxwq, RegMem::reg(xmm12), w_xmm12),
        "66450F3824E4",
        "pmovsxwq %xmm12, %xmm12",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovsxdq, RegMem::reg(xmm10), w_xmm8),
        "66450F3825C2",
        "pmovsxdq %xmm10, %xmm8",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovzxbd, RegMem::reg(xmm5), w_xmm6),
        "660F3831F5",
        "pmovzxbd %xmm5, %xmm6",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovzxbw, RegMem::reg(xmm5), w_xmm13),
        "66440F3830ED",
        "pmovzxbw %xmm5, %xmm13",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovzxbq, RegMem::reg(xmm10), w_xmm11),
        "66450F3832DA",
        "pmovzxbq %xmm10, %xmm11",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovzxwd, RegMem::reg(xmm2), w_xmm10),
        "66440F3833D2",
        "pmovzxwd %xmm2, %xmm10",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovzxwq, RegMem::reg(xmm7), w_xmm4),
        "660F3834E7",
        "pmovzxwq %xmm7, %xmm4",
    ));

    insns.push((
        Inst::xmm_mov(SseOpcode::Pmovzxdq, RegMem::reg(xmm3), w_xmm4),
        "660F3835E3",
        "pmovzxdq %xmm3, %xmm4",
    ));

    // XmmUnary: moves and unary float ops
    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Movss, RegMem::reg(xmm13), w_xmm2),
        "F3410F10D5",
        "movss   %xmm13, %xmm2",
    ));

    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Movsd, RegMem::reg(xmm0), w_xmm1),
        "F20F10C8",
        "movsd   %xmm0, %xmm1",
    ));
    insns.push((
        Inst::xmm_unary_rm_r(
            SseOpcode::Movsd,
            RegMem::mem(Amode::imm_reg(0, rsi)),
            w_xmm2,
        ),
        "F20F1016",
        "movsd   0(%rsi), %xmm2",
    ));
    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Movsd, RegMem::reg(xmm14), w_xmm3),
        "F2410F10DE",
        "movsd   %xmm14, %xmm3",
    ));

    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Movaps, RegMem::reg(xmm5), w_xmm14),
        "440F28F5",
        "movaps  %xmm5, %xmm14",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Sqrtss, RegMem::reg(xmm7), w_xmm8),
        "F3440F51C7",
        "sqrtss  %xmm8, %xmm7, %xmm8",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Sqrtsd, RegMem::reg(xmm1), w_xmm2),
        "F20F51D1",
        "sqrtsd  %xmm2, %xmm1, %xmm2",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Cvtss2sd, RegMem::reg(xmm0), w_xmm1),
        "F30F5AC8",
        "cvtss2sd %xmm1, %xmm0, %xmm1",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Cvtsd2ss, RegMem::reg(xmm1), w_xmm0),
        "F20F5AC1",
        "cvtsd2ss %xmm0, %xmm1, %xmm0",
    ));

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
        Inst::xmm_unary_rm_r(SseOpcode::Cvtdq2pd, RegMem::reg(xmm2), w_xmm8),
        "F3440FE6C2",
        "cvtdq2pd %xmm2, %xmm8",
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

    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Cvtpd2ps, RegMem::reg(xmm7), w_xmm7),
        "660F5AFF",
        "cvtpd2ps %xmm7, %xmm7",
    ));

    insns.push((
        Inst::xmm_unary_rm_r(SseOpcode::Cvtps2pd, RegMem::reg(xmm11), w_xmm9),
        "450F5ACB",
        "cvtps2pd %xmm11, %xmm9",
    ));

    // Xmm to int conversions, and conversely.

    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Movd, xmm0, w_rsi, OperandSize::Size32),
        "660F7EC6",
        "movd    %xmm0, %esi",
    ));
    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Movq, xmm2, w_rdi, OperandSize::Size64),
        "66480F7ED7",
        "movq    %xmm2, %rdi",
    ));
    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Cvttss2si, xmm0, w_rsi, OperandSize::Size32),
        "F30F2CF0",
        "cvttss2si %xmm0, %esi",
    ));
    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Cvttss2si, xmm0, w_rdi, OperandSize::Size64),
        "F3480F2CF8",
        "cvttss2si %xmm0, %rdi",
    ));
    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Cvttsd2si, xmm0, w_rax, OperandSize::Size32),
        "F20F2CC0",
        "cvttsd2si %xmm0, %eax",
    ));
    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Cvttsd2si, xmm0, w_r15, OperandSize::Size64),
        "F24C0F2CF8",
        "cvttsd2si %xmm0, %r15",
    ));

    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Pmovmskb, xmm10, w_rax, OperandSize::Size32),
        "66410FD7C2",
        "pmovmskb %xmm10, %eax",
    ));
    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Movmskps, xmm2, w_rax, OperandSize::Size32),
        "0F50C2",
        "movmskps %xmm2, %eax",
    ));
    insns.push((
        Inst::xmm_to_gpr(SseOpcode::Movmskpd, xmm0, w_rcx, OperandSize::Size32),
        "660F50C8",
        "movmskpd %xmm0, %ecx",
    ));

    insns.push((
        Inst::gpr_to_xmm(
            SseOpcode::Movd,
            RegMem::reg(rax),
            OperandSize::Size32,
            w_xmm15,
        ),
        "66440F6EF8",
        "movd    %eax, %xmm15",
    ));
    insns.push((
        Inst::gpr_to_xmm(
            SseOpcode::Movd,
            RegMem::mem(Amode::imm_reg(2, r10)),
            OperandSize::Size32,
            w_xmm9,
        ),
        "66450F6E4A02",
        "movd    2(%r10), %xmm9",
    ));
    insns.push((
        Inst::gpr_to_xmm(
            SseOpcode::Movd,
            RegMem::reg(rsi),
            OperandSize::Size32,
            w_xmm1,
        ),
        "660F6ECE",
        "movd    %esi, %xmm1",
    ));
    insns.push((
        Inst::gpr_to_xmm(
            SseOpcode::Movq,
            RegMem::reg(rdi),
            OperandSize::Size64,
            w_xmm15,
        ),
        "664C0F6EFF",
        "movq    %rdi, %xmm15",
    ));

    // ========================================================
    // XmmRmi
    insns.push((
        Inst::xmm_rmi_reg(SseOpcode::Psraw, RegMemImm::reg(xmm10), w_xmm1),
        "66410FE1CA",
        "psraw   %xmm1, %xmm10, %xmm1",
    ));
    insns.push((
        Inst::xmm_rmi_reg(SseOpcode::Pslld, RegMemImm::imm(31), w_xmm1),
        "660F72F11F",
        "pslld   %xmm1, $31, %xmm1",
    ));
    insns.push((
        Inst::xmm_rmi_reg(SseOpcode::Psrlq, RegMemImm::imm(1), w_xmm3),
        "660F73D301",
        "psrlq   %xmm3, $1, %xmm3",
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

    insns.push((
        Inst::xmm_unary_rm_r_imm(SseOpcode::Roundps, RegMem::reg(xmm7), w_xmm8, 3),
        "66440F3A08C703",
        "roundps $3, %xmm7, %xmm8",
    ));
    insns.push((
        Inst::xmm_unary_rm_r_imm(SseOpcode::Roundpd, RegMem::reg(xmm10), w_xmm7, 2),
        "66410F3A09FA02",
        "roundpd $2, %xmm10, %xmm7",
    ));
    insns.push((
        Inst::xmm_unary_rm_r_imm(SseOpcode::Roundps, RegMem::reg(xmm4), w_xmm8, 1),
        "66440F3A08C401",
        "roundps $1, %xmm4, %xmm8",
    ));
    insns.push((
        Inst::xmm_unary_rm_r_imm(SseOpcode::Roundpd, RegMem::reg(xmm15), w_xmm15, 0),
        "66450F3A09FF00",
        "roundpd $0, %xmm15, %xmm15",
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

    // AtomicRmwSeq
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I8,
            op: inst_common::MachAtomicRmwOp::Or,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "490FB6014989C34D09D3F0450FB0190F85EFFFFFFF",
        "atomically { 8_bits_at_[%r9]) Or= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I16,
            op: inst_common::MachAtomicRmwOp::And,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "490FB7014989C34D21D366F0450FB1190F85EEFFFFFF",
        "atomically { 16_bits_at_[%r9]) And= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I32,
            op: inst_common::MachAtomicRmwOp::Xchg,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "418B014989C34D89D3F0450FB1190F85EFFFFFFF",
        "atomically { 32_bits_at_[%r9]) Xchg= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I32,
            op: inst_common::MachAtomicRmwOp::Umin,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "418B014989C34539DA4D0F46DAF0450FB1190F85EBFFFFFF",
        "atomically { 32_bits_at_[%r9]) Umin= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I64,
            op: inst_common::MachAtomicRmwOp::Add,
            mem: am3.clone(),
            operand: r10,
            temp: w_r11,
            dst_old: w_rax
        },
        "498B014989C34D01D3F04D0FB1190F85EFFFFFFF",
        "atomically { 64_bits_at_[%r9]) Add= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
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

    let trap_code = TrapCode::UnreachableCodeReached;
    insns.push((Inst::Ud2 { trap_code }, "0F0B", "ud2 unreachable"));

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
        assert_eq!(expected_printing, actual_printing);
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

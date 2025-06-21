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
}

#[test]
fn test_x64_emit() {
    let rax = regs::rax();
    let rbx = regs::rbx();
    let rcx = regs::rcx();
    let rdx = regs::rdx();
    let rsi = regs::rsi();
    let _rdi = regs::rdi();
    let rsp = regs::rsp();
    let rbp = regs::rbp();
    let r8 = regs::r8();
    let r9 = regs::r9();
    let r10 = regs::r10();
    let r11 = regs::r11();
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
    let _xmm15 = regs::xmm15();

    // And Writable<> versions of the same:
    let w_rax = Writable::<Reg>::from_reg(rax);
    let w_rbx = Writable::<Reg>::from_reg(rbx);
    let w_rcx = Writable::<Reg>::from_reg(rcx);
    let w_rdx = Writable::<Reg>::from_reg(rdx);
    let w_rsi = Writable::<Reg>::from_reg(rsi);
    let _w_rsp = Writable::<Reg>::from_reg(rsp);
    let _w_rbp = Writable::<Reg>::from_reg(rbp);
    let w_r8 = Writable::<Reg>::from_reg(r8);
    let w_r9 = Writable::<Reg>::from_reg(r9);
    let w_r11 = Writable::<Reg>::from_reg(r11);
    let w_r14 = Writable::<Reg>::from_reg(r14);
    let _w_r15 = Writable::<Reg>::from_reg(r15);

    let w_xmm0 = Writable::<Reg>::from_reg(xmm0);
    let w_xmm1 = Writable::<Reg>::from_reg(xmm1);
    let w_xmm2 = Writable::<Reg>::from_reg(xmm2);
    let w_xmm3 = Writable::<Reg>::from_reg(xmm3);
    let w_xmm4 = Writable::<Reg>::from_reg(xmm4);
    let w_xmm6 = Writable::<Reg>::from_reg(xmm6);
    let _w_xmm7 = Writable::<Reg>::from_reg(xmm7);
    let w_xmm8 = Writable::<Reg>::from_reg(xmm8);
    let w_xmm9 = Writable::<Reg>::from_reg(xmm9);
    let w_xmm10 = Writable::<Reg>::from_reg(xmm10);
    let w_xmm11 = Writable::<Reg>::from_reg(xmm11);
    let w_xmm12 = Writable::<Reg>::from_reg(xmm12);

    let mut insns = Vec::<(Inst, &str, &str)>::new();

    // End of test cases for Addr
    // ========================================================

    // ========================================================
    // General tests for each insn.  Don't forget to follow the
    // guidelines commented just prior to `fn x64_emit`.
    //

    // ========================================================
    // SetCC
    insns.push((Inst::setcc(CC::O, w_rsi), "400F90C6", "seto    %sil"));
    insns.push((Inst::setcc(CC::NLE, w_rsi), "400F9FC6", "setnle  %sil"));
    insns.push((Inst::setcc(CC::Z, w_r14), "410F94C6", "setz    %r14b"));
    insns.push((Inst::setcc(CC::LE, w_r14), "410F9EC6", "setle   %r14b"));
    insns.push((Inst::setcc(CC::P, w_r9), "410F9AC1", "setp    %r9b"));
    insns.push((Inst::setcc(CC::NP, w_r8), "410F9BC0", "setnp   %r8b"));

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
    // Pertaining to atomics.
    // Use `r9` with a 0 offset.
    let am3: SyntheticAmode = Amode::imm_reg(0, r9).into();

    // AtomicRmwSeq
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I8,
            op: AtomicRmwSeqOp::Or,
            mem: am3.clone(),
            operand: Gpr::unwrap_new(r10),
            temp: w_r11.map(Gpr::unwrap_new),
            dst_old: w_rax.map(Gpr::unwrap_new),
        },
        "490FB6014989C34D0BDAF0450FB0190F85EFFFFFFF",
        "atomically { 8_bits_at_[%r9] Or= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }",
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I16,
            op: AtomicRmwSeqOp::And,
            mem: am3.clone(),
            operand: Gpr::unwrap_new(r10),
            temp: w_r11.map(Gpr::unwrap_new),
            dst_old: w_rax.map(Gpr::unwrap_new)
        },
        "490FB7014989C34D23DAF066450FB1190F85EEFFFFFF",
        "atomically { 16_bits_at_[%r9] And= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I32,
            op: AtomicRmwSeqOp::Nand,
            mem: am3.clone(),
            operand: Gpr::unwrap_new(r10),
            temp: w_r11.map(Gpr::unwrap_new),
            dst_old: w_rax.map(Gpr::unwrap_new)
        },
        "418B014989C34D23DA49F7D3F0450FB1190F85ECFFFFFF",
        "atomically { 32_bits_at_[%r9] Nand= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I32,
            op: AtomicRmwSeqOp::Umin,
            mem: am3.clone(),
            operand: Gpr::unwrap_new(r10),
            temp: w_r11.map(Gpr::unwrap_new),
            dst_old: w_rax.map(Gpr::unwrap_new)
        },
        "418B014989C34539DA4D0F46DAF0450FB1190F85EBFFFFFF",
        "atomically { 32_bits_at_[%r9] Umin= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));
    insns.push((
        Inst::AtomicRmwSeq {
            ty: types::I64,
            op: AtomicRmwSeqOp::Smax,
            mem: am3.clone(),
            operand: Gpr::unwrap_new(r10),
            temp: w_r11.map(Gpr::unwrap_new),
            dst_old: w_rax.map(Gpr::unwrap_new)
        },
        "498B014989C34D39DA4D0F4DDAF04D0FB1190F85EBFFFFFF",
        "atomically { 64_bits_at_[%r9] Smax= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }"
    ));

    // Atomic128RmwSeq
    insns.push((
        Inst::Atomic128RmwSeq {
            op: Atomic128RmwSeqOp::Or,
            mem: Box::new(am3.clone()),
            operand_low: Gpr::unwrap_new(r10),
            operand_high: Gpr::unwrap_new(r11),
            temp_low: w_rbx.map(Gpr::unwrap_new),
            temp_high: w_rcx.map(Gpr::unwrap_new),
            dst_old_low: w_rax.map(Gpr::unwrap_new),
            dst_old_high: w_rdx.map(Gpr::unwrap_new),
        },
        "498B01498B51084889C34889D1490BDA490BCBF0490FC7090F85E9FFFFFF",
        "atomically { %rdx:%rax = 0(%r9); %rcx:%rbx = %rdx:%rax Or %r11:%r10; 0(%r9) = %rcx:%rbx }",
    ));
    insns.push((
        Inst::Atomic128RmwSeq {
            op: Atomic128RmwSeqOp::And,
            mem: Box::new(am3.clone()),
            operand_low: Gpr::unwrap_new(r10),
            operand_high: Gpr::unwrap_new(r11),
            temp_low: w_rbx.map(Gpr::unwrap_new),
            temp_high: w_rcx.map(Gpr::unwrap_new),
            dst_old_low: w_rax.map(Gpr::unwrap_new),
            dst_old_high: w_rdx.map(Gpr::unwrap_new),
        },
        "498B01498B51084889C34889D14923DA4923CBF0490FC7090F85E9FFFFFF",
        "atomically { %rdx:%rax = 0(%r9); %rcx:%rbx = %rdx:%rax And %r11:%r10; 0(%r9) = %rcx:%rbx }"
    ));
    insns.push((
        Inst::Atomic128RmwSeq {
            op: Atomic128RmwSeqOp::Umin,
            mem: Box::new(am3.clone()),
            operand_low: Gpr::unwrap_new(r10),
            operand_high: Gpr::unwrap_new(r11),
            temp_low: w_rbx.map(Gpr::unwrap_new),
            temp_high: w_rcx.map(Gpr::unwrap_new),
            dst_old_low: w_rax.map(Gpr::unwrap_new),
            dst_old_high: w_rdx.map(Gpr::unwrap_new),
        },
        "498B01498B51084889C34889D14C39D3491BCB4889D1490F43DA490F43CBF0490FC7090F85DEFFFFFF",
        "atomically { %rdx:%rax = 0(%r9); %rcx:%rbx = %rdx:%rax Umin %r11:%r10; 0(%r9) = %rcx:%rbx }"
    ));
    insns.push((
        Inst::Atomic128RmwSeq {
            op: Atomic128RmwSeqOp::Add,
            mem: Box::new(am3.clone()),
            operand_low: Gpr::unwrap_new(r10),
            operand_high: Gpr::unwrap_new(r11),
            temp_low: w_rbx.map(Gpr::unwrap_new),
            temp_high: w_rcx.map(Gpr::unwrap_new),
            dst_old_low: w_rax.map(Gpr::unwrap_new),
            dst_old_high: w_rdx.map(Gpr::unwrap_new),
        },
        "498B01498B51084889C34889D14903DA4913CBF0490FC7090F85E9FFFFFF",
        "atomically { %rdx:%rax = 0(%r9); %rcx:%rbx = %rdx:%rax Add %r11:%r10; 0(%r9) = %rcx:%rbx }"
    ));
    insns.push((
        Inst::Atomic128XchgSeq {
            mem: am3.clone(),
            operand_low: Gpr::unwrap_new(rbx),
            operand_high: Gpr::unwrap_new(rcx),
            dst_old_low: w_rax.map(Gpr::unwrap_new),
            dst_old_high: w_rdx.map(Gpr::unwrap_new),
        },
        "498B01498B5108F0490FC7090F85F5FFFFFF",
        "atomically { %rdx:%rax = 0(%r9); 0(%r9) = %rcx:%rbx }",
    ));

    // ========================================================
    // Misc instructions.

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

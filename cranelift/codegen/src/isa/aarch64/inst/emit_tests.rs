use crate::ir::types::*;
use crate::isa::aarch64::inst::*;
use crate::isa::test_utils;
use crate::settings;

use alloc::boxed::Box;
use alloc::vec::Vec;

#[test]
fn test_aarch64_binemit() {
    let flags = settings::Flags::new(settings::builder());
    let mut insns = Vec::<(Inst, &str, &str)>::new();

    // N.B.: the architecture is little-endian, so when transcribing the 32-bit
    // hex instructions from e.g. objdump disassembly, one must swap the bytes
    // seen below. (E.g., a `ret` is normally written as the u32 `D65F03C0`,
    // but we write it here as C0035FD6.)

    // Useful helper script to produce the encodings from the text:
    //
    //      #!/bin/sh
    //      tmp=`mktemp /tmp/XXXXXXXX.o`
    //      aarch64-linux-gnu-as /dev/stdin -o $tmp
    //      aarch64-linux-gnu-objdump -d $tmp
    //      rm -f $tmp
    //
    // Then:
    //
    //      $ echo "mov x1, x2" | aarch64inst.sh
    insns.push((Inst::Ret, "C0035FD6", "ret"));
    insns.push((Inst::Nop0, "", "nop-zero-len"));
    insns.push((Inst::Nop4, "1F2003D5", "nop"));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Add32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        },
        "4100030B",
        "add w1, w2, w3",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Add64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A400068B",
        "add x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Sub32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        },
        "4100034B",
        "sub w1, w2, w3",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Sub64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A40006CB",
        "sub x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Orr32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        },
        "4100032A",
        "orr w1, w2, w3",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Orr64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A40006AA",
        "orr x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::And32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        },
        "4100030A",
        "and w1, w2, w3",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::And64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A400068A",
        "and x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::SubS32,
            rd: writable_zero_reg(),
            rn: xreg(2),
            rm: xreg(3),
        },
        "5F00036B",
        // TODO: Display as cmp
        "subs wzr, w2, w3",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::SubS32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        },
        "4100036B",
        "subs w1, w2, w3",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::SubS64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A40006EB",
        "subs x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::AddS32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        },
        "4100032B",
        "adds w1, w2, w3",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::AddS64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A40006AB",
        "adds x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRImm12 {
            alu_op: ALUOp::AddS64,
            rd: writable_zero_reg(),
            rn: xreg(5),
            imm12: Imm12::maybe_from_u64(1).unwrap(),
        },
        "BF0400B1",
        // TODO: Display as cmn.
        "adds xzr, x5, #1",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::SDiv64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A40CC69A",
        "sdiv x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::UDiv64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A408C69A",
        "udiv x4, x5, x6",
    ));

    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Eor32,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A400064A",
        "eor w4, w5, w6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Eor64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A40006CA",
        "eor x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::AndNot32,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A400260A",
        "bic w4, w5, w6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::AndNot64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A400268A",
        "bic x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::OrrNot32,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A400262A",
        "orn w4, w5, w6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::OrrNot64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A40026AA",
        "orn x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::EorNot32,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A400264A",
        "eon w4, w5, w6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::EorNot64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A40026CA",
        "eon x4, x5, x6",
    ));

    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::RotR32,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A42CC61A",
        "ror w4, w5, w6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::RotR64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A42CC69A",
        "ror x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Lsr32,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A424C61A",
        "lsr w4, w5, w6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Lsr64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A424C69A",
        "lsr x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Asr32,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A428C61A",
        "asr w4, w5, w6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Asr64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A428C69A",
        "asr x4, x5, x6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Lsl32,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A420C61A",
        "lsl w4, w5, w6",
    ));
    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::Lsl64,
            rd: writable_xreg(4),
            rn: xreg(5),
            rm: xreg(6),
        },
        "A420C69A",
        "lsl x4, x5, x6",
    ));

    insns.push((
        Inst::AluRRImm12 {
            alu_op: ALUOp::Add32,
            rd: writable_xreg(7),
            rn: xreg(8),
            imm12: Imm12 {
                bits: 0x123,
                shift12: false,
            },
        },
        "078D0411",
        "add w7, w8, #291",
    ));
    insns.push((
        Inst::AluRRImm12 {
            alu_op: ALUOp::Add32,
            rd: writable_xreg(7),
            rn: xreg(8),
            imm12: Imm12 {
                bits: 0x123,
                shift12: true,
            },
        },
        "078D4411",
        "add w7, w8, #1191936",
    ));
    insns.push((
        Inst::AluRRImm12 {
            alu_op: ALUOp::Add64,
            rd: writable_xreg(7),
            rn: xreg(8),
            imm12: Imm12 {
                bits: 0x123,
                shift12: false,
            },
        },
        "078D0491",
        "add x7, x8, #291",
    ));
    insns.push((
        Inst::AluRRImm12 {
            alu_op: ALUOp::Sub32,
            rd: writable_xreg(7),
            rn: xreg(8),
            imm12: Imm12 {
                bits: 0x123,
                shift12: false,
            },
        },
        "078D0451",
        "sub w7, w8, #291",
    ));
    insns.push((
        Inst::AluRRImm12 {
            alu_op: ALUOp::Sub64,
            rd: writable_xreg(7),
            rn: xreg(8),
            imm12: Imm12 {
                bits: 0x123,
                shift12: false,
            },
        },
        "078D04D1",
        "sub x7, x8, #291",
    ));
    insns.push((
        Inst::AluRRImm12 {
            alu_op: ALUOp::SubS32,
            rd: writable_xreg(7),
            rn: xreg(8),
            imm12: Imm12 {
                bits: 0x123,
                shift12: false,
            },
        },
        "078D0471",
        "subs w7, w8, #291",
    ));
    insns.push((
        Inst::AluRRImm12 {
            alu_op: ALUOp::SubS64,
            rd: writable_xreg(7),
            rn: xreg(8),
            imm12: Imm12 {
                bits: 0x123,
                shift12: false,
            },
        },
        "078D04F1",
        "subs x7, x8, #291",
    ));

    insns.push((
        Inst::AluRRRExtend {
            alu_op: ALUOp::Add32,
            rd: writable_xreg(7),
            rn: xreg(8),
            rm: xreg(9),
            extendop: ExtendOp::SXTB,
        },
        "0781290B",
        "add w7, w8, w9, SXTB",
    ));

    insns.push((
        Inst::AluRRRExtend {
            alu_op: ALUOp::Add64,
            rd: writable_xreg(15),
            rn: xreg(16),
            rm: xreg(17),
            extendop: ExtendOp::UXTB,
        },
        "0F02318B",
        "add x15, x16, x17, UXTB",
    ));

    insns.push((
        Inst::AluRRRExtend {
            alu_op: ALUOp::Sub32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
            extendop: ExtendOp::SXTH,
        },
        "41A0234B",
        "sub w1, w2, w3, SXTH",
    ));

    insns.push((
        Inst::AluRRRExtend {
            alu_op: ALUOp::Sub64,
            rd: writable_xreg(20),
            rn: xreg(21),
            rm: xreg(22),
            extendop: ExtendOp::UXTW,
        },
        "B44236CB",
        "sub x20, x21, x22, UXTW",
    ));

    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::Add32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(20).unwrap(),
            ),
        },
        "6A510C0B",
        "add w10, w11, w12, LSL 20",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::Add64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::ASR,
                ShiftOpShiftImm::maybe_from_shift(42).unwrap(),
            ),
        },
        "6AA98C8B",
        "add x10, x11, x12, ASR 42",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::Sub32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0C4B",
        "sub w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::Sub64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0CCB",
        "sub x10, x11, x12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::Orr32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0C2A",
        "orr w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::Orr64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0CAA",
        "orr x10, x11, x12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::And32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0C0A",
        "and w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::And64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0C8A",
        "and x10, x11, x12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::Eor32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0C4A",
        "eor w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::Eor64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0CCA",
        "eor x10, x11, x12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::OrrNot32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D2C2A",
        "orn w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::OrrNot64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D2CAA",
        "orn x10, x11, x12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::AndNot32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D2C0A",
        "bic w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::AndNot64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D2C8A",
        "bic x10, x11, x12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::EorNot32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D2C4A",
        "eon w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::EorNot64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D2CCA",
        "eon x10, x11, x12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::AddS32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0C2B",
        "adds w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::AddS64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0CAB",
        "adds x10, x11, x12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::SubS32,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0C6B",
        "subs w10, w11, w12, LSL 23",
    ));
    insns.push((
        Inst::AluRRRShift {
            alu_op: ALUOp::SubS64,
            rd: writable_xreg(10),
            rn: xreg(11),
            rm: xreg(12),
            shiftop: ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(23).unwrap(),
            ),
        },
        "6A5D0CEB",
        "subs x10, x11, x12, LSL 23",
    ));

    insns.push((
        Inst::AluRRR {
            alu_op: ALUOp::SubS64XR,
            rd: writable_zero_reg(),
            rn: stack_reg(),
            rm: xreg(12),
        },
        "FF632CEB",
        "subs xzr, sp, x12",
    ));

    insns.push((
        Inst::AluRRRR {
            alu_op: ALUOp::MAdd32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
            ra: xreg(4),
        },
        "4110031B",
        "madd w1, w2, w3, w4",
    ));
    insns.push((
        Inst::AluRRRR {
            alu_op: ALUOp::MAdd64,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
            ra: xreg(4),
        },
        "4110039B",
        "madd x1, x2, x3, x4",
    ));
    insns.push((
        Inst::AluRRRR {
            alu_op: ALUOp::MSub32,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
            ra: xreg(4),
        },
        "4190031B",
        "msub w1, w2, w3, w4",
    ));
    insns.push((
        Inst::AluRRRR {
            alu_op: ALUOp::MSub64,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
            ra: xreg(4),
        },
        "4190039B",
        "msub x1, x2, x3, x4",
    ));
    insns.push((
        Inst::AluRRRR {
            alu_op: ALUOp::SMulH,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
            ra: zero_reg(),
        },
        "417C439B",
        "smulh x1, x2, x3",
    ));
    insns.push((
        Inst::AluRRRR {
            alu_op: ALUOp::UMulH,
            rd: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
            ra: zero_reg(),
        },
        "417CC39B",
        "umulh x1, x2, x3",
    ));

    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::RotR32,
            rd: writable_xreg(20),
            rn: xreg(21),
            immshift: ImmShift::maybe_from_u64(19).unwrap(),
        },
        "B44E9513",
        "ror w20, w21, #19",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::RotR64,
            rd: writable_xreg(20),
            rn: xreg(21),
            immshift: ImmShift::maybe_from_u64(42).unwrap(),
        },
        "B4AAD593",
        "ror x20, x21, #42",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::Lsr32,
            rd: writable_xreg(10),
            rn: xreg(11),
            immshift: ImmShift::maybe_from_u64(13).unwrap(),
        },
        "6A7D0D53",
        "lsr w10, w11, #13",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::Lsr64,
            rd: writable_xreg(10),
            rn: xreg(11),
            immshift: ImmShift::maybe_from_u64(57).unwrap(),
        },
        "6AFD79D3",
        "lsr x10, x11, #57",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::Asr32,
            rd: writable_xreg(4),
            rn: xreg(5),
            immshift: ImmShift::maybe_from_u64(7).unwrap(),
        },
        "A47C0713",
        "asr w4, w5, #7",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::Asr64,
            rd: writable_xreg(4),
            rn: xreg(5),
            immshift: ImmShift::maybe_from_u64(35).unwrap(),
        },
        "A4FC6393",
        "asr x4, x5, #35",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::Lsl32,
            rd: writable_xreg(8),
            rn: xreg(9),
            immshift: ImmShift::maybe_from_u64(24).unwrap(),
        },
        "281D0853",
        "lsl w8, w9, #24",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::Lsl64,
            rd: writable_xreg(8),
            rn: xreg(9),
            immshift: ImmShift::maybe_from_u64(63).unwrap(),
        },
        "280141D3",
        "lsl x8, x9, #63",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::Lsl32,
            rd: writable_xreg(10),
            rn: xreg(11),
            immshift: ImmShift::maybe_from_u64(0).unwrap(),
        },
        "6A7D0053",
        "lsl w10, w11, #0",
    ));
    insns.push((
        Inst::AluRRImmShift {
            alu_op: ALUOp::Lsl64,
            rd: writable_xreg(10),
            rn: xreg(11),
            immshift: ImmShift::maybe_from_u64(0).unwrap(),
        },
        "6AFD40D3",
        "lsl x10, x11, #0",
    ));

    insns.push((
        Inst::AluRRImmLogic {
            alu_op: ALUOp::And32,
            rd: writable_xreg(21),
            rn: xreg(27),
            imml: ImmLogic::maybe_from_u64(0x80003fff, I32).unwrap(),
        },
        "753B0112",
        "and w21, w27, #2147500031",
    ));
    insns.push((
        Inst::AluRRImmLogic {
            alu_op: ALUOp::And64,
            rd: writable_xreg(7),
            rn: xreg(6),
            imml: ImmLogic::maybe_from_u64(0x3fff80003fff800, I64).unwrap(),
        },
        "C7381592",
        "and x7, x6, #288221580125796352",
    ));
    insns.push((
        Inst::AluRRImmLogic {
            alu_op: ALUOp::Orr32,
            rd: writable_xreg(1),
            rn: xreg(5),
            imml: ImmLogic::maybe_from_u64(0x100000, I32).unwrap(),
        },
        "A1000C32",
        "orr w1, w5, #1048576",
    ));
    insns.push((
        Inst::AluRRImmLogic {
            alu_op: ALUOp::Orr64,
            rd: writable_xreg(4),
            rn: xreg(5),
            imml: ImmLogic::maybe_from_u64(0x8181818181818181, I64).unwrap(),
        },
        "A4C401B2",
        "orr x4, x5, #9331882296111890817",
    ));
    insns.push((
        Inst::AluRRImmLogic {
            alu_op: ALUOp::Eor32,
            rd: writable_xreg(1),
            rn: xreg(5),
            imml: ImmLogic::maybe_from_u64(0x00007fff, I32).unwrap(),
        },
        "A1380052",
        "eor w1, w5, #32767",
    ));
    insns.push((
        Inst::AluRRImmLogic {
            alu_op: ALUOp::Eor64,
            rd: writable_xreg(10),
            rn: xreg(8),
            imml: ImmLogic::maybe_from_u64(0x8181818181818181, I64).unwrap(),
        },
        "0AC501D2",
        "eor x10, x8, #9331882296111890817",
    ));

    insns.push((
        Inst::BitRR {
            op: BitOp::RBit32,
            rd: writable_xreg(1),
            rn: xreg(10),
        },
        "4101C05A",
        "rbit w1, w10",
    ));

    insns.push((
        Inst::BitRR {
            op: BitOp::RBit64,
            rd: writable_xreg(1),
            rn: xreg(10),
        },
        "4101C0DA",
        "rbit x1, x10",
    ));

    insns.push((
        Inst::BitRR {
            op: BitOp::Clz32,
            rd: writable_xreg(15),
            rn: xreg(3),
        },
        "6F10C05A",
        "clz w15, w3",
    ));

    insns.push((
        Inst::BitRR {
            op: BitOp::Clz64,
            rd: writable_xreg(15),
            rn: xreg(3),
        },
        "6F10C0DA",
        "clz x15, x3",
    ));

    insns.push((
        Inst::BitRR {
            op: BitOp::Cls32,
            rd: writable_xreg(21),
            rn: xreg(16),
        },
        "1516C05A",
        "cls w21, w16",
    ));

    insns.push((
        Inst::BitRR {
            op: BitOp::Cls64,
            rd: writable_xreg(21),
            rn: xreg(16),
        },
        "1516C0DA",
        "cls x21, x16",
    ));

    insns.push((
        Inst::ULoad8 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "41004038",
        "ldurb w1, [x2]",
    ));
    insns.push((
        Inst::ULoad8 {
            rd: writable_xreg(1),
            mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::zero(I8)),
            srcloc: None,
        },
        "41004039",
        "ldrb w1, [x2]",
    ));
    insns.push((
        Inst::ULoad8 {
            rd: writable_xreg(1),
            mem: MemArg::RegReg(xreg(2), xreg(5)),
            srcloc: None,
        },
        "41686538",
        "ldrb w1, [x2, x5]",
    ));
    insns.push((
        Inst::SLoad8 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "41008038",
        "ldursb x1, [x2]",
    ));
    insns.push((
        Inst::SLoad8 {
            rd: writable_xreg(1),
            mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(63, I8).unwrap()),
            srcloc: None,
        },
        "41FC8039",
        "ldrsb x1, [x2, #63]",
    ));
    insns.push((
        Inst::SLoad8 {
            rd: writable_xreg(1),
            mem: MemArg::RegReg(xreg(2), xreg(5)),
            srcloc: None,
        },
        "4168A538",
        "ldrsb x1, [x2, x5]",
    ));
    insns.push((
        Inst::ULoad16 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::maybe_from_i64(5).unwrap()),
            srcloc: None,
        },
        "41504078",
        "ldurh w1, [x2, #5]",
    ));
    insns.push((
        Inst::ULoad16 {
            rd: writable_xreg(1),
            mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(8, I16).unwrap()),
            srcloc: None,
        },
        "41104079",
        "ldrh w1, [x2, #8]",
    ));
    insns.push((
        Inst::ULoad16 {
            rd: writable_xreg(1),
            mem: MemArg::RegScaled(xreg(2), xreg(3), I16),
            srcloc: None,
        },
        "41786378",
        "ldrh w1, [x2, x3, LSL #1]",
    ));
    insns.push((
        Inst::SLoad16 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "41008078",
        "ldursh x1, [x2]",
    ));
    insns.push((
        Inst::SLoad16 {
            rd: writable_xreg(28),
            mem: MemArg::UnsignedOffset(xreg(20), UImm12Scaled::maybe_from_i64(24, I16).unwrap()),
            srcloc: None,
        },
        "9C328079",
        "ldrsh x28, [x20, #24]",
    ));
    insns.push((
        Inst::SLoad16 {
            rd: writable_xreg(28),
            mem: MemArg::RegScaled(xreg(20), xreg(20), I16),
            srcloc: None,
        },
        "9C7AB478",
        "ldrsh x28, [x20, x20, LSL #1]",
    ));
    insns.push((
        Inst::ULoad32 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "410040B8",
        "ldur w1, [x2]",
    ));
    insns.push((
        Inst::ULoad32 {
            rd: writable_xreg(12),
            mem: MemArg::UnsignedOffset(xreg(0), UImm12Scaled::maybe_from_i64(204, I32).unwrap()),
            srcloc: None,
        },
        "0CCC40B9",
        "ldr w12, [x0, #204]",
    ));
    insns.push((
        Inst::ULoad32 {
            rd: writable_xreg(1),
            mem: MemArg::RegScaled(xreg(2), xreg(12), I32),
            srcloc: None,
        },
        "41786CB8",
        "ldr w1, [x2, x12, LSL #2]",
    ));
    insns.push((
        Inst::SLoad32 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "410080B8",
        "ldursw x1, [x2]",
    ));
    insns.push((
        Inst::SLoad32 {
            rd: writable_xreg(12),
            mem: MemArg::UnsignedOffset(xreg(1), UImm12Scaled::maybe_from_i64(16380, I32).unwrap()),
            srcloc: None,
        },
        "2CFCBFB9",
        "ldrsw x12, [x1, #16380]",
    ));
    insns.push((
        Inst::SLoad32 {
            rd: writable_xreg(1),
            mem: MemArg::RegScaled(xreg(5), xreg(1), I32),
            srcloc: None,
        },
        "A178A1B8",
        "ldrsw x1, [x5, x1, LSL #2]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "410040F8",
        "ldur x1, [x2]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::maybe_from_i64(-256).unwrap()),
            srcloc: None,
        },
        "410050F8",
        "ldur x1, [x2, #-256]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::maybe_from_i64(255).unwrap()),
            srcloc: None,
        },
        "41F04FF8",
        "ldur x1, [x2, #255]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(32760, I64).unwrap()),
            srcloc: None,
        },
        "41FC7FF9",
        "ldr x1, [x2, #32760]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::RegReg(xreg(2), xreg(3)),
            srcloc: None,
        },
        "416863F8",
        "ldr x1, [x2, x3]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::RegScaled(xreg(2), xreg(3), I64),
            srcloc: None,
        },
        "417863F8",
        "ldr x1, [x2, x3, LSL #3]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::RegScaledExtended(xreg(2), xreg(3), I64, ExtendOp::SXTW),
            srcloc: None,
        },
        "41D863F8",
        "ldr x1, [x2, w3, SXTW #3]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::RegExtended(xreg(2), xreg(3), ExtendOp::SXTW),
            srcloc: None,
        },
        "41C863F8",
        "ldr x1, [x2, w3, SXTW]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::Label(MemLabel::PCRel(64)),
            srcloc: None,
        },
        "01020058",
        "ldr x1, pc+64",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::PreIndexed(writable_xreg(2), SImm9::maybe_from_i64(16).unwrap()),
            srcloc: None,
        },
        "410C41F8",
        "ldr x1, [x2, #16]!",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::PostIndexed(writable_xreg(2), SImm9::maybe_from_i64(16).unwrap()),
            srcloc: None,
        },
        "410441F8",
        "ldr x1, [x2], #16",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::FPOffset(32768, I8),
            srcloc: None,
        },
        "100090D2B063308B010240F9",
        "movz x16, #32768 ; add x16, fp, x16, UXTX ; ldr x1, [x16]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::FPOffset(-32768, I8),
            srcloc: None,
        },
        "F0FF8F92B063308B010240F9",
        "movn x16, #32767 ; add x16, fp, x16, UXTX ; ldr x1, [x16]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::FPOffset(1048576, I8), // 2^20
            srcloc: None,
        },
        "1002A0D2B063308B010240F9",
        "movz x16, #16, LSL #16 ; add x16, fp, x16, UXTX ; ldr x1, [x16]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::FPOffset(1048576 + 1, I8), // 2^20 + 1
            srcloc: None,
        },
        "300080D21002A0F2B063308B010240F9",
        "movz x16, #1 ; movk x16, #16, LSL #16 ; add x16, fp, x16, UXTX ; ldr x1, [x16]",
    ));

    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::RegOffset(xreg(7), 8, I64),
            srcloc: None,
        },
        "E18040F8",
        "ldur x1, [x7, #8]",
    ));

    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::RegOffset(xreg(7), 1024, I64),
            srcloc: None,
        },
        "E10042F9",
        "ldr x1, [x7, #1024]",
    ));

    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::RegOffset(xreg(7), 1048576, I64),
            srcloc: None,
        },
        "1002A0D2F060308B010240F9",
        "movz x16, #16, LSL #16 ; add x16, x7, x16, UXTX ; ldr x1, [x16]",
    ));

    insns.push((
        Inst::Store8 {
            rd: xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "41000038",
        "sturb w1, [x2]",
    ));
    insns.push((
        Inst::Store8 {
            rd: xreg(1),
            mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(4095, I8).unwrap()),
            srcloc: None,
        },
        "41FC3F39",
        "strb w1, [x2, #4095]",
    ));
    insns.push((
        Inst::Store16 {
            rd: xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "41000078",
        "sturh w1, [x2]",
    ));
    insns.push((
        Inst::Store16 {
            rd: xreg(1),
            mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(8190, I16).unwrap()),
            srcloc: None,
        },
        "41FC3F79",
        "strh w1, [x2, #8190]",
    ));
    insns.push((
        Inst::Store32 {
            rd: xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "410000B8",
        "stur w1, [x2]",
    ));
    insns.push((
        Inst::Store32 {
            rd: xreg(1),
            mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(16380, I32).unwrap()),
            srcloc: None,
        },
        "41FC3FB9",
        "str w1, [x2, #16380]",
    ));
    insns.push((
        Inst::Store64 {
            rd: xreg(1),
            mem: MemArg::Unscaled(xreg(2), SImm9::zero()),
            srcloc: None,
        },
        "410000F8",
        "stur x1, [x2]",
    ));
    insns.push((
        Inst::Store64 {
            rd: xreg(1),
            mem: MemArg::UnsignedOffset(xreg(2), UImm12Scaled::maybe_from_i64(32760, I64).unwrap()),
            srcloc: None,
        },
        "41FC3FF9",
        "str x1, [x2, #32760]",
    ));
    insns.push((
        Inst::Store64 {
            rd: xreg(1),
            mem: MemArg::RegReg(xreg(2), xreg(3)),
            srcloc: None,
        },
        "416823F8",
        "str x1, [x2, x3]",
    ));
    insns.push((
        Inst::Store64 {
            rd: xreg(1),
            mem: MemArg::RegScaled(xreg(2), xreg(3), I64),
            srcloc: None,
        },
        "417823F8",
        "str x1, [x2, x3, LSL #3]",
    ));
    insns.push((
        Inst::Store64 {
            rd: xreg(1),
            mem: MemArg::RegScaledExtended(xreg(2), xreg(3), I64, ExtendOp::UXTW),
            srcloc: None,
        },
        "415823F8",
        "str x1, [x2, w3, UXTW #3]",
    ));
    insns.push((
        Inst::Store64 {
            rd: xreg(1),
            mem: MemArg::RegExtended(xreg(2), xreg(3), ExtendOp::UXTW),
            srcloc: None,
        },
        "414823F8",
        "str x1, [x2, w3, UXTW]",
    ));
    insns.push((
        Inst::Store64 {
            rd: xreg(1),
            mem: MemArg::PreIndexed(writable_xreg(2), SImm9::maybe_from_i64(16).unwrap()),
            srcloc: None,
        },
        "410C01F8",
        "str x1, [x2, #16]!",
    ));
    insns.push((
        Inst::Store64 {
            rd: xreg(1),
            mem: MemArg::PostIndexed(writable_xreg(2), SImm9::maybe_from_i64(16).unwrap()),
            srcloc: None,
        },
        "410401F8",
        "str x1, [x2], #16",
    ));

    insns.push((
        Inst::StoreP64 {
            rt: xreg(8),
            rt2: xreg(9),
            mem: PairMemArg::SignedOffset(xreg(10), SImm7Scaled::zero(I64)),
        },
        "482500A9",
        "stp x8, x9, [x10]",
    ));
    insns.push((
        Inst::StoreP64 {
            rt: xreg(8),
            rt2: xreg(9),
            mem: PairMemArg::SignedOffset(xreg(10), SImm7Scaled::maybe_from_i64(504, I64).unwrap()),
        },
        "48A51FA9",
        "stp x8, x9, [x10, #504]",
    ));
    insns.push((
        Inst::StoreP64 {
            rt: xreg(8),
            rt2: xreg(9),
            mem: PairMemArg::SignedOffset(xreg(10), SImm7Scaled::maybe_from_i64(-64, I64).unwrap()),
        },
        "48253CA9",
        "stp x8, x9, [x10, #-64]",
    ));
    insns.push((
        Inst::StoreP64 {
            rt: xreg(21),
            rt2: xreg(28),
            mem: PairMemArg::SignedOffset(xreg(1), SImm7Scaled::maybe_from_i64(-512, I64).unwrap()),
        },
        "357020A9",
        "stp x21, x28, [x1, #-512]",
    ));
    insns.push((
        Inst::StoreP64 {
            rt: xreg(8),
            rt2: xreg(9),
            mem: PairMemArg::PreIndexed(
                writable_xreg(10),
                SImm7Scaled::maybe_from_i64(-64, I64).unwrap(),
            ),
        },
        "4825BCA9",
        "stp x8, x9, [x10, #-64]!",
    ));
    insns.push((
        Inst::StoreP64 {
            rt: xreg(15),
            rt2: xreg(16),
            mem: PairMemArg::PostIndexed(
                writable_xreg(20),
                SImm7Scaled::maybe_from_i64(504, I64).unwrap(),
            ),
        },
        "8FC29FA8",
        "stp x15, x16, [x20], #504",
    ));

    insns.push((
        Inst::LoadP64 {
            rt: writable_xreg(8),
            rt2: writable_xreg(9),
            mem: PairMemArg::SignedOffset(xreg(10), SImm7Scaled::zero(I64)),
        },
        "482540A9",
        "ldp x8, x9, [x10]",
    ));
    insns.push((
        Inst::LoadP64 {
            rt: writable_xreg(8),
            rt2: writable_xreg(9),
            mem: PairMemArg::SignedOffset(xreg(10), SImm7Scaled::maybe_from_i64(504, I64).unwrap()),
        },
        "48A55FA9",
        "ldp x8, x9, [x10, #504]",
    ));
    insns.push((
        Inst::LoadP64 {
            rt: writable_xreg(8),
            rt2: writable_xreg(9),
            mem: PairMemArg::SignedOffset(xreg(10), SImm7Scaled::maybe_from_i64(-64, I64).unwrap()),
        },
        "48257CA9",
        "ldp x8, x9, [x10, #-64]",
    ));
    insns.push((
        Inst::LoadP64 {
            rt: writable_xreg(8),
            rt2: writable_xreg(9),
            mem: PairMemArg::SignedOffset(
                xreg(10),
                SImm7Scaled::maybe_from_i64(-512, I64).unwrap(),
            ),
        },
        "482560A9",
        "ldp x8, x9, [x10, #-512]",
    ));
    insns.push((
        Inst::LoadP64 {
            rt: writable_xreg(8),
            rt2: writable_xreg(9),
            mem: PairMemArg::PreIndexed(
                writable_xreg(10),
                SImm7Scaled::maybe_from_i64(-64, I64).unwrap(),
            ),
        },
        "4825FCA9",
        "ldp x8, x9, [x10, #-64]!",
    ));
    insns.push((
        Inst::LoadP64 {
            rt: writable_xreg(8),
            rt2: writable_xreg(25),
            mem: PairMemArg::PostIndexed(
                writable_xreg(12),
                SImm7Scaled::maybe_from_i64(504, I64).unwrap(),
            ),
        },
        "88E5DFA8",
        "ldp x8, x25, [x12], #504",
    ));

    insns.push((
        Inst::Mov {
            rd: writable_xreg(8),
            rm: xreg(9),
        },
        "E80309AA",
        "mov x8, x9",
    ));
    insns.push((
        Inst::Mov32 {
            rd: writable_xreg(8),
            rm: xreg(9),
        },
        "E803092A",
        "mov w8, w9",
    ));

    insns.push((
        Inst::MovZ {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0x0000_0000_0000_ffff).unwrap(),
        },
        "E8FF9FD2",
        "movz x8, #65535",
    ));
    insns.push((
        Inst::MovZ {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0x0000_0000_ffff_0000).unwrap(),
        },
        "E8FFBFD2",
        "movz x8, #65535, LSL #16",
    ));
    insns.push((
        Inst::MovZ {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0x0000_ffff_0000_0000).unwrap(),
        },
        "E8FFDFD2",
        "movz x8, #65535, LSL #32",
    ));
    insns.push((
        Inst::MovZ {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0xffff_0000_0000_0000).unwrap(),
        },
        "E8FFFFD2",
        "movz x8, #65535, LSL #48",
    ));

    insns.push((
        Inst::MovN {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0x0000_0000_0000_ffff).unwrap(),
        },
        "E8FF9F92",
        "movn x8, #65535",
    ));
    insns.push((
        Inst::MovN {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0x0000_0000_ffff_0000).unwrap(),
        },
        "E8FFBF92",
        "movn x8, #65535, LSL #16",
    ));
    insns.push((
        Inst::MovN {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0x0000_ffff_0000_0000).unwrap(),
        },
        "E8FFDF92",
        "movn x8, #65535, LSL #32",
    ));
    insns.push((
        Inst::MovN {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0xffff_0000_0000_0000).unwrap(),
        },
        "E8FFFF92",
        "movn x8, #65535, LSL #48",
    ));

    insns.push((
        Inst::MovK {
            rd: writable_xreg(12),
            imm: MoveWideConst::maybe_from_u64(0x0000_0000_0000_0000).unwrap(),
        },
        "0C0080F2",
        "movk x12, #0",
    ));
    insns.push((
        Inst::MovK {
            rd: writable_xreg(19),
            imm: MoveWideConst::maybe_with_shift(0x0000, 16).unwrap(),
        },
        "1300A0F2",
        "movk x19, #0, LSL #16",
    ));
    insns.push((
        Inst::MovK {
            rd: writable_xreg(3),
            imm: MoveWideConst::maybe_from_u64(0x0000_0000_0000_ffff).unwrap(),
        },
        "E3FF9FF2",
        "movk x3, #65535",
    ));
    insns.push((
        Inst::MovK {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0x0000_0000_ffff_0000).unwrap(),
        },
        "E8FFBFF2",
        "movk x8, #65535, LSL #16",
    ));
    insns.push((
        Inst::MovK {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0x0000_ffff_0000_0000).unwrap(),
        },
        "E8FFDFF2",
        "movk x8, #65535, LSL #32",
    ));
    insns.push((
        Inst::MovK {
            rd: writable_xreg(8),
            imm: MoveWideConst::maybe_from_u64(0xffff_0000_0000_0000).unwrap(),
        },
        "E8FFFFF2",
        "movk x8, #65535, LSL #48",
    ));

    insns.push((
        Inst::CSel {
            rd: writable_xreg(10),
            rn: xreg(12),
            rm: xreg(14),
            cond: Cond::Hs,
        },
        "8A218E9A",
        "csel x10, x12, x14, hs",
    ));
    insns.push((
        Inst::CSet {
            rd: writable_xreg(15),
            cond: Cond::Ge,
        },
        "EFB79F9A",
        "cset x15, ge",
    ));
    insns.push((
        Inst::CCmpImm {
            size: OperandSize::Size64,
            rn: xreg(22),
            imm: UImm5::maybe_from_u8(5).unwrap(),
            nzcv: NZCV::new(false, false, true, true),
            cond: Cond::Eq,
        },
        "C30A45FA",
        "ccmp x22, #5, #nzCV, eq",
    ));
    insns.push((
        Inst::CCmpImm {
            size: OperandSize::Size32,
            rn: xreg(3),
            imm: UImm5::maybe_from_u8(30).unwrap(),
            nzcv: NZCV::new(true, true, true, true),
            cond: Cond::Gt,
        },
        "6FC85E7A",
        "ccmp w3, #30, #NZCV, gt",
    ));
    insns.push((
        Inst::MovToVec64 {
            rd: writable_vreg(20),
            rn: xreg(21),
        },
        "B41E084E",
        "mov v20.d[0], x21",
    ));
    insns.push((
        Inst::MovFromVec {
            rd: writable_xreg(3),
            rn: vreg(27),
            idx: 14,
            size: VectorSize::Size8x16,
        },
        "633F1D0E",
        "umov w3, v27.b[14]",
    ));
    insns.push((
        Inst::MovFromVec {
            rd: writable_xreg(24),
            rn: vreg(5),
            idx: 3,
            size: VectorSize::Size16x8,
        },
        "B83C0E0E",
        "umov w24, v5.h[3]",
    ));
    insns.push((
        Inst::MovFromVec {
            rd: writable_xreg(12),
            rn: vreg(17),
            idx: 1,
            size: VectorSize::Size32x4,
        },
        "2C3E0C0E",
        "mov w12, v17.s[1]",
    ));
    insns.push((
        Inst::MovFromVec {
            rd: writable_xreg(21),
            rn: vreg(20),
            idx: 0,
            size: VectorSize::Size64x2,
        },
        "953E084E",
        "mov x21, v20.d[0]",
    ));
    insns.push((
        Inst::MovFromVecSigned {
            rd: writable_xreg(0),
            rn: vreg(0),
            idx: 15,
            size: VectorSize::Size8x16,
            scalar_size: OperandSize::Size32,
        },
        "002C1F0E",
        "smov w0, v0.b[15]",
    ));
    insns.push((
        Inst::MovFromVecSigned {
            rd: writable_xreg(12),
            rn: vreg(13),
            idx: 7,
            size: VectorSize::Size8x8,
            scalar_size: OperandSize::Size64,
        },
        "AC2D0F4E",
        "smov x12, v13.b[7]",
    ));
    insns.push((
        Inst::MovFromVecSigned {
            rd: writable_xreg(23),
            rn: vreg(31),
            idx: 7,
            size: VectorSize::Size16x8,
            scalar_size: OperandSize::Size32,
        },
        "F72F1E0E",
        "smov w23, v31.h[7]",
    ));
    insns.push((
        Inst::MovFromVecSigned {
            rd: writable_xreg(24),
            rn: vreg(5),
            idx: 1,
            size: VectorSize::Size32x2,
            scalar_size: OperandSize::Size64,
        },
        "B82C0C4E",
        "smov x24, v5.s[1]",
    ));
    insns.push((
        Inst::MovToNZCV { rn: xreg(13) },
        "0D421BD5",
        "msr nzcv, x13",
    ));
    insns.push((
        Inst::MovFromNZCV {
            rd: writable_xreg(27),
        },
        "1B423BD5",
        "mrs x27, nzcv",
    ));
    insns.push((
        Inst::CondSet {
            rd: writable_xreg(5),
            cond: Cond::Hi,
        },
        "E5979F9A",
        "cset x5, hi",
    ));
    insns.push((
        Inst::VecDup {
            rd: writable_vreg(25),
            rn: xreg(7),
            size: VectorSize::Size8x16,
        },
        "F90C014E",
        "dup v25.16b, w7",
    ));
    insns.push((
        Inst::VecDup {
            rd: writable_vreg(2),
            rn: xreg(23),
            size: VectorSize::Size16x8,
        },
        "E20E024E",
        "dup v2.8h, w23",
    ));
    insns.push((
        Inst::VecDup {
            rd: writable_vreg(0),
            rn: xreg(28),
            size: VectorSize::Size32x4,
        },
        "800F044E",
        "dup v0.4s, w28",
    ));
    insns.push((
        Inst::VecDup {
            rd: writable_vreg(31),
            rn: xreg(5),
            size: VectorSize::Size64x2,
        },
        "BF0C084E",
        "dup v31.2d, x5",
    ));
    insns.push((
        Inst::VecDupFromFpu {
            rd: writable_vreg(14),
            rn: vreg(19),
            size: VectorSize::Size32x4,
        },
        "6E06044E",
        "dup v14.4s, v19.s[0]",
    ));
    insns.push((
        Inst::VecDupFromFpu {
            rd: writable_vreg(18),
            rn: vreg(10),
            size: VectorSize::Size64x2,
        },
        "5205084E",
        "dup v18.2d, v10.d[0]",
    ));
    insns.push((
        Inst::VecExtend {
            t: VecExtendOp::Sxtl8,
            rd: writable_vreg(4),
            rn: vreg(27),
        },
        "64A7080F",
        "sxtl v4.8h, v27.8b",
    ));
    insns.push((
        Inst::VecExtend {
            t: VecExtendOp::Sxtl16,
            rd: writable_vreg(17),
            rn: vreg(19),
        },
        "71A6100F",
        "sxtl v17.4s, v19.4h",
    ));
    insns.push((
        Inst::VecExtend {
            t: VecExtendOp::Sxtl32,
            rd: writable_vreg(30),
            rn: vreg(6),
        },
        "DEA4200F",
        "sxtl v30.2d, v6.2s",
    ));
    insns.push((
        Inst::VecExtend {
            t: VecExtendOp::Uxtl8,
            rd: writable_vreg(3),
            rn: vreg(29),
        },
        "A3A7082F",
        "uxtl v3.8h, v29.8b",
    ));
    insns.push((
        Inst::VecExtend {
            t: VecExtendOp::Uxtl16,
            rd: writable_vreg(15),
            rn: vreg(12),
        },
        "8FA5102F",
        "uxtl v15.4s, v12.4h",
    ));
    insns.push((
        Inst::VecExtend {
            t: VecExtendOp::Uxtl32,
            rd: writable_vreg(28),
            rn: vreg(2),
        },
        "5CA4202F",
        "uxtl v28.2d, v2.2s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sqadd,
            rd: writable_vreg(1),
            rn: vreg(2),
            rm: vreg(8),
            size: VectorSize::Size8x16,
        },
        "410C284E",
        "sqadd v1.16b, v2.16b, v8.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sqadd,
            rd: writable_vreg(1),
            rn: vreg(12),
            rm: vreg(28),
            size: VectorSize::Size16x8,
        },
        "810D7C4E",
        "sqadd v1.8h, v12.8h, v28.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sqadd,
            rd: writable_vreg(12),
            rn: vreg(2),
            rm: vreg(6),
            size: VectorSize::Size32x4,
        },
        "4C0CA64E",
        "sqadd v12.4s, v2.4s, v6.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sqadd,
            rd: writable_vreg(20),
            rn: vreg(7),
            rm: vreg(13),
            size: VectorSize::Size64x2,
        },
        "F40CED4E",
        "sqadd v20.2d, v7.2d, v13.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sqsub,
            rd: writable_vreg(1),
            rn: vreg(2),
            rm: vreg(8),
            size: VectorSize::Size8x16,
        },
        "412C284E",
        "sqsub v1.16b, v2.16b, v8.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sqsub,
            rd: writable_vreg(1),
            rn: vreg(12),
            rm: vreg(28),
            size: VectorSize::Size16x8,
        },
        "812D7C4E",
        "sqsub v1.8h, v12.8h, v28.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sqsub,
            rd: writable_vreg(12),
            rn: vreg(2),
            rm: vreg(6),
            size: VectorSize::Size32x4,
        },
        "4C2CA64E",
        "sqsub v12.4s, v2.4s, v6.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sqsub,
            rd: writable_vreg(20),
            rn: vreg(7),
            rm: vreg(13),
            size: VectorSize::Size64x2,
        },
        "F42CED4E",
        "sqsub v20.2d, v7.2d, v13.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Uqadd,
            rd: writable_vreg(1),
            rn: vreg(2),
            rm: vreg(8),
            size: VectorSize::Size8x16,
        },
        "410C286E",
        "uqadd v1.16b, v2.16b, v8.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Uqadd,
            rd: writable_vreg(1),
            rn: vreg(12),
            rm: vreg(28),
            size: VectorSize::Size16x8,
        },
        "810D7C6E",
        "uqadd v1.8h, v12.8h, v28.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Uqadd,
            rd: writable_vreg(12),
            rn: vreg(2),
            rm: vreg(6),
            size: VectorSize::Size32x4,
        },
        "4C0CA66E",
        "uqadd v12.4s, v2.4s, v6.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Uqadd,
            rd: writable_vreg(20),
            rn: vreg(7),
            rm: vreg(13),
            size: VectorSize::Size64x2,
        },
        "F40CED6E",
        "uqadd v20.2d, v7.2d, v13.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Uqsub,
            rd: writable_vreg(1),
            rn: vreg(2),
            rm: vreg(8),
            size: VectorSize::Size8x16,
        },
        "412C286E",
        "uqsub v1.16b, v2.16b, v8.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Uqsub,
            rd: writable_vreg(1),
            rn: vreg(12),
            rm: vreg(28),
            size: VectorSize::Size16x8,
        },
        "812D7C6E",
        "uqsub v1.8h, v12.8h, v28.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Uqsub,
            rd: writable_vreg(12),
            rn: vreg(2),
            rm: vreg(6),
            size: VectorSize::Size32x4,
        },
        "4C2CA66E",
        "uqsub v12.4s, v2.4s, v6.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Uqsub,
            rd: writable_vreg(20),
            rn: vreg(7),
            rm: vreg(13),
            size: VectorSize::Size64x2,
        },
        "F42CED6E",
        "uqsub v20.2d, v7.2d, v13.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmeq,
            rd: writable_vreg(3),
            rn: vreg(23),
            rm: vreg(24),
            size: VectorSize::Size8x16,
        },
        "E38E386E",
        "cmeq v3.16b, v23.16b, v24.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmgt,
            rd: writable_vreg(3),
            rn: vreg(23),
            rm: vreg(24),
            size: VectorSize::Size8x16,
        },
        "E336384E",
        "cmgt v3.16b, v23.16b, v24.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmge,
            rd: writable_vreg(23),
            rn: vreg(9),
            rm: vreg(12),
            size: VectorSize::Size8x16,
        },
        "373D2C4E",
        "cmge v23.16b, v9.16b, v12.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmhi,
            rd: writable_vreg(5),
            rn: vreg(1),
            rm: vreg(1),
            size: VectorSize::Size8x16,
        },
        "2534216E",
        "cmhi v5.16b, v1.16b, v1.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmhs,
            rd: writable_vreg(8),
            rn: vreg(2),
            rm: vreg(15),
            size: VectorSize::Size8x16,
        },
        "483C2F6E",
        "cmhs v8.16b, v2.16b, v15.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmeq,
            rd: writable_vreg(3),
            rn: vreg(23),
            rm: vreg(24),
            size: VectorSize::Size16x8,
        },
        "E38E786E",
        "cmeq v3.8h, v23.8h, v24.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmgt,
            rd: writable_vreg(3),
            rn: vreg(23),
            rm: vreg(24),
            size: VectorSize::Size16x8,
        },
        "E336784E",
        "cmgt v3.8h, v23.8h, v24.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmge,
            rd: writable_vreg(23),
            rn: vreg(9),
            rm: vreg(12),
            size: VectorSize::Size16x8,
        },
        "373D6C4E",
        "cmge v23.8h, v9.8h, v12.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmhi,
            rd: writable_vreg(5),
            rn: vreg(1),
            rm: vreg(1),
            size: VectorSize::Size16x8,
        },
        "2534616E",
        "cmhi v5.8h, v1.8h, v1.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmhs,
            rd: writable_vreg(8),
            rn: vreg(2),
            rm: vreg(15),
            size: VectorSize::Size16x8,
        },
        "483C6F6E",
        "cmhs v8.8h, v2.8h, v15.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmeq,
            rd: writable_vreg(3),
            rn: vreg(23),
            rm: vreg(24),
            size: VectorSize::Size32x4,
        },
        "E38EB86E",
        "cmeq v3.4s, v23.4s, v24.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmgt,
            rd: writable_vreg(3),
            rn: vreg(23),
            rm: vreg(24),
            size: VectorSize::Size32x4,
        },
        "E336B84E",
        "cmgt v3.4s, v23.4s, v24.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmge,
            rd: writable_vreg(23),
            rn: vreg(9),
            rm: vreg(12),
            size: VectorSize::Size32x4,
        },
        "373DAC4E",
        "cmge v23.4s, v9.4s, v12.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmhi,
            rd: writable_vreg(5),
            rn: vreg(1),
            rm: vreg(1),
            size: VectorSize::Size32x4,
        },
        "2534A16E",
        "cmhi v5.4s, v1.4s, v1.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Cmhs,
            rd: writable_vreg(8),
            rn: vreg(2),
            rm: vreg(15),
            size: VectorSize::Size32x4,
        },
        "483CAF6E",
        "cmhs v8.4s, v2.4s, v15.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Fcmeq,
            rd: writable_vreg(28),
            rn: vreg(12),
            rm: vreg(4),
            size: VectorSize::Size32x4,
        },
        "9CE5244E",
        "fcmeq v28.4s, v12.4s, v4.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Fcmgt,
            rd: writable_vreg(3),
            rn: vreg(16),
            rm: vreg(31),
            size: VectorSize::Size64x2,
        },
        "03E6FF6E",
        "fcmgt v3.2d, v16.2d, v31.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Fcmge,
            rd: writable_vreg(18),
            rn: vreg(23),
            rm: vreg(0),
            size: VectorSize::Size64x2,
        },
        "F2E6606E",
        "fcmge v18.2d, v23.2d, v0.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::And,
            rd: writable_vreg(20),
            rn: vreg(19),
            rm: vreg(18),
            size: VectorSize::Size32x4,
        },
        "741E324E",
        "and v20.16b, v19.16b, v18.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Bic,
            rd: writable_vreg(8),
            rn: vreg(11),
            rm: vreg(1),
            size: VectorSize::Size8x16,
        },
        "681D614E",
        "bic v8.16b, v11.16b, v1.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Orr,
            rd: writable_vreg(15),
            rn: vreg(2),
            rm: vreg(12),
            size: VectorSize::Size16x8,
        },
        "4F1CAC4E",
        "orr v15.16b, v2.16b, v12.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Eor,
            rd: writable_vreg(18),
            rn: vreg(3),
            rm: vreg(22),
            size: VectorSize::Size8x16,
        },
        "721C366E",
        "eor v18.16b, v3.16b, v22.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Bsl,
            rd: writable_vreg(8),
            rn: vreg(9),
            rm: vreg(1),
            size: VectorSize::Size8x16,
        },
        "281D616E",
        "bsl v8.16b, v9.16b, v1.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umaxp,
            rd: writable_vreg(8),
            rn: vreg(12),
            rm: vreg(1),
            size: VectorSize::Size8x16,
        },
        "88A5216E",
        "umaxp v8.16b, v12.16b, v1.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umaxp,
            rd: writable_vreg(1),
            rn: vreg(6),
            rm: vreg(1),
            size: VectorSize::Size16x8,
        },
        "C1A4616E",
        "umaxp v1.8h, v6.8h, v1.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umaxp,
            rd: writable_vreg(1),
            rn: vreg(20),
            rm: vreg(16),
            size: VectorSize::Size32x4,
        },
        "81A6B06E",
        "umaxp v1.4s, v20.4s, v16.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Add,
            rd: writable_vreg(5),
            rn: vreg(1),
            rm: vreg(1),
            size: VectorSize::Size8x16,
        },
        "2584214E",
        "add v5.16b, v1.16b, v1.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Add,
            rd: writable_vreg(7),
            rn: vreg(13),
            rm: vreg(2),
            size: VectorSize::Size16x8,
        },
        "A785624E",
        "add v7.8h, v13.8h, v2.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Add,
            rd: writable_vreg(18),
            rn: vreg(9),
            rm: vreg(6),
            size: VectorSize::Size32x4,
        },
        "3285A64E",
        "add v18.4s, v9.4s, v6.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Add,
            rd: writable_vreg(1),
            rn: vreg(3),
            rm: vreg(2),
            size: VectorSize::Size64x2,
        },
        "6184E24E",
        "add v1.2d, v3.2d, v2.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sub,
            rd: writable_vreg(5),
            rn: vreg(1),
            rm: vreg(1),
            size: VectorSize::Size8x16,
        },
        "2584216E",
        "sub v5.16b, v1.16b, v1.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sub,
            rd: writable_vreg(7),
            rn: vreg(13),
            rm: vreg(2),
            size: VectorSize::Size16x8,
        },
        "A785626E",
        "sub v7.8h, v13.8h, v2.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sub,
            rd: writable_vreg(18),
            rn: vreg(9),
            rm: vreg(6),
            size: VectorSize::Size32x4,
        },
        "3285A66E",
        "sub v18.4s, v9.4s, v6.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sub,
            rd: writable_vreg(18),
            rn: vreg(0),
            rm: vreg(8),
            size: VectorSize::Size64x2,
        },
        "1284E86E",
        "sub v18.2d, v0.2d, v8.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Mul,
            rd: writable_vreg(25),
            rn: vreg(9),
            rm: vreg(8),
            size: VectorSize::Size8x16,
        },
        "399D284E",
        "mul v25.16b, v9.16b, v8.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Mul,
            rd: writable_vreg(30),
            rn: vreg(30),
            rm: vreg(12),
            size: VectorSize::Size16x8,
        },
        "DE9F6C4E",
        "mul v30.8h, v30.8h, v12.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Mul,
            rd: writable_vreg(18),
            rn: vreg(18),
            rm: vreg(18),
            size: VectorSize::Size32x4,
        },
        "529EB24E",
        "mul v18.4s, v18.4s, v18.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Ushl,
            rd: writable_vreg(18),
            rn: vreg(18),
            rm: vreg(18),
            size: VectorSize::Size8x16,
        },
        "5246326E",
        "ushl v18.16b, v18.16b, v18.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Ushl,
            rd: writable_vreg(18),
            rn: vreg(18),
            rm: vreg(18),
            size: VectorSize::Size16x8,
        },
        "5246726E",
        "ushl v18.8h, v18.8h, v18.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Ushl,
            rd: writable_vreg(18),
            rn: vreg(1),
            rm: vreg(21),
            size: VectorSize::Size32x4,
        },
        "3244B56E",
        "ushl v18.4s, v1.4s, v21.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Ushl,
            rd: writable_vreg(5),
            rn: vreg(7),
            rm: vreg(19),
            size: VectorSize::Size64x2,
        },
        "E544F36E",
        "ushl v5.2d, v7.2d, v19.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sshl,
            rd: writable_vreg(18),
            rn: vreg(18),
            rm: vreg(18),
            size: VectorSize::Size8x16,
        },
        "5246324E",
        "sshl v18.16b, v18.16b, v18.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sshl,
            rd: writable_vreg(30),
            rn: vreg(1),
            rm: vreg(29),
            size: VectorSize::Size16x8,
        },
        "3E447D4E",
        "sshl v30.8h, v1.8h, v29.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sshl,
            rd: writable_vreg(8),
            rn: vreg(22),
            rm: vreg(21),
            size: VectorSize::Size32x4,
        },
        "C846B54E",
        "sshl v8.4s, v22.4s, v21.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Sshl,
            rd: writable_vreg(8),
            rn: vreg(22),
            rm: vreg(2),
            size: VectorSize::Size64x2,
        },
        "C846E24E",
        "sshl v8.2d, v22.2d, v2.2d",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umin,
            rd: writable_vreg(1),
            rn: vreg(12),
            rm: vreg(3),
            size: VectorSize::Size8x16,
        },
        "816D236E",
        "umin v1.16b, v12.16b, v3.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umin,
            rd: writable_vreg(30),
            rn: vreg(20),
            rm: vreg(10),
            size: VectorSize::Size16x8,
        },
        "9E6E6A6E",
        "umin v30.8h, v20.8h, v10.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umin,
            rd: writable_vreg(8),
            rn: vreg(22),
            rm: vreg(21),
            size: VectorSize::Size32x4,
        },
        "C86EB56E",
        "umin v8.4s, v22.4s, v21.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Smin,
            rd: writable_vreg(1),
            rn: vreg(12),
            rm: vreg(3),
            size: VectorSize::Size8x16,
        },
        "816D234E",
        "smin v1.16b, v12.16b, v3.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Smin,
            rd: writable_vreg(30),
            rn: vreg(20),
            rm: vreg(10),
            size: VectorSize::Size16x8,
        },
        "9E6E6A4E",
        "smin v30.8h, v20.8h, v10.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Smin,
            rd: writable_vreg(8),
            rn: vreg(22),
            rm: vreg(21),
            size: VectorSize::Size32x4,
        },
        "C86EB54E",
        "smin v8.4s, v22.4s, v21.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umax,
            rd: writable_vreg(6),
            rn: vreg(9),
            rm: vreg(8),
            size: VectorSize::Size8x16,
        },
        "2665286E",
        "umax v6.16b, v9.16b, v8.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umax,
            rd: writable_vreg(11),
            rn: vreg(13),
            rm: vreg(2),
            size: VectorSize::Size16x8,
        },
        "AB65626E",
        "umax v11.8h, v13.8h, v2.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Umax,
            rd: writable_vreg(8),
            rn: vreg(12),
            rm: vreg(14),
            size: VectorSize::Size32x4,
        },
        "8865AE6E",
        "umax v8.4s, v12.4s, v14.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Smax,
            rd: writable_vreg(6),
            rn: vreg(9),
            rm: vreg(8),
            size: VectorSize::Size8x16,
        },
        "2665284E",
        "smax v6.16b, v9.16b, v8.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Smax,
            rd: writable_vreg(11),
            rn: vreg(13),
            rm: vreg(2),
            size: VectorSize::Size16x8,
        },
        "AB65624E",
        "smax v11.8h, v13.8h, v2.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Smax,
            rd: writable_vreg(8),
            rn: vreg(12),
            rm: vreg(14),
            size: VectorSize::Size32x4,
        },
        "8865AE4E",
        "smax v8.4s, v12.4s, v14.4s",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Urhadd,
            rd: writable_vreg(8),
            rn: vreg(1),
            rm: vreg(3),
            size: VectorSize::Size8x16,
        },
        "2814236E",
        "urhadd v8.16b, v1.16b, v3.16b",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Urhadd,
            rd: writable_vreg(2),
            rn: vreg(13),
            rm: vreg(6),
            size: VectorSize::Size16x8,
        },
        "A215666E",
        "urhadd v2.8h, v13.8h, v6.8h",
    ));

    insns.push((
        Inst::VecRRR {
            alu_op: VecALUOp::Urhadd,
            rd: writable_vreg(8),
            rn: vreg(12),
            rm: vreg(14),
            size: VectorSize::Size32x4,
        },
        "8815AE6E",
        "urhadd v8.4s, v12.4s, v14.4s",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Not,
            rd: writable_vreg(2),
            rn: vreg(1),
            size: VectorSize::Size32x4,
        },
        "2258206E",
        "mvn v2.16b, v1.16b",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Neg,
            rd: writable_vreg(8),
            rn: vreg(12),
            size: VectorSize::Size8x16,
        },
        "88B9206E",
        "neg v8.16b, v12.16b",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Neg,
            rd: writable_vreg(0),
            rn: vreg(31),
            size: VectorSize::Size16x8,
        },
        "E0BB606E",
        "neg v0.8h, v31.8h",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Neg,
            rd: writable_vreg(2),
            rn: vreg(3),
            size: VectorSize::Size32x4,
        },
        "62B8A06E",
        "neg v2.4s, v3.4s",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Neg,
            rd: writable_vreg(10),
            rn: vreg(8),
            size: VectorSize::Size64x2,
        },
        "0AB9E06E",
        "neg v10.2d, v8.2d",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Abs,
            rd: writable_vreg(1),
            rn: vreg(1),
            size: VectorSize::Size8x16,
        },
        "21B8204E",
        "abs v1.16b, v1.16b",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Abs,
            rd: writable_vreg(29),
            rn: vreg(28),
            size: VectorSize::Size16x8,
        },
        "9DBB604E",
        "abs v29.8h, v28.8h",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Abs,
            rd: writable_vreg(7),
            rn: vreg(8),
            size: VectorSize::Size32x4,
        },
        "07B9A04E",
        "abs v7.4s, v8.4s",
    ));

    insns.push((
        Inst::VecMisc {
            op: VecMisc2::Abs,
            rd: writable_vreg(1),
            rn: vreg(10),
            size: VectorSize::Size64x2,
        },
        "41B9E04E",
        "abs v1.2d, v10.2d",
    ));

    insns.push((
        Inst::VecLanes {
            op: VecLanesOp::Uminv,
            rd: writable_vreg(2),
            rn: vreg(1),
            size: VectorSize::Size8x16,
        },
        "22A8316E",
        "uminv b2, v1.16b",
    ));

    insns.push((
        Inst::VecLanes {
            op: VecLanesOp::Uminv,
            rd: writable_vreg(3),
            rn: vreg(11),
            size: VectorSize::Size16x8,
        },
        "63A9716E",
        "uminv h3, v11.8h",
    ));

    insns.push((
        Inst::VecLanes {
            op: VecLanesOp::Uminv,
            rd: writable_vreg(18),
            rn: vreg(4),
            size: VectorSize::Size32x4,
        },
        "92A8B16E",
        "uminv s18, v4.4s",
    ));

    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: false,
            from_bits: 8,
            to_bits: 32,
        },
        "411C0053",
        "uxtb w1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: true,
            from_bits: 8,
            to_bits: 32,
        },
        "411C0013",
        "sxtb w1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: false,
            from_bits: 16,
            to_bits: 32,
        },
        "413C0053",
        "uxth w1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: true,
            from_bits: 16,
            to_bits: 32,
        },
        "413C0013",
        "sxth w1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: false,
            from_bits: 8,
            to_bits: 64,
        },
        "411C0053",
        "uxtb x1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: true,
            from_bits: 8,
            to_bits: 64,
        },
        "411C4093",
        "sxtb x1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: false,
            from_bits: 16,
            to_bits: 64,
        },
        "413C0053",
        "uxth x1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: true,
            from_bits: 16,
            to_bits: 64,
        },
        "413C4093",
        "sxth x1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: false,
            from_bits: 32,
            to_bits: 64,
        },
        "E103022A",
        "mov w1, w2",
    ));
    insns.push((
        Inst::Extend {
            rd: writable_xreg(1),
            rn: xreg(2),
            signed: true,
            from_bits: 32,
            to_bits: 64,
        },
        "417C4093",
        "sxtw x1, w2",
    ));

    insns.push((
        Inst::Jump {
            dest: BranchTarget::ResolvedOffset(64),
        },
        "10000014",
        "b 64",
    ));

    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::NotZero(xreg(8)),
        },
        "480000B40000A0D4",
        "cbz x8, 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Zero(xreg(8)),
        },
        "480000B50000A0D4",
        "cbnz x8, 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Ne),
        },
        "400000540000A0D4",
        "b.eq 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Eq),
        },
        "410000540000A0D4",
        "b.ne 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Lo),
        },
        "420000540000A0D4",
        "b.hs 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Hs),
        },
        "430000540000A0D4",
        "b.lo 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Pl),
        },
        "440000540000A0D4",
        "b.mi 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Mi),
        },
        "450000540000A0D4",
        "b.pl 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Vc),
        },
        "460000540000A0D4",
        "b.vs 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Vs),
        },
        "470000540000A0D4",
        "b.vc 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Ls),
        },
        "480000540000A0D4",
        "b.hi 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Hi),
        },
        "490000540000A0D4",
        "b.ls 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Lt),
        },
        "4A0000540000A0D4",
        "b.ge 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Ge),
        },
        "4B0000540000A0D4",
        "b.lt 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Le),
        },
        "4C0000540000A0D4",
        "b.gt 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Gt),
        },
        "4D0000540000A0D4",
        "b.le 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Nv),
        },
        "4E0000540000A0D4",
        "b.al 8 ; udf",
    ));
    insns.push((
        Inst::TrapIf {
            trap_info: (SourceLoc::default(), TrapCode::Interrupt),
            kind: CondBrKind::Cond(Cond::Al),
        },
        "4F0000540000A0D4",
        "b.nv 8 ; udf",
    ));

    insns.push((
        Inst::CondBr {
            taken: BranchTarget::ResolvedOffset(64),
            not_taken: BranchTarget::ResolvedOffset(128),
            kind: CondBrKind::Cond(Cond::Le),
        },
        "0D02005420000014",
        "b.le 64 ; b 128",
    ));

    insns.push((
        Inst::Call {
            info: Box::new(CallInfo {
                dest: ExternalName::testcase("test0"),
                uses: Vec::new(),
                defs: Vec::new(),
                loc: SourceLoc::default(),
                opcode: Opcode::Call,
            }),
        },
        "00000094",
        "bl 0",
    ));

    insns.push((
        Inst::CallInd {
            info: Box::new(CallIndInfo {
                rn: xreg(10),
                uses: Vec::new(),
                defs: Vec::new(),
                loc: SourceLoc::default(),
                opcode: Opcode::CallIndirect,
            }),
        },
        "40013FD6",
        "blr x10",
    ));

    insns.push((
        Inst::IndirectBr {
            rn: xreg(3),
            targets: vec![],
        },
        "60001FD6",
        "br x3",
    ));

    insns.push((Inst::Brk, "000020D4", "brk #0"));

    insns.push((
        Inst::Adr {
            rd: writable_xreg(15),
            off: (1 << 20) - 4,
        },
        "EFFF7F10",
        "adr x15, pc+1048572",
    ));

    insns.push((
        Inst::FpuMove64 {
            rd: writable_vreg(8),
            rn: vreg(4),
        },
        "881CA40E",
        "mov v8.8b, v4.8b",
    ));

    insns.push((
        Inst::FpuMove128 {
            rd: writable_vreg(17),
            rn: vreg(26),
        },
        "511FBA4E",
        "mov v17.16b, v26.16b",
    ));

    insns.push((
        Inst::FpuMoveFromVec {
            rd: writable_vreg(1),
            rn: vreg(30),
            idx: 2,
            size: VectorSize::Size32x4,
        },
        "C107145E",
        "mov s1, v30.s[2]",
    ));

    insns.push((
        Inst::FpuMoveFromVec {
            rd: writable_vreg(23),
            rn: vreg(11),
            idx: 0,
            size: VectorSize::Size64x2,
        },
        "7705085E",
        "mov d23, v11.d[0]",
    ));

    insns.push((
        Inst::FpuRR {
            fpu_op: FPUOp1::Abs32,
            rd: writable_vreg(15),
            rn: vreg(30),
        },
        "CFC3201E",
        "fabs s15, s30",
    ));

    insns.push((
        Inst::FpuRR {
            fpu_op: FPUOp1::Abs64,
            rd: writable_vreg(15),
            rn: vreg(30),
        },
        "CFC3601E",
        "fabs d15, d30",
    ));

    insns.push((
        Inst::FpuRR {
            fpu_op: FPUOp1::Neg32,
            rd: writable_vreg(15),
            rn: vreg(30),
        },
        "CF43211E",
        "fneg s15, s30",
    ));

    insns.push((
        Inst::FpuRR {
            fpu_op: FPUOp1::Neg64,
            rd: writable_vreg(15),
            rn: vreg(30),
        },
        "CF43611E",
        "fneg d15, d30",
    ));

    insns.push((
        Inst::FpuRR {
            fpu_op: FPUOp1::Sqrt32,
            rd: writable_vreg(15),
            rn: vreg(30),
        },
        "CFC3211E",
        "fsqrt s15, s30",
    ));

    insns.push((
        Inst::FpuRR {
            fpu_op: FPUOp1::Sqrt64,
            rd: writable_vreg(15),
            rn: vreg(30),
        },
        "CFC3611E",
        "fsqrt d15, d30",
    ));

    insns.push((
        Inst::FpuRR {
            fpu_op: FPUOp1::Cvt32To64,
            rd: writable_vreg(15),
            rn: vreg(30),
        },
        "CFC3221E",
        "fcvt d15, s30",
    ));

    insns.push((
        Inst::FpuRR {
            fpu_op: FPUOp1::Cvt64To32,
            rd: writable_vreg(15),
            rn: vreg(30),
        },
        "CF43621E",
        "fcvt s15, d30",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Add32,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF2B3F1E",
        "fadd s15, s30, s31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Add64,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF2B7F1E",
        "fadd d15, d30, d31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Sub32,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF3B3F1E",
        "fsub s15, s30, s31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Sub64,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF3B7F1E",
        "fsub d15, d30, d31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Mul32,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF0B3F1E",
        "fmul s15, s30, s31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Mul64,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF0B7F1E",
        "fmul d15, d30, d31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Div32,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF1B3F1E",
        "fdiv s15, s30, s31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Div64,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF1B7F1E",
        "fdiv d15, d30, d31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Max32,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF4B3F1E",
        "fmax s15, s30, s31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Max64,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF4B7F1E",
        "fmax d15, d30, d31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Min32,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF5B3F1E",
        "fmin s15, s30, s31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Min64,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
        },
        "CF5B7F1E",
        "fmin d15, d30, d31",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Uqadd64,
            rd: writable_vreg(21),
            rn: vreg(22),
            rm: vreg(23),
        },
        "D50EF77E",
        "uqadd d21, d22, d23",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Sqadd64,
            rd: writable_vreg(21),
            rn: vreg(22),
            rm: vreg(23),
        },
        "D50EF75E",
        "sqadd d21, d22, d23",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Uqsub64,
            rd: writable_vreg(21),
            rn: vreg(22),
            rm: vreg(23),
        },
        "D52EF77E",
        "uqsub d21, d22, d23",
    ));

    insns.push((
        Inst::FpuRRR {
            fpu_op: FPUOp2::Sqsub64,
            rd: writable_vreg(21),
            rn: vreg(22),
            rm: vreg(23),
        },
        "D52EF75E",
        "sqsub d21, d22, d23",
    ));

    insns.push((
        Inst::FpuRRRR {
            fpu_op: FPUOp3::MAdd32,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
            ra: vreg(1),
        },
        "CF071F1F",
        "fmadd s15, s30, s31, s1",
    ));

    insns.push((
        Inst::FpuRRRR {
            fpu_op: FPUOp3::MAdd64,
            rd: writable_vreg(15),
            rn: vreg(30),
            rm: vreg(31),
            ra: vreg(1),
        },
        "CF075F1F",
        "fmadd d15, d30, d31, d1",
    ));

    insns.push((
        Inst::FpuRRI {
            fpu_op: FPUOpRI::UShr32(FPURightShiftImm::maybe_from_u8(32, 32).unwrap()),
            rd: writable_vreg(2),
            rn: vreg(5),
        },
        "A204202F",
        "ushr v2.2s, v5.2s, #32",
    ));

    insns.push((
        Inst::FpuRRI {
            fpu_op: FPUOpRI::UShr64(FPURightShiftImm::maybe_from_u8(63, 64).unwrap()),
            rd: writable_vreg(2),
            rn: vreg(5),
        },
        "A204417F",
        "ushr d2, d5, #63",
    ));

    insns.push((
        Inst::FpuRRI {
            fpu_op: FPUOpRI::Sli32(FPULeftShiftImm::maybe_from_u8(31, 32).unwrap()),
            rd: writable_vreg(4),
            rn: vreg(10),
        },
        "44553F2F",
        "sli v4.2s, v10.2s, #31",
    ));

    insns.push((
        Inst::FpuRRI {
            fpu_op: FPUOpRI::Sli64(FPULeftShiftImm::maybe_from_u8(63, 64).unwrap()),
            rd: writable_vreg(4),
            rn: vreg(10),
        },
        "44557F7F",
        "sli d4, d10, #63",
    ));

    insns.push((
        Inst::FpuToInt {
            op: FpuToIntOp::F32ToU32,
            rd: writable_xreg(1),
            rn: vreg(4),
        },
        "8100391E",
        "fcvtzu w1, s4",
    ));

    insns.push((
        Inst::FpuToInt {
            op: FpuToIntOp::F32ToU64,
            rd: writable_xreg(1),
            rn: vreg(4),
        },
        "8100399E",
        "fcvtzu x1, s4",
    ));

    insns.push((
        Inst::FpuToInt {
            op: FpuToIntOp::F32ToI32,
            rd: writable_xreg(1),
            rn: vreg(4),
        },
        "8100381E",
        "fcvtzs w1, s4",
    ));

    insns.push((
        Inst::FpuToInt {
            op: FpuToIntOp::F32ToI64,
            rd: writable_xreg(1),
            rn: vreg(4),
        },
        "8100389E",
        "fcvtzs x1, s4",
    ));

    insns.push((
        Inst::FpuToInt {
            op: FpuToIntOp::F64ToU32,
            rd: writable_xreg(1),
            rn: vreg(4),
        },
        "8100791E",
        "fcvtzu w1, d4",
    ));

    insns.push((
        Inst::FpuToInt {
            op: FpuToIntOp::F64ToU64,
            rd: writable_xreg(1),
            rn: vreg(4),
        },
        "8100799E",
        "fcvtzu x1, d4",
    ));

    insns.push((
        Inst::FpuToInt {
            op: FpuToIntOp::F64ToI32,
            rd: writable_xreg(1),
            rn: vreg(4),
        },
        "8100781E",
        "fcvtzs w1, d4",
    ));

    insns.push((
        Inst::FpuToInt {
            op: FpuToIntOp::F64ToI64,
            rd: writable_xreg(1),
            rn: vreg(4),
        },
        "8100789E",
        "fcvtzs x1, d4",
    ));

    insns.push((
        Inst::IntToFpu {
            op: IntToFpuOp::U32ToF32,
            rd: writable_vreg(1),
            rn: xreg(4),
        },
        "8100231E",
        "ucvtf s1, w4",
    ));

    insns.push((
        Inst::IntToFpu {
            op: IntToFpuOp::I32ToF32,
            rd: writable_vreg(1),
            rn: xreg(4),
        },
        "8100221E",
        "scvtf s1, w4",
    ));

    insns.push((
        Inst::IntToFpu {
            op: IntToFpuOp::U32ToF64,
            rd: writable_vreg(1),
            rn: xreg(4),
        },
        "8100631E",
        "ucvtf d1, w4",
    ));

    insns.push((
        Inst::IntToFpu {
            op: IntToFpuOp::I32ToF64,
            rd: writable_vreg(1),
            rn: xreg(4),
        },
        "8100621E",
        "scvtf d1, w4",
    ));

    insns.push((
        Inst::IntToFpu {
            op: IntToFpuOp::U64ToF32,
            rd: writable_vreg(1),
            rn: xreg(4),
        },
        "8100239E",
        "ucvtf s1, x4",
    ));

    insns.push((
        Inst::IntToFpu {
            op: IntToFpuOp::I64ToF32,
            rd: writable_vreg(1),
            rn: xreg(4),
        },
        "8100229E",
        "scvtf s1, x4",
    ));

    insns.push((
        Inst::IntToFpu {
            op: IntToFpuOp::U64ToF64,
            rd: writable_vreg(1),
            rn: xreg(4),
        },
        "8100639E",
        "ucvtf d1, x4",
    ));

    insns.push((
        Inst::IntToFpu {
            op: IntToFpuOp::I64ToF64,
            rd: writable_vreg(1),
            rn: xreg(4),
        },
        "8100629E",
        "scvtf d1, x4",
    ));

    insns.push((
        Inst::FpuCmp32 {
            rn: vreg(23),
            rm: vreg(24),
        },
        "E022381E",
        "fcmp s23, s24",
    ));

    insns.push((
        Inst::FpuCmp64 {
            rn: vreg(23),
            rm: vreg(24),
        },
        "E022781E",
        "fcmp d23, d24",
    ));

    insns.push((
        Inst::FpuLoad32 {
            rd: writable_vreg(16),
            mem: MemArg::RegScaled(xreg(8), xreg(9), F32),
            srcloc: None,
        },
        "107969BC",
        "ldr s16, [x8, x9, LSL #2]",
    ));

    insns.push((
        Inst::FpuLoad64 {
            rd: writable_vreg(16),
            mem: MemArg::RegScaled(xreg(8), xreg(9), F64),
            srcloc: None,
        },
        "107969FC",
        "ldr d16, [x8, x9, LSL #3]",
    ));

    insns.push((
        Inst::FpuLoad128 {
            rd: writable_vreg(16),
            mem: MemArg::RegScaled(xreg(8), xreg(9), I128),
            srcloc: None,
        },
        "1079E93C",
        "ldr q16, [x8, x9, LSL #4]",
    ));

    insns.push((
        Inst::FpuLoad32 {
            rd: writable_vreg(16),
            mem: MemArg::Label(MemLabel::PCRel(8)),
            srcloc: None,
        },
        "5000001C",
        "ldr s16, pc+8",
    ));

    insns.push((
        Inst::FpuLoad64 {
            rd: writable_vreg(16),
            mem: MemArg::Label(MemLabel::PCRel(8)),
            srcloc: None,
        },
        "5000005C",
        "ldr d16, pc+8",
    ));

    insns.push((
        Inst::FpuLoad128 {
            rd: writable_vreg(16),
            mem: MemArg::Label(MemLabel::PCRel(8)),
            srcloc: None,
        },
        "5000009C",
        "ldr q16, pc+8",
    ));

    insns.push((
        Inst::FpuStore32 {
            rd: vreg(16),
            mem: MemArg::RegScaled(xreg(8), xreg(9), F32),
            srcloc: None,
        },
        "107929BC",
        "str s16, [x8, x9, LSL #2]",
    ));

    insns.push((
        Inst::FpuStore64 {
            rd: vreg(16),
            mem: MemArg::RegScaled(xreg(8), xreg(9), F64),
            srcloc: None,
        },
        "107929FC",
        "str d16, [x8, x9, LSL #3]",
    ));

    insns.push((
        Inst::FpuStore128 {
            rd: vreg(16),
            mem: MemArg::RegScaled(xreg(8), xreg(9), I128),
            srcloc: None,
        },
        "1079A93C",
        "str q16, [x8, x9, LSL #4]",
    ));

    insns.push((
        Inst::LoadFpuConst32 {
            rd: writable_vreg(16),
            const_data: 1.0,
        },
        "5000001C020000140000803F",
        "ldr s16, pc+8 ; b 8 ; data.f32 1",
    ));

    insns.push((
        Inst::LoadFpuConst64 {
            rd: writable_vreg(16),
            const_data: 1.0,
        },
        "5000005C03000014000000000000F03F",
        "ldr d16, pc+8 ; b 12 ; data.f64 1",
    ));

    insns.push((
        Inst::LoadFpuConst128 {
            rd: writable_vreg(5),
            const_data: 0x0f0e0d0c0b0a09080706050403020100,
        },
        "4500009C05000014000102030405060708090A0B0C0D0E0F",
        "ldr q5, pc+8 ; b 20 ; data.f128 0x0f0e0d0c0b0a09080706050403020100",
    ));

    insns.push((
        Inst::FpuCSel32 {
            rd: writable_vreg(1),
            rn: vreg(2),
            rm: vreg(3),
            cond: Cond::Hi,
        },
        "418C231E",
        "fcsel s1, s2, s3, hi",
    ));

    insns.push((
        Inst::FpuCSel64 {
            rd: writable_vreg(1),
            rn: vreg(2),
            rm: vreg(3),
            cond: Cond::Eq,
        },
        "410C631E",
        "fcsel d1, d2, d3, eq",
    ));

    insns.push((
        Inst::FpuRound {
            rd: writable_vreg(23),
            rn: vreg(24),
            op: FpuRoundMode::Minus32,
        },
        "1743251E",
        "frintm s23, s24",
    ));
    insns.push((
        Inst::FpuRound {
            rd: writable_vreg(23),
            rn: vreg(24),
            op: FpuRoundMode::Minus64,
        },
        "1743651E",
        "frintm d23, d24",
    ));
    insns.push((
        Inst::FpuRound {
            rd: writable_vreg(23),
            rn: vreg(24),
            op: FpuRoundMode::Plus32,
        },
        "17C3241E",
        "frintp s23, s24",
    ));
    insns.push((
        Inst::FpuRound {
            rd: writable_vreg(23),
            rn: vreg(24),
            op: FpuRoundMode::Plus64,
        },
        "17C3641E",
        "frintp d23, d24",
    ));
    insns.push((
        Inst::FpuRound {
            rd: writable_vreg(23),
            rn: vreg(24),
            op: FpuRoundMode::Zero32,
        },
        "17C3251E",
        "frintz s23, s24",
    ));
    insns.push((
        Inst::FpuRound {
            rd: writable_vreg(23),
            rn: vreg(24),
            op: FpuRoundMode::Zero64,
        },
        "17C3651E",
        "frintz d23, d24",
    ));
    insns.push((
        Inst::FpuRound {
            rd: writable_vreg(23),
            rn: vreg(24),
            op: FpuRoundMode::Nearest32,
        },
        "1743241E",
        "frintn s23, s24",
    ));
    insns.push((
        Inst::FpuRound {
            rd: writable_vreg(23),
            rn: vreg(24),
            op: FpuRoundMode::Nearest64,
        },
        "1743641E",
        "frintn d23, d24",
    ));

    let rru = create_reg_universe(&settings::Flags::new(settings::builder()));
    for (insn, expected_encoding, expected_printing) in insns {
        println!(
            "AArch64: {:?}, {}, {}",
            insn, expected_encoding, expected_printing
        );

        // Check the printed text is as expected.
        let actual_printing = insn.show_rru(Some(&rru));
        assert_eq!(expected_printing, actual_printing);

        let mut sink = test_utils::TestCodeSink::new();
        let mut buffer = MachBuffer::new();
        insn.emit(&mut buffer, &flags, &mut Default::default());
        let buffer = buffer.finish();
        buffer.emit(&mut sink);
        let actual_encoding = &sink.stringify();
        assert_eq!(expected_encoding, actual_encoding);
    }
}

#[test]
fn test_cond_invert() {
    for cond in vec![
        Cond::Eq,
        Cond::Ne,
        Cond::Hs,
        Cond::Lo,
        Cond::Mi,
        Cond::Pl,
        Cond::Vs,
        Cond::Vc,
        Cond::Hi,
        Cond::Ls,
        Cond::Ge,
        Cond::Lt,
        Cond::Gt,
        Cond::Le,
        Cond::Al,
        Cond::Nv,
    ]
    .into_iter()
    {
        assert_eq!(cond.invert().invert(), cond);
    }
}

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
            mem: MemArg::FPOffset(32768),
            srcloc: None,
        },
        "100090D2B063308B010240F9",
        "movz x16, #32768 ; add x16, fp, x16, UXTX ; ldr x1, [x16]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::FPOffset(-32768),
            srcloc: None,
        },
        "F0FF8F92B063308B010240F9",
        "movn x16, #32767 ; add x16, fp, x16, UXTX ; ldr x1, [x16]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::FPOffset(1048576), // 2^20
            srcloc: None,
        },
        "1002A0D2B063308B010240F9",
        "movz x16, #16, LSL #16 ; add x16, fp, x16, UXTX ; ldr x1, [x16]",
    ));
    insns.push((
        Inst::ULoad64 {
            rd: writable_xreg(1),
            mem: MemArg::FPOffset(1048576 + 1), // 2^20 + 1
            srcloc: None,
        },
        "300080D21002A0F2B063308B010240F9",
        "movz x16, #1 ; movk x16, #16, LSL #16 ; add x16, fp, x16, UXTX ; ldr x1, [x16]",
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
            size: InstSize::Size64,
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
            size: InstSize::Size32,
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
        Inst::MovFromVec64 {
            rd: writable_xreg(21),
            rn: vreg(20),
        },
        "953E084E",
        "mov x21, v20.d[0]",
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
        Inst::VecRRR {
            rd: writable_vreg(21),
            rn: vreg(22),
            rm: vreg(23),
            alu_op: VecALUOp::UQAddScalar,
        },
        "D50EF77E",
        "uqadd d21, d22, d23",
    ));
    insns.push((
        Inst::VecRRR {
            rd: writable_vreg(21),
            rn: vreg(22),
            rm: vreg(23),
            alu_op: VecALUOp::SQAddScalar,
        },
        "D50EF75E",
        "sqadd d21, d22, d23",
    ));
    insns.push((
        Inst::VecRRR {
            rd: writable_vreg(21),
            rn: vreg(22),
            rm: vreg(23),
            alu_op: VecALUOp::UQSubScalar,
        },
        "D52EF77E",
        "uqsub d21, d22, d23",
    ));
    insns.push((
        Inst::VecRRR {
            rd: writable_vreg(21),
            rn: vreg(22),
            rm: vreg(23),
            alu_op: VecALUOp::SQSubScalar,
        },
        "D52EF75E",
        "sqsub d21, d22, d23",
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
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Zero(xreg(8)),
        },
        "080200B4",
        "cbz x8, 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::NotZero(xreg(8)),
        },
        "080200B5",
        "cbnz x8, 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Eq),
        },
        "00020054",
        "b.eq 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Ne),
        },
        "01020054",
        "b.ne 64",
    ));

    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Hs),
        },
        "02020054",
        "b.hs 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Lo),
        },
        "03020054",
        "b.lo 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Mi),
        },
        "04020054",
        "b.mi 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Pl),
        },
        "05020054",
        "b.pl 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Vs),
        },
        "06020054",
        "b.vs 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Vc),
        },
        "07020054",
        "b.vc 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Hi),
        },
        "08020054",
        "b.hi 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Ls),
        },
        "09020054",
        "b.ls 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Ge),
        },
        "0A020054",
        "b.ge 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Lt),
        },
        "0B020054",
        "b.lt 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Gt),
        },
        "0C020054",
        "b.gt 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Le),
        },
        "0D020054",
        "b.le 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Al),
        },
        "0E020054",
        "b.al 64",
    ));
    insns.push((
        Inst::OneWayCondBr {
            target: BranchTarget::ResolvedOffset(64),
            kind: CondBrKind::Cond(Cond::Nv),
        },
        "0F020054",
        "b.nv 64",
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
            dest: Box::new(ExternalName::testcase("test0")),
            uses: Box::new(Set::empty()),
            defs: Box::new(Set::empty()),
            loc: SourceLoc::default(),
            opcode: Opcode::Call,
        },
        "00000094",
        "bl 0",
    ));

    insns.push((
        Inst::CallInd {
            rn: xreg(10),
            uses: Box::new(Set::empty()),
            defs: Box::new(Set::empty()),
            loc: SourceLoc::default(),
            opcode: Opcode::CallIndirect,
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

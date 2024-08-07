//! Disassembly tests.

use pulley_interpreter::*;

fn encoded(ops: &[Op]) -> Vec<u8> {
    let mut encoded = vec![];
    for op in ops {
        op.encode(&mut encoded);
    }
    log::trace!("encoded: {encoded:?}");
    encoded
}

fn assert_disas(ops: &[Op], expected: &str) {
    let expected = expected.trim();
    eprintln!("=== expected ===\n{expected}");

    let bytecode = encoded(ops);

    let actual = disas::Disassembler::disassemble_all(&bytecode).expect("decoding failed");
    let actual = actual.trim();
    eprintln!("=== actual ===\n{actual}");

    assert_eq!(expected, actual);
}

#[test]
fn simple() {
    assert_disas(
        &[
            // Prologue.
            Op::Xconst8(Xconst8 {
                dst: XReg::x26,
                imm: -16i8,
            }),
            Op::Xadd32(Xadd32 {
                operands: BinaryOperands {
                    dst: XReg::sp,
                    src1: XReg::sp,
                    src2: XReg::x26,
                },
            }),
            Op::Store64Offset8(Store64Offset8 {
                ptr: XReg::sp,
                offset: 8,
                src: XReg::lr,
            }),
            Op::Store64(Store64 {
                ptr: XReg::sp,
                src: XReg::fp,
            }),
            Op::Xmov(Xmov {
                dst: XReg::fp,
                src: XReg::sp,
            }),
            // Function body.
            Op::Xadd32(Xadd32 {
                operands: BinaryOperands {
                    dst: XReg::x0,
                    src1: XReg::x0,
                    src2: XReg::x1,
                },
            }),
            // Epilogue.
            Op::Xmov(Xmov {
                dst: XReg::sp,
                src: XReg::fp,
            }),
            Op::Load64Offset8(Load64Offset8 {
                dst: XReg::lr,
                ptr: XReg::sp,
                offset: 8,
            }),
            Op::Load64(Load64 {
                dst: XReg::fp,
                ptr: XReg::sp,
            }),
            Op::Xconst8(Xconst8 {
                dst: XReg::x26,
                imm: 16,
            }),
            Op::Xadd32(Xadd32 {
                operands: BinaryOperands {
                    dst: XReg::sp,
                    src1: XReg::sp,
                    src2: XReg::x26,
                },
            }),
            Op::Ret(Ret {}),
        ],
        r#"
       0: 0e 1a f0                        xconst8 x26, -16
       3: 12 7b 6b                        xadd32 sp, sp, x26
       6: 2c 1b 08 1c                     store64_offset8 sp, 8, lr
       a: 2a 1b 1d                        store64 sp, fp
       d: 0b 1d 1b                        xmov fp, sp
      10: 12 00 04                        xadd32 x0, x0, x1
      13: 0b 1b 1d                        xmov sp, fp
      16: 25 1c 1b 08                     load64_offset8 lr, sp, 8
      1a: 22 1d 1b                        load64 fp, sp
      1d: 0e 1a 10                        xconst8 x26, 16
      20: 12 7b 6b                        xadd32 sp, sp, x26
      23: 00                              ret
        "#,
    );
}

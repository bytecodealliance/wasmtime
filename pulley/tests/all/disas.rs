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
                dst: XReg::x31,
                imm: -16i8,
            }),
            Op::Xadd32(Xadd32 {
                operands: BinaryOperands {
                    dst: XReg::x0,
                    src1: XReg::x1,
                    src2: XReg::x31,
                },
            }),
            Op::Store64Offset8(Store64Offset8 {
                ptr: XReg::x0,
                offset: 8,
                src: XReg::x31,
            }),
            Op::Store64(Store64 {
                ptr: XReg::x0,
                src: XReg::x1,
            }),
            Op::Xmov(Xmov {
                dst: XReg::x0,
                src: XReg::x1,
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
                dst: XReg::x0,
                src: XReg::x1,
            }),
            Op::Load64Offset8(Load64Offset8 {
                dst: XReg::x0,
                ptr: XReg::x1,
                offset: 8,
            }),
            Op::Load64(Load64 {
                dst: XReg::x0,
                ptr: XReg::x1,
            }),
            Op::Xconst8(Xconst8 {
                dst: XReg::x31,
                imm: 16,
            }),
            Op::Xadd32(Xadd32 {
                operands: BinaryOperands {
                    dst: XReg::x0,
                    src1: XReg::x1,
                    src2: XReg::x31,
                },
            }),
            Op::Ret(Ret {}),
        ],
        r#"
       0: 0e 1f f0                        xconst8 x31, -16
       3: 12 20 7c                        xadd32 x0, x1, x31
       6: 29 00 08 1f                     store64_offset8 x0, 8, x31
       a: 27 00 01                        store64 x0, x1
       d: 0b 00 01                        xmov x0, x1
      10: 12 00 04                        xadd32 x0, x0, x1
      13: 0b 00 01                        xmov x0, x1
      16: 25 00 01 08                     load64_offset8 x0, x1, 8
      1a: 22 00 01                        load64 x0, x1
      1d: 0e 1f 10                        xconst8 x31, 16
      20: 12 20 7c                        xadd32 x0, x1, x31
      23: 00                              ret
        "#,
    );
}

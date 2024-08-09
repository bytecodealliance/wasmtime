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
            Op::PushFrame(PushFrame {}),
            // Function body.
            Op::Xadd32(Xadd32 {
                operands: BinaryOperands {
                    dst: XReg::x0,
                    src1: XReg::x0,
                    src2: XReg::x1,
                },
            }),
            // Epilogue.
            Op::PopFrame(PopFrame {}),
            Op::Ret(Ret {}),
        ],
        r#"
       0: 2f                              push_frame
       1: 12 00 04                        xadd32 x0, x0, x1
       4: 30                              pop_frame
       5: 00                              ret
        "#,
    );
}

#[test]
fn push_pop_many() {
    assert_disas(
        &[
            // Prologue.
            Op::PushFrame(PushFrame {}),
            Op::XPush32Many(XPush32Many {
                srcs: RegSet::from_iter([XReg::x0, XReg::x1, XReg::x2, XReg::x3, XReg::x4]),
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
            Op::XPop32Many(XPop32Many {
                dsts: RegSet::from_iter([XReg::x0, XReg::x1, XReg::x2, XReg::x3, XReg::x4]),
            }),
            Op::PopFrame(PopFrame {}),
            Op::Ret(Ret {}),
        ],
        r#"
       0: 2f                              push_frame
       1: 32 1f 00 00 00                  xpush32_many x0, x1, x2, x3, x4
       6: 12 00 04                        xadd32 x0, x0, x1
       9: 36 1f 00 00 00                  xpop32_many x0, x1, x2, x3, x4
       e: 30                              pop_frame
       f: 00                              ret
        "#,
    );
}

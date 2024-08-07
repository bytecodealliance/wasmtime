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
            Op::EnterFrame(EnterFrame { frame_size: 0 }),
            // Function body.
            Op::Xadd32(Xadd32 {
                operands: BinaryOperands {
                    dst: XReg::x0,
                    src1: XReg::x0,
                    src2: XReg::x1,
                },
            }),
            // Epilogue.
            Op::ExitFrame(ExitFrame { frame_size: 0 }),
            Op::Ret(Ret {}),
        ],
        r#"
       0: 2a 00                           enter_frame 0
       2: 12 00 04                        xadd32 x0, x0, x1
       5: 2b 00                           exit_frame 0
       7: 00                              ret
        "#,
    );
}

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

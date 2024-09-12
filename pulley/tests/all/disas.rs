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

#[track_caller]
fn assert_disas_with_disassembler(dis: &mut disas::Disassembler<'_>, expected: &str) {
    let expected = expected.trim();
    eprintln!("=== expected ===\n{expected}");

    decode::Decoder::decode_all(dis).expect("decoding should succeed");

    let actual = dis.disas().trim();
    eprintln!("=== actual ===\n{actual}");

    assert_eq!(expected, actual);
}

#[track_caller]
fn assert_disas(ops: &[Op], expected: &str) {
    let bytecode = encoded(ops);
    assert_disas_with_disassembler(&mut disas::Disassembler::new(&bytecode), expected);
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
       0: 35                              push_frame
       1: 18 00 04                        xadd32 x0, x0, x1
       4: 36                              pop_frame
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
       0: 35                              push_frame
       1: 38 1f 00 00 00                  xpush32_many x0, x1, x2, x3, x4
       6: 18 00 04                        xadd32 x0, x0, x1
       9: 3c 1f 00 00 00                  xpop32_many x0, x1, x2, x3, x4
       e: 36                              pop_frame
       f: 00                              ret
        "#,
    );
}

#[test]
fn no_offsets() {
    let bytecode = encoded(&[
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
    ]);

    assert_disas_with_disassembler(
        disas::Disassembler::new(&bytecode).offsets(false),
        r#"
35                              push_frame
18 00 04                        xadd32 x0, x0, x1
36                              pop_frame
00                              ret
        "#,
    );
}

#[test]
fn no_hexdump() {
    let bytecode = encoded(&[
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
    ]);

    assert_disas_with_disassembler(
        disas::Disassembler::new(&bytecode).hexdump(false),
        r#"
       0: push_frame
       1: xadd32 x0, x0, x1
       4: pop_frame
       5: ret
        "#,
    );
}

#[test]
fn no_offsets_or_hexdump() {
    let bytecode = encoded(&[
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
    ]);

    assert_disas_with_disassembler(
        disas::Disassembler::new(&bytecode)
            .offsets(false)
            .hexdump(false),
        r#"
push_frame
xadd32 x0, x0, x1
pop_frame
ret
        "#,
    );
}

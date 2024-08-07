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
    let x0 = XReg::new(0).unwrap();
    let x1 = XReg::new(1).unwrap();
    let x31 = XReg::new(31).unwrap();

    assert_disas(
        &[
            // Prologue.
            Op::Xconst8(Xconst8 {
                dst: x31,
                imm: -16i8,
            }),
            Op::Xadd32(Xadd32 {
                dst: x0,
                src1: x1,
                src2: x31,
            }),
            Op::Store64Offset8(Store64Offset8 {
                ptr: x0,
                offset: 8,
                src: x31,
            }),
            Op::Store64(Store64 { ptr: x0, src: x1 }),
            Op::Xmov(Xmov { dst: x0, src: x1 }),
            // Function body.
            Op::Xadd32(Xadd32 {
                dst: x0,
                src1: x0,
                src2: x1,
            }),
            // Epilogue.
            Op::Xmov(Xmov { dst: x0, src: x1 }),
            Op::Load64Offset8(Load64Offset8 {
                dst: x0,
                ptr: x1,
                offset: 8,
            }),
            Op::Load64(Load64 { dst: x0, ptr: x1 }),
            Op::Xconst8(Xconst8 { dst: x31, imm: 16 }),
            Op::Xadd32(Xadd32 {
                dst: x0,
                src1: x1,
                src2: x31,
            }),
            Op::Ret(Ret {}),
        ],
        r#"
       0: 0e 1f f0                        xconst8 x31, -16
       3: 12 00 01 1f                     xadd32 x0, x1, x31
       7: 29 00 08 1f                     store64_offset8 x0, 8, x31
       b: 27 00 01                        store64 x0, x1
       e: 0b 00 01                        xmov x0, x1
      11: 12 00 00 01                     xadd32 x0, x0, x1
      15: 0b 00 01                        xmov x0, x1
      18: 25 00 01 08                     load64_offset8 x0, x1, 8
      1c: 22 00 01                        load64 x0, x1
      1f: 0e 1f 10                        xconst8 x31, 16
      22: 12 00 01 1f                     xadd32 x0, x1, x31
      26: 00                              ret
        "#,
    );
}

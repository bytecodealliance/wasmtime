//! Interpreter tests.

use interp::Val;
use pulley_interpreter::{interp::Vm, *};
use std::{cell::UnsafeCell, fmt::Debug, ptr::NonNull};

fn encoded(ops: &[Op]) -> Vec<u8> {
    let mut encoded = vec![];
    for op in ops {
        op.encode(&mut encoded);
    }
    log::trace!("encoded: {encoded:?}");
    encoded
}

unsafe fn run(vm: &mut Vm, ops: &[Op]) -> Result<(), *mut u8> {
    let _ = env_logger::try_init();
    let ops = encoded(ops);
    let _ = vm.call(NonNull::from(&ops[..]).cast(), &[], [])?;
    Ok(())
}

unsafe fn assert_one<R0, R1, V>(
    xs: impl IntoIterator<Item = (R0, V)>,
    op: impl Into<Op> + Debug,
    result: R1,
    expected: u64,
) where
    R0: Into<AnyReg>,
    R1: Into<AnyReg>,
    V: Into<Val>,
{
    eprintln!("=======================================================");
    let mut vm = Vm::new();

    for (reg, val) in xs {
        let reg = reg.into();
        let val = val.into();
        eprintln!("{reg} = {val:#018x}");
        match (reg, val) {
            (AnyReg::X(r), Val::XReg(v)) => vm.state_mut()[r] = v,
            (AnyReg::F(r), Val::FReg(v)) => vm.state_mut()[r] = v,
            (AnyReg::V(_), Val::VReg(_)) => todo!(),
            (kind, val) => panic!("register kind and value mismatch: {kind:?} and {val:?}"),
        }
    }

    eprintln!("op = {op:?}");
    let op = op.into();

    run(&mut vm, &[op, Op::Ret(Ret {})]).expect("should not trap");

    eprintln!("expected = {expected:#018x}");

    let actual = match result.into() {
        AnyReg::X(r) => vm.state_mut()[r].get_u64(),
        AnyReg::F(r) => vm.state_mut()[r].get_f64().to_bits(),
        AnyReg::V(_) => todo!(),
    };
    eprintln!("actual   = {actual:#018x}");

    assert_eq!(expected, actual);
}

fn x(x: u8) -> XReg {
    XReg::new(x).unwrap()
}

fn f(f: u8) -> FReg {
    FReg::new(f).unwrap()
}

#[test]
fn xconst8() {
    for (expected, imm) in [(42u64, 42i8), (u64::MAX, -1i8)] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678u64)],
                Xconst8 { dst: x(0), imm },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xconst16() {
    for (expected, imm) in [(42u64, 42i16), (u64::MAX, -1i16)] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678u64)],
                Xconst16 { dst: x(0), imm },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xconst32() {
    for (expected, imm) in [(42u64, 42i32), (u64::MAX, -1i32)] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678u64)],
                Xconst32 { dst: x(0), imm },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xconst64() {
    for (expected, imm) in [(42u64, 42i64), (u64::MAX, -1i64)] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678u64)],
                Xconst64 { dst: x(0), imm },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xadd32() {
    for (expected, a, b) in [
        (42u64 | 0x1234567800000000, 10u64, 32u64),
        (0x1234567800000000, u32::MAX as _, 1),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xadd32 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xadd64() {
    for (expected, a, b) in [(42u64, 10u64, 32u64), (0, u64::MAX, 1)] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xadd64 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xeq64() {
    for (expected, a, b) in [
        (1u64, 0u64, 0u64),
        (0, 0, 1),
        (1, u64::MAX, u64::MAX),
        (0, u64::MAX, u64::MAX - 1),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xeq64 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xneq64() {
    for (expected, a, b) in [
        (0u64, 0u64, 0u64),
        (1, 0, 1),
        (0, u64::MAX, u64::MAX),
        (1, u64::MAX, u64::MAX - 1),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xneq64 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xslt64() {
    for (expected, a, b) in [
        (0u64, 0u64, 0u64),
        (0, 1, 0),
        (1, 0, 1),
        (0, 0, -1 as _),
        (1, -1 as _, 0),
        (0, i64::MAX as u64, i64::MAX as u64),
        (0, i64::MAX as u64, i64::MAX as u64 - 1),
        (1, i64::MAX as u64 - 1, i64::MAX as u64),
        (0, i64::MIN as u64, i64::MIN as u64),
        (0, i64::MIN as u64 + 1, i64::MIN as u64),
        (1, i64::MIN as u64, i64::MIN as u64 + 1),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xslt64 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xslteq64() {
    for (expected, a, b) in [
        (1u64, 0u64, 0u64),
        (0, 1, 0),
        (1, 0, 1),
        (0, 0, -1 as _),
        (1, -1 as _, 0),
        (1, i64::MAX as u64, i64::MAX as u64),
        (0, i64::MAX as u64, i64::MAX as u64 - 1),
        (1, i64::MAX as u64 - 1, i64::MAX as u64),
        (1, i64::MIN as u64, i64::MIN as u64),
        (0, i64::MIN as u64 + 1, i64::MIN as u64),
        (1, i64::MIN as u64, i64::MIN as u64 + 1),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xslteq64 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xult64() {
    for (expected, a, b) in [
        (0u64, 0u64, 0u64),
        (0, 1, 0),
        (1, 0, 1),
        (0, u64::MAX, u64::MAX),
        (0, u64::MAX, u64::MAX - 1),
        (1, u64::MAX - 1, u64::MAX),
        (0, i64::MIN as u64, 0),
        (1, 0, i64::MIN as u64),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xult64 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xulteq64() {
    for (expected, a, b) in [
        (1u64, 0u64, 0u64),
        (0, 1, 0),
        (1, 0, 1),
        (1, u64::MAX, u64::MAX),
        (0, u64::MAX, u64::MAX - 1),
        (1, u64::MAX - 1, u64::MAX),
        (0, i64::MIN as u64, 0),
        (1, 0, i64::MIN as u64),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xulteq64 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xeq32() {
    for (expected, a, b) in [
        (1u64, 0u64, 0u64),
        (0, 0, 1),
        (1, u64::MAX, u64::MAX),
        (0, u64::MAX, u64::MAX - 1),
        (1, 0xffffffff00000001, 1),
        (0, 0xffffffff00000000, 1),
        (0, 0xffffffff00000001, 0),
        (1, 0xffffffff00000000, 0),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xeq32 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xneq32() {
    for (expected, a, b) in [
        (0u64, 0u64, 0u64),
        (1, 0, 1),
        (0, u64::MAX, u64::MAX),
        (1, u64::MAX, u64::MAX - 1),
        (0, 0xffffffff00000000, 0),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xneq32 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xslt32() {
    for (expected, a, b) in [
        (0u64, 0u64, 0u64),
        (0, 1, 0),
        (1, 0, 1),
        (0, 0, -1 as _),
        (1, -1 as _, 0),
        (0, i64::MAX as u64, i64::MAX as u64),
        (0, i64::MAX as u64, i64::MAX as u64 - 1),
        (1, i64::MAX as u64 - 1, i64::MAX as u64),
        (0, i64::MIN as u64, i64::MIN as u64),
        (0, i64::MIN as u64 + 1, i64::MIN as u64),
        (1, i64::MIN as u64, i64::MIN as u64 + 1),
        (1, 0x00000000ffffffff, 0),
        (0, 0, 0x00000000ffffffff),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xslt32 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xslteq32() {
    for (expected, a, b) in [
        (1u64, 0u64, 0u64),
        (0, 1, 0),
        (1, 0, 1),
        (0, 0, -1 as _),
        (1, -1 as _, 0),
        (1, i64::MAX as u64, i64::MAX as u64),
        (0, i64::MAX as u64, i64::MAX as u64 - 1),
        (1, i64::MAX as u64 - 1, i64::MAX as u64),
        (1, i64::MIN as u64, i64::MIN as u64),
        (0, i64::MIN as u64 + 1, i64::MIN as u64),
        (1, i64::MIN as u64, i64::MIN as u64 + 1),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xslteq32 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xult32() {
    for (expected, a, b) in [
        (0u64, 0u64, 0u64),
        (0, 1, 0),
        (1, 0, 1),
        (0, 0x00000000ffffffff, 0xfffffffffffffffe),
        (1, 0xfffffffffffffffe, 0x00000000ffffffff),
        (0, 0x00000000ffffffff, 0xffffffffffffffff),
        (0, 0xfffffffffffffffe, 0x00000000fffffffe),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xult32 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn xulteq32() {
    for (expected, a, b) in [
        (1u64, 0u64, 0u64),
        (0, 1, 0),
        (1, 0, 1),
        (0, 0x00000000ffffffff, 0xfffffffffffffffe),
        (1, 0xfffffffffffffffe, 0x00000000ffffffff),
        (1, 0x00000000ffffffff, 0xffffffffffffffff),
        (1, 0xfffffffffffffffe, 0x00000000fffffffe),
    ] {
        unsafe {
            assert_one(
                [(x(0), 0x1234567812345678), (x(1), a), (x(2), b)],
                Xulteq32 {
                    dst: x(0),
                    src1: x(1),
                    src2: x(2),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn load32_u() {
    let a = UnsafeCell::new(11u32);
    let b = UnsafeCell::new(22u32);
    let c = UnsafeCell::new(33u32);
    let d = UnsafeCell::new(i32::MIN as u32);

    for (expected, addr) in [
        (11, a.get()),
        (22, b.get()),
        (33, c.get()),
        (i32::MIN as u32 as u64, d.get()),
    ] {
        unsafe {
            assert_one(
                [
                    (x(0), Val::from(0x1234567812345678u64)),
                    (x(1), Val::from(addr.cast::<u8>())),
                ],
                Load32U {
                    dst: x(0),
                    ptr: x(1),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn load32_s() {
    let a = UnsafeCell::new(11u32);
    let b = UnsafeCell::new(22u32);
    let c = UnsafeCell::new(33u32);
    let d = UnsafeCell::new(-1i32 as u32);

    for (expected, addr) in [
        (11, a.get()),
        (22, b.get()),
        (33, c.get()),
        (-1i64 as u64, d.get()),
    ] {
        unsafe {
            assert_one(
                [
                    (x(0), Val::from(0x1234567812345678u64)),
                    (x(1), Val::from(addr.cast::<u8>())),
                ],
                Load32S {
                    dst: x(0),
                    ptr: x(1),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn load64() {
    let a = UnsafeCell::new(11u64);
    let b = UnsafeCell::new(22u64);
    let c = UnsafeCell::new(33u64);
    let d = UnsafeCell::new(-1i64 as u64);

    for (expected, addr) in [
        (11, a.get()),
        (22, b.get()),
        (33, c.get()),
        (-1i64 as u64, d.get()),
    ] {
        unsafe {
            assert_one(
                [
                    (x(0), Val::from(0x1234567812345678u64)),
                    (x(1), Val::from(addr)),
                ],
                Load64 {
                    dst: x(0),
                    ptr: x(1),
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn load32_u_offset8() {
    let a = UnsafeCell::new([11u32, 22]);
    let b = UnsafeCell::new([33u32, 44]);
    let c = UnsafeCell::new([55u32, 66]);
    let d = UnsafeCell::new([i32::MIN as u32, i32::MAX as u32]);

    for (expected, addr, offset) in [
        (11, a.get(), 0),
        (22, a.get(), 4),
        (33, b.get(), 0),
        (44, b.get(), 4),
        (55, c.get(), 0),
        (66, c.get(), 4),
        (i32::MIN as u32 as u64, d.get(), 0),
        (i32::MAX as u32 as u64, d.get(), 4),
    ] {
        unsafe {
            assert_one(
                [
                    (x(0), Val::from(0x1234567812345678u64)),
                    (x(1), Val::from(addr.cast::<u8>())),
                ],
                Load32UOffset8 {
                    dst: x(0),
                    ptr: x(1),
                    offset,
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn load32_s_offset8() {
    let a = UnsafeCell::new([11u32, 22]);
    let b = UnsafeCell::new([33u32, 44]);
    let c = UnsafeCell::new([55u32, 66]);
    let d = UnsafeCell::new([-1i32 as u32, i32::MAX as u32]);

    for (expected, addr, offset) in [
        (11, a.get(), 0),
        (22, a.get(), 4),
        (33, b.get(), 0),
        (44, b.get(), 4),
        (55, c.get(), 0),
        (55, unsafe { c.get().byte_add(4) }, -4),
        (66, c.get(), 4),
        (-1i64 as u64, d.get(), 0),
        (i32::MAX as u32 as u64, d.get(), 4),
    ] {
        unsafe {
            assert_one(
                [
                    (x(0), Val::from(0x1234567812345678u64)),
                    (x(1), Val::from(addr.cast::<u8>())),
                ],
                Load32SOffset8 {
                    dst: x(0),
                    ptr: x(1),
                    offset,
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn load64_offset8() {
    let a = UnsafeCell::new([11u64, 22]);
    let b = UnsafeCell::new([33u64, 44]);
    let c = UnsafeCell::new([55u64, 66]);
    let d = UnsafeCell::new([-1i64 as u64, i64::MAX as u64]);

    for (expected, addr, offset) in [
        (11, a.get(), 0),
        (22, a.get(), 8),
        (33, b.get(), 0),
        (44, b.get(), 8),
        (55, c.get(), 0),
        (66, c.get(), 8),
        (-1i64 as u64, d.get(), 0),
        (i64::MAX as u64, d.get(), 8),
    ] {
        unsafe {
            assert_one(
                [
                    (x(0), Val::from(0x1234567812345678u64)),
                    (x(1), Val::from(addr)),
                ],
                Load64Offset8 {
                    dst: x(0),
                    ptr: x(1),
                    offset,
                },
                x(0),
                expected,
            );
        }
    }
}

#[test]
fn store32() {
    let a = UnsafeCell::new([0x12u8, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78]);
    let b = UnsafeCell::new([0x12u8, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78]);
    let c = UnsafeCell::new([0x12u8, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78]);

    unsafe {
        for (val, addr) in [
            (0x11111111u32, a.get()),
            (0x22222222, b.get().byte_add(4)),
            (0x33333333, c.get().byte_add(2)),
        ] {
            let val = val as u64;
            assert_one(
                [(x(0), Val::from(addr)), (x(1), Val::from(val))],
                Store32 {
                    ptr: x(0),
                    src: x(1),
                },
                x(1),
                val,
            );
        }
    }

    let a = u64::from_be_bytes(a.into_inner());
    let expected = 0x1111111112345678u64;
    eprintln!("expected(a) = {expected:#018x}");
    eprintln!("actual(a)   = {a:#018x}");
    assert_eq!(a, expected);

    let b = u64::from_be_bytes(b.into_inner());
    let expected = 0x1234567822222222u64;
    eprintln!("expected(b) = {expected:#018x}");
    eprintln!("actual(b)   = {b:#018x}");
    assert_eq!(b, expected);

    let c = u64::from_be_bytes(c.into_inner());
    let expected = 0x1234333333335678u64;
    eprintln!("expected(c) = {expected:#018x}");
    eprintln!("actual(c)   = {c:#018x}");
    assert_eq!(c, expected);
}

#[test]
fn store64() {
    let a = UnsafeCell::new(0x1234567812345678);
    let b = UnsafeCell::new(0x1234567812345678);
    let c = UnsafeCell::new(0x1234567812345678);

    unsafe {
        for (val, addr) in [
            (0x1111111111111111u64, a.get()),
            (0x2222222222222222, b.get()),
            (0x3333333333333333, c.get()),
        ] {
            assert_one(
                [(x(0), Val::from(addr)), (x(1), Val::from(val))],
                Store64 {
                    ptr: x(0),
                    src: x(1),
                },
                x(1),
                val,
            );
        }
    }

    let a = a.into_inner();
    let expected = 0x1111111111111111u64;
    eprintln!("expected(a) = {expected:#018x}");
    eprintln!("actual(a)   = {a:#018x}");
    assert_eq!(a, expected);

    let b = b.into_inner();
    let expected = 0x2222222222222222u64;
    eprintln!("expected(b) = {expected:#018x}");
    eprintln!("actual(b)   = {b:#018x}");
    assert_eq!(b, expected);

    let c = c.into_inner();
    let expected = 0x3333333333333333u64;
    eprintln!("expected(c) = {expected:#018x}");
    eprintln!("actual(c)   = {c:#018x}");
    assert_eq!(c, expected);
}

#[test]
fn store32_offset8() {
    let a = UnsafeCell::new([0x12u8, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78]);
    let b = UnsafeCell::new([0x12u8, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78]);
    let c = UnsafeCell::new([0x12u8, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78]);

    unsafe {
        for (val, addr, offset) in [
            (0x11111111u32, a.get(), 0),
            (0x22222222, b.get(), 4),
            (0x33333333, c.get(), 2),
        ] {
            let val = val as u64;
            assert_one(
                [(x(0), Val::from(addr)), (x(1), Val::from(val))],
                Store32SOffset8 {
                    ptr: x(0),
                    src: x(1),
                    offset,
                },
                x(1),
                val,
            );
        }
    }

    let a = u64::from_be_bytes(a.into_inner());
    let expected = 0x1111111112345678u64;
    eprintln!("expected(a) = {expected:#018x}");
    eprintln!("actual(a)   = {a:#018x}");
    assert_eq!(a, expected);

    let b = u64::from_be_bytes(b.into_inner());
    let expected = 0x1234567822222222u64;
    eprintln!("expected(b) = {expected:#018x}");
    eprintln!("actual(b)   = {b:#018x}");
    assert_eq!(b, expected);

    let c = u64::from_be_bytes(c.into_inner());
    let expected = 0x1234333333335678u64;
    eprintln!("expected(c) = {expected:#018x}");
    eprintln!("actual(c)   = {c:#018x}");
    assert_eq!(c, expected);
}

#[test]
fn store64_offset8() {
    let a = UnsafeCell::new([0x1234567812345678, 0x1234567812345678, 0x1234567812345678]);

    unsafe {
        for (val, addr, offset) in [
            (0x1111111111111111u64, a.get(), 0),
            (0x2222222222222222, a.get(), 8),
            (0x3333333333333333, a.get(), 16),
        ] {
            assert_one(
                [(x(0), Val::from(addr)), (x(1), Val::from(val))],
                Store64Offset8 {
                    ptr: x(0),
                    src: x(1),
                    offset,
                },
                x(1),
                val,
            );
        }
    }

    let [a, b, c] = a.into_inner();

    let expected = 0x1111111111111111u64;
    eprintln!("expected(a) = {expected:#018x}");
    eprintln!("actual(a)   = {a:#018x}");
    assert_eq!(a, expected);

    let expected = 0x2222222222222222u64;
    eprintln!("expected(b) = {expected:#018x}");
    eprintln!("actual(b)   = {b:#018x}");
    assert_eq!(b, expected);

    let expected = 0x3333333333333333u64;
    eprintln!("expected(c) = {expected:#018x}");
    eprintln!("actual(c)   = {c:#018x}");
    assert_eq!(c, expected);
}

#[test]
fn bitcast_int_from_float_32() {
    for val in [
        0.0,
        1.0,
        9.87654321,
        f32::MAX,
        f32::MIN,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::EPSILON,
        f32::MIN_POSITIVE,
    ] {
        unsafe {
            assert_one(
                [(f(0), val)],
                BitcastIntFromFloat32 {
                    dst: x(0),
                    src: f(0),
                },
                x(0),
                val.to_bits() as u64,
            );
        }
    }
}

#[test]
fn bitcast_int_from_float_64() {
    for val in [
        0.0,
        1.0,
        9.87654321,
        f64::MAX,
        f64::MIN,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::EPSILON,
        f64::MIN_POSITIVE,
    ] {
        unsafe {
            assert_one(
                [(f(0), val)],
                BitcastIntFromFloat64 {
                    dst: x(0),
                    src: f(0),
                },
                x(0),
                val.to_bits(),
            );
        }
    }
}

#[test]
fn bitcast_float_from_int_32() {
    for val in [
        0.0,
        1.0,
        9.87654321,
        f32::MAX,
        f32::MIN,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::EPSILON,
        f32::MIN_POSITIVE,
    ] {
        let val = val.to_bits() as u64;
        unsafe {
            assert_one(
                [(x(0), val)],
                BitcastFloatFromInt32 {
                    dst: f(0),
                    src: x(0),
                },
                f(0),
                val,
            );
        }
    }
}

#[test]
fn bitcast_float_from_int_64() {
    for val in [
        0.0,
        1.0,
        9.87654321,
        f64::MAX,
        f64::MIN,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::EPSILON,
        f64::MIN_POSITIVE,
    ] {
        let val = val.to_bits();
        unsafe {
            assert_one(
                [(x(0), val)],
                BitcastFloatFromInt64 {
                    dst: f(0),
                    src: x(0),
                },
                f(0),
                val,
            );
        }
    }
}

#[test]
fn trap() {
    let mut vm = Vm::new();
    let dst = XReg::new(0).unwrap();

    unsafe {
        run(
            &mut vm,
            &[
                Op::Xconst16(Xconst16 { dst, imm: 1 }),
                Op::ExtendedOp(ExtendedOp::Trap(Trap {})),
                Op::Xconst16(Xconst16 { dst, imm: 2 }),
                Op::Ret(Ret {}),
            ],
        )
        .unwrap_err();
    }

    // `dst` should not have been written to the second time.
    assert_eq!(vm.state()[dst].get_u32(), 1);
}

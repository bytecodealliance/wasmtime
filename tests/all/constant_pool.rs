//! Tests for Cranelift's `separate_constant_pool` setting, which keeps the
//! constants loaded by compiled code out of executable pages.

#![cfg(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(miri)))]

use object::{Object, ObjectSection};
use wasmtime::*;

const CONSTANT: [u8; 16] = [
    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xf0, 0x0f,
];

const WAT: &str = r#"
    (module
        (func (export "constant") (result v128)
            v128.const i8x16
                0x11 0x22 0x33 0x44 0x55 0x66 0x77 0x88
                0x99 0xaa 0xbb 0xcc 0xdd 0xee 0xf0 0x0f)
    )
"#;

fn engine(separate_constant_pool: bool) -> Result<Engine> {
    let mut config = Config::new();
    unsafe {
        config.cranelift_flag_set(
            "separate_constant_pool",
            if separate_constant_pool {
                "true"
            } else {
                "false"
            },
        );
    }
    Engine::new(&config)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn constants_are_outside_the_text_section() -> Result<()> {
    let engine = engine(true)?;
    let cwasm = engine.precompile_module(WAT.as_bytes())?;
    let elf = object::File::parse(&cwasm[..])?;

    let text = elf.section_by_name(".text").unwrap();
    assert!(!contains(text.data()?, &CONSTANT));

    // The constants are in a section of their own, right after the text
    // section and before the unwind information.
    let constants = elf.section_by_name(".wasmtime.constants").unwrap();
    assert!(contains(constants.data()?, &CONSTANT));
    assert_eq!(constants.file_range().unwrap().0, {
        let (offset, size) = text.file_range().unwrap();
        offset + size
    });
    assert_eq!(text.size() % 0x1000, 0);
    if let Some(eh_frame) = elf.section_by_name(".eh_frame") {
        let (offset, size) = constants.file_range().unwrap();
        assert_eq!(eh_frame.file_range().unwrap().0, offset + size);
    }
    Ok(())
}

#[test]
fn constants_are_in_the_text_section_by_default() -> Result<()> {
    let engine = engine(false)?;
    let cwasm = engine.precompile_module(WAT.as_bytes())?;
    let elf = object::File::parse(&cwasm[..])?;

    assert!(contains(
        elf.section_by_name(".text").unwrap().data()?,
        &CONSTANT
    ));
    assert!(elf.section_by_name(".wasmtime.constants").is_none());
    Ok(())
}

#[test]
fn code_loads_constants_from_the_new_section() -> Result<()> {
    let engine = engine(true)?;
    let cwasm = engine.precompile_module(WAT.as_bytes())?;
    let module = unsafe { Module::deserialize(&engine, &cwasm)? };
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let constant = instance.get_typed_func::<(), V128>(&mut store, "constant")?;
    assert_eq!(
        constant.call(&mut store, ())?,
        V128::from(u128::from_le_bytes(CONSTANT))
    );
    Ok(())
}

/// The pool spans several 4 KiB pages, so loads need different page offsets
/// and, on AArch64, different `adrp` page deltas.
#[test]
fn constants_spanning_several_pages_load_correctly() -> Result<()> {
    use std::fmt::Write;

    const N: usize = 600;
    let consts: Vec<[u64; 2]> = (0..N as u64)
        .map(|i| {
            let x = (i + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            [x, x.rotate_left(29) ^ i]
        })
        .collect();

    let mut wat = String::from("(module (func (export \"f\") (param i64) (result i64)\n");
    wat.push_str("(local v128) (local.set 1 (i64x2.splat (local.get 0)))\n");
    for [lo, hi] in &consts {
        writeln!(
            wat,
            "(local.set 1 (i64x2.add (v128.xor (local.get 1) (v128.const i64x2 {lo} {hi})) \
             (v128.const i64x2 {lo} {hi})))"
        )?;
    }
    wat.push_str(
        "(i64.xor (i64x2.extract_lane 0 (local.get 1)) (i64x2.extract_lane 1 (local.get 1)))))",
    );

    let arg = 7_u64;
    let mut acc = [arg; 2];
    for c in &consts {
        for lane in 0..2 {
            acc[lane] = (acc[lane] ^ c[lane]).wrapping_add(c[lane]);
        }
    }
    let expected = (acc[0] ^ acc[1]) as i64;

    let engine = engine(true)?;
    let cwasm = engine.precompile_module(wat.as_bytes())?;
    let elf = object::File::parse(&cwasm[..])?;
    assert!(elf.section_by_name(".wasmtime.constants").unwrap().size() > 2 * 0x1000);

    let module = unsafe { Module::deserialize(&engine, &cwasm)? };
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<i64, i64>(&mut store, "f")?;
    assert_eq!(f.call(&mut store, arg as i64)?, expected);
    Ok(())
}

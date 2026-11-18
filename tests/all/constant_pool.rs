//! Tests for Cranelift's `separate_constant_pool` setting, which keeps the
//! constants loaded by compiled code out of executable pages.

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
#[cfg_attr(any(not(target_arch = "x86_64"), miri), ignore)]
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
#[cfg_attr(any(not(target_arch = "x86_64"), miri), ignore)]
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
#[cfg_attr(any(not(target_arch = "x86_64"), miri), ignore)]
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

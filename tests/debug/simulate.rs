use super::dump::{get_dwarfdump, DwarfDumpSection};
use super::obj::compile_cranelift;
use anyhow::{format_err, Result};
use filecheck::{CheckerBuilder, NO_VARIABLES};
use tempfile::NamedTempFile;
use wat::parse_str;

#[allow(dead_code)]
fn check_wat(wat: &str) -> Result<()> {
    let wasm = parse_str(wat)?;
    let obj_file = NamedTempFile::new()?;
    let obj_path = obj_file.path().to_str().unwrap();
    compile_cranelift(&wasm, None, obj_path)?;
    let dump = get_dwarfdump(obj_path, DwarfDumpSection::DebugInfo)?;
    let mut builder = CheckerBuilder::new();
    builder
        .text(wat)
        .map_err(|e| format_err!("unable to build checker: {:?}", e))?;
    let checker = builder.finish();
    let check = checker
        .explain(&dump, NO_VARIABLES)
        .map_err(|e| format_err!("{:?}", e))?;
    assert!(check.0, "didn't pass check {}", check.1);
    Ok(())
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
fn test_debug_dwarf_simulate_simple_x86_64() -> Result<()> {
    check_wat(
        r#"
;; check: DW_TAG_compile_unit 
(module
;; check: DW_TAG_subprogram 
;; check: DW_AT_name	("wasm-function[0]")
;; check:   DW_TAG_formal_parameter
;; check:     DW_AT_name	("var0")
;; check:     DW_AT_type
;; sameln:	            "i32"
;; check:   DW_TAG_variable
;; check:     DW_AT_name	("var1")
;; check:     DW_AT_type
;; sameln:	            "i32"
    (func (param i32) (result i32)
        (local i32)
        local.get 0
        local.set 1
        local.get 1
    )
)"#,
    )
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
fn test_debug_dwarf_simulate_with_imports_x86_64() -> Result<()> {
    check_wat(
        r#"
;; check: DW_TAG_compile_unit 
(module
;; check: DW_TAG_subprogram 
;; check: DW_AT_name	("func1")
    (import "foo" "bar" (func $import1) )
    (func $func1 (result i32)
        i32.const 1
    )
)"#,
    )
}

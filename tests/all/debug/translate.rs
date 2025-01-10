use super::dump::{DwarfDumpSection, get_dwarfdump};
use super::obj::compile_cranelift;
use anyhow::{Result, format_err};
use filecheck::{CheckerBuilder, NO_VARIABLES};
use std::fs::read;
use tempfile::NamedTempFile;

#[allow(dead_code)]
fn check_wasm(wasm_path: &str, directives: &str) -> Result<()> {
    let wasm = read(wasm_path)?;
    let obj_file = NamedTempFile::new()?;
    let obj_path = obj_file.path().to_str().unwrap();
    compile_cranelift(&wasm, Some(wasm_path.as_ref()), None, obj_path)?;
    let dump = get_dwarfdump(obj_path, DwarfDumpSection::DebugInfo)?;
    let mut builder = CheckerBuilder::new();
    builder
        .text(directives)
        .map_err(|e| format_err!("unable to build checker: {:?}", e))?;
    let checker = builder.finish();
    let check = checker
        .explain(&dump, NO_VARIABLES)
        .map_err(|e| format_err!("{:?}", e))?;
    assert!(check.0, "didn't pass check {}", check.1);
    Ok(())
}

#[allow(dead_code)]
fn check_line_program(wasm_path: &str, directives: &str) -> Result<()> {
    let wasm = read(wasm_path)?;
    let obj_file = NamedTempFile::new()?;
    let obj_path = obj_file.path().to_str().unwrap();
    compile_cranelift(&wasm, Some(wasm_path.as_ref()), None, obj_path)?;
    let dump = get_dwarfdump(obj_path, DwarfDumpSection::DebugLine)?;
    let mut builder = CheckerBuilder::new();
    builder
        .text(directives)
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
fn test_debug_dwarf_translate_dead_code() -> Result<()> {
    check_wasm(
        "tests/all/debug/testsuite/dead_code.wasm",
        r##"
check: DW_TAG_compile_unit
# We don't have "bar" function because it is dead code
not:      DW_AT_name	("bar")
# We have "foo" function
check: DW_TAG_subprogram
check:      DW_AT_name	("foo")
# We have "baz" function
# it was marked `noinline` so isn't dead code
check: DW_TAG_subprogram
check:      DW_AT_name	("baz")
    "##,
    )
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
fn test_debug_dwarf_translate() -> Result<()> {
    check_wasm(
        "tests/all/debug/testsuite/fib-wasm.wasm",
        r##"
check: DW_TAG_compile_unit
# We have "fib" function
check: DW_TAG_subprogram
check:      DW_AT_name	("fib")
# Accepts one parameter
check:      DW_TAG_formal_parameter
check:        DW_AT_name	("n")
check:        DW_AT_decl_line	(8)
# Has four locals: t, a, b, i
check:      DW_TAG_variable
check:        DW_AT_name	("t")
check:        DW_AT_decl_line	(9)
check:      DW_TAG_variable
check:        DW_AT_name	("a")
check:      DW_TAG_variable
check:        DW_AT_name	("b")
check:      DW_TAG_variable
check:        DW_AT_name	("i")
check:        DW_AT_decl_line	(10)
    "##,
    )
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
fn test_debug_dwarf5_translate() -> Result<()> {
    check_wasm(
        "tests/all/debug/testsuite/fib-wasm-dwarf5.wasm",
        r##"
check: DW_TAG_compile_unit
# We have "fib" function
check: DW_TAG_subprogram
check:      DW_AT_name	("fib")
# Accepts one parameter
check:      DW_TAG_formal_parameter
check:        DW_AT_name	("n")
check:        DW_AT_decl_line	(8)
# Has four locals: t, a, b, i
check:      DW_TAG_variable
check:        DW_AT_name	("t")
check:        DW_AT_decl_line	(9)
check:      DW_TAG_variable
check:        DW_AT_name	("a")
check:      DW_TAG_variable
check:        DW_AT_name	("b")
check:      DW_TAG_variable
check:        DW_AT_name	("i")
check:        DW_AT_decl_line	(10)
    "##,
    )
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
fn test_debug_split_dwarf4_translate() -> Result<()> {
    check_wasm(
        "tests/all/debug/testsuite/fib-wasm-split4.wasm",
        r##"
check: DW_TAG_compile_unit
# We have "fib" function
check: DW_TAG_subprogram
check:      DW_AT_name	("fib")
# Accepts one parameter
check:      DW_TAG_formal_parameter
check:        DW_AT_name	("n")
check:        DW_AT_decl_line	(8)
# Has four locals: t, a, b, i
check:      DW_TAG_variable
check:        DW_AT_name	("t")
check:        DW_AT_decl_line	(9)
check:      DW_TAG_variable
check:        DW_AT_name	("a")
check:      DW_TAG_variable
check:        DW_AT_name	("b")
check:      DW_TAG_variable
check:        DW_AT_name	("i")
check:        DW_AT_decl_line	(10)
    "##,
    )
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
fn test_debug_dwarf_translate_generated() -> Result<()> {
    check_wasm(
        "tests/all/debug/testsuite/fraction-norm.wasm",
        r##"
check: DW_TAG_compile_unit
check: DW_TAG_compile_unit
check:   DW_AT_producer	("wasmtime")
check:   DW_AT_name	("<gen-$(=\d+)>.wasm")
check:   DW_AT_comp_dir	("/<wasm-module>")
check:   DW_TAG_subprogram
check:     DW_AT_name	("__wasm_call_ctors")
check:     DW_AT_decl_file	("/<wasm-module>/<gen-$(=\d+)>.wasm")
check:     DW_AT_decl_line	(124)
    "##,
    )
}

#[test]
#[ignore]
#[cfg(all(
    any(target_os = "linux", target_os = "macos"),
    target_pointer_width = "64"
))]
fn test_debug_dwarf_translate_fission() -> Result<()> {
    check_wasm(
        "tests/all/debug/testsuite/dwarf_fission.wasm",
        r##"
check: DW_TAG_compile_unit
check:   DW_AT_producer	("clang version 19.0.0git (https:/github.com/llvm/llvm-project ccdebbae4d77d3efc236af92c22941de5d437e01)")
check:   DW_AT_language	(DW_LANG_C11)
check:   DW_AT_name	("dwarf_fission.c")
check:   DW_AT_ranges	(0x00000000
check:   DW_AT_stmt_list	(0x00000000)
check:   DW_AT_comp_dir	("C:\\tmp\\dwarfc")
    "##,
    )
}

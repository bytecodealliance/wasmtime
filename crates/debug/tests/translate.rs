use anyhow::{format_err, Result};
use dump::{get_dwarfdump, DwarfDumpSection};
use filecheck::{CheckerBuilder, NO_VARIABLES};
use obj::compile_cranelift;
use std::fs::read;
use tempfile::NamedTempFile;

mod dump;
mod obj;

fn check_wasm(wasm_path: &str, directives: &str) -> Result<()> {
    let wasm = read(wasm_path)?;
    let obj_file = NamedTempFile::new()?;
    let obj_path = obj_file.path().to_str().unwrap();
    compile_cranelift(&wasm, None, obj_path)?;
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

#[test]
fn test_dwarf_translate() -> Result<()> {
    check_wasm(
        "examples/fib-wasm.wasm",
        r##"
check: DW_TAG_compile_unit
# We have "fib" function
check: DW_TAG_subprogram
check:      DW_AT_name	("fib")
# Accepts one parameter
check:      DW_TAG_formal_parameter
check:        DW_AT_name	("n")
check:        DW_AT_decl_line	(5)
# Has four locals: i, t, a, b
check:      DW_TAG_variable
check:        DW_AT_name	("i")
check:        DW_AT_decl_line	(6)
check:      DW_TAG_variable
check:        DW_AT_name	("t")
check:      DW_TAG_variable
# checking if the variable location was transformed
check:        DW_AT_location
regex: GET_REG_LOCATION=DW_OP_reg\d+
nextln:          $GET_REG_LOCATION
nextln:          $GET_REG_LOCATION
nextln:          $GET_REG_LOCATION
check:        DW_AT_name	("a")
check:      DW_TAG_variable
check:        DW_AT_name	("b")
    "##,
    )
}

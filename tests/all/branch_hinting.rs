//! Tests for the WebAssembly branch-hinting proposal. The spec conformance
//! tests live in this repo at `tests/spec_testsuite/custom/branch_hint.wast`;
//! per that spec hints may only be attached to `br_if` and `if` instructions.
//! Proposal: <https://github.com/WebAssembly/branch-hinting>.

use wasmtime::*;

// Both functions return 10 when their condition is true (arg != 0) and 20
// otherwise. The `(@metadata.code.branch_hint ...)` annotation must immediately
// precede the `if`/`br_if` it applies to.
const MODULE: &str = r#"
(module
  (func (export "via_if") (param i32) (result i32)
    local.get 0
    (@metadata.code.branch_hint "\00")
    if (result i32)
      i32.const 10
    else
      i32.const 20
    end)

  (func (export "via_br_if") (param i32) (result i32)
    (block $b (result i32)
      i32.const 10
      local.get 0
      (@metadata.code.branch_hint "\01")
      br_if $b
      drop
      i32.const 20)))
"#;

fn results(branch_hinting: Option<bool>) -> Result<Vec<(i32, i32, i32)>> {
    let mut config = Config::new();
    if let Some(enable) = branch_hinting {
        config.wasm_branch_hinting(enable);
    }
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, MODULE)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let via_if = instance.get_typed_func::<i32, i32>(&mut store, "via_if")?;
    let via_br_if = instance.get_typed_func::<i32, i32>(&mut store, "via_br_if")?;

    let mut out = Vec::new();
    for arg in [0, 1, 7, -3] {
        out.push((
            arg,
            via_if.call(&mut store, arg)?,
            via_br_if.call(&mut store, arg)?,
        ));
    }
    Ok(out)
}

// A module carrying branch hints must compile and run identically whether the
// proposal is enabled, explicitly disabled, or left at its (disabled) default:
// hints are advisory and never change semantics.
#[test]
#[cfg_attr(miri, ignore)]
fn branch_hints_are_semantically_neutral() -> Result<()> {
    let enabled = results(Some(true))?;
    assert_eq!(enabled, results(Some(false))?);
    assert_eq!(enabled, results(None)?);

    for (arg, via_if, via_br_if) in enabled {
        let expected = if arg != 0 { 10 } else { 20 };
        assert_eq!(via_if, expected, "via_if({arg})");
        assert_eq!(via_br_if, expected, "via_br_if({arg})");
    }
    Ok(())
}

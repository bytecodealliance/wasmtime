//! Tests for `br_table` lowering, especially with very large jump tables.

use wasmtime::*;
use wasmtime_test_macros::wasmtime_test;

/// Generate a module with a single exported function `f` that executes a
/// `br_table` with `n` non-default targets. Every non-default target branches
/// to a block that returns 5; the default target returns 0.
fn br_table_module(n: usize) -> String {
    let mut wat = String::with_capacity(n * 2 + 256);
    wat.push_str(
        r#"
(module
  (func (export "f") (param i32) (result i32) (local $r i32)
    (block $exit
      (block $t
        (br_table
"#,
    );
    for _ in 0..n {
        wat.push_str("0 ");
    }
    wat.push_str(
        r#"
          1 (local.get 0)))
      (local.set $r (i32.const 5)))
    (local.get $r)))
"#,
    );
    wat
}

fn run_br_table_test(config: &mut Config, n: usize) -> Result<()> {
    let engine = Engine::new(config)?;
    let module = Module::new(&engine, br_table_module(n))?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<u32, u32>(&mut store, "f")?;

    let n = u32::try_from(n).unwrap();
    for (index, expected) in [
        (0, 5),
        (5, 5),
        (n - 1, 5),
        (n, 0),
        (n + 1, 0),
        (n + 37_600, 0),
        (u32::MAX / 2, 0),
        (u32::MAX - 1, 0),
        (u32::MAX, 0),
    ] {
        assert_eq!(
            f.call(&mut store, index)?,
            expected,
            "br_table with {n} targets, index {index}"
        );
    }
    Ok(())
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn br_table_small(config: &mut Config) -> Result<()> {
    run_br_table_test(config, 16)
}

/// The largest `br_table` permitted by our implementation limit.
#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn br_table_at_limit(config: &mut Config) -> Result<()> {
    run_br_table_test(config, 65_520)
}

/// One target beyond the implementation limit is rejected with an error.
#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn br_table_over_limit(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let err = Module::new(&engine, br_table_module(65_521)).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("exceeds the implementation limit of 65520"),
        "unexpected error: {err}"
    );
    Ok(())
}

/// The implementation limit also applies to `br_table`s in unreachable code.
#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn br_table_over_limit_unreachable(config: &mut Config) -> Result<()> {
    let engine = Engine::new(config)?;
    let mut wat = String::from("(module (func (param i32) unreachable local.get 0 br_table ");
    for _ in 0..65_521 {
        wat.push_str("0 ");
    }
    wat.push_str("0))");
    let err = Module::new(&engine, wat).unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("exceeds the implementation limit of 65520"),
        "unexpected error: {err}"
    );
    Ok(())
}

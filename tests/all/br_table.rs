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

/// A jump table large enough that the default target is more than 1 MiB away
/// from the start of the jump-table sequence (each entry is 4 bytes on
/// AArch64, so this is beyond the range of a 19-bit conditional branch).
#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn br_table_huge(config: &mut Config) -> Result<()> {
    run_br_table_test(config, 262_400)
}

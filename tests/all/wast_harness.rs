//! Tests for the `wasmtime-wast` harness itself.

use crate::ErrorExt;
use wasmtime::Result;
use wasmtime_test_util::component::engine;
use wasmtime_wast::{Async, WastContext};

/// A component whose `f` export returns a record with two fields.
const TWO_FIELD_RECORD: &str = r#"
(component
  (core module $m
    (memory (export "mem") 1)
    (data (i32.const 8) "\01\00\00\00\02\00\00\00")
    (func (export "f") (result i32) i32.const 8)
  )
  (core instance $i (instantiate $m))
  (type $r (record (field "a" u32) (field "b" u32)))
  (export $r' "r" (type $r))
  (func (export "f") (result $r')
    (canon lift (core func $i "f") (memory (core memory $i "mem"))))
)
"#;

fn run_with_two_field_record(assertion: &str) -> Result<()> {
    let mut wast = WastContext::new(&engine(), Async::No, |_| {});
    let script = format!("{TWO_FIELD_RECORD}\n{assertion}");
    wast.run_wast("record.wast", script.as_bytes())
}

#[test]
#[cfg_attr(miri, ignore)]
fn assert_return_checks_record_field_count() -> Result<()> {
    run_with_two_field_record(
        r#"(assert_return (invoke "f")
             (record.const (field "a" u32.const 1) (field "b" u32.const 2)))"#,
    )?;

    let err = run_with_two_field_record(
        r#"(assert_return (invoke "f")
             (record.const (field "a" u32.const 1)))"#,
    )
    .unwrap_err();
    err.assert_contains("mismatched number of record fields: expected 1, found 2");

    let err = run_with_two_field_record(
        r#"(assert_return (invoke "f")
             (record.const
               (field "a" u32.const 1)
               (field "b" u32.const 2)
               (field "c" u32.const 3)))"#,
    )
    .unwrap_err();
    err.assert_contains("mismatched number of record fields: expected 3, found 2");

    Ok(())
}

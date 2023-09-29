#![cfg(not(miri))]

use anyhow::Result;
use wasmtime::*;

#[test]
fn code_too_large_without_panic() -> Result<()> {
    const N: usize = 120000;

    // Build a module with a function whose body will allocate too many
    // temporaries for our current (Cranelift-based) compiler backend to
    // handle. This test ensures that we propagate the failure upward
    // and return it programmatically, rather than panic'ing. If we ever
    // improve our compiler backend to actually handle such a large
    // function body, we'll need to increase the limits here too!
    let mut s = String::new();
    s.push_str("(module\n");
    s.push_str("(table 1 1 funcref)\n");
    s.push_str("(func (export \"\") (result i32)\n");
    s.push_str("i32.const 0\n");
    for _ in 0..N {
        s.push_str("table.get 0\n");
        s.push_str("ref.is_null\n");
    }
    s.push_str("))\n");

    let store = Store::<()>::default();
    let result = Module::new(store.engine(), &s);
    match result {
        Err(e) => assert!(e
            .to_string()
            .starts_with("Compilation error: Code for function is too large")),
        Ok(_) => panic!("Please adjust limits to make the module too large to compile!"),
    }
    Ok(())
}

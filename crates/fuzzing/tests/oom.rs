use std::alloc::{Layout, alloc};
use wasmtime::{Config, Result};
use wasmtime_fuzzing::oom::{OomTest, OomTestAllocator};

#[global_allocator]
static GLOBAL_ALOCATOR: OomTestAllocator = OomTestAllocator::new();

#[test]
fn smoke_test_ok() -> Result<()> {
    OomTest::new().test(|| Ok(()))
}

#[test]
fn smoke_test_missed_oom() -> Result<()> {
    let err = OomTest::new()
        .test(|| {
            let _ = unsafe { alloc(Layout::new::<u64>()) };
            Ok(())
        })
        .unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("OOM test function missed an OOM"),
        "should have missed an OOM, got: {err}"
    );
    Ok(())
}

#[test]
fn config_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        Ok(())
    })
}

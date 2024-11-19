// This test only works on Linux. It may be portable to MacOS as well,
// but the original author did not have a machine available to test it.
#![cfg(target_os = "linux")]

use wasmtime::*;

#[derive(Default)]
struct MemoryGrowFailureDetector {
    current: usize,
    desired: usize,
    error: Option<String>,
}

impl ResourceLimiter for MemoryGrowFailureDetector {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        self.current = current;
        self.desired = desired;
        Ok(true)
    }
    fn memory_grow_failed(&mut self, err: anyhow::Error) -> Result<()> {
        self.error = Some(err.to_string());
        Ok(())
    }
    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        Ok(true)
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn custom_limiter_detect_os_oom_failure() -> Result<()> {
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return Ok(());
    }

    // Skip this test if it looks like we're in a cross-compiled situation,
    // and we're emulating this test for a different platform. In that
    // scenario QEMU ignores the data rlimit, which this test relies on. See
    // QEMU commits 5dfa88f7162f ("linux-user: do setrlimit selectively") and
    // 055d92f8673c ("linux-user: do prlimit selectively") for more
    // information.
    if std::env::vars()
        .filter(|(k, _v)| k.starts_with("CARGO_TARGET") && k.ends_with("RUNNER"))
        .count()
        > 0
    {
        return Ok(());
    }

    // Default behavior of on-demand memory allocation so that a
    // memory grow will hit Linux for a larger mmap.
    let mut config = Config::new();
    config.wasm_reference_types(false);
    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);
    let module = Module::new(&engine, r#"(module (memory (export "m") 0))"#).unwrap();

    // Ask Linux to limit this process to 256MiB of memory
    let process_max_memory: usize = 256 * 1024 * 1024;
    unsafe {
        // limit process to 256MiB memory
        let rlimit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: process_max_memory as u64,
        };
        let res = libc::setrlimit(libc::RLIMIT_DATA, &rlimit);
        assert_eq!(res, 0, "setrlimit failed: {res}");
    };

    let context = MemoryGrowFailureDetector::default();

    let mut store = Store::new(&engine, context);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let memory = instance.get_memory(&mut store, "m").unwrap();

    // Small (640KiB) grow should succeed
    memory.grow(&mut store, 10).unwrap();
    assert!(store.data().error.is_none());
    assert_eq!(store.data().current, 0);
    assert_eq!(store.data().desired, 10 * 64 * 1024);

    // Try to grow past the process's memory limit.
    // This should fail.
    let pages_exceeding_limit = process_max_memory / (64 * 1024);
    let err_msg = memory
        .grow(&mut store, pages_exceeding_limit as u64)
        .unwrap_err()
        .to_string();
    assert!(
        err_msg.starts_with("failed to grow memory"),
        "unexpected error: {err_msg}"
    );

    assert_eq!(store.data().current, 10 * 64 * 1024);
    assert_eq!(
        store.data().desired,
        (pages_exceeding_limit + 10) * 64 * 1024
    );
    // The memory_grow_failed hook should show Linux gave OOM:
    let err_msg = store.data().error.as_ref().unwrap();
    assert!(
        err_msg.starts_with("Cannot allocate memory"),
        "unexpected error: {err_msg}"
    );
    Ok(())
}

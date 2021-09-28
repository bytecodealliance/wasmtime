// This test only works on Linux. It may be portable to MacOS as well,
// but the original author did not have a machine available to test it.
#![cfg(target_os = "linux")]

use anyhow::Result;
use wasmtime::*;

#[derive(Default)]
struct MemoryGrowFailureDetector {
    current: usize,
    desired: usize,
    error: Option<String>,
}

impl ResourceLimiter for MemoryGrowFailureDetector {
    fn memory_growing(&mut self, current: usize, desired: usize, _maximum: Option<usize>) -> bool {
        self.current = current;
        self.desired = desired;
        true
    }
    fn memory_grow_failed(&mut self, err: &anyhow::Error) {
        self.error = Some(err.to_string());
    }
    fn table_growing(&mut self, _current: u32, _desired: u32, _maximum: Option<u32>) -> bool {
        true
    }
}

#[test]
fn custom_limiter_detect_os_oom_failure() -> Result<()> {
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        return Ok(());
    }

    // Default behavior of on-demand memory allocation so that a
    // memory grow will hit Linux for a larger mmap.
    let engine = Engine::default();
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
        assert_eq!(res, 0, "setrlimit failed: {}", res);
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
        "unexpected error: {}",
        err_msg
    );

    assert_eq!(store.data().current, 10 * 64 * 1024);
    assert_eq!(
        store.data().desired,
        (pages_exceeding_limit + 10) * 64 * 1024
    );
    // The memory_grow_failed hook should show Linux gave OOM:
    let err_msg = store.data().error.as_ref().unwrap();
    assert!(
        err_msg.starts_with("System call failed: Cannot allocate memory"),
        "unexpected error: {}",
        err_msg
    );
    Ok(())
}

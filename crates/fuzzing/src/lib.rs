//! Fuzzing infrastructure for Wasmtime.

#![deny(missing_docs, missing_debug_implementations)]

pub mod generators;
pub mod oracles;

use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{atomic, Once};

/// Run a fuzz test on Wasm test case with automatic logging.
///
/// This is intended for defining the body of a `libfuzzer_sys::fuzz_target!`
/// invocation.
///
/// Automatically prints out how to create a regression test that runs the exact
/// same set of oracles.
///
/// It also binds the expression getting the wasm bytes to the variable, for
/// example below the `wasm` variable is assigned the value
/// `&my_test_case.as_wasm_bytes()`. This variable can be used within the body.
///
/// ```ignore
/// use wasmtime_fuzzing::{oracles, with_log_wasm_test_case};
///
/// with_log_wasm_test_case!(&my_test_case.as_wasm_bytes(), |wasm| {
///     oracles::compile(wasm);
///     oracles::instantiate(wasm);
/// });
/// ```
#[macro_export]
macro_rules! with_log_wasm_test_case {
    ( $wasm:expr , |$wasm_var:ident| $oracle:expr ) => {{
        let $wasm_var = $wasm;
        wasmtime_fuzzing::log_wasm_test_case(
            &$wasm_var,
            stringify!($wasm_var),
            stringify!($oracle),
        );
        $oracle;
    }};
}

/// Given that we are going to do a fuzz test of the given Wasm buffer, log the
/// Wasm and its WAT disassembly, and preserve them to the filesystem so that if
/// we panic or crash, we can easily inspect the test case.
///
/// This is intended to be used via the `with_log_wasm_test_case` macro.
pub fn log_wasm_test_case(wasm: &[u8], wasm_var: &'static str, oracle_expr: &'static str) {
    init_logging();

    let wasm_path = wasm_test_case_path();
    fs::write(&wasm_path, wasm)
        .with_context(|| format!("Failed to write wasm to {}", wasm_path.display()))
        .unwrap();
    log::info!("Wrote Wasm test case to: {}", wasm_path.display());

    match wasmprinter::print_bytes(wasm) {
        Ok(wat) => {
            log::info!("WAT disassembly:\n{}", wat);

            let wat_path = wat_disassembly_path();
            fs::write(&wat_path, &wat)
                .with_context(|| {
                    format!("Failed to write WAT disassembly to {}", wat_path.display())
                })
                .unwrap();
            log::info!("Wrote WAT disassembly to: {}", wat_path.display());

            log::info!(
                "If this fuzz test fails, copy `{wat_path}` to `wasmtime/crates/fuzzing/tests/regressions/my-regression.wat` and add the following test to `wasmtime/crates/fuzzing/tests/regressions.rs`:

```
#[test]
fn my_fuzzing_regression_test() {{
    let {wasm_var} = wat::parse_str(
        include_str!(\"./regressions/my-regression.wat\")
    ).unwrap();
    {oracle_expr}
}}
```",
                wat_path = wat_path.display(),
                wasm_var = wasm_var,
                oracle_expr = oracle_expr
            );
        }
        Err(e) => {
            log::info!("Failed to disassemble Wasm into WAT:\n{:?}", e);
            log::info!(
                "If this fuzz test fails, copy `{wasm_path}` to `wasmtime/crates/fuzzing/tests/regressions/my-regression.wasm` and add the following test to `wasmtime/crates/fuzzing/tests/regressions.rs`:

```
#[test]
fn my_fuzzing_regression_test() {{
    let {wasm_var} = include_bytes!(\"./regressions/my-regression.wasm\");
    {oracle_expr}
}}
```",
                wasm_path = wasm_path.display(),
                wasm_var = wasm_var,
                oracle_expr = oracle_expr
            );
        }
    }
}

pub(crate) fn scratch_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        // Pop "fuzzing".
        .join("..")
        // Pop "crates".
        .join("..")
        .join("target")
        .join("scratch");

    static CREATE: Once = Once::new();
    CREATE.call_once(|| {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create {}", dir.display()))
            .unwrap();
    });

    dir
}

fn wasm_test_case_path() -> PathBuf {
    static WASM_TEST_CASE_COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

    thread_local! {
        static WASM_TEST_CASE_PATH: PathBuf = {
            let dir = scratch_dir();
            dir.join(format!("{}-{}.wasm",
                             process::id(),
                             WASM_TEST_CASE_COUNTER.fetch_add(1, atomic::Ordering::SeqCst)
            ))
        };
    }

    WASM_TEST_CASE_PATH.with(|p| p.clone())
}

fn wat_disassembly_path() -> PathBuf {
    static WAT_DISASSEMBLY_COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

    thread_local! {
        static WAT_DISASSEMBLY_PATH: PathBuf = {
            let dir = scratch_dir();
            dir.join(format!(
                "{}-{}.wat",
                process::id(),
                WAT_DISASSEMBLY_COUNTER.fetch_add(1, atomic::Ordering::SeqCst)
            ))
        };
    }

    WAT_DISASSEMBLY_PATH.with(|p| p.clone())
}

#[cfg(feature = "env_logger")]
fn init_logging() {
    static INIT_LOGGING: Once = Once::new();
    INIT_LOGGING.call_once(|| env_logger::init());
}

#[cfg(not(feature = "env_logger"))]
fn init_logging() {}

// TODO: Config::new always does `std::env::var`, and if that triggers an OOM in
// the Rust standard library then this test infinite loops. Basically Rust's
// libstd isn't OOM-safe in that regard. Need to figure out a way around that.
#![cfg(not(windows))]

use wasmtime::{Config, Result};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn config_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        Ok(())
    })
}

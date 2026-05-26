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

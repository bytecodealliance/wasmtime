use std::path::Path;
use std::process::Command;

#[cfg(feature = "component")]
pub mod component;
#[cfg(feature = "component-fuzz")]
pub mod component_fuzz;
#[cfg(feature = "wasmtime-wast")]
pub mod wasmtime_wast;
#[cfg(feature = "wast")]
pub mod wast;

pub fn cargo_test_runner() -> Option<String> {
    // Note that this technically should look for the current target as well
    // instead of picking "any runner", but that's left for a future
    // refactoring.
    let (_, runner) = std::env::vars()
        .filter(|(k, _v)| k.starts_with("CARGO_TARGET") && k.ends_with("RUNNER"))
        .next()?;
    Some(runner)
}

pub fn command(bin: impl AsRef<Path>) -> Command {
    let bin = bin.as_ref();
    match cargo_test_runner() {
        Some(runner) => {
            let mut parts = runner.split_whitespace();
            let mut cmd = Command::new(parts.next().unwrap());
            for arg in parts {
                cmd.arg(arg);
            }
            cmd.arg(bin);
            cmd
        }
        None => Command::new(bin),
    }
}

use anyhow::Result;
use wasmtime::{Config, Engine, Module};

fn pulley_target() -> &'static str {
    if cfg!(target_pointer_width = "64") {
        "pulley64"
    } else {
        "pulley32"
    }
}

fn pulley_config() -> Config {
    let mut config = Config::new();
    config.target(pulley_target()).unwrap();
    config
}

#[test]
fn can_compile_pulley_module() -> Result<()> {
    let engine = Engine::new(&pulley_config())?;
    Module::new(&engine, "(module)")?;
    Ok(())
}

#[test]
fn can_deserialize_pulley_module() -> Result<()> {
    let engine = Engine::new(&pulley_config())?;
    let bytes = engine.precompile_module(b"(module)")?;
    unsafe {
        Module::deserialize(&engine, &bytes)?;
    }
    Ok(())
}

#[test]
fn pulley_wrong_architecture_is_rejected() -> Result<()> {
    let mut config = Config::new();
    // Intentionally swap pointer widths here to ensure pulley is the wrong one.
    if cfg!(target_pointer_width = "64") {
        config.target("pulley32").unwrap();
    } else {
        config.target("pulley64").unwrap();
    }

    // Creating `Module` should fail as we can't run the wrong architecture.
    let engine = Engine::new(&config)?;
    assert!(Module::new(&engine, "(module)").is_err());

    // Precompiling should succeed but deserialization should fail because it's
    // the wrong pointer width.
    let engine = Engine::new(&config)?;
    let bytes = engine.precompile_module(b"(module)")?;
    unsafe {
        assert!(Module::deserialize(&engine, &bytes).is_err());
    }
    Ok(())
}

// CLI subcommands should support `--target`
#[test]
#[cfg(not(miri))]
fn can_run_on_cli() -> Result<()> {
    use crate::cli_tests::run_wasmtime;
    run_wasmtime(&[
        "--target",
        pulley_target(),
        "tests/all/cli_tests/empty-module.wat",
    ])?;
    run_wasmtime(&[
        "run",
        "--target",
        pulley_target(),
        "tests/all/cli_tests/empty-module.wat",
    ])?;
    Ok(())
}

use wasmtime::{Config, Engine, Result};
use wasmtime_test_macros::wasmtime_test;

#[wasmtime_test(strategies(only(CraneliftNative)))]
fn requires_signals_based_traps(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.signals_based_traps(false);
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption requires signals-based traps",
    );
    Ok(())
}

// With Cranelift only the x64 and aarch64 backends are supported.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn requires_supported_isa(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.target("riscv64").unwrap();
    config.signals_based_traps(true);
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption is supported only on x86_64 and aarch64 Linux, not `riscv64-unknown-unknown-elf`",
    );
    Ok(())
}

#[wasmtime_test(strategies(only(CraneliftNative)))]
fn requires_linux_target(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.signals_based_traps(true);
    config.target("x86_64-apple-darwin").unwrap();
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption is supported only on x86_64 and aarch64 Linux, not `x86_64-apple-darwin`",
    );
    Ok(())
}

// The Winch backend does not support this feature.
#[wasmtime_test(strategies(only(Winch)))]
fn rejected_by_winch(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "Winch does not currently support MMU interruption",
    );
    Ok(())
}

// Pulley does not support this feature, since compiled-in interruption checks
// will be absent.
#[wasmtime_test(strategies(only(CraneliftPulley)))]
fn rejected_by_pulley(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.signals_based_traps(true);
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption is supported only on x86_64 and aarch64 Linux, not `pulley64-unknown-unknown-elf`",
    );
    Ok(())
}

// AOT compilation succeeds with the right flags set.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn precompile_succeeds_for_valid_config_on_any_host(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.target("x86_64-unknown-linux-gnu").unwrap();
    config.signals_based_traps(true);
    let engine = Engine::new(config).unwrap();
    engine
        .precompile_module(r#"(module (memory 0) (func))"#.as_bytes())
        .expect("precompilation should succeed regardless of host");
    Ok(())
}

#[cfg(not(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
)))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn compile_and_run_fails_on_unsupported_host(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.signals_based_traps(true);
    let err = match Engine::new(config) {
        Err(err) => err,
        Ok(engine) => wasmtime::Module::new(&engine, "(module)")
            .expect_err("compile-and-run should fail on an unsupported host"),
    };
    let err = format!("{err:?}");
    assert!(
        err.contains("supported only on x86_64 and aarch64"),
        "unexpected error: {err}"
    );
    Ok(())
}

#![cfg(not(miri))]

use wasmtime::{Config, Engine, Strategy};

#[test]
fn requires_signals_based_traps() {
    let mut config = Config::new();
    config.mmu_interruption(true);
    config.signals_based_traps(false);
    let err = Engine::new(&config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption requires signals-based traps",
    );
}

// With Cranelift only the x64 and aarch64 backends are supported.
#[test]
fn requires_supported_isa() {
    let mut config = Config::new();
    config.mmu_interruption(true);
    config.target("riscv64").unwrap();
    config.signals_based_traps(true);
    let err = Engine::new(&config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption is supported only on x86_64 and aarch64 Linux, not `riscv64-unknown-unknown-elf`",
    );
}

#[test]
fn requires_linux_target() {
    let mut config = Config::new();
    config.mmu_interruption(true);
    config.signals_based_traps(true);
    config.target("x86_64-apple-darwin").unwrap();
    let err = Engine::new(&config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption is supported only on x86_64 and aarch64 Linux, not `x86_64-apple-darwin`",
    );
}

// The Winch backend does not support this feature.
#[test]
fn rejected_by_winch() {
    let mut config = Config::new();
    config.mmu_interruption(true);
    config.strategy(Strategy::Winch);
    config.target("x86_64-unknown-linux-gnu").unwrap();
    let err = Engine::new(&config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "Winch does not currently support MMU interruption",
    );
}

// Pulley does not support this feature, since compiled-in interruption checks
// will be absent.
#[test]
fn rejected_by_pulley() {
    let mut config = Config::new();
    config.mmu_interruption(true);
    config.signals_based_traps(true);
    config.target("pulley64").unwrap();
    let err = Engine::new(&config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption is supported only on x86_64 and aarch64 Linux, not `pulley64-unknown-unknown-elf`",
    );
}

// AOT compilation succeeds for a supported target, regardless of host.
#[test]
fn precompile_succeeds_for_valid_config_on_any_host() {
    let mut config = Config::new();
    config.mmu_interruption(true);
    config.target("x86_64-unknown-linux-gnu").unwrap();
    config.signals_based_traps(true);
    let engine = Engine::new(&config).unwrap();
    engine
        .precompile_module(r#"(module (memory 0) (func))"#.as_bytes())
        .expect("precompilation should succeed regardless of host");
}

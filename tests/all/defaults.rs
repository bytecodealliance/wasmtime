use wasmtime::*;

#[test]
fn test_tail_call_default() -> Result<()> {
    for (expected, cfg) in [
        (
            true,
            Config::new()
                .strategy(Strategy::Cranelift)
                .target("x86_64")?,
        ),
        (
            true,
            Config::new()
                .strategy(Strategy::Cranelift)
                .target("aarch64")?,
        ),
        (
            true,
            Config::new()
                .strategy(Strategy::Cranelift)
                .target("riscv64")?,
        ),
        (
            false,
            Config::new()
                .strategy(Strategy::Cranelift)
                .target("s390x")?,
        ),
        (
            false,
            Config::new().strategy(Strategy::Winch).target("x86_64")?,
        ),
        (
            false,
            Config::new().strategy(Strategy::Winch).target("aarch64")?,
        ),
        (
            false,
            Config::new()
                .strategy(Strategy::Cranelift)
                .wasm_tail_call(false)
                .target("x86_64")?,
        ),
    ] {
        let engine = Engine::new(cfg)?;

        let wat = r#"
            (module $from_name_section
                (func (export "run") (return_call 0))
            )
        "#;

        let result = engine.precompile_module(wat.as_bytes()).map(|_| ());

        eprintln!("for config {cfg:?}, got: {result:?}");

        assert_eq!(expected, result.is_ok());
    }

    Ok(())
}

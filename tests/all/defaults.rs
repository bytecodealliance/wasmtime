use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn test_tail_call_default() -> Result<()> {
    for (line, expected, cfg) in [
        (
            line!(),
            true,
            Config::new()
                .strategy(Strategy::Cranelift)
                .target("x86_64")?,
        ),
        (
            line!(),
            true,
            Config::new()
                .strategy(Strategy::Cranelift)
                .target("aarch64")?,
        ),
        (
            line!(),
            true,
            Config::new()
                .strategy(Strategy::Cranelift)
                .target("riscv64")?,
        ),
        (
            line!(),
            true,
            Config::new()
                .strategy(Strategy::Cranelift)
                .target("s390x")?,
        ),
        (
            line!(),
            false,
            Config::new().strategy(Strategy::Winch).target("x86_64")?,
        ),
        (
            line!(),
            false,
            Config::new().strategy(Strategy::Winch).target("aarch64")?,
        ),
        (
            line!(),
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

        eprintln!("for config on line {line}, got: {result:?}");

        assert_eq!(expected, result.is_ok());
    }

    Ok(())
}

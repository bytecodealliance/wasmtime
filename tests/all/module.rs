use wasmtime::*;

#[test]
fn caches_across_engines() {
    let mut c = Config::new();
    c.cranelift_clear_cpu_flags();

    let bytes = Module::new(&Engine::new(&c).unwrap(), "(module)")
        .unwrap()
        .serialize()
        .unwrap();

    let res = Module::deserialize(
        &Engine::new(&Config::new().cranelift_clear_cpu_flags()).unwrap(),
        &bytes,
    );
    assert!(res.is_ok());

    // differ in shared cranelift flags
    let res = Module::deserialize(
        &Engine::new(
            &Config::new()
                .cranelift_clear_cpu_flags()
                .cranelift_nan_canonicalization(true),
        )
        .unwrap(),
        &bytes,
    );
    assert!(res.is_err());

    // differ in cranelift settings
    let res = Module::deserialize(
        &Engine::new(
            &Config::new()
                .cranelift_clear_cpu_flags()
                .cranelift_opt_level(OptLevel::None),
        )
        .unwrap(),
        &bytes,
    );
    assert!(res.is_err());

    // differ in cpu-specific flags
    if cfg!(target_arch = "x86_64") {
        let res = Module::deserialize(
            &Engine::new(unsafe {
                &Config::new()
                    .cranelift_clear_cpu_flags()
                    .cranelift_other_flag("has_sse3", "true")
                    .unwrap()
            })
            .unwrap(),
            &bytes,
        );
        assert!(res.is_err());
    }
}

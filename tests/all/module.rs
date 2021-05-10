use anyhow::Result;
use wasmtime::*;

#[test]
fn checks_incompatible_target() -> Result<()> {
    let mut target = target_lexicon::Triple::host();
    target.operating_system = target_lexicon::OperatingSystem::Unknown;
    match Module::new(
        &Engine::new(Config::new().target(&target.to_string())?)?,
        "(module)",
    ) {
        Ok(_) => unreachable!(),
        Err(e) => assert!(e
            .to_string()
            .contains("configuration does not match the host")),
    }

    Ok(())
}

#[test]
fn caches_across_engines() {
    let c = Config::new();

    let bytes = Module::new(&Engine::new(&c).unwrap(), "(module)")
        .unwrap()
        .serialize()
        .unwrap();

    unsafe {
        let res = Module::deserialize(&Engine::default(), &bytes);
        assert!(res.is_ok());

        // differ in shared cranelift flags
        let res = Module::deserialize(
            &Engine::new(Config::new().cranelift_nan_canonicalization(true)).unwrap(),
            &bytes,
        );
        assert!(res.is_err());

        // differ in cranelift settings
        let res = Module::deserialize(
            &Engine::new(Config::new().cranelift_opt_level(OptLevel::None)).unwrap(),
            &bytes,
        );
        assert!(res.is_err());

        // Missing required cpu flags
        if cfg!(target_arch = "x86_64") {
            let res = Module::deserialize(
                &Engine::new(
                    Config::new()
                        .target(&target_lexicon::Triple::host().to_string())
                        .unwrap(),
                )
                .unwrap(),
                &bytes,
            );
            assert!(res.is_err());
        }
    }
}

#[test]
fn aot_compiles() -> Result<()> {
    let engine = Engine::default();
    let bytes = engine.precompile_module(
        "(module (func (export \"f\") (param i32) (result i32) local.get 0))".as_bytes(),
    )?;

    let module = unsafe { Module::deserialize(&engine, &bytes)? };

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let f = instance.get_typed_func::<i32, i32, _>(&mut store, "f")?;
    assert_eq!(f.call(&mut store, 101)?, 101);

    Ok(())
}

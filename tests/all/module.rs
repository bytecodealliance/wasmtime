use anyhow::Result;
use std::io::BufWriter;
use wasmtime::*;

#[test]
fn caches_across_engines() {
    let c = Config::new();

    let bytes = Module::new(&Engine::new(&c).unwrap(), "(module)")
        .unwrap()
        .serialize()
        .unwrap();

    let res = Module::deserialize(&Engine::new(&Config::new()).unwrap(), &bytes);
    assert!(res.is_ok());

    // differ in shared cranelift flags
    let res = Module::deserialize(
        &Engine::new(&Config::new().cranelift_nan_canonicalization(true)).unwrap(),
        &bytes,
    );
    assert!(res.is_err());

    // differ in cranelift settings
    let res = Module::deserialize(
        &Engine::new(&Config::new().cranelift_opt_level(OptLevel::None)).unwrap(),
        &bytes,
    );
    assert!(res.is_err());

    // Missing required cpu flags
    if cfg!(target_arch = "x86_64") {
        let res = Module::deserialize(
            &Engine::new(&Config::new().cranelift_clear_cpu_flags()).unwrap(),
            &bytes,
        );
        assert!(res.is_err());
    }
}

#[test]
fn aot_compiles() -> Result<()> {
    let engine = Engine::default();
    let mut writer = BufWriter::new(Vec::new());
    Module::compile(
        &engine,
        "(module (func (export \"f\") (param i32) (result i32) local.get 0))".as_bytes(),
        &mut writer,
    )?;

    let bytes = writer.into_inner()?;
    let module = Module::from_binary(&engine, &bytes)?;

    let store = Store::new(&engine);
    let instance = Instance::new(&store, &module, &[])?;

    let f = instance.get_typed_func::<i32, i32>("f")?;
    assert_eq!(f.call(101).unwrap(), 101);

    Ok(())
}

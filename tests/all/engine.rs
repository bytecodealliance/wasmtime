use super::*;
use wasmtime::*;

#[test]
fn engine_without_compiler_cannot_compile() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    let engine = Engine::new(&config)?;
    match Module::new(&engine, r#"(module (func (export "f") nop))"#) {
        Ok(_) => panic!("should not compile without a compiler"),
        Err(err) => err.assert_contains("Engine was not configured with a compiler"),
    }
    Ok(())
}

#[test]
fn engine_without_compiler_can_deserialize_and_run() -> Result<()> {
    let engine_with_compiler = Engine::default();
    let module = Module::new(&engine_with_compiler, r#"(module (func (export "f") nop))"#)?;
    let serialized = module.serialize()?;

    let mut config = Config::new();
    config.enable_compiler(false);
    let engine_without_compiler = Engine::new(&config)?;

    let deserialized_module = unsafe { Module::deserialize(&engine_without_compiler, &serialized)? };

    let mut store = Store::new(&engine_without_compiler, ());
    let instance = Instance::new(&mut store, &deserialized_module, &[])?;
    let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;
    f.call(&mut store, ())?;

    Ok(())
}

use anyhow::Result;
use wasmtime::*;

#[test]
fn wrong_import_numbers() -> Result<()> {
    let store = Store::default();
    let module = Module::new(store.engine(), r#"(module (import "" "" (func)))"#)?;

    assert!(Instance::new(&store, &module, &[]).is_err());
    let func = Func::wrap(&store, || {});
    assert!(Instance::new(&store, &module, &[func.clone().into(), func.into()]).is_err());
    Ok(())
}

#[test]
fn initializes_linear_memory() -> Result<()> {
    // Test for https://github.com/bytecodealliance/wasmtime/issues/2784
    let wat = r#"
        (module
            (memory (export "memory") 2)
            (data (i32.const 0) "Hello World!")
        )"#;
    let module = Module::new(&Engine::default(), wat)?;

    let store = Store::new(module.engine());
    let instance = Instance::new(&store, &module, &[])?;
    let memory = instance.get_memory("memory").unwrap();

    let mut bytes = [0; 12];
    memory.read(0, &mut bytes)?;
    assert_eq!(bytes, "Hello World!".as_bytes());
    Ok(())
}

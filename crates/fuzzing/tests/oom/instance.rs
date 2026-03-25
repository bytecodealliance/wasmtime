#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Linker, Module, Result, Store};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn call_exported_func() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        let module = Module::new(
            &engine,
            r#"
                (module
                    (func (export "add") (param i32 i32) (result i32)
                        (i32.add (local.get 0) (local.get 1))
                    )
                )
            "#,
        )?;
        module.serialize()?
    };

    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let add = instance.get_typed_func::<(i32, i32), i32>(&mut store, "add")?;
        let result = add.call(&mut store, (1, 2))?;
        assert_eq!(result, 3);
        Ok(())
    })
}

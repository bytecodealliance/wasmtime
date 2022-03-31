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
        Err(e) => assert!(
            format!("{:?}", e).contains("configuration does not match the host"),
            "bad error: {:?}",
            e
        ),
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

        // differ in runtime settings
        let res = Module::deserialize(
            &Engine::new(Config::new().static_memory_maximum_size(0)).unwrap(),
            &bytes,
        );
        assert!(res.is_err());

        // differ in wasm features enabled (which can affect
        // runtime/compilation settings)
        let res = Module::deserialize(
            &Engine::new(Config::new().wasm_simd(false)).unwrap(),
            &bytes,
        );
        assert!(res.is_err());
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

#[test]
fn serialize_deterministic() {
    let engine = Engine::default();

    let assert_deterministic = |wasm: &str| {
        let p1 = engine.precompile_module(wasm.as_bytes()).unwrap();
        let p2 = engine.precompile_module(wasm.as_bytes()).unwrap();
        if p1 != p2 {
            panic!("precompile_module not determinisitc for:\n{}", wasm);
        }

        let module1 = Module::new(&engine, wasm).unwrap();
        let a1 = module1.serialize().unwrap();
        let a2 = module1.serialize().unwrap();
        if a1 != a2 {
            panic!("Module::serialize not determinisitc for:\n{}", wasm);
        }

        let module2 = Module::new(&engine, wasm).unwrap();
        let b1 = module2.serialize().unwrap();
        let b2 = module2.serialize().unwrap();
        if b1 != b2 {
            panic!("Module::serialize not determinisitc for:\n{}", wasm);
        }

        if a1 != b2 {
            panic!("not matching across modules:\n{}", wasm);
        }
        if b1 != p2 {
            panic!("not matching across engine/module:\n{}", wasm);
        }
    };

    assert_deterministic("(module)");
    assert_deterministic("(module (func))");
    assert_deterministic("(module (func nop))");
    assert_deterministic("(module (func) (func (param i32)))");
    assert_deterministic("(module (func (export \"f\")) (func (export \"y\")))");
    assert_deterministic("(module (func $f) (func $g))");
    assert_deterministic("(module (data \"\") (data \"\"))");
    assert_deterministic("(module (elem) (elem))");
}

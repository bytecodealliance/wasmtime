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

// This test asserts that the optimization to transform separate data segments
// into an initialization image doesn't unnecessarily create a massive module by
// accident with a very large initialization image in it.
#[test]
fn serialize_not_overly_massive() -> Result<()> {
    let mut config = Config::new();
    config.memory_guaranteed_dense_image_size(1 << 20);
    let engine = Engine::new(&config)?;

    let assert_smaller_than_1mb = |module: &str| -> Result<()> {
        println!("{}", module);
        let bytes = Module::new(&engine, module)?.serialize()?;
        assert!(bytes.len() < (1 << 20));
        Ok(())
    };

    // Tons of space between data segments should use sparse initialization,
    // along with various permutations of empty and nonempty segments.
    assert_smaller_than_1mb(
        r#"(module
            (memory 20000)
            (data (i32.const 0) "a")
            (data (i32.const 0x200000) "b")
        )"#,
    )?;
    assert_smaller_than_1mb(
        r#"(module
            (memory 20000)
            (data (i32.const 0) "a")
            (data (i32.const 0x200000) "")
        )"#,
    )?;
    assert_smaller_than_1mb(
        r#"(module
            (memory 20000)
            (data (i32.const 0) "")
            (data (i32.const 0x200000) "b")
        )"#,
    )?;
    assert_smaller_than_1mb(
        r#"(module
            (memory 20000)
            (data (i32.const 0) "")
            (data (i32.const 0x200000) "")
        )"#,
    )?;

    // lone data segment
    assert_smaller_than_1mb(
        r#"(module
            (memory 20000)
            (data (i32.const 0x200000) "b")
        )"#,
    )?;

    Ok(())
}

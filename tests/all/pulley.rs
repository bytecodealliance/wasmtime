use anyhow::Result;
use wasmtime::{Config, Engine, Func, FuncType, Instance, Module, Store, Trap, Val, ValType};
use wasmtime_environ::TripleExt;

fn pulley_target() -> String {
    target_lexicon::Triple::pulley_host().to_string()
}

fn pulley_config() -> Config {
    let mut config = Config::new();
    config.target(&pulley_target()).unwrap();
    config
}

#[test]
fn can_compile_pulley_module() -> Result<()> {
    let engine = Engine::new(&pulley_config())?;
    Module::new(&engine, "(module)")?;

    Ok(())
}

#[test]
fn can_deserialize_pulley_module() -> Result<()> {
    let engine = Engine::new(&pulley_config())?;
    let bytes = engine.precompile_module(b"(module)")?;
    unsafe {
        Module::deserialize(&engine, &bytes)?;
    }
    Ok(())
}

#[test]
fn pulley_wrong_architecture_is_rejected() -> Result<()> {
    let mut config = Config::new();
    // Intentionally swap pointer widths here to ensure pulley is the wrong one.
    if cfg!(target_pointer_width = "64") {
        config.target("pulley32").unwrap();
    } else {
        config.target("pulley64").unwrap();
    }

    // Creating `Module` should fail as we can't run the wrong architecture.
    let engine = Engine::new(&config)?;
    assert!(Module::new(&engine, "(module)").is_err());

    // Precompiling should succeed but deserialization should fail because it's
    // the wrong pointer width.
    let engine = Engine::new(&config)?;
    let bytes = engine.precompile_module(b"(module)")?;
    unsafe {
        assert!(Module::deserialize(&engine, &bytes).is_err());
    }
    Ok(())
}

// CLI subcommands should support `--target`
#[test]
#[cfg(not(miri))]
fn can_run_on_cli() -> Result<()> {
    use crate::cli_tests::run_wasmtime;
    run_wasmtime(&[
        "--target",
        &pulley_target(),
        "tests/all/cli_tests/empty-module.wat",
    ])?;
    run_wasmtime(&[
        "run",
        "--target",
        &pulley_target(),
        "tests/all/cli_tests/empty-module.wat",
    ])?;
    Ok(())
}

/// This is a one-size-fits-all test to test out pointer provenance in Pulley.
///
/// The goal of this test is to exercise an actual wasm module being run in
/// Pulley in MIRI to ensure that it's not undefined behavior. The main reason
/// we don't do this for the entire test suite is that Cranelift compilation in
/// MIRI is excessively slow to the point that it's not tenable to run. Thus
/// the way this test is run is a little nonstandard.
///
/// * The test here is ignored on MIRI by default. That means this runs on
///   native platforms otherwise though.
///
/// * A script `./ci/miri-provenance-test.sh` is provided to execute just this
///   one test. That will precompile the wasm module in question here using
///   native code, leaving a `*.cwasm` in place.
///
/// Thus in native code this compiles the module here just-in-time. On MIRI
/// this test must be run through the above script and this will deserialize
/// the file from disk. In the end we skip Cranelift on MIRI while still getting
/// to execute some wasm.
///
/// This test then has a wasm module with a number of "interesting" constructs
/// and instructions. The goal is to kind of do a dry run of interesting
/// shapes/sizes of what you can do with core wasm and ensure MIRI gives us a
/// clean bill of health.
#[test]
#[cfg_attr(miri, ignore)]
fn pulley_provenance_test() -> Result<()> {
    let mut config = pulley_config();
    config.wasm_function_references(true);
    config.memory_reservation(1 << 20);
    config.memory_guard_size(0);
    config.signals_based_traps(false);
    let engine = Engine::new(&config)?;
    let module = if cfg!(miri) {
        unsafe { Module::deserialize_file(&engine, "./tests/all/pulley_provenance_test.cwasm")? }
    } else {
        Module::from_file(&engine, "./tests/all/pulley_provenance_test.wat")?
    };
    let mut store = Store::new(&engine, ());
    let host_wrap = Func::wrap(&mut store, || (1_i32, 2_i32, 3_i32));
    let host_new_ty = FuncType::new(
        store.engine(),
        vec![],
        vec![ValType::I32, ValType::I32, ValType::I32],
    );
    let host_new = Func::new(&mut store, host_new_ty, |_, _params, results| {
        results[0] = Val::I32(1);
        results[1] = Val::I32(2);
        results[2] = Val::I32(3);
        Ok(())
    });
    let instance = Instance::new(&mut store, &module, &[host_wrap.into(), host_new.into()])?;

    for func in [
        "call-wasm",
        "call-native-wrap",
        "call-native-new",
        "return-call-wasm",
        "call_indirect-wasm",
    ] {
        println!("testing func {func:?}");
        let func = instance
            .get_typed_func::<(), (i32, i32, i32)>(&mut store, func)
            .unwrap();
        let results = func.call(&mut store, ())?;
        assert_eq!(results, (1, 2, 3));
    }

    let funcref = instance.get_func(&mut store, "call-wasm").unwrap();
    for func in ["call_ref-wasm", "return_call_ref-wasm"] {
        println!("testing func {func:?}");
        let func = instance.get_typed_func::<Func, (i32, i32, i32)>(&mut store, func)?;
        let results = func.call(&mut store, funcref)?;
        assert_eq!(results, (1, 2, 3));
    }

    let trap = instance
        .get_typed_func::<(), ()>(&mut store, "unreachable")?
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::UnreachableCodeReached);

    let trap = instance
        .get_typed_func::<(), i32>(&mut store, "divide-by-zero")?
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::IntegerDivisionByZero);

    instance
        .get_typed_func::<(), ()>(&mut store, "memory-intrinsics")?
        .call(&mut store, ())?;
    instance
        .get_typed_func::<(), ()>(&mut store, "table-intrinsics")?
        .call(&mut store, ())?;

    Ok(())
}

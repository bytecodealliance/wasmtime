use anyhow::Result;
use wasmtime::component::{self, Component};
use wasmtime::{
    Caller, Config, Engine, Func, FuncType, Instance, Module, Store, Trap, Val, ValType,
};
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

fn provenance_test_config() -> Config {
    let mut config = pulley_config();
    config.wasm_function_references(true);
    config.memory_reservation(1 << 20);
    config.memory_guard_size(0);
    config.signals_based_traps(false);
    config
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
    let config = provenance_test_config();
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

    let funcref = Func::wrap(&mut store, move |mut caller: Caller<'_, ()>| {
        let func = instance.get_typed_func::<(), (i32, i32, i32)>(&mut caller, "call-wasm")?;
        func.call(&mut caller, ())
    });
    let func = instance.get_typed_func::<Func, (i32, i32, i32)>(&mut store, "call_ref-wasm")?;
    let results = func.call(&mut store, funcref)?;
    assert_eq!(results, (1, 2, 3));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn pulley_provenance_test_components() -> Result<()> {
    let config = provenance_test_config();
    let engine = Engine::new(&config)?;
    let component = if cfg!(miri) {
        unsafe {
            Component::deserialize_file(
                &engine,
                "./tests/all/pulley_provenance_test_component.cwasm",
            )?
        }
    } else {
        Component::from_file(&engine, "./tests/all/pulley_provenance_test_component.wat")?
    };
    {
        use wasmtime::component::{ComponentType, Lift, Lower};

        #[derive(ComponentType, Lift, Lower, Clone, Copy, PartialEq, Debug)]
        #[component(enum)]
        #[repr(u8)]
        enum E {
            #[expect(dead_code, reason = "only testing other variants")]
            A,
            B,
            #[expect(dead_code, reason = "only testing other variants")]
            C,
        }

        let mut store = Store::new(&engine, ());
        let mut linker = component::Linker::new(&engine);
        linker
            .root()
            .func_wrap("host-u32", |_, (value,): (u32,)| Ok((value,)))?;
        linker
            .root()
            .func_wrap("host-enum", |_, (value,): (E,)| Ok((value,)))?;
        linker
            .root()
            .func_wrap("host-option", |_, (value,): (Option<u8>,)| Ok((value,)))?;
        linker
            .root()
            .func_wrap("host-result", |_, (value,): (Result<u16, i64>,)| {
                Ok((value,))
            })?;
        linker
            .root()
            .func_wrap("host-string", |_, (value,): (String,)| Ok((value,)))?;
        linker
            .root()
            .func_wrap("host-list", |_, (value,): (Vec<String>,)| Ok((value,)))?;
        let instance = linker.instantiate(&mut store, &component)?;

        let guest_u32 = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "guest-u32")?;
        let guest_enum = instance.get_typed_func::<(E,), (E,)>(&mut store, "guest-enum")?;
        let guest_option =
            instance.get_typed_func::<(Option<u8>,), (Option<u8>,)>(&mut store, "guest-option")?;
        let guest_result = instance.get_typed_func::<(Result<u16, i64>,), (Result<u16, i64>,)>(
            &mut store,
            "guest-result",
        )?;
        let guest_string =
            instance.get_typed_func::<(&str,), (String,)>(&mut store, "guest-string")?;
        let guest_list =
            instance.get_typed_func::<(&[&str],), (Vec<String>,)>(&mut store, "guest-list")?;

        let (result,) = guest_u32.call(&mut store, (42,))?;
        assert_eq!(result, 42);
        guest_u32.post_return(&mut store)?;

        let (result,) = guest_enum.call(&mut store, (E::B,))?;
        assert_eq!(result, E::B);
        guest_enum.post_return(&mut store)?;

        let (result,) = guest_option.call(&mut store, (None,))?;
        assert_eq!(result, None);
        guest_option.post_return(&mut store)?;
        let (result,) = guest_option.call(&mut store, (Some(200),))?;
        assert_eq!(result, Some(200));
        guest_option.post_return(&mut store)?;

        let (result,) = guest_result.call(&mut store, (Ok(10),))?;
        assert_eq!(result, Ok(10));
        guest_result.post_return(&mut store)?;
        let (result,) = guest_result.call(&mut store, (Err(i64::MIN),))?;
        assert_eq!(result, Err(i64::MIN));
        guest_result.post_return(&mut store)?;

        let (result,) = guest_string.call(&mut store, ("",))?;
        assert_eq!(result, "");
        guest_string.post_return(&mut store)?;
        let (result,) = guest_string.call(&mut store, ("hello",))?;
        assert_eq!(result, "hello");
        guest_string.post_return(&mut store)?;

        let (result,) = guest_list.call(&mut store, (&[],))?;
        assert!(result.is_empty());
        guest_list.post_return(&mut store)?;
        let (result,) = guest_list.call(&mut store, (&["a", "", "b", "c"],))?;
        assert_eq!(result, ["a", "", "b", "c"]);
        guest_list.post_return(&mut store)?;

        instance
            .get_typed_func::<(), ()>(&mut store, "resource-intrinsics")?
            .call(&mut store, ())?;
    }
    {
        use wasmtime::component::Val;
        let mut store = Store::new(&engine, ());
        let mut linker = component::Linker::new(&engine);
        linker.root().func_new("host-u32", |_, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        linker.root().func_new("host-enum", |_, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        linker.root().func_new("host-option", |_, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        linker.root().func_new("host-result", |_, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        linker.root().func_new("host-string", |_, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        linker.root().func_new("host-list", |_, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        let instance = linker.instantiate(&mut store, &component)?;

        let guest_u32 = instance.get_func(&mut store, "guest-u32").unwrap();
        let guest_enum = instance.get_func(&mut store, "guest-enum").unwrap();
        let guest_option = instance.get_func(&mut store, "guest-option").unwrap();
        let guest_result = instance.get_func(&mut store, "guest-result").unwrap();
        let guest_string = instance.get_func(&mut store, "guest-string").unwrap();
        let guest_list = instance.get_func(&mut store, "guest-list").unwrap();

        let mut results = [Val::U32(0)];
        guest_u32.call(&mut store, &[Val::U32(42)], &mut results)?;
        assert_eq!(results[0], Val::U32(42));
        guest_u32.post_return(&mut store)?;

        guest_enum.call(&mut store, &[Val::Enum("B".into())], &mut results)?;
        assert_eq!(results[0], Val::Enum("B".into()));
        guest_enum.post_return(&mut store)?;

        guest_option.call(&mut store, &[Val::Option(None)], &mut results)?;
        assert_eq!(results[0], Val::Option(None));
        guest_option.post_return(&mut store)?;
        guest_option.call(
            &mut store,
            &[Val::Option(Some(Box::new(Val::U8(201))))],
            &mut results,
        )?;
        assert_eq!(results[0], Val::Option(Some(Box::new(Val::U8(201)))));
        guest_option.post_return(&mut store)?;

        guest_result.call(
            &mut store,
            &[Val::Result(Ok(Some(Box::new(Val::U16(20)))))],
            &mut results,
        )?;
        assert_eq!(results[0], Val::Result(Ok(Some(Box::new(Val::U16(20))))));
        guest_result.post_return(&mut store)?;
        guest_result.call(
            &mut store,
            &[Val::Result(Err(Some(Box::new(Val::S64(i64::MAX)))))],
            &mut results,
        )?;
        assert_eq!(
            results[0],
            Val::Result(Err(Some(Box::new(Val::S64(i64::MAX)))))
        );
        guest_result.post_return(&mut store)?;

        guest_string.call(&mut store, &[Val::String("B".into())], &mut results)?;
        assert_eq!(results[0], Val::String("B".into()));
        guest_string.post_return(&mut store)?;
        guest_string.call(&mut store, &[Val::String("".into())], &mut results)?;
        assert_eq!(results[0], Val::String("".into()));
        guest_string.post_return(&mut store)?;

        guest_list.call(&mut store, &[Val::List(Vec::new())], &mut results)?;
        assert_eq!(results[0], Val::List(Vec::new()));
        guest_list.post_return(&mut store)?;
    }

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn enabling_debug_info_doesnt_break_anything() -> Result<()> {
    let mut config = pulley_config();
    config.debug_info(true);
    let engine = Engine::new(&config)?;
    assert!(Module::from_file(&engine, "./tests/all/cli_tests/greeter_command.wat").is_err());
    Ok(())
}

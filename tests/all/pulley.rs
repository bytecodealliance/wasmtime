use std::ptr::NonNull;
use wasmtime::Result;
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
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_threading(true);
    config.wasm_component_model_error_context(true);
    config.guest_debug(true);
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
    let module_clone = module.clone();
    let host_new = Func::new(
        &mut store,
        host_new_ty,
        move |mut caller, _params, results| {
            let caller_frame = caller.debug_exit_frames().next().unwrap();
            let caller_module = caller_frame.module(&mut caller).unwrap().unwrap();
            assert!(Module::same(caller_module, &module_clone));
            let (caller_func, pc) = caller_frame
                .wasm_function_index_and_pc(&mut caller)
                .unwrap()
                .unwrap();
            assert_eq!(caller_func.as_u32(), 3);
            assert_eq!(pc.raw(), 418);
            let parent_frame = caller_frame.parent(&mut caller).unwrap();
            assert!(parent_frame.is_none());

            results[0] = Val::I32(1);
            results[1] = Val::I32(2);
            results[2] = Val::I32(3);
            Ok(())
        },
    );
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
    instance
        .get_typed_func::<(), ()>(&mut store, "table-intrinsics2")?
        .call(&mut store, ())?;

    let funcref = Func::wrap(&mut store, move |mut caller: Caller<'_, ()>| {
        let func = instance.get_typed_func::<(), (i32, i32, i32)>(&mut caller, "call-wasm")?;
        func.call(&mut caller, ())
    });
    let func = instance.get_typed_func::<Func, (i32, i32, i32)>(&mut store, "call_ref-wasm")?;
    let results = func.call(&mut store, funcref)?;
    assert_eq!(results, (1, 2, 3));

    instance
        .get_typed_func::<(), ()>(&mut store, "ref-func-myself")?
        .call(&mut store, ())?;

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
        linker.root().func_wrap("host-empty", |_, (): ()| Ok(()))?;
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

        let guest_empty = instance.get_typed_func::<(), ()>(&mut store, "guest-empty")?;
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

        guest_empty.call(&mut store, ())?;

        let (result,) = guest_u32.call(&mut store, (42,))?;
        assert_eq!(result, 42);

        let (result,) = guest_enum.call(&mut store, (E::B,))?;
        assert_eq!(result, E::B);

        let (result,) = guest_option.call(&mut store, (None,))?;
        assert_eq!(result, None);
        let (result,) = guest_option.call(&mut store, (Some(200),))?;
        assert_eq!(result, Some(200));

        let (result,) = guest_result.call(&mut store, (Ok(10),))?;
        assert_eq!(result, Ok(10));
        let (result,) = guest_result.call(&mut store, (Err(i64::MIN),))?;
        assert_eq!(result, Err(i64::MIN));

        let (result,) = guest_string.call(&mut store, ("",))?;
        assert_eq!(result, "");
        let (result,) = guest_string.call(&mut store, ("hello",))?;
        assert_eq!(result, "hello");

        let (result,) = guest_list.call(&mut store, (&[],))?;
        assert!(result.is_empty());
        let (result,) = guest_list.call(&mut store, (&["a", "", "b", "c"],))?;
        assert_eq!(result, ["a", "", "b", "c"]);

        instance
            .get_typed_func::<(), ()>(&mut store, "resource-intrinsics")?
            .call(&mut store, ())?;
    }
    {
        use wasmtime::component::Val;
        let mut store = Store::new(&engine, ());
        let mut linker = component::Linker::new(&engine);
        linker
            .root()
            .func_new("host-empty", |_, _, _args, _results| Ok(()))?;
        linker.root().func_new("host-u32", |_, _, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        linker.root().func_new("host-enum", |_, _, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        linker
            .root()
            .func_new("host-option", |_, _, args, results| {
                results[0] = args[0].clone();
                Ok(())
            })?;
        linker
            .root()
            .func_new("host-result", |_, _, args, results| {
                results[0] = args[0].clone();
                Ok(())
            })?;
        linker
            .root()
            .func_new("host-string", |_, _, args, results| {
                results[0] = args[0].clone();
                Ok(())
            })?;
        linker.root().func_new("host-list", |_, _, args, results| {
            results[0] = args[0].clone();
            Ok(())
        })?;
        let instance = linker.instantiate(&mut store, &component)?;

        let guest_empty = instance.get_func(&mut store, "guest-empty").unwrap();
        let guest_u32 = instance.get_func(&mut store, "guest-u32").unwrap();
        let guest_enum = instance.get_func(&mut store, "guest-enum").unwrap();
        let guest_option = instance.get_func(&mut store, "guest-option").unwrap();
        let guest_result = instance.get_func(&mut store, "guest-result").unwrap();
        let guest_string = instance.get_func(&mut store, "guest-string").unwrap();
        let guest_list = instance.get_func(&mut store, "guest-list").unwrap();

        let mut results = [];
        guest_empty.call(&mut store, &[], &mut results)?;

        let mut results = [Val::U32(0)];
        guest_u32.call(&mut store, &[Val::U32(42)], &mut results)?;
        assert_eq!(results[0], Val::U32(42));

        guest_enum.call(&mut store, &[Val::Enum("B".into())], &mut results)?;
        assert_eq!(results[0], Val::Enum("B".into()));

        guest_option.call(&mut store, &[Val::Option(None)], &mut results)?;
        assert_eq!(results[0], Val::Option(None));
        guest_option.call(
            &mut store,
            &[Val::Option(Some(Box::new(Val::U8(201))))],
            &mut results,
        )?;
        assert_eq!(results[0], Val::Option(Some(Box::new(Val::U8(201)))));

        guest_result.call(
            &mut store,
            &[Val::Result(Ok(Some(Box::new(Val::U16(20)))))],
            &mut results,
        )?;
        assert_eq!(results[0], Val::Result(Ok(Some(Box::new(Val::U16(20))))));
        guest_result.call(
            &mut store,
            &[Val::Result(Err(Some(Box::new(Val::S64(i64::MAX)))))],
            &mut results,
        )?;
        assert_eq!(
            results[0],
            Val::Result(Err(Some(Box::new(Val::S64(i64::MAX)))))
        );

        guest_string.call(&mut store, &[Val::String("B".into())], &mut results)?;
        assert_eq!(results[0], Val::String("B".into()));
        guest_string.call(&mut store, &[Val::String("".into())], &mut results)?;
        assert_eq!(results[0], Val::String("".into()));

        guest_list.call(&mut store, &[Val::List(Vec::new())], &mut results)?;
        assert_eq!(results[0], Val::List(Vec::new()));
    }

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn pulley_provenance_test_async_components() -> Result<()> {
    let config = provenance_test_config();
    let engine = Engine::new(&config)?;
    let component = if cfg!(miri) {
        unsafe {
            Component::deserialize_file(
                &engine,
                "./tests/all/pulley_provenance_test_async_component.cwasm",
            )?
        }
    } else {
        Component::from_file(
            &engine,
            "./tests/all/pulley_provenance_test_async_component.wat",
        )?
    };
    {
        let mut store = Store::new(&engine, ());
        let mut linker = component::Linker::new(&engine);
        linker
            .root()
            .func_wrap_concurrent("return-slowly", |_, ()| {
                Box::pin(async {
                    for _ in 0..5 {
                        tokio::task::yield_now().await;
                    }
                    Ok(())
                })
            })?;

        let instance = linker.instantiate_async(&mut store, &component).await?;

        let run = instance.get_typed_func::<(), ()>(&mut store, "run-stackless")?;
        store
            .run_concurrent(async move |accessor| {
                wasmtime::error::Ok(run.call_concurrent(accessor, ()).await?)
            })
            .await??;

        let run = instance.get_typed_func::<(), ()>(&mut store, "run-stackful")?;
        store
            .run_concurrent(async move |accessor| {
                wasmtime::error::Ok(run.call_concurrent(accessor, ()).await?)
            })
            .await??;

        let run = instance.get_typed_func::<(), ()>(&mut store, "run-stackless-stackless")?;
        store
            .run_concurrent(async move |accessor| {
                wasmtime::error::Ok(run.call_concurrent(accessor, ()).await?)
            })
            .await??;

        let run = instance.get_typed_func::<(), ()>(&mut store, "run-stackful-stackful")?;
        store
            .run_concurrent(async move |accessor| {
                wasmtime::error::Ok(run.call_concurrent(accessor, ()).await?)
            })
            .await??;

        let run = instance.get_typed_func::<(), ()>(&mut store, "intra-component-stream")?;
        store
            .run_concurrent(async move |accessor| {
                wasmtime::error::Ok(run.call_concurrent(accessor, ()).await?)
            })
            .await??;
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

// Ensure that Pulley doesn't require that the input image is aligned at any
// particular boundary, namely for now this double-checks that the `unaligned`
// feature of the `object` crate is enabled.
#[test]
fn decode_unaligned() -> Result<()> {
    let engine = Engine::new(&pulley_config())?;
    let mut bytes = Module::new(&engine, "(module)")?.serialize()?;

    for i in 0..10 {
        let serialized = &bytes[i..];
        unsafe {
            Module::deserialize_raw(&engine, NonNull::from(serialized))?;
        }
        bytes.insert(0, 0);
    }

    Ok(())
}

// Runtime-semantics tests for the call_indirect fusion stack
// (`tests/disas/pulley-fusion-*.wat` covers the static disasm side).
// Each test runs the same wasm under Pulley and native Cranelift and
// asserts the results agree.

/// Pulley config for tests that exercise traps. The interpreter can't
/// catch signals, so trap emission must be explicit.
fn pulley_trap_safe_config() -> Config {
    let mut config = pulley_config();
    config.signals_based_traps(false);
    config
}

fn pulley_and_native_agree<Params, Results>(
    wat: &str,
    func_name: &str,
    params: Params,
) -> Result<Results>
where
    Params: wasmtime::WasmParams + Copy,
    Results: wasmtime::WasmResults + std::fmt::Debug + PartialEq,
{
    let bytes = wat::parse_str(wat)?;
    let pulley = {
        let engine = Engine::new(&pulley_trap_safe_config())?;
        let module = Module::new(&engine, &bytes)?;
        let mut store = Store::new(&engine, ());
        let inst = Instance::new(&mut store, &module, &[])?;
        let f = inst.get_typed_func::<Params, Results>(&mut store, func_name)?;
        f.call(&mut store, params)?
    };
    let native = {
        let engine = Engine::new(&Config::new())?;
        let module = Module::new(&engine, &bytes)?;
        let mut store = Store::new(&engine, ());
        let inst = Instance::new(&mut store, &module, &[])?;
        let f = inst.get_typed_func::<Params, Results>(&mut store, func_name)?;
        f.call(&mut store, params)?
    };
    assert_eq!(
        pulley, native,
        "Pulley and native diverged for `{func_name}` — fusion lowering bug?"
    );
    Ok(pulley)
}

/// Fusion returns the right callee for every in-bounds index and traps
/// on OOB.
#[test]
fn fusion_call_indirect_every_index() -> Result<()> {
    let wat = r#"
    (module
      (table 3 3 funcref)
      (func $f0 (result i32) i32.const 100)
      (func $f1 (result i32) i32.const 101)
      (func $f2 (result i32) i32.const 102)
      (func (export "call") (param i32) (result i32)
        local.get 0
        call_indirect (result i32))
      (elem (i32.const 0) func $f0 $f1 $f2))
    "#;
    for (idx, expected) in [(0_i32, 100_i32), (1, 101), (2, 102)] {
        let got: i32 = pulley_and_native_agree(wat, "call", idx)?;
        assert_eq!(got, expected, "idx {idx}");
    }
    // Pulley only — native signal-based traps interact badly with
    // `cargo test`'s debug-mode signal handlers.
    let bytes = wat::parse_str(wat)?;
    let engine = Engine::new(&pulley_trap_safe_config())?;
    let module = Module::new(&engine, &bytes)?;
    let mut store = Store::new(&engine, ());
    let inst = Instance::new(&mut store, &module, &[])?;
    let f = inst.get_typed_func::<i32, i32>(&mut store, "call")?;
    let err = f.call(&mut store, 3).unwrap_err();
    let trap = err.downcast_ref::<Trap>().expect("Trap");
    assert_eq!(*trap, Trap::TableOutOfBounds);
    Ok(())
}

/// Two call_indirect sites in the same function; each must fuse
/// independently.
#[test]
fn fusion_call_indirect_multi_site() -> Result<()> {
    let wat = r#"
    (module
      (table 3 3 funcref)
      (func $f0 (result i32) i32.const 10)
      (func $f1 (result i32) i32.const 20)
      (func $f2 (result i32) i32.const 30)
      (func (export "sum") (param i32 i32) (result i32)
        local.get 0 call_indirect (result i32)
        local.get 1 call_indirect (result i32)
        i32.add)
      (elem (i32.const 0) func $f0 $f1 $f2))
    "#;
    for (a, b, expected) in [(0_i32, 1_i32, 30_i32), (1, 2, 50), (2, 0, 40), (1, 1, 40)] {
        let got: i32 = pulley_and_native_agree(wat, "sum", (a, b))?;
        assert_eq!(got, expected, "a={a} b={b}");
    }
    Ok(())
}

/// `return_call_indirect` correctness with fusion applied.
#[test]
fn fusion_return_call_indirect() -> Result<()> {
    let wat = r#"
    (module
      (table 2 2 funcref)
      (type $sig (func (result i32)))
      (func $f0 (result i32) i32.const 7)
      (func $f1 (result i32) i32.const 11)
      (func (export "tail") (param i32) (result i32)
        local.get 0
        return_call_indirect (type $sig))
      (elem (i32.const 0) func $f0 $f1))
    "#;
    for (idx, expected) in [(0_i32, 7_i32), (1, 11)] {
        let got: i32 = pulley_and_native_agree(wat, "tail", idx)?;
        assert_eq!(got, expected, "idx {idx}");
    }
    Ok(())
}

/// Host mutates a slot to `ref.null func`; call_indirect must trap
/// `IndirectCallToNull`.
#[test]
fn fusion_call_indirect_with_host_null_set() -> Result<()> {
    let wat = r#"
    (module
      (table (export "t") 2 2 funcref)
      (func $f0 (result i32) i32.const 100)
      (func (export "call") (param i32) (result i32)
        local.get 0
        call_indirect (result i32))
      (elem (i32.const 0) func $f0 $f0))
    "#;
    let bytes = wat::parse_str(wat)?;

    // Pulley only (see note on `fusion_call_indirect_null_slot`).
    let engine = Engine::new(&pulley_trap_safe_config())?;
    let module = Module::new(&engine, &bytes)?;
    let mut store = Store::new(&engine, ());
    let inst = Instance::new(&mut store, &module, &[])?;
    let call = inst.get_typed_func::<i32, i32>(&mut store, "call")?;
    assert_eq!(call.call(&mut store, 0)?, 100);
    assert_eq!(call.call(&mut store, 1)?, 100);

    let table = inst.get_table(&mut store, "t").expect("table export");
    table.set(&mut store, 1, wasmtime::Ref::Func(None))?;

    assert_eq!(call.call(&mut store, 0)?, 100);
    let err = call.call(&mut store, 1).unwrap_err();
    let trap = err.downcast_ref::<Trap>().expect("Trap");
    assert_eq!(*trap, Trap::IndirectCallToNull);
    Ok(())
}

/// Host `Table::set` swaps to a different funcref between calls; the
/// second call must observe the new target.
#[test]
fn fusion_call_indirect_with_host_swap() -> Result<()> {
    let wat = r#"
    (module
      (table (export "t") 1 1 funcref)
      (func $f0 (result i32) i32.const 100)
      (func $f1 (result i32) i32.const 200)
      (func (export "f1_ref") (result funcref) ref.func $f1)
      (func (export "call") (param i32) (result i32)
        local.get 0
        call_indirect (result i32))
      (elem declare func $f1)
      (elem (i32.const 0) func $f0))
    "#;
    let bytes = wat::parse_str(wat)?;

    for use_pulley in [true, false] {
        let cfg = if use_pulley {
            pulley_trap_safe_config()
        } else {
            Config::new()
        };
        let engine = Engine::new(&cfg)?;
        let module = Module::new(&engine, &bytes)?;
        let mut store = Store::new(&engine, ());
        let inst = Instance::new(&mut store, &module, &[])?;
        let call = inst.get_typed_func::<i32, i32>(&mut store, "call")?;
        assert_eq!(call.call(&mut store, 0)?, 100);

        let f1_ref = inst
            .get_typed_func::<(), Option<wasmtime::Func>>(&mut store, "f1_ref")?
            .call(&mut store, ())?
            .expect("f1_ref returned None");
        let table = inst.get_table(&mut store, "t").expect("table export");
        table.set(&mut store, 0, wasmtime::Ref::Func(Some(f1_ref)))?;

        assert_eq!(call.call(&mut store, 0)?, 200, "use_pulley={use_pulley}");
    }
    Ok(())
}

/// Module B imports module A's table and calls into it. Tables are
/// imported, so the importer's `tables_mutated` is `true` and no
/// fusion fires on B's side; the call must still produce the right
/// result.
#[test]
fn fusion_call_indirect_imported_table() -> Result<()> {
    let wat_a = r#"
    (module
      (table (export "t") 2 2 funcref)
      (func $f0 (result i32) i32.const 42)
      (func $f1 (result i32) i32.const 84)
      (elem (i32.const 0) func $f0 $f1))
    "#;
    let wat_b = r#"
    (module
      (import "a" "t" (table 2 2 funcref))
      (func (export "call") (param i32) (result i32)
        local.get 0
        call_indirect (result i32)))
    "#;
    let bytes_a = wat::parse_str(wat_a)?;
    let bytes_b = wat::parse_str(wat_b)?;

    for use_pulley in [true, false] {
        let cfg = if use_pulley {
            pulley_trap_safe_config()
        } else {
            Config::new()
        };
        let engine = Engine::new(&cfg)?;
        let module_a = Module::new(&engine, &bytes_a)?;
        let module_b = Module::new(&engine, &bytes_b)?;
        let mut store = Store::new(&engine, ());
        let inst_a = Instance::new(&mut store, &module_a, &[])?;
        let table_export = inst_a.get_export(&mut store, "t").expect("a.t");

        let mut linker = wasmtime::Linker::new(&engine);
        linker.define(&store, "a", "t", table_export)?;
        let inst_b = linker.instantiate(&mut store, &module_b)?;

        let call = inst_b.get_typed_func::<i32, i32>(&mut store, "call")?;
        for (idx, expected) in [(0_i32, 42_i32), (1, 84)] {
            assert_eq!(
                call.call(&mut store, idx)?,
                expected,
                "use_pulley={use_pulley} idx={idx}"
            );
        }
    }
    Ok(())
}

/// Single call_indirect to an uninitialised slot — the phase-2 fused
/// op's runtime null check must trap cleanly with the right trap kind,
/// not crash on the field deref.
///
/// Call into an uninitialised table slot must trap.
#[test]
fn fusion_call_indirect_null_slot() -> Result<()> {
    let wat = r#"
    (module
      (table (export "t") 1 1 funcref)
      (func (export "call") (param i32) (result i32)
        local.get 0
        call_indirect (result i32)))
    "#;
    let bytes = wat::parse_str(wat)?;
    // Pulley only — see note on `fusion_call_indirect_every_index`.
    let engine = Engine::new(&pulley_trap_safe_config())?;
    let module = Module::new(&engine, &bytes)?;
    let mut store = Store::new(&engine, ());
    let inst = Instance::new(&mut store, &module, &[])?;
    let call = inst.get_typed_func::<i32, i32>(&mut store, "call")?;
    let err = call.call(&mut store, 0).unwrap_err();
    let trap = err.downcast_ref::<Trap>().expect("Trap");
    assert_eq!(*trap, Trap::IndirectCallToNull);
    Ok(())
}

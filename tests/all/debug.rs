//! Tests for instrumentation-based debugging.

use wasmtime::{Caller, Config, Engine, Extern, Func, Instance, Module, Store, ValType};

fn test_stack_values<C: Fn(&mut Config), F: Fn(Caller<'_, ()>) + Send + Sync + 'static>(
    wat: &str,
    c: C,
    f: F,
) -> anyhow::Result<()> {
    let mut config = Config::default();
    config.debug_instrumentation(true);
    config.wasm_exceptions(true);
    c(&mut config);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wat)?;

    let mut store = Store::new(&engine, ());
    let func = Func::wrap(&mut store, move |caller: Caller<'_, ()>| {
        f(caller);
    });
    let instance = Instance::new(&mut store, &module, &[Extern::Func(func)])?;
    let mut results = [];
    instance
        .get_func(&mut store, "main")
        .unwrap()
        .call(&mut store, &[], &mut results)?;

    Ok(())
}

#[test]
fn stack_values_two_frames() -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    for inlining in [false, true] {
        test_stack_values(
            r#"
    (module
      (import "" "host" (func))
      (func (export "main")
        i32.const 1
        i32.const 2
        call 2
        drop)
      (func (param i32 i32) (result i32)
        local.get 0
        local.get 1
        call 0
        i32.add))
    "#,
            |config| {
                config.compiler_inlining(inlining);
                if inlining {
                    unsafe {
                        config.cranelift_flag_set("wasmtime_inlining_intra_module", "true");
                    }
                }
            },
            |mut caller: Caller<'_, ()>| {
                let mut stack = caller.stack_values().unwrap();
                let frame = stack.next().unwrap();
                assert_eq!(
                    frame
                        .wasm_function_index_and_pc(&mut stack)
                        .unwrap()
                        .0
                        .as_u32(),
                    1
                );
                assert_eq!(frame.wasm_function_index_and_pc(&mut stack).unwrap().1, 65);

                assert_eq!(frame.num_locals(), 2);
                assert_eq!(frame.num_stacks(), 2);
                assert!(matches!(frame.local(&mut stack, 0).0, ValType::I32));
                assert!(matches!(frame.local(&mut stack, 1).0, ValType::I32));
                assert_eq!(frame.local(&mut stack, 0).1.unwrap_i32(), 1);
                assert_eq!(frame.local(&mut stack, 1).1.unwrap_i32(), 2);
                assert!(matches!(frame.stack(&mut stack, 0).0, ValType::I32));
                assert!(matches!(frame.stack(&mut stack, 1).0, ValType::I32));
                assert_eq!(frame.stack(&mut stack, 0).1.unwrap_i32(), 1);
                assert_eq!(frame.stack(&mut stack, 1).1.unwrap_i32(), 2);

                let frame = stack.next().unwrap();
                assert_eq!(
                    frame
                        .wasm_function_index_and_pc(&mut stack)
                        .unwrap()
                        .0
                        .as_u32(),
                    0
                );
                assert_eq!(frame.wasm_function_index_and_pc(&mut stack).unwrap().1, 55);

                assert!(stack.next().is_none());
            },
        )?;
    }
    Ok(())
}

#[test]
fn stack_values_exceptions() -> anyhow::Result<()> {
    test_stack_values(
        r#"
    (module
      (tag $t (param i32))
      (import "" "host" (func))
      (func (export "main")
        (block $b (result i32)
          (try_table (catch $t $b)
            (throw $t (i32.const 42)))
          i32.const 0)
        (call 0)
        (drop)))
    "#,
        |_config| {},
        |mut caller: Caller<'_, ()>| {
            let mut stack = caller.stack_values().unwrap();
            let frame = stack.next().unwrap();
            assert_eq!(frame.num_stacks(), 1);
            assert!(matches!(frame.stack(&mut stack, 0).0, ValType::I32));
            assert_eq!(frame.stack(&mut stack, 0).1.unwrap_i32(), 42);
            assert!(stack.next().is_none());
        },
    )
}

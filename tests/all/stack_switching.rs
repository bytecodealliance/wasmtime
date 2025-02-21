use anyhow::Result;
use wasmtime::*;

mod test_utils {
    use anyhow::{bail, Result};
    use std::any::*;
    use std::panic::AssertUnwindSafe;
    use wasmtime::*;

    pub struct Runner {
        pub engine: Engine,
        pub store: Store<()>,
    }

    impl Runner {
        pub fn new() -> Runner {
            let mut config = Config::default();
            config.wasm_function_references(true);
            config.wasm_stack_switching(true);
            // Required in order to use recursive types.
            config.wasm_gc(true);

            let engine = Engine::new(&config).unwrap();

            let store = Store::<()>::new(&engine, ());

            Runner { engine, store }
        }

        /// Uses this `Runner` to run the module defined in `wat`, satisfying
        /// its imports using `imports`. The module must export a function
        /// `entry`, taking no parameters and returning `Results`.
        pub fn run_test<Results: WasmResults>(
            mut self,
            wat: &str,
            imports: &[Extern],
        ) -> Result<Results> {
            let module = Module::new(&self.engine, wat)?;

            let instance = Instance::new(&mut self.store, &module, imports)?;
            let entry = instance.get_typed_func::<(), Results>(&mut self.store, "entry")?;

            entry.call(&mut self.store, ())
        }

        /// Uses this `Runner` to run the module defined in `wat`, satisfying
        /// its imports using `imports`. The module must export a function
        /// `entry`, taking no parameters and without return values. Execution
        /// of `entry` is expected to cause a panic (and that this is panic is
        /// not handled by wasmtime previously).
        /// Returns the `Error` payload.
        pub fn run_test_expect_panic(
            mut self,
            wat: &str,
            imports: &[Extern],
        ) -> Box<dyn Any + Send + 'static> {
            let module = Module::new(&self.engine, wat).unwrap();

            let instance = Instance::new(&mut self.store, &module, imports).unwrap();
            let entry = instance.get_func(&mut self.store, "entry").unwrap();

            std::panic::catch_unwind(AssertUnwindSafe(|| {
                drop(entry.call(&mut self.store, &[], &mut []))
            }))
            .unwrap_err()
        }
    }

    /// Creates a simple Host function that increments an i32
    pub fn make_i32_inc_host_func(runner: &mut Runner) -> Func {
        Func::new(
            &mut runner.store,
            FuncType::new(&runner.engine, vec![ValType::I32], vec![ValType::I32]),
            |mut _caller, args: &[Val], results: &mut [Val]| {
                let res = match args {
                    [Val::I32(i)] => i + 1,
                    _ => bail!("Error: Received illegal argument (should be single i32)"),
                };
                results[0] = Val::I32(res);
                Ok(())
            },
        )
    }

    /// Creates a host function of type i32 -> i32. `export_func` must denote an
    /// exported function of type i32 -> i32. The created host function
    /// increments its argument by 1, passes it to the exported function, and in
    /// turn increments the result before returning it as the overall result.
    pub fn make_i32_inc_via_export_host_func(
        runner: &mut Runner,
        export_func: &'static str,
    ) -> Func {
        Func::new(
            &mut runner.store,
            FuncType::new(&runner.engine, vec![ValType::I32], vec![ValType::I32]),
            |mut caller, args: &[Val], results: &mut [Val]| {
                let export = caller
                    .get_export(export_func)
                    .ok_or(anyhow::anyhow!("could not get export"))?;
                let func = export
                    .into_func()
                    .ok_or(anyhow::anyhow!("export is not a Func"))?;
                let func_typed = func.typed::<i32, i32>(caller.as_context())?;
                let arg = args[0].unwrap_i32();
                let res = func_typed.call(caller.as_context_mut(), arg + 1)?;
                results[0] = Val::I32(res + 1);
                Ok(())
            },
        )
    }

    /// Creates a function without parameters or return values that simply calls
    /// the given function.
    pub fn make_call_export_host_func(runner: &mut Runner, export_func: &'static str) -> Func {
        Func::new(
            &mut runner.store,
            FuncType::new(&runner.engine, vec![], vec![]),
            |mut caller, _args: &[Val], _results: &mut [Val]| {
                let export = caller
                    .get_export(export_func)
                    .ok_or(anyhow::anyhow!("could not get export"))?;
                let func = export
                    .into_func()
                    .ok_or(anyhow::anyhow!("export is not a Func"))?;
                let func_typed = func.typed::<(), ()>(caller.as_context())?;
                let _res = func_typed.call(caller.as_context_mut(), ())?;
                Ok(())
            },
        )
    }

    pub fn make_panicking_host_func(store: &mut Store<()>, msg: &'static str) -> Func {
        Func::wrap(store, move || -> () { std::panic::panic_any(msg) })
    }
}

mod wasi {
    use anyhow::Result;
    use wasmtime::{Config, Engine, Linker, Module, Store};
    use wasmtime_wasi::preview1::{self, WasiP1Ctx};
    use wasmtime_wasi::WasiCtxBuilder;

    fn run_wasi_test(wat: &'static str) -> Result<i32> {
        // Construct the wasm engine with async support disabled.
        let mut config = Config::new();
        config
            .async_support(false)
            .wasm_function_references(true)
            .wasm_stack_switching(true);
        let engine = Engine::new(&config)?;

        // Add the WASI preview1 API to the linker (will be implemented in terms of
        // the preview2 API)
        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        preview1::add_to_linker_sync(&mut linker, |t| t)?;

        // Add capabilities (e.g. filesystem access) to the WASI preview2 context here.
        let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build_p1();

        let mut store: Store<WasiP1Ctx> = Store::new(&engine, wasi_ctx);

        // Instantiate our wasm module.
        let module = Module::new(&engine, wat)?;
        let func = linker
            .module(&mut store, "", &module)?
            .get_default(&mut store, "")?
            .typed::<(), i32>(&store)?;

        // Invoke the WASI program default function.
        func.call(&mut store, ())
    }

    async fn run_wasi_test_async(wat: &'static str) -> Result<i32> {
        // Construct the wasm engine with async support enabled.
        let mut config = Config::new();
        config
            .async_support(true)
            .wasm_function_references(true)
            .wasm_stack_switching(true);
        let engine = Engine::new(&config)?;

        // Add the WASI preview1 API to the linker (will be implemented in terms of
        // the preview2 API)
        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        preview1::add_to_linker_async(&mut linker, |t| t)?;

        // Add capabilities (e.g. filesystem access) to the WASI preview2 context here.
        let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build_p1();

        let mut store: Store<WasiP1Ctx> = Store::new(&engine, wasi_ctx);

        // Instantiate our wasm module.
        let module = Module::new(&engine, wat)?;
        let func = linker
            .module_async(&mut store, "", &module)
            .await?
            .get_default(&mut store, "")?
            .typed::<(), i32>(&store)?;

        // Invoke the WASI program default function.
        func.call_async(&mut store, ()).await
    }

    static WRITE_SOMETHING_WAT: &'static str = &r#"
(module
  (type $ft (func (result i32)))
  (type $ct (cont $ft))
  (import "wasi_snapshot_preview1" "fd_write"
     (func $print (param $fd i32)
                  (param $iovec i32)
                  (param $len i32)
                  (param $written i32) (result i32)))
  (memory 1)
  (export "memory" (memory 0))

  ;; 9 is the offset to write to
  (data (i32.const 9) "something\n")

  (func $f (result i32)
    (i32.const 0) ;; offset
    (i32.const 9) ;; value start of the string
    (i32.store)

    (i32.const 4)                ;; offset
    (i32.const 11)               ;; value, the length of the string
    (i32.store offset=0 align=2) ;; size_buf_len

    (i32.const 1)   ;; 1 for stdout
    (i32.const 0)   ;; 0 as we stored the beginning of __wasi_ciovec_t
    (i32.const 1)   ;;
    (i32.const 20)  ;; nwritten
    (call $print)
  )
  (elem declare func $f)

  (func (export "_start") (result i32)
    (ref.func $f)
    (cont.new $ct)
    (resume $ct)
  )
)"#;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn write_something_test() -> Result<()> {
        assert_eq!(run_wasi_test(WRITE_SOMETHING_WAT)?, 0);
        Ok(())
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn write_something_test_async() -> Result<()> {
        assert_eq!(run_wasi_test_async(WRITE_SOMETHING_WAT).await?, 0);
        Ok(())
    }

    static SCHED_YIELD_WAT: &'static str = r#"
(module
  (type $ft (func (result i32)))
  (type $ct (cont $ft))
  (import "wasi_snapshot_preview1" "sched_yield"
     (func $sched_yield (result i32)))
  (memory 1)
  (export "memory" (memory 0))

  (func $g (result i32)
    (call $sched_yield))
  (elem declare func $g)

  (func (export "_start") (result i32)
    (cont.new $ct (ref.func $g))
    (resume $ct)
  )
)"#;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn sched_yield_test() -> Result<()> {
        assert_eq!(run_wasi_test(SCHED_YIELD_WAT)?, 0);
        Ok(())
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn sched_yield_test_async() -> Result<()> {
        assert_eq!(run_wasi_test_async(SCHED_YIELD_WAT).await?, 0);
        Ok(())
    }
}

/// Test that two distinct instantiations of the same module yield
/// different control tag identities.
#[test]
#[cfg_attr(miri, ignore)]
fn inter_instance_suspend() -> Result<()> {
    let mut config = Config::default();
    config.wasm_function_references(true);
    config.wasm_stack_switching(true);

    let engine = Engine::new(&config)?;

    let mut store = Store::<()>::new(&engine, ());

    let wat_other = r#"
        (module

          (type $ft (func))
          (type $ct (cont $ft))
          (tag $tag)


          (func $suspend (export "suspend")
            (suspend $tag)
          )

          (func $resume (export "resume") (param $f (ref $ct))
            (block $handler (result (ref $ct))
              (resume $ct (on $tag $handler) (local.get $f))
              (unreachable)
            )
            (drop)
          )
        )
    "#;

    let wat_main = r#"
        (module

          (type $ft (func))
          (type $ct (cont $ft))

          (import "other" "suspend" (func $suspend))
          (import "other" "resume" (func $resume (param (ref $ct))))

          (elem declare func $suspend)


          (func $entry (export "entry")
            (call $resume (cont.new $ct (ref.func $suspend)))
          )
        )
    "#;

    let module_other = Module::new(&engine, wat_other)?;

    let other_inst1 = Instance::new(&mut store, &module_other, &[])?;
    let other_inst2 = Instance::new(&mut store, &module_other, &[])?;

    // Crucially, suspend and resume are from two separate instances
    // of the same module.
    let suspend = other_inst1.get_func(&mut store, "suspend").unwrap();
    let resume = other_inst2.get_func(&mut store, "resume").unwrap();

    let module_main = Module::new(&engine, wat_main)?;
    let main_instance = Instance::new(&mut store, &module_main, &[suspend.into(), resume.into()])?;
    let entry_func = main_instance.get_func(&mut store, "entry").unwrap();

    let result = entry_func.call(&mut store, &[], &mut []);
    assert!(result.is_err());
    Ok(())
}

/// Tests interaction with host functions. Note that the interaction with host
/// functions and traps is covered by the module `traps` further down.
mod host {
    use super::test_utils::*;
    use wasmtime::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests calling a host function from within a wasm function running inside a continuation.
    /// Call chain:
    /// $entry -resume-> a -call-> host_func_a
    fn call_host_from_continuation() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func (result i32)))
            (type $ct (cont $ft))

            (import "" "" (func $host_func_a (param i32) (result i32)))

            (func $a (export "a") (result i32)
                (call $host_func_a (i32.const 122))
            )
            (func $entry (export "entry") (result i32)
                (resume $ct (cont.new $ct (ref.func $a)))
            )
        )
    "#;

        let mut runner = Runner::new();

        let host_func_a = make_i32_inc_host_func(&mut runner);

        let result = runner.run_test::<i32>(wat, &[host_func_a.into()]).unwrap();
        assert_eq!(result, 123);
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// We re-enter wasm from a host function and execute a continuation.
    /// Call chain:
    /// $entry -call-> $a -call-> $host_func_a -call-> $b -resume-> $c
    fn re_enter_wasm_ok1() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func (param i32) (result i32)))
            (type $ct (cont $ft))

            (import "" "" (func $host_func_a (param i32) (result i32)))


            (func $a (export "a") (param $x i32) (result i32)
                (call $host_func_a (local.get $x))
            )

            (func $b (export "b") (param $x i32) (result i32)
                (resume $ct (local.get $x) (cont.new $ct (ref.func $c)))
            )

            (func $c (export "c") (param $x i32) (result i32)
                (return (i32.add (local.get $x) (i32.const 1)))
            )


            (func $entry (export "entry") (result i32)
                (call $a (i32.const 120))
            )
        )
    "#;

        let mut runner = Runner::new();

        let host_func_a = make_i32_inc_via_export_host_func(&mut runner, "b");

        let result = runner.run_test::<i32>(wat, &[host_func_a.into()]).unwrap();
        assert_eq!(result, 123);
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Similar to `re_enter_wasm_ok2, but we run a continuation before the host call.
    /// Call chain:
    /// $entry -call-> $a -call-> $host_func_a -call-> $b -resume-> $c
    fn re_enter_wasm_ok2() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func (param i32) (result i32)))
            (type $ct (cont $ft))

            (import "" "" (func $host_func_a (param i32) (result i32)))


            (func $a (export "a") (param $x i32) (result i32)
                ;; Running continuation before calling into host is fine
                (resume $ct (local.get $x) (cont.new $ct (ref.func $c)))
                (drop)

                (call $host_func_a (local.get $x))
            )

            (func $b (export "b") (param $x i32) (result i32)
                (resume $ct (local.get $x) (cont.new $ct (ref.func $c)))
            )

            (func $c (export "c") (param $x i32) (result i32)
                (return (i32.add (local.get $x) (i32.const 1)))
            )


            (func $entry (export "entry") (result i32)
                (call $a (i32.const 120))
            )
        )
    "#;

        let mut runner = Runner::new();

        let host_func_a = make_i32_inc_via_export_host_func(&mut runner, "b");

        let result = runner.run_test::<i32>(wat, &[host_func_a.into()]).unwrap();
        assert_eq!(result, 123);
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// We re-enter wasm from a host function while we were already on a continuation stack.
    /// This is currently forbidden (see wasmfx/wasmfxtime#109), but may be
    /// allowed in the future.
    /// Call chain:
    /// $entry -resume-> $a -call-> $host_func_a -call-> $b
    fn re_enter_wasm_ok3() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func (param i32) (result i32)))
            (type $ct (cont $ft))

            (import "" "" (func $host_func_a (param i32) (result i32)))


            (func $a (export "a") (param $x i32) (result i32)
                (call $host_func_a (local.get $x))
            )


            (func $b (export "b") (param $x i32) (result i32)
                 (return (i32.add (local.get $x) (i32.const 1)))
            )


            (func $entry (export "entry") (result i32)
                (resume $ct (i32.const 120) (cont.new $ct (ref.func $a)))
            )
        )
    "#;
        let mut runner = Runner::new();

        let host_func_a = make_i32_inc_via_export_host_func(&mut runner, "b");

        let result = runner.run_test::<i32>(wat, &[host_func_a.into()]).unwrap();
        assert_eq!(result, 123);
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// After crossing from the host back into wasm, we suspend to a tag that is
    /// handled by the surrounding function (i.e., without needing to cross the
    /// host frame to reach the handler).
    /// Call chain:
    /// $entry -resume-> $a -call-> $host_func_a -call-> $b -resume-> $c
    fn call_host_from_continuation_nested_suspend_ok() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func (param i32) (result i32)))
            (type $ct (cont $ft))
            (tag $t (result i32))

            (import "" "" (func $host_func_a (param i32) (result i32)))


            (func $a (export "a") (param $x i32) (result i32)
                (call $host_func_a (local.get $x))
            )


            (func $b (export "b") (param $x i32) (result i32)
                (block $h (result (ref $ct))
                  (resume $ct (on $t $h) (local.get $x) (cont.new $ct (ref.func $c)))
                  (unreachable)
                )
                (drop)
                ;; note that we do not run the continuation to completion
                (i32.add (local.get $x) (i32.const 1))
            )

            (func $c (export "c") (param $x i32) (result i32)
                (suspend $t)
            )


            (func $entry (export "entry") (result i32)
                (resume $ct (i32.const 120) (cont.new $ct (ref.func $a)))
            )
        )
    "#;

        let mut runner = Runner::new();

        let host_func_a = make_i32_inc_via_export_host_func(&mut runner, "b");

        let result = runner.run_test::<i32>(wat, &[host_func_a.into()]).unwrap();
        assert_eq!(result, 123);
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Similar to `call_host_from_continuation_nested_suspend_ok`. However, we
    /// suspend to a tag that is only handled if we were to cross a host
    /// function boundary. That's not allowed, so we effectively suspend with an
    /// unhandled tag.
    /// Call chain:
    /// $entry -resume-> $a -call-> $host_func_a -call-> $b -resume-> $c
    fn call_host_from_continuation_nested_suspend_unhandled() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func (param i32) (result i32)))
            (type $ct (cont $ft))
            (tag $t (result i32))

            (import "" "" (func $host_func_a (param i32) (result i32)))


            (func $a (export "a") (param $x i32) (result i32)
                (call $host_func_a (local.get $x))
            )


            (func $b (export "b") (param $x i32) (result i32)
                (resume $ct (local.get $x) (cont.new $ct (ref.func $c)))
            )

            (func $c (export "c") (param $x i32) (result i32)
                (suspend $t)
            )


            (func $entry (export "entry") (result i32)
                (block $h (result (ref $ct))
                    (resume $ct
                        (on $t $h)
                        (i32.const 122)
                        (cont.new $ct (ref.func $a)))
                    (return)
                )
                (unreachable)
            )
        )
    "#;

        let mut runner = Runner::new();

        let host_func_a = Func::new(
            &mut runner.store,
            FuncType::new(&runner.engine, vec![ValType::I32], vec![ValType::I32]),
            |mut caller, args: &[Val], results: &mut [Val]| {
                let export = caller
                    .get_export("b")
                    .ok_or(anyhow::anyhow!("could not get export"))?;
                let func = export
                    .into_func()
                    .ok_or(anyhow::anyhow!("export is not a Func"))?;

                let func_typed = func.typed::<i32, i32>(caller.as_context())?;
                let arg = args[0].unwrap_i32();
                let res = func_typed.call(caller.as_context_mut(), arg + 1);
                let err = res.unwrap_err();

                assert!(err.root_cause().is::<Trap>());
                assert_eq!(*err.downcast_ref::<Trap>().unwrap(), Trap::UnhandledTag);

                let trace = err.downcast_ref::<WasmBacktrace>().unwrap();
                let frames: Vec<_> = trace
                    .frames()
                    .iter()
                    .map(|frame| {
                        frame
                            .func_name()
                            .expect("Expecting all functions in actual backtrace to have names")
                    })
                    .rev()
                    .collect();
                assert_eq!(frames, &["entry", "a", "b", "c"]);

                results[0] = Val::I32(arg + 1);
                Ok(())
            },
        );

        let result = runner.run_test::<i32>(wat, &[host_func_a.into()]).unwrap();
        assert_eq!(result, 123);
        Ok(())
    }
}

mod traps {
    use super::test_utils::*;
    use wasmtime::*;

    fn backtrace_from_err(err: &Error) -> impl Iterator<Item = &'_ str> {
        let trace = err.downcast_ref::<WasmBacktrace>().unwrap();

        trace
            .frames()
            .iter()
            .map(|frame| {
                frame
                    .func_name()
                    .expect("Expecting all functions in actual backtrace to have names")
            })
            .rev()
    }

    /// Runs the module given as `wat`. We expect execution to cause the
    /// `expected_trap` and a backtrace containing exactly the function names
    /// given by `expected_backtrace`.
    fn run_test_expect_trap_backtrace(wat: &str, expected_trap: Trap, expected_backtrace: &[&str]) {
        let runner = Runner::new();
        let result = runner.run_test::<()>(wat, &[]);

        let err = result.expect_err("Was expecting wasm execution to yield error");

        assert!(err.root_cause().is::<Trap>());
        assert_eq!(*err.downcast_ref::<Trap>().unwrap(), expected_trap);

        let actual_func_name_it = backtrace_from_err(&err);

        let expected_func_name_it = expected_backtrace.iter().copied();
        assert!(actual_func_name_it.eq(expected_func_name_it));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct backtraces if we trap deep inside multiple continuations.
    /// Call chain:
    /// $entry -call-> $a -resume-> $b -call-> $c -resume-> $d -call-> $e -resume-> $f
    /// Here, $f traps.
    fn trap_in_continuation_nested() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (resume $ct (cont.new $ct (ref.func $b)))
            )

            (func $b (export "b")
                (call $c)
            )

            (func $c (export "c")
                (resume $ct (cont.new $ct (ref.func $d)))
            )

            (func $d (export "d")
              (call $e)
            )

            (func $e (export "e")
                (resume $ct (cont.new $ct (ref.func $f)))
            )

            (func $f (export "f")
                (unreachable)
            )
        )
        "#;

        run_test_expect_trap_backtrace(
            wat,
            Trap::UnreachableCodeReached,
            &["entry", "a", "b", "c", "d", "e", "f"],
        );

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct backtraces if we trap after returning from one
    /// continuation to its parent.
    fn trap_in_continuation_back_to_parent() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (resume $ct (cont.new $ct (ref.func $b)))
            )

            (func $b (export "b")
                (call $c)
            )

            (func $c (export "c")
                (resume $ct (cont.new $ct (ref.func $d)))
                (unreachable)
            )

            (func $d (export "d")
                (call $e)
            )

            (func $e (export "e"))

        )
        "#;

        run_test_expect_trap_backtrace(
            wat,
            Trap::UnreachableCodeReached,
            &["entry", "a", "b", "c"],
        );

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct backtraces if we trap after returning from
    /// several continuations back to the main stack.
    fn trap_in_continuation_back_to_main() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (resume $ct (cont.new $ct (ref.func $b)))
                (unreachable)
            )

            (func $b (export "b")
                (call $c)
            )

            (func $c (export "c")
                (resume $ct (cont.new $ct (ref.func $d)))
            )

            (func $d (export "d")
              (call $e)
            )

            (func $e (export "e"))

        )
        "#;

        run_test_expect_trap_backtrace(wat, Trap::UnreachableCodeReached, &["entry", "a"]);

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct backtraces after suspending a continuation.
    fn trap_in_continuation_suspend() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))

            (tag $t)

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (resume $ct (cont.new $ct (ref.func $b)))
                (unreachable)
            )

            (func $b (export "b")
                (call $c)
            )

            (func $c (export "c")
                (block $handler (result  (ref $ct))
                    (resume $ct (on $t $handler) (cont.new $ct (ref.func $d)))
                    (return)
                )
                (unreachable)
            )

            (func $d (export "d")
                (call $e)
            )

            (func $e (export "e")
                (suspend $t)
            )

        )
    "#;

        run_test_expect_trap_backtrace(
            wat,
            Trap::UnreachableCodeReached,
            &["entry", "a", "b", "c"],
        );

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct backtraces after suspending a continuation and
    /// then resuming it from a different stack frame.
    fn trap_in_continuation_suspend_resume() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))

            (tag $t)

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (resume $ct (cont.new $ct (ref.func $b)))
            )

            (func $b (export "b")
                (resume $ct (call $c))
            )

            (func $c (export "c") (result (ref $ct))
                (block $handler (result  (ref $ct))
                    (resume $ct (on $t $handler) (cont.new $ct (ref.func $d)))

                    ;; We never want to get here, but also don't want to use
                    ;; (unreachable), which is the trap we deliberately use in
                    ;; this test. Instead, we call a null function ref here,
                    ;; which is guaranteed to trap.
                    (call_ref $ft (ref.null $ft))

                    (return (cont.new $ct (ref.func $d)))
                )
                ;; implicitly returning the continuation here
            )

            (func $d (export "d")
                (call $e)
                (unreachable)
            )

            (func $e (export "e")
                (suspend $t)
            )

        )
    "#;

        // Note that c does not appear in the stack trace:
        // In $b, we resume the suspended computation, which started in $d,
        // suspended in $e, and traps in $d
        run_test_expect_trap_backtrace(
            wat,
            Trap::UnreachableCodeReached,
            &["entry", "a", "b", "d"],
        );

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct backtraces after suspending a continuation
    /// where we need to forward to an outer handler.
    fn trap_in_continuation_forward() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))
            (tag $t)

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (block $handler (result  (ref $ct))
                    (resume $ct (on $t $handler) (cont.new $ct (ref.func $b)))
                    ;; We don't actually want to get here
                    (return)
                )
                (unreachable)
            )

            (func $b (export "b")
                (call $c)
            )

            (func $c (export "c")
                (resume $ct (cont.new $ct (ref.func $d)))
            )

            (func $d (export "d")
                (call $e)
            )

            (func $e (export "e")
                (suspend $t)
            )

        )
    "#;

        // Note that c does not appear in the stack trace:
        // In $b, we resume the suspended computation, which started in $d,
        // suspended in $e, and traps in $d
        run_test_expect_trap_backtrace(wat, Trap::UnreachableCodeReached, &["entry", "a"]);

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct backtraces after suspending a continuation
    /// where we need to forward to an outer handler. We then resume the
    /// continuation from within another continuation.
    fn trap_in_continuation_forward_resume() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))
            (tag $t)

            (global $k (mut (ref null $ct)) (ref.null $ct))

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (resume $ct (cont.new $ct (ref.func $b)))
            )

            (func $b (export "b")
                (block $handler (result  (ref $ct))
                    (resume $ct (on $t $handler) (cont.new $ct (ref.func $c)))
                    ;; We don't actually want to get here
                    (return)
                )
                (global.set $k)
                ;; $f will resume $k
                (resume $ct (cont.new $ct (ref.func $f)))
            )

            (func $c (export "c")
                (resume $ct (cont.new $ct (ref.func $d)))
            )

            (func $d (export "d")
                (call $e)
            )

            (func $e (export "e")
                (suspend $t)
                (unreachable)
            )

           (func $f  (export "f")
               (resume $ct (global.get $k))
           )
        )
       "#;

        run_test_expect_trap_backtrace(
            wat,
            Trap::UnreachableCodeReached,
            &["entry", "a", "b", "f", "c", "d", "e"],
        );

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct backtraces after switch.
    /// We first create the a stack with the following shape:
    /// entry -> a -> b, then switch, leading to
    /// entry -> c -> d, at which point we resume the a -> b continuation:
    /// entry -> c -> d -> a -> b
    /// We trap at that point.
    fn trap_switch_and_resume() -> Result<()> {
        let wat = r#"
        (module
            (rec
                (type $ft0 (func (param (ref null $ct0))))
                (type $ct0 (cont $ft0)))

            (type $ft1 (func))
            (type $ct1 (cont $ft1))

            (tag $t)

            (func $a (type $ft1)
                (cont.new $ct1 (ref.func $b))
                (resume $ct1)
            )
            (elem declare func $a)

            (func $b (type $ft1)
                (cont.new $ct0 (ref.func $c))
                (switch $ct0 $t)

                ;; we want a backtrace here
                (unreachable)
            )
            (elem declare func $b)

            (func $c (type $ft0)
                (local.get 0)
                (cont.new $ct0 (ref.func $d))
                (resume $ct0)
            )
            (elem declare func $c)

            (func $d (type $ft0)
                (block $handler (result (ref $ct1))
                    (ref.null $ct0) ;; passed as payload
                    (local.get 0) ;; resumed
                    (resume $ct0 (on $t $handler))
                    (unreachable) ;; f1 will suspend after the switch
                 )
                (resume $ct1)
            )
            (elem declare func $d)

            (func $entry (export "entry")
                (cont.new $ct1 (ref.func $a))
                (resume $ct1 (on $t switch))
            )
        )
        "#;

        run_test_expect_trap_backtrace(
            wat,
            Trap::UnreachableCodeReached,
            &["entry", "c", "d", "a", "b"],
        );

        Ok(())
    }

    // Test that we get correct backtraces after trapping inside a continuation
    // after re-entering Wasm while already inside a different continuation.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn trap_after_re_enter() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))
            (tag $t)

            (import "" "" (func $host_func_a))
            (import "" "" (func $host_func_b))

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (resume $ct (cont.new $ct (ref.func $b)))
            )

            (func $b (export "b")
                 (call $host_func_a)
            )

            (func $c (export "c")
                (resume $ct (cont.new $ct (ref.func $d)))

            )

            (func $d (export "d")
                (i32.const 0)
                (call $host_func_b)
                (drop)
            )

            (func $e (export "e")
                (resume $ct (cont.new $ct (ref.func $f)))
            )

           (func $f  (export "f")
               (unreachable)
           )
        )
       "#;

        let mut runner = Runner::new();
        let host_func_a = make_call_export_host_func(&mut runner, "c");
        let host_func_b = make_call_export_host_func(&mut runner, "e");

        let result = runner.run_test::<()>(wat, &[host_func_a.into(), host_func_b.into()]);
        let err = result.unwrap_err();

        assert!(err.root_cause().is::<Trap>());
        assert_eq!(
            *err.downcast_ref::<Trap>().unwrap(),
            Trap::UnreachableCodeReached
        );

        let backtrace = backtrace_from_err(&err);
        assert!(backtrace.eq(["entry", "a", "b", "c", "d", "e", "f"].into_iter()));

        Ok(())
    }

    // Tests that we properly clean up the instance/store after trapping while
    // running inside a continuation: There must be no leftovers of the old
    // stack chain if we re-use the instance later.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn reuse_instance_after_trap1() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))

            (tag $t)

            (func $entry1 (export "entry1")
              (local $c (ref $ct))
              (local.set $c (cont.new $ct (ref.func $a)))
              (block $handlet (result (ref $ct))
                (resume $ct (on $t $handlet) (local.get $c))
                (return)
              )
              (unreachable)
            )

            (func $a (export "a")
              (unreachable)
            )

            (func $entry2 (export "entry2")
              (suspend $t)
            )
        )
        "#;

        let mut config = Config::default();
        config.wasm_function_references(true);
        config.wasm_stack_switching(true);

        let engine = Engine::new(&config).unwrap();
        let mut store = Store::<()>::new(&engine, ());
        let module = Module::new(&engine, wat)?;

        let instance = Instance::new(&mut store, &module, &[])?;

        // We execute entry 1, which traps with (unreachable) while $a is running inside a continuation.
        let entry1 = instance.get_typed_func::<(), ()>(&mut store, "entry1")?;
        let result1 = entry1.call(&mut store, ());
        let err1 = result1.expect_err("Was expecting wasm execution to yield error");
        assert!(err1.root_cause().is::<Trap>());
        assert_eq!(
            *err1.downcast_ref::<Trap>().unwrap(),
            Trap::UnreachableCodeReached
        );
        let trace1 = backtrace_from_err(&err1);
        assert!(trace1.eq(["entry1", "a"].into_iter()));

        // Now we re-enter the instance and immediately suspend with tag $t.
        // This should trap, as there is no handler for it.
        // In particular, we must not try to use the handler for $t installed by $entry1.
        let entry2 = instance.get_typed_func::<(), ()>(&mut store, "entry2")?;
        let result2 = entry2.call(&mut store, ());
        let err2 = result2.unwrap_err();
        assert!(err2.root_cause().is::<Trap>());
        assert_eq!(*err2.downcast_ref::<Trap>().unwrap(), Trap::UnhandledTag);
        let trace2 = backtrace_from_err(&err2);
        assert!(trace2.eq(["entry2"].into_iter()));

        Ok(())
    }

    // Tests that we properly clean up the instance/store after trapping while
    // running inside a continuation:
    // This test is similar to `reuse_instance_after_trap1`, but here we don't
    // trap the second time we enter the instance.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn reuse_instance_after_trap2() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))

            (tag $t)

            (func $entry1 (export "entry1")
              (local $c (ref $ct))
              (local.set $c (cont.new $ct (ref.func $a)))
              (block $handlet (result (ref $ct))
                (resume $ct (on $t $handlet) (local.get $c))
                (return)
              )
              (unreachable)
            )

            (func $entry2 (export "entry2") (param $x i32) (result i32)
              (local $c (ref $ct))
              (local.set $c (cont.new $ct (ref.func $b)))
              (block $handlet (result (ref $ct))
                (resume $ct (on $t $handlet) (local.get $c))
                (unreachable)
              )
              ;; note that we don't run the continuation to completion.
              (drop)
              (i32.add (local.get $x) (i32.const 1))
            )

            (func $a (export "a")
              (unreachable)
            )

            (func $b (export "b")
              (suspend $t)
            )

        )
        "#;

        let mut config = Config::default();
        config.wasm_function_references(true);
        config.wasm_stack_switching(true);

        let engine = Engine::new(&config).unwrap();
        let mut store = Store::<()>::new(&engine, ());
        let module = Module::new(&engine, wat)?;

        let instance = Instance::new(&mut store, &module, &[])?;

        // We execute entry 1, which traps with (unreachable) while $a is running inside a continuation.
        let entry1 = instance.get_typed_func::<(), ()>(&mut store, "entry1")?;
        let result1 = entry1.call(&mut store, ());
        let err1 = result1.expect_err("Was expecting wasm execution to yield error");
        assert!(err1.root_cause().is::<Trap>());
        assert_eq!(
            *err1.downcast_ref::<Trap>().unwrap(),
            Trap::UnreachableCodeReached
        );
        let trace1 = backtrace_from_err(&err1);
        assert!(trace1.eq(["entry1", "a"].into_iter()));

        // Now we re-enter the instance and succesfully run some stack-switchy code.
        let entry1 = instance.get_typed_func::<i32, i32>(&mut store, "entry2")?;
        let result1 = entry1.call(&mut store, 122);
        let result_value = result1.unwrap();
        assert_eq!(result_value, 123);

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Tests that we get correct panic payloads if we panic deep inside multiple
    /// continuations. Note that wasmtime does not create its own backtraces for panics.
    fn panic_in_continuation() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func))
            (type $ct (cont $ft))

            (import "" "" (func $f))

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                (resume $ct (cont.new $ct (ref.func $b)))
            )

            (func $b (export "b")
                (call $c)
            )

            (func $c (export "c")
                (resume $ct (cont.new $ct (ref.func $d)))
            )

            (func $d (export "d")
                (call $e)
            )

            (func $e (export "e")
                (call $f)
            )

        )
        "#;

        let mut runner = Runner::new();

        let msg = "Host function f panics";

        let f = make_panicking_host_func(&mut runner.store, msg);
        let error = runner.run_test_expect_panic(wat, &[f.into()]);
        assert_eq!(error.downcast_ref::<&'static str>(), Some(&msg));

        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn stack_overflow_in_continuation() -> Result<()> {
        let wat = r#"
        (module
            (type $ft (func (param i32)))
            (type $ct (cont $ft))

            (func $entry (export "entry")
                (call $a)
            )

            (func $a (export "a")
                ;; We ask for a billion recursive calls
                (i32.const 1_000_000_000)

                (resume $ct (cont.new $ct (ref.func $overflow)))
            )

            (func $overflow (export "overflow") (param $i i32)
                (block $continue
                    (local.get $i)
                    ;; return if $i == 0
                    (br_if $continue)
                    (return)
                )
                (i32.sub (local.get $i) (i32.const 1))
                (call $overflow)
            )

        )
    "#;

        let runner = Runner::new();

        let error = runner
            .run_test::<()>(wat, &[])
            .expect_err("Expecting execution to yield error");

        assert!(error.root_cause().is::<Trap>());
        assert_eq!(*error.downcast_ref::<Trap>().unwrap(), Trap::StackOverflow);

        Ok(())
    }
}

mod misc {
    use super::test_utils::*;
    use wasmtime::*;

    #[ignore]
    #[test]
    pub fn continuation_revision_counter_wraparound() -> Result<()> {
        let wat = r#"
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (tag $yield)

  (func $loop
    (loop $loop
      (suspend $yield)
      (br $loop)
    )
  )
  (elem declare func $loop)

  ;; Loops 65536 times to overflow the 16 bit revision counter on the continuation reference.
  (func (export "entry")
    (local $k (ref $ct))
    (local $i i32)
    (local.set $k (cont.new $ct (ref.func $loop)))
    (loop $go-again
      (block $on-yield (result (ref $ct))
        (resume $ct (on $yield $on-yield) (local.get $k))
        (unreachable)
      )
      (local.set $k)
      (local.set $i (i32.add (i32.const 1) (local.get $i)))
      (br_if $go-again (i32.lt_u (local.get $i) (i32.const 65536)))
    )
  )
)
"#;

        let runner = Runner::new();
        let error = runner
            .run_test::<()>(wat, &[])
            .expect_err("expected an overflow");
        assert!(error.root_cause().is::<Trap>());
        assert_eq!(
            *error.downcast_ref::<Trap>().unwrap(),
            Trap::IntegerOverflow
        );
        Ok(())
    }
}

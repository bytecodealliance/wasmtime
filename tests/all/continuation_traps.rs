use wasmtime::*;

const MODULE: &str = r#"
    (module
        (import "host" "reenter" (func $host-reenter))
        (import "host" "trap" (func $host-trap))

        (type $ft (func))
        (type $ct (cont $ft))

        (global $stacks-left (mut i32) (i32.const 0))
        (global $frames-left (mut i32) (i32.const 0))
        (global $frames-per-stack (mut i32) (i32.const 0))
        (global $trap-in-host (mut i32) (i32.const 0))
        (global $g (export "g") (mut i32) (i32.const 0))

        (func $increment
            (global.set $g
                (i32.add (global.get $g) (i32.const 1))))

        (func $continue (export "continue")
            (if (i32.gt_u (global.get $frames-left) (i32.const 0))
                (then
                    (global.set $frames-left
                        (i32.sub (global.get $frames-left) (i32.const 1)))
                    (call $host-reenter))
                (else
                    (if (i32.gt_u (global.get $stacks-left) (i32.const 0))
                        (then
                            ;; Install a new child stack.
                            (global.set $stacks-left
                                (i32.sub (global.get $stacks-left) (i32.const 1)))
                            (global.set $frames-left (global.get $frames-per-stack))
                            (resume $ct (cont.new $ct (ref.func $continue))))
                        (else
                            ;; Call either the host provided trap, or the native Wasm trapping instruction `unreachable`.
                            (if (global.get $trap-in-host)
                                (then (call $host-trap))
                                (else unreachable))))))

            ;; No frame, on any stack, may resume after the terminal trap.
            (call $increment))

        (func (export "run")
            (param $stacks i32)
            (param $frames-per-stack-arg i32)
            (param $trap-in-host-arg i32)
            (global.set $stacks-left (local.get $stacks))
            (global.set $frames-left (local.get $frames-per-stack-arg))
            (global.set $frames-per-stack (local.get $frames-per-stack-arg))
            (global.set $trap-in-host (local.get $trap-in-host-arg))
            (call $continue))

        (elem declare func $continue)
    )
"#;

fn increment_global<T>(caller: &mut Caller<'_, T>) -> Result<()> {
    let g = caller.get_export("g").unwrap().into_global().unwrap();
    let value = g.get(&mut *caller).unwrap_i32();
    g.set(caller, Val::I32(value + 1))
}

fn assert_expected_trap(error: &Error, trap_in_host: bool, stacks: i32, frames_per_stack: i32) {
    if trap_in_host {
        assert!(
            format!("{error:#}").contains("intentional host trap"),
            "unexpected error for stacks={stacks}, frames_per_stack={frames_per_stack}: {error:#}"
        );
    } else {
        assert_eq!(
            error.downcast_ref::<Trap>(),
            Some(&Trap::UnreachableCodeReached),
            "unexpected error for stacks={stacks}, frames_per_stack={frames_per_stack}: {error:#}"
        );
    }
}

fn host_trap() -> Result<()> {
    bail!("intentional host trap")
}

// TODO(dhil): Enable ASAN. ASAN produces a false positive here,
// because ASAN thinks the thread is on the default stack. We need to
// instrument the stack switching runtime to inform ASAN about the
// switching of stacks.
#[test]
#[cfg_attr(any(asan, miri), ignore)]
fn traps_cross_continuation_stacks_and_host_frames() -> Result<()> {
    let mut config = Config::new();
    config.wasm_stack_switching(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, MODULE)?;

    // Exercise short continuation chains with different numbers of alternating
    // Wasm and host frames on each stack. Test both a Wasm trap and a host trap
    // as the youngest frame.
    for stacks in 0..=10 {
        for frames_per_stack in 0..=10 {
            for trap_in_host in [false, true] {
                let mut store = Store::new(&engine, ());
                let reenter = Func::wrap(&mut store, |mut caller: Caller<'_, ()>| -> Result<()> {
                    let continue_ = caller
                        .get_export("continue")
                        .unwrap()
                        .into_func()
                        .unwrap()
                        .typed::<(), ()>(&caller)?;
                    continue_.call(&mut caller, ())?;

                    // This must not execute: the recursive call always traps.
                    increment_global(&mut caller)
                });
                let host_trap = Func::wrap(&mut store, host_trap);
                let instance =
                    Instance::new(&mut store, &module, &[reenter.into(), host_trap.into()])?;

                let run = instance.get_typed_func::<(i32, i32, i32), ()>(&mut store, "run")?;
                let error = run
                    .call(
                        &mut store,
                        (stacks, frames_per_stack, i32::from(trap_in_host)),
                    )
                    .unwrap_err();

                assert_expected_trap(&error, trap_in_host, stacks, frames_per_stack);

                let g = instance.get_global(&mut store, "g").unwrap();
                assert_eq!(g.get(&mut store).unwrap_i32(), 0);
            }
        }
    }

    Ok(())
}

#[derive(Default)]
struct CatchState {
    error: Option<Error>,
}

// TODO(dhil): Enable ASAN.
#[test]
#[cfg_attr(any(asan, miri), ignore)]
fn parent_frames_resume_after_host_catches_trap() -> Result<()> {
    let mut config = Config::new();
    config.wasm_stack_switching(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, MODULE)?;

    for stacks in 0..=10 {
        // At least one host re-entry is needed to catch the terminal trap.
        for frames_per_stack in 1..=10 {
            for trap_in_host in [false, true] {
                let mut store = Store::new(&engine, CatchState::default());
                let reenter = Func::wrap(
                    &mut store,
                    |mut caller: Caller<'_, CatchState>| -> Result<()> {
                        let continue_ = caller
                            .get_export("continue")
                            .unwrap()
                            .into_func()
                            .unwrap()
                            .typed::<(), ()>(&caller)?;

                        if let Err(error) = continue_.call(&mut caller, ()) {
                            let previous = caller.data_mut().error.replace(error);
                            assert!(previous.is_none(), "more than one host frame caught a trap");
                        }

                        increment_global(&mut caller)
                    },
                );
                let host_trap = Func::wrap(&mut store, host_trap);
                let instance =
                    Instance::new(&mut store, &module, &[reenter.into(), host_trap.into()])?;

                let run = instance.get_typed_func::<(i32, i32, i32), ()>(&mut store, "run")?;
                run.call(
                    &mut store,
                    (stacks, frames_per_stack, i32::from(trap_in_host)),
                )?;

                let error = store
                    .data()
                    .error
                    .as_ref()
                    .expect("a host frame should have caught the terminal trap");
                assert_expected_trap(error, trap_in_host, stacks, frames_per_stack);

                // `stacks` counts child stacks, so there are `stacks + 1`
                // stacks in total. Each has `frames_per_stack` host frames and
                // `frames_per_stack + 1` Wasm frames. All frames except the
                // terminal trapping Wasm frame return and increment `g`.
                let expected = (stacks + 1) * (2 * frames_per_stack + 1) - 1;
                let g = instance.get_global(&mut store, "g").unwrap();
                assert_eq!(
                    g.get(&mut store).unwrap_i32(),
                    expected,
                    "wrong count for stacks={stacks}, frames_per_stack={frames_per_stack}"
                );
            }
        }
    }

    Ok(())
}

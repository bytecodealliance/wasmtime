use anyhow::Result;
use wasmtime::component::types::ComponentItem;
use wasmtime::component::{Component, Linker, Type};
use wasmtime::{Config, Engine, Module, Precompiled, Store};

#[tokio::test]
async fn threads() -> Result<()> {
    use std::io::IsTerminal;
    use tracing_subscriber::{EnvFilter, FmtSubscriber};
    let builder = FmtSubscriber::builder()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_env("WASMTIME_LOG"))
        .with_ansi(std::io::stderr().is_terminal())
        .init();
    let mut config = Config::new();
    config.async_support(true);
    config.wasm_component_model_async(true);
    config.wasm_component_model_threading(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_async_builtins(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#";;! component_model_async = true
;;! component_model_threading = true

;; Tests for basic functioning of all threading builtins with the implicit thread + one explicit thread
;; Switches between threads using all of the different threading intrinsics.

(component
    ;; Defines the table for the thread start function
    (core module $libc
        (table (export "__indirect_function_table") 1 funcref))
    ;; Defines the thread start function and a function that calls thread.new_indirect
    (core module $m
        ;; Import the threading builtins and the table from libc
        (import "" "thread.new_indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
        (import "" "thread.suspend" (func $thread-suspend (result i32)))
        (import "" "thread.yield-to" (func $thread-yield-to (param i32) (result i32)))
        (import "" "thread.switch-to" (func $thread-switch-to (param i32) (result i32)))
        (import "" "thread.yield" (func $thread-yield (result i32)))
        (import "" "thread.index" (func $thread-index (result i32)))
        (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
        (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))

        ;; A global that we will set from the spawned thread
        (global $g (mut i32) (i32.const 0))
        (global $main-thread-index (mut i32) (i32.const 0))

        ;; The thread entry point, which sets the global to incrementing values starting from the context value
        (func $thread-start (param i32)
            ;; Set the global to the context value
            (global.set $g (local.get 0))
            ;; The main thread switched to us, so is no longer scheduled, so we explicitly schedule it
            (call $thread-resume-later (global.get $main-thread-index))
            ;; Yield back to the main thread (since that is the only other one)
            (drop (call $thread-yield)
            ;; Increment the global
            (global.set $g (i32.add (global.get $g) (i32.const 1)))
            ;; The main thread will have explicitly requested suspension, so yield to it directly
            (drop (call $thread-yield-to (global.get $main-thread-index)))
            ;; Increment the global again
            (global.set $g (i32.add (global.get $g) (i32.const 1)))
            ;; Reschedule the main thread so that it runs after we exit
            (call $thread-resume-later (global.get $main-thread-index))))
        (export "thread-start" (func $thread-start))

        ;; Initialize the function table with our thread-start function; this will be
        ;; used by thread.new_indirect
        (elem (table $indirect-function-table) (i32.const 0) func $thread-start)

        ;; The main entry point, which spawns a new thread to run `thread-start`, passing 42
        ;; as the context value, and then yields to it
        (func (export "run") (result i32)
            ;; Store the main thread's index for the spawned thread to yield to
            (global.set $main-thread-index (call $thread-index))
            ;; Create a new thread, which starts suspended, and switch to it
            (drop 
                (call $thread-switch-to 
                    (call $thread-new-indirect (i32.const 0) (i32.const 42))))
            ;; After the thread yields back to us, check that the global was set to 42
            (if (i32.ne (global.get $g) (i32.const 42)) (then unreachable))
            ;; Suspend ourselves, which will cause the spawned thread to run
            (drop (call $thread-suspend))
            ;; The spawned thread will resume us after incrementing the global, so check that it is now 43
            (if (i32.ne (global.get $g) (i32.const 43)) (then unreachable))
            ;; Suspend again, which will cause the spawned thread to run again
            (drop (call $thread-suspend))
            ;; The spawned thread will reschedule us before it exits, so when we resume here the global should be 44
            (if (i32.ne (global.get $g) (i32.const 44)) (then unreachable))
            ;; Return success
            (i32.const 42)))
    
    ;; Instantiate the libc module to get the table
    (core instance $libc (instantiate $libc))
    ;; Get access to `thread.new_indirect` that uses the table from libc
    (core type $start-func-ty (func (param i32)))
    (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))

    (core func $thread-new-indirect 
        (canon thread.new_indirect $start-func-ty (table $indirect-function-table)))
    (core func $thread-yield (canon thread.yield))
    (core func $thread-index (canon thread.index))
    (core func $thread-yield-to (canon thread.yield-to))
    (core func $thread-resume-later (canon thread.resume-later))
    (core func $thread-switch-to (canon thread.switch-to))
    (core func $thread-suspend (canon thread.suspend))

    ;; Instantiate the main module
    (core instance $i (
        instantiate $m 
            (with "" (instance  
                (export "thread.new_indirect" (func $thread-new-indirect))
                (export "thread.index" (func $thread-index))
                (export "thread.yield-to" (func $thread-yield-to))
                (export "thread.yield" (func $thread-yield))
                (export "thread.switch-to" (func $thread-switch-to))
                (export "thread.suspend" (func $thread-suspend))
                (export "thread.resume-later" (func $thread-resume-later))))
            (with "libc" (instance $libc))))

    ;; Export the main entry point
    (func (export "run") (result u32) (canon lift (core func $i "run"))))
        "#,
    )?
    .serialize()?;

    let component = unsafe { Component::deserialize(&engine, &component)? };
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let func = instance.get_typed_func::<(), (u32,)>(&mut store, "run")?;
    assert_eq!(func.call_async(&mut store, ()).await?, (42,));

    Ok(())
}

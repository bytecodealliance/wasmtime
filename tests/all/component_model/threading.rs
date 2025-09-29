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
    let engine = Engine::new(&config)?;
    let component = Component::new(
        &engine,
        r#"
            (component
    ;; Defines the table for the thread start function
    (core module $libc
        (table (export "__indirect_function_table") 1 funcref))
    ;; Defines the thread start function and a function that calls thread.new_indirect
    (core module $m
        ;; Import the threading builtins and the table from libc
        (import "" "thread.new_indirect" (func $thread-new-indirect (param i32 i32) (result i32)))
        (import "" "thread.yield-to" (func $thread-yield-to (param i32) (result i32)))
        (import "" "thread.switch-to" (func $thread-switch-to (param i32) (result i32)))
        (import "" "thread.yield" (func $thread-yield (result i32)))
        (import "" "thread.resume-later" (func $thread-resume-later (param i32)))
        (import "libc" "__indirect_function_table" (table $indirect-function-table 1 funcref))

        ;; A global that we will set from the spawned thread
        (global $g (mut i32) (i32.const 0))

        ;; The thread entry point, which sets the global to the value passed in
        (func $thread-start (param i32)
            local.get 0
            global.set $g)
        (export "thread-start" (func $thread-start))

        ;; Initialize the function table with our thread-start function; this will be
        ;; used by thread.new_indirect
        (elem (table $indirect-function-table) (i32.const 0) func $thread-start)

        ;; The main entry point, which spawns a new thread to run `thread-start`, passing 42
        ;; as the context value, and then yields to it
        (func (export "run") (result i32)
            i32.const 0
            i32.const 42
            call $thread-new-indirect
            call $thread-yield-to
            drop
            global.get $g))
    
    ;; Instantiate the libc module to get the table
    (core instance $libc (instantiate $libc))
    ;; Get access to `thread.new_indirect` that uses the table from libc
    (core type $start-func-ty (func (param i32)))
    (alias core export $libc "__indirect_function_table" (core table $indirect-function-table))

    (core func $thread-new-indirect 
        (canon thread.new_indirect $start-func-ty (table $indirect-function-table)))
    (core func $thread-yield (canon thread.yield))
    (core func $thread-yield-to (canon thread.yield-to))
    (core func $thread-resume-later (canon thread.resume-later))
    (core func $thread-switch-to (canon thread.switch-to))

    ;; Instantiate the main module
    (core instance $i (
        instantiate $m 
            (with "" (instance
                (export "thread.new_indirect" (func $thread-new-indirect))
                (export "thread.yield-to" (func $thread-yield-to))
                (export "thread.yield" (func $thread-yield))
                (export "thread.switch-to" (func $thread-switch-to))
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

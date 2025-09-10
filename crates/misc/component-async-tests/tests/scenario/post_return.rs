use super::util::test_run;
use crate::scenario::util::{config, make_component};
use anyhow::Result;
use component_async_tests::{Ctx, sleep};
use std::sync::{Arc, Mutex};
use wasmtime::component::{Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;

mod sleep_post_return {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "sleep-post-return-callee",
        exports: { default: task_exit },
    });
}

// No-op function; we only test this by composing it in `async_post_return_caller`
#[allow(
    dead_code,
    reason = "here only to make the `assert_test_exists` macro happy"
)]
pub fn async_post_return_callee() {}

#[tokio::test]
pub async fn async_post_return_caller() -> Result<()> {
    test_run(&[
        test_programs_artifacts::ASYNC_POST_RETURN_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_POST_RETURN_CALLEE_COMPONENT,
    ])
    .await
}

#[tokio::test]
pub async fn async_sleep_post_return_caller() -> Result<()> {
    test_sleep_post_return(&[
        test_programs_artifacts::ASYNC_SLEEP_POST_RETURN_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_SLEEP_POST_RETURN_CALLEE_COMPONENT,
    ])
    .await
}

#[tokio::test]
pub async fn async_sleep_post_return_callee() -> Result<()> {
    test_sleep_post_return(&[test_programs_artifacts::ASYNC_SLEEP_POST_RETURN_CALLEE_COMPONENT])
        .await
}

async fn test_sleep_post_return(components: &[&str]) -> Result<()> {
    let engine = Engine::new(&config())?;

    let component = make_component(&engine, components).await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    sleep::local::local::sleep::add_to_linker::<_, Ctx>(&mut linker, |ctx| ctx)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
            wakers: Arc::new(Mutex::new(None)),
        },
    );

    let instance = linker.instantiate_async(&mut store, &component).await?;
    let guest = sleep_post_return::SleepPostReturnCallee::new(&mut store, &instance)?;
    instance
        .run_concurrent(&mut store, async |accessor| {
            // This function should return immediately, then sleep the specified
            // number of milliseconds after returning, and then finally exit.
            let exit = guest
                .local_local_sleep_post_return()
                .call_run(accessor, 100)
                .await?
                .1;
            // The function has returned, now we wait for it (and any subtasks
            // it may have spawned) to exit.
            exit.block(accessor).await;
            anyhow::Ok(())
        })
        .await??;
    // At this point, all subtasks should have exited, meaning no waitables,
    // tasks, or other concurrent state should remain present in the instance.
    instance.assert_concurrent_state_empty(&mut store);
    Ok(())
}

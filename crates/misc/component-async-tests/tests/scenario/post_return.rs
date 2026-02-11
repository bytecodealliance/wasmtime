use super::util::test_run;
use crate::scenario::util::{config, make_component};
use component_async_tests::util;
use component_async_tests::{Ctx, yield_};
use std::future;
use std::pin::pin;
use std::task::Poll;
use wasmtime::Result;
use wasmtime::component::{Accessor, Linker, ResourceTable};
use wasmtime::{AsContextMut, Engine, Store, StoreContextMut};
use wasmtime_wasi::WasiCtxBuilder;

mod yield_post_return {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "yield-post-return-callee",
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
pub async fn async_yield_post_return_caller() -> Result<()> {
    test_yield_post_return(&[
        test_programs_artifacts::ASYNC_YIELD_POST_RETURN_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_YIELD_POST_RETURN_CALLEE_COMPONENT,
    ])
    .await
}

#[tokio::test]
pub async fn async_yield_post_return_callee() -> Result<()> {
    test_yield_post_return(&[test_programs_artifacts::ASYNC_YIELD_POST_RETURN_CALLEE_COMPONENT])
        .await
}

async fn test_yield_post_return(components: &[&str]) -> Result<()> {
    let engine = Engine::new(&config())?;

    let component = make_component(&engine, components).await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    yield_::local::local::yield_::add_to_linker::<_, Ctx>(&mut linker, |ctx| ctx)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
        },
    );

    let guest = yield_post_return::YieldPostReturnCallee::instantiate_async(
        &mut store, &component, &linker,
    )
    .await?;

    async fn run_with(
        accessor: &Accessor<Ctx>,
        guest: &yield_post_return::YieldPostReturnCallee,
    ) -> Result<()> {
        // This function should return immediately, then yield the specified
        // number of times after returning, and then finally exit.
        let exit = guest
            .local_local_yield_post_return()
            .call_run(accessor, 100)
            .await?
            .1;
        // The function has returned, now we wait for it (and any subtasks
        // it may have spawned) to exit.
        exit.block(accessor).await;
        wasmtime::error::Ok(())
    }

    async fn run(
        store: StoreContextMut<'_, Ctx>,
        guest: &yield_post_return::YieldPostReturnCallee,
    ) -> Result<()> {
        store
            .run_concurrent(async |accessor| {
                run_with(accessor, guest).await?;

                // Go idle for a bit before doing it again.  This tests that
                // `StoreContextMut::run_concurrent` is okay with having no
                // outstanding guest or host tasks to poll for a while, trusting
                // that we'll resolve the future independently, with or without
                // giving it more work to do.
                util::yield_times(100).await;

                run_with(accessor, guest).await?;

                wasmtime::error::Ok(())
            })
            .await?
    }

    run(store.as_context_mut(), &guest).await?;
    // At this point, all subtasks should have exited, meaning no waitables,
    // tasks, or other concurrent state should remain present in the store.
    store.assert_concurrent_state_empty();

    // Do it again, but this time cancel the event loop before it exits:
    assert!(
        future::poll_fn(|cx| Poll::Ready(pin!(run(store.as_context_mut(), &guest)).poll(cx)))
            .await
            .is_pending()
    );

    // Assuming the event loop is cancel-safe, this should complete without
    // errors or panics:
    run(store.as_context_mut(), &guest).await?;

    Ok(())
}

use component_async_tests::Ctx;
use std::{
    env, future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use wasmtime::{
    Engine, Result, Store,
    component::{Linker, ResourceTable},
};
use wasmtime_wasi::WasiCtxBuilder;

use super::util::{config, make_component, test_run};

mod callee {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "backpressure-callee",
        exports: { default: async | store },
    });
}

#[tokio::test]
pub async fn async_backpressure_callee() -> Result<()> {
    let mut config = config();
    // As of this writing, miri/pulley/epochs is a problematic combination, so
    // we don't test it.
    if env::var_os("MIRI_TEST_CWASM_DIR").is_none() {
        config.epoch_interruption(true);
    }

    let engine = Engine::new(&config)?;
    let component = make_component(
        &engine,
        &[test_programs_artifacts::ASYNC_BACKPRESSURE_CALLEE_COMPONENT],
    )
    .await?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
        },
    );

    if env::var_os("MIRI_TEST_CWASM_DIR").is_none() {
        store.set_epoch_deadline(1);

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(10));
            engine.increment_epoch();
        });
    }

    let guest =
        callee::BackpressureCallee::instantiate_async(&mut store, &component, &linker).await?;

    store
        .run_concurrent(async |accessor| {
            guest
                .local_local_backpressure()
                .call_inc_then_later_dec_backpressure(accessor)
                .await?;

            let func = *guest.local_local_run().func_run().func();

            let mut a = Some(Box::pin(guest.local_local_run().call_run(accessor)));
            let mut b = Some(Box::pin(guest.local_local_run().call_run(accessor)));
            let mut c = Some(Box::pin(guest.local_local_run().call_run(accessor)));

            let mut backpressure_is_set = true;
            future::poll_fn(move |cx| {
                let instance_ready = accessor.poll_ready_for_concurrent_call(func, cx).is_ready();
                let a_ready = is_ready(cx, &mut a);
                let b_ready = is_ready(cx, &mut b);
                let c_ready = is_ready(cx, &mut c);

                if backpressure_is_set {
                    assert!(!instance_ready);
                    assert!(!a_ready);
                    assert!(!b_ready);
                    assert!(!c_ready);

                    backpressure_is_set = false;

                    Poll::Pending
                } else if instance_ready && a_ready && b_ready && c_ready {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;

            wasmtime::error::Ok(())
        })
        .await??;

    Ok(())
}

fn is_ready(cx: &mut Context, fut: &mut Option<Pin<Box<impl Future>>>) -> bool {
    if let Some(v) = fut.as_mut() {
        if v.as_mut().poll(cx).is_ready() {
            *fut = None;
            true
        } else {
            false
        }
    } else {
        true
    }
}

#[tokio::test]
pub async fn async_backpressure_caller() -> Result<()> {
    test_run(&[
        test_programs_artifacts::ASYNC_BACKPRESSURE_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_BACKPRESSURE_CALLEE_COMPONENT,
    ])
    .await
}

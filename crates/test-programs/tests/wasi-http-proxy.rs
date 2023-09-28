#![cfg(all(feature = "test_programs", not(skip_wasi_http_tests)))]
use anyhow::Context;
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::{
    self, pipe::MemoryOutputPipe, IsATTY, Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi_http::{proxy::Proxy, WasiHttpCtx, WasiHttpView};

lazy_static::lazy_static! {
    static ref ENGINE: Engine = {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_component_model(true);
        config.async_support(true);
        let engine = Engine::new(&config).unwrap();
        engine
    };
}
// uses ENGINE, creates a fn get_module(&str) -> Module
include!(concat!(
    env!("OUT_DIR"),
    "/wasi_http_proxy_tests_components.rs"
));

struct Ctx {
    table: Table,
    wasi: WasiCtx,
    http: WasiHttpCtx,
}

impl WasiView for Ctx {
    fn table(&self) -> &Table {
        &self.table
    }
    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }
    fn ctx(&self) -> &WasiCtx {
        &self.wasi
    }
    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl WasiHttpView for Ctx {
    fn table(&mut self) -> &mut Table {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }
}

async fn instantiate(component: Component, ctx: Ctx) -> Result<(Store<Ctx>, Proxy), anyhow::Error> {
    let mut linker = Linker::new(&ENGINE);
    wasmtime_wasi_http::proxy::add_to_linker(&mut linker)?;

    let mut store = Store::new(&ENGINE, ctx);

    let (proxy, _instance) = Proxy::instantiate_async(&mut store, &component, &linker).await?;
    Ok((store, proxy))
}

#[test_log::test(tokio::test)]
async fn wasi_http_proxy_tests() -> anyhow::Result<()> {
    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);

    let mut table = Table::new();
    let component = get_component("wasi_http_proxy_tests");

    // Create our wasi context.
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone(), IsATTY::No);
    builder.stderr(stderr.clone(), IsATTY::No);
    for (var, val) in test_programs::wasi_tests_environment() {
        builder.env(var, val);
    }
    let wasi = builder.build(&mut table)?;
    let http = WasiHttpCtx;

    let ctx = Ctx { table, wasi, http };

    let (mut store, proxy) = instantiate(component, ctx).await?;

    let req = {
        use http_body_util::{BodyExt, Empty};

        let req = hyper::Request::builder().method(http::Method::GET).body(
            Empty::<bytes::Bytes>::new()
                .map_err(|e| anyhow::anyhow!(e))
                .boxed(),
        )?;
        store.data_mut().new_incoming_request(req)?
    };

    let (sender, receiver) = tokio::sync::oneshot::channel();
    let out = store.data_mut().new_response_outparam(sender)?;

    let handle = preview2::spawn(async move {
        proxy
            .wasi_http_incoming_handler()
            .call_handle(&mut store, req, out)
            .await?;

        Ok::<_, anyhow::Error>(())
    });

    let resp = match receiver.await {
        Ok(Ok(resp)) => {
            use http_body_util::BodyExt;
            let (parts, body) = resp.into_parts();
            let collected = BodyExt::collect(body).await?;
            Ok(hyper::Response::from_parts(parts, collected))
        }

        Ok(Err(e)) => Err(e),

        // This happens if the wasm never calls `set-response-outparam`
        Err(e) => panic!("Failed to receive a response: {e:?}"),
    };

    // Now that the response has been processed, we can wait on the wasm to finish without
    // deadlocking.
    handle.await.context("Component execution")?;

    let stdout = stdout.contents();
    if !stdout.is_empty() {
        println!("[guest] stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
    }
    let stderr = stderr.contents();
    if !stderr.is_empty() {
        println!("[guest] stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
    }

    match resp {
        Ok(resp) => println!("response: {resp:?}"),
        Err(e) => panic!("Error given in response: {e:?}"),
    };

    Ok(())
}

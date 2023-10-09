use crate::http_server::Server;
use anyhow::{Context, Result};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::{pipe::MemoryOutputPipe, Table, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

mod http_server;

struct Ctx {
    table: Table,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
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
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut Table {
        &mut self.table
    }
}

fn store(engine: &Engine, server: &Server) -> Store<Ctx> {
    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);

    // Create our wasi context.
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.stderr(stderr.clone());
    builder.env("HTTP_SERVER", server.addr().to_string());
    let ctx = Ctx {
        table: Table::new(),
        wasi: builder.build(),
        http: WasiHttpCtx {},
        stderr,
        stdout,
    };

    Store::new(&engine, ctx)
}

impl Drop for Ctx {
    fn drop(&mut self) {
        let stdout = self.stdout.contents();
        if !stdout.is_empty() {
            println!("[guest] stdout:\n{}\n===", String::from_utf8_lossy(&stdout));
        }
        let stderr = self.stderr.contents();
        if !stderr.is_empty() {
            println!("[guest] stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
        }
    }
}

// Assert that each of `sync` and `async` below are testing everything through
// assertion of the existence of the test function itself.
macro_rules! assert_test_exists {
    ($name:ident) => {
        #[allow(unused_imports)]
        use self::$name as _;
    };
}

mod async_;
mod sync;

#[test_log::test(tokio::test)]
async fn wasi_http_proxy_tests() -> anyhow::Result<()> {
    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);
    let table = Table::new();

    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, test_programs_artifacts::API_PROXY_COMPONENT)?;

    // Create our wasi context.
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.stderr(stderr.clone());
    let wasi = builder.build();
    let http = WasiHttpCtx;
    let ctx = Ctx {
        table,
        wasi,
        http,
        stderr,
        stdout,
    };
    let mut store = Store::new(&engine, ctx);

    let mut linker = Linker::new(&engine);
    wasmtime_wasi_http::proxy::add_to_linker(&mut linker)?;
    let (proxy, _) =
        wasmtime_wasi_http::proxy::Proxy::instantiate_async(&mut store, &component, &linker)
            .await?;

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

    let handle = wasmtime_wasi::preview2::spawn(async move {
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

    match resp {
        Ok(resp) => println!("response: {resp:?}"),
        Err(e) => panic!("Error given in response: {e:?}"),
    };

    Ok(())
}

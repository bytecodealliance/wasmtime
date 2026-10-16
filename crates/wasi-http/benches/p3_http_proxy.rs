//! Benchmark the `p3_http_proxy` test program against a host which answers
//! every outgoing request with a fixed short response.

use bytes::Bytes;
use criterion::{Criterion, criterion_group, criterion_main};
use http_body_util::{BodyExt, Empty, Full};
use std::time::Instant;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Result, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::p3::Request;
use wasmtime_wasi_http::p3::bindings::{Service, ServicePre};
use wasmtime_wasi_http::{
    Error, RequestOptions, WasiBody, WasiHttpCtx, WasiHttpCtxView, WasiHttpHooks, WasiHttpView,
};

criterion_group!(benches, bench_p3_http_proxy);
criterion_main!(benches);

const RESPONSE_BODY: &[u8] = b"hello";

/// Hooks which never touch the network: every outgoing request is answered
/// with a `200 OK` carrying `RESPONSE_BODY`.
struct FixedResponseHooks;

impl WasiHttpHooks for FixedResponseHooks {
    fn send_request(
        &mut self,
        request: http::Request<WasiBody>,
        _options: Option<RequestOptions>,
        _fut: Box<dyn Future<Output = Result<(), Error>> + Send>,
    ) -> Box<
        dyn Future<
                Output = Result<
                    (
                        http::Response<WasiBody>,
                        Box<dyn Future<Output = Result<(), Error>> + Send>,
                    ),
                    Error,
                >,
            > + Send,
    > {
        drop(request);
        Box::new(async {
            let body = Full::new(Bytes::from_static(RESPONSE_BODY))
                .map_err(|never| match never {})
                .boxed_unsync();
            Ok((
                http::Response::new(body),
                Box::new(async { Ok(()) }) as Box<dyn Future<Output = _> + Send>,
            ))
        })
    }
}

struct Ctx {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    hooks: FixedResponseHooks,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for Ctx {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: &mut self.hooks,
        }
    }
}

fn bench_p3_http_proxy(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_component_model_async(true);
        // Emit symbols for profiling.
        config.profiler(wasmtime::ProfilingStrategy::PerfMap);
    });
    let component =
        Component::from_file(&engine, test_programs_artifacts::P3_HTTP_PROXY_COMPONENT).unwrap();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker).unwrap();
    wasmtime_wasi::p3::add_to_linker(&mut linker).unwrap();
    wasmtime_wasi_http::p3::add_to_linker(&mut linker).unwrap();
    let pre = ServicePre::new(linker.instantiate_pre(&component).unwrap()).unwrap();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("wasi-http/p3_http_proxy", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async {
                // Instantiate once per batch, outside of the timed region, and
                // reuse the same instance for every request in the batch.
                let mut store = Store::new(
                    &engine,
                    Ctx {
                        table: ResourceTable::default(),
                        wasi: WasiCtx::builder().build(),
                        http: WasiHttpCtx::new(),
                        hooks: FixedResponseHooks,
                    },
                );
                let service = pre.instantiate_async(&mut store).await.unwrap();

                let start = Instant::now();
                for _ in 0..iters {
                    proxy_one_request(&mut store, &service).await.unwrap();
                }
                let elapsed = start.elapsed();

                // Explicit tear down after the timer has stopped.
                drop(store);
                elapsed
            })
        })
    });
}

/// Send a single request to the proxy and read the full response.
async fn proxy_one_request(store: &mut Store<Ctx>, service: &Service) -> Result<()> {
    let req = http::Request::builder()
        .uri("http://localhost/")
        .method(http::Method::GET)
        .header("url", "http://example.com/")
        .body(Empty::<Bytes>::new())?;
    let (req, io) = Request::from_http(&mut store.data_mut().hooks, req);
    let body = store
        .run_concurrent(async |store| {
            let (body, ()) = futures::try_join!(
                async {
                    let res = service
                        .handle(store, req)
                        .await?
                        .map_err(|e| wasmtime::format_err!("guest returned error: {e:?}"))?;
                    let res = store.with(|store| res.into_http(store, async { Ok(()) }))?;
                    assert_eq!(res.status(), http::StatusCode::OK);
                    let body = res.into_body().collect().await?.to_bytes();
                    wasmtime::error::Ok(body)
                },
                async { Ok(io.await?) },
            )?;
            wasmtime::error::Ok(body)
        })
        .await??;

    assert_eq!(body.as_ref(), RESPONSE_BODY);
    Ok(())
}

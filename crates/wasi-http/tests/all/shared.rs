use crate::http_server::Server;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Result, Store};
use wasmtime_wasi::{NamedId, WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::{
    WasiHttpCtx, WasiHttpCtxView, WasiHttpHooks, WasiHttpNamedView, WasiHttpView,
};

pub struct NamedImportsData {
    wasi: WasiCtx,
    http: WasiHttpCtx,
    table: ResourceTable,
    a: NamedImportsHooks,
    b: NamedImportsHooks,
}

struct NamedImportsHooks(&'static str);

impl WasiView for NamedImportsData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for NamedImportsData {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: Default::default(),
        }
    }
}

impl WasiHttpNamedView for NamedImportsData {
    fn http(&mut self, id: NamedId) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: match id.0 {
                0 => &mut self.a,
                1 => &mut self.b,
                _ => panic!("unexpected id: {}", id.0),
            },
        }
    }
}

impl WasiHttpHooks for NamedImportsHooks {
    fn forbidden_header_filter(&mut self) -> wasmtime_wasi_http::ForbiddenHeaderFilter {
        let forbidden = self.0;
        std::sync::Arc::new(move |name| name == forbidden)
    }
}

pub async fn run_named_imports_test<T>(
    component: &str,
    add_only_http_to_linker: fn(&mut Linker<NamedImportsData>) -> Result<()>,
    add_named_to_linker: fn(
        &mut Linker<NamedImportsData>,
        &Component,
        fn(T, &str) -> Result<NamedId>,
    ) -> Result<()>,
    run: impl AsyncFnOnce(
        &mut Store<NamedImportsData>,
        &Component,
        &Linker<NamedImportsData>,
    ) -> Result<()>,
) -> Result<()> {
    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_component_model_implements(true);
    });
    let component = Component::from_file(&engine, component)?;
    let mut store = Store::new(
        &engine,
        NamedImportsData {
            wasi: WasiCtx::builder().inherit_stdio().build(),
            table: ResourceTable::new(),
            http: Default::default(),
            a: NamedImportsHooks("a"),
            b: NamedImportsHooks("b"),
        },
    );
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    add_only_http_to_linker(&mut linker)?;
    add_named_to_linker(&mut linker, &component, |_, name| {
        Ok(match name {
            "a" => NamedId(0),
            "b" => NamedId(1),
            _ => panic!("unexpected name: {name}"),
        })
    })?;
    run(&mut store, &component, &linker).await
}

pub fn ambiguous_responses() -> &'static [&'static [u8]] {
    &[
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTRANSFER-ENCODING: Chunked\r\nContent-Length: 5\r\n\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\nContent-Length: 5\r\n\r\nAAAAA",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTransfer-Encoding: gzip\r\nContent-Length: 5\r\n\r\n0\r\n\r\n",
    ]
}

/// Runs a guest which fetches `/dup` from a raw server, whose response
/// carries two `content-length` values, and then forwards those
/// headers in a new request to `/victim`, which the host must reject.
///
/// `run_guest` instantiates and runs the guest against the given server; the
/// guest itself asserts the error it observes. Afterwards this asserts that
/// nothing but the guest's `/dup` requests ever reached the wire.
pub async fn forward_duplicate_content_length(
    run_guest: impl AsyncFnOnce(&Server) -> Result<()>,
) -> Result<()> {
    // `hyper` forwards both headers to the guest, and the guest must not be
    // able to forward them into a new request.
    const DUP: &[u8] = b"HTTP/1.1 200 OK\r\n\
        content-length: 5\r\n\
        content-length: 5\r\n\
        \r\n\
        AAAAA";
    // The guest fetches `/dup` twice, and must never send `/victim`; the
    // extra connections let a `/victim` request be observed should it
    // escape.
    let (server, requests) = Server::raw(4, |head| {
        if head.starts_with(b"GET /dup ") {
            DUP.to_vec()
        } else {
            b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n".to_vec()
        }
    })?;
    run_guest(&server).await?;
    drop(server);

    let requests = requests.get();
    assert_eq!(requests.len(), 2, "unexpected requests: {requests:?}");
    for head in &requests {
        assert!(
            head.starts_with(b"GET /dup "),
            "unexpected request reached the wire: {}",
            String::from_utf8_lossy(head)
        );
    }
    Ok(())
}

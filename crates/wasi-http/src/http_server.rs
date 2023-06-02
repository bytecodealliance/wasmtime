use crate::wasi::http::types::Method;
use crate::WasiHttp;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use wasmtime::{AsContext, AsContextMut, Store};

use crate::r#struct::ActiveRequest;

struct Host {
    wasi_http: WasiHttp,
    wasi: wasmtime_wasi::WasiCtx,
}

impl Host {
    pub fn new() -> Self {
        Self {
            wasi_http: WasiHttp::new(),
            wasi: wasmtime_wasi::WasiCtxBuilder::new()
                .stdin(Box::new(wasmtime_wasi::stdio::stdin()))
                .build(),
        }
    }
}

#[derive(Clone, Copy)]
struct HttpHandler<'a> {
    engine: &'a wasmtime::Engine,
    module: &'a PathBuf,
}

impl hyper::service::Service<Request<Incoming>> for HttpHandler<'_> {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        let mut host = Host::new();
        let ptr = host.wasi_http.request_id_base;
        host.wasi_http.request_id_base += 1;

        let mut request = ActiveRequest::new(ptr);
        request.method = Method::new(req.method());
        request.scheme = match req.uri().scheme() {
            Some(s) => Some(s.into()),
            None => None,
        };
        request.authority = match req.uri().authority() {
            Some(s) => s.to_string(),
            None => "".to_string(),
        };
        request.path_with_query =
            req.uri().path().to_string() + "?" + req.uri().query().unwrap_or("");

        for (name, value) in req.headers().iter() {
            let val = value.to_str().unwrap().to_string();
            let key = name.to_string();
            match request.headers.get_mut(&key) {
                Some(vec) => vec.push(val.into()),
                None => {
                    let mut vec = std::vec::Vec::new();
                    vec.push(val.into_bytes());
                    request.headers.insert(key.to_string(), vec);
                }
            }
        }

        host.wasi_http.requests.insert(ptr, request);
        let outparam_id = host.wasi_http.outparams_id_base;
        host.wasi_http.outparams_id_base += 1;

        let mut linker = wasmtime::Linker::new(self.engine);
        let mut store = Store::new(self.engine, host);
        let path = self.module.clone();

        Box::pin(async move {
            wasmtime_wasi::tokio::add_to_linker(&mut linker, |h: &mut Host| &mut h.wasi).unwrap();
            crate::add_to_linker(&mut linker, |h: &mut Host| &mut h.wasi_http, true).unwrap();

            let module = wasmtime::Module::from_file(&store.engine(), path).unwrap();
            let instance = linker.instantiate_async(&mut store, &module).await.unwrap();
            let func = instance
                .get_func(&mut store, "wasi:http/incoming-handler#handle")
                .unwrap();
            let typed = func.typed::<(u32, u32), ()>(store.as_context()).unwrap();
            typed
                .call_async(store.as_context_mut(), (ptr, outparam_id))
                .await
                .unwrap();

            let host = store.data_mut();
            let response_id = host
                .wasi_http
                .response_outparams
                .get(&outparam_id)
                .unwrap()
                .unwrap();
            let response = host.wasi_http.responses.get(&response_id).unwrap();
            let body = Full::<Bytes>::new(
                host.wasi_http
                    .streams
                    .entry(response.body)
                    .or_default()
                    .into(),
            );
            let code = StatusCode::from_u16(response.status).unwrap();
            let res = Ok(Response::builder().status(code).body(body).unwrap());
            res
        })
    }
}

// adapted from https://docs.rs/hyper/1.0.0-rc.3/hyper/server/conn/index.html
pub async fn async_http_server(engine: &wasmtime::Engine, module: &PathBuf) {
    let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();

    let tcp_listener = TcpListener::bind(addr).await.unwrap();
    let server: HttpHandler = HttpHandler {
        engine: engine,
        module: module,
    };

    loop {
        let (tcp_stream, _) = tcp_listener.accept().await.unwrap();
        if let Err(http_err) = http1::Builder::new()
            .keep_alive(true)
            .serve_connection(tcp_stream, server)
            .await
        {
            eprintln!("Error while serving HTTP connection: {}", http_err);
        }
    }
}

pub fn spawn_http_server(engine: &wasmtime::Engine, module: &PathBuf) {
    let (handle, _runtime) = match tokio::runtime::Handle::try_current() {
        Ok(h) => (h, None),
        Err(_) => {
            let rt = Runtime::new().unwrap();
            let _enter = rt.enter();
            (rt.handle().clone(), Some(rt))
        }
    };

    handle.block_on(async_http_server(engine, module))
}

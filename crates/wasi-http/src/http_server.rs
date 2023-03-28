use crate::types::Method;
use crate::WasiHttp;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use wasmtime::{AsContext, AsContextMut, Store};

use crate::r#struct::ActiveRequest;

struct HttpHandler<'a, T> {
    pub linker: Option<&'a wasmtime::Linker<T>>,
    pub store: Option<&'a mut wasmtime::Store<T>>,
    pub get_cx: Box<dyn Fn(&mut T) -> &mut WasiHttp>,
}

impl<T> hyper::service::Service<Request<Incoming>> for HttpHandler<'_, T> {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        let mut wasi_http = (self.get_cx)(self.store.as_mut().unwrap().data_mut());
        let ptr = wasi_http.request_id_base;
        wasi_http.request_id_base += 1;

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
        request.path = req.uri().path().to_string();

        for (name, value) in req.headers().iter() {
            let val = value.to_str().unwrap().to_string();
            let key = name.to_string();
            match request.headers.get_mut(&key) {
                Some(vec) => vec.push(val),
                None => {
                    let mut vec = std::vec::Vec::new();
                    vec.push(val);
                    request.headers.insert(key.to_string(), vec);
                }
            }
        }

        wasi_http.requests.insert(ptr, request);
        let outparam_id = wasi_http.outparams_id_base;
        wasi_http.outparams_id_base += 1;

        let http = self
            .linker
            .as_ref()
            .unwrap()
            .get(&mut *self.store.as_mut().unwrap(), "", "HTTP#handle")
            .unwrap();
        let func = http.into_func().unwrap();
        let typed = func
            .typed::<(u32, u32), ()>(self.store.as_mut().unwrap().as_context())
            .unwrap();
        typed
            .call(
                self.store.as_mut().unwrap().as_context_mut(),
                (ptr, outparam_id),
            )
            .unwrap();

        wasi_http = (self.get_cx)(self.store.as_mut().unwrap().data_mut());
        let response_id = wasi_http
            .response_outparams
            .get(&outparam_id)
            .unwrap()
            .unwrap();
        let response = wasi_http.responses.get(&response_id).unwrap();
        let body = Full::<Bytes>::new(wasi_http.streams.entry(response.body).or_default().into());
        let code = StatusCode::from_u16(response.status).unwrap();
        let res = Ok(Response::builder().status(code).body(body).unwrap());
        Box::pin(async { res })
    }
}

// adapted from https://docs.rs/hyper/1.0.0-rc.3/hyper/server/conn/index.html
pub async fn async_http_server<T>(
    linker: &mut wasmtime::Linker<T>,
    store: &mut Store<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiHttp + Send + Sync + Copy + 'static,
) {
    let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();

    let tcp_listener = TcpListener::bind(addr).await.unwrap();
    loop {
        let server: HttpHandler<T> = HttpHandler {
            linker: Some(linker),
            store: Some(store),
            get_cx: Box::new(get_cx),
        };

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

pub fn spawn_http_server<T>(
    linker: &mut wasmtime::Linker<T>,
    wasi_http: &mut Store<T>,
    get_cx: impl Fn(&mut T) -> &mut WasiHttp + Send + Sync + Copy + 'static,
) {
    let rt = Runtime::new().unwrap();
    let _enter = rt.enter();

    rt.block_on(async_http_server(linker, wasi_http, get_cx))
}

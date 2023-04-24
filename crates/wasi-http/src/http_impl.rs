use crate::r#struct::{ActiveResponse, WasiHttp};
use crate::types::{RequestOptions, Scheme};
use anyhow::{bail, Context};
use bytes::{BufMut, BytesMut};
use http::Uri;
use http_body_util::{BodyExt, Full};
use hyper::Method;
use hyper::Request;
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::timeout;

impl crate::default_outgoing_http::Host for WasiHttp {
    fn handle(
        &mut self,
        request_id: crate::default_outgoing_http::OutgoingRequest,
        options: Option<crate::default_outgoing_http::RequestOptions>,
    ) -> wasmtime::Result<crate::default_outgoing_http::FutureIncomingResponse> {
        // TODO: Initialize this once?
        let rt = Runtime::new().unwrap();
        let _enter = rt.enter();

        let f = self.handle_async(request_id, options);
        match rt.block_on(f) {
            Ok(r) => {
                println!("{} OK", r);
                Ok(r)
            }
            Err(e) => {
                println!("{} ERR", e);
                Err(e)
            }
        }
    }
}

impl WasiHttp {
    async fn handle_async(
        &mut self,
        request_id: crate::default_outgoing_http::OutgoingRequest,
        options: Option<crate::default_outgoing_http::RequestOptions>,
    ) -> wasmtime::Result<crate::default_outgoing_http::FutureIncomingResponse> {
        let request = match self.requests.get(&request_id) {
            Some(r) => r,
            None => bail!("not found!"),
        };

        let method = match request.method {
            crate::types::Method::Get => Method::GET,
            crate::types::Method::Head => Method::HEAD,
            crate::types::Method::Post => Method::POST,
            crate::types::Method::Put => Method::PUT,
            crate::types::Method::Delete => Method::DELETE,
            crate::types::Method::Connect => Method::CONNECT,
            crate::types::Method::Options => Method::OPTIONS,
            crate::types::Method::Trace => Method::TRACE,
            crate::types::Method::Patch => Method::PATCH,
            _ => bail!("unknown method!"),
        };
        let mut uri = Uri::builder()
            .authority(request.authority.as_str())
            // NOTE: this is broken, but will be fixed by `wasi-http` dependency update
            .path_and_query(request.path.to_owned() + &request.query);
        match &request.scheme {
            Some(Scheme::Http) => uri = uri.scheme("http"),
            Some(Scheme::Https) => uri = uri.scheme("https"),
            Some(scheme) => bail!("unsupported scheme `{scheme:?}`"),
            _ => {}
        }
        // NOTE: This does not belong here, the complete struct should have been constructed
        // on request creation
        let uri = uri.build().context("failed to build URI")?;

        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header(hyper::header::HOST, &request.authority);
        for (key, val) in request.headers.iter() {
            for item in val {
                req = req.header(key, item.clone());
            }
        }
        let body = self
            .streams
            .get(&request.body)
            .map(|stream| stream.clone().into())
            .unwrap_or_default();
        let req = req.body(Full::new(body))?;

        let connect_timeout = if let Some(RequestOptions {
            connect_timeout_ms: Some(connect_timeout_ms),
            ..
        }) = options
        {
            Duration::from_millis(connect_timeout_ms.into())
        } else {
            // TODO: Configurable default
            Duration::from_millis(600)
        };

        let first_byte_timeout = if let Some(RequestOptions {
            first_byte_timeout_ms: Some(first_byte_timeout_ms),
            ..
        }) = options
        {
            Duration::from_millis(first_byte_timeout_ms.into())
        } else {
            // TODO: Configurable default
            Duration::from_millis(600)
        };

        let res = self
            .outgoing_handler
            .handle(req, connect_timeout, first_byte_timeout)
            .await?;

        let response_id = self.response_id_base;
        self.response_id_base = self.response_id_base + 1;
        let mut response = ActiveResponse::new(response_id);
        response.status = res.status().try_into()?;
        for (key, value) in res.headers().iter() {
            let mut vec = std::vec::Vec::new();
            vec.push(value.to_str()?.to_string());
            response
                .response_headers
                .insert(key.as_str().to_string(), vec);
        }

        let between_bytes_timeout = if let Some(RequestOptions {
            between_bytes_timeout_ms: Some(between_bytes_timeout_ms),
            ..
        }) = options
        {
            Duration::from_millis(between_bytes_timeout_ms.into())
        } else {
            // TODO: Configurable default
            Duration::from_millis(600)
        };
        let body = res.into_body();
        let mut body = body.lock().await;
        let mut buf = BytesMut::new();
        while let Some(next) = timeout(between_bytes_timeout, body.frame()).await? {
            let frame = next?;
            if let Some(chunk) = frame.data_ref() {
                buf.put(chunk.clone());
            }
            if let Some(trailers) = frame.trailers_ref() {
                response.trailers = self.fields_id_base;
                self.fields_id_base += 1;
                let mut map: HashMap<String, Vec<String>> = HashMap::new();
                for (name, value) in trailers.iter() {
                    let key = name.to_string();
                    match map.get_mut(&key) {
                        Some(vec) => vec.push(value.to_str()?.to_string()),
                        None => {
                            let mut vec = Vec::new();
                            vec.push(value.to_str()?.to_string());
                            map.insert(key, vec);
                        }
                    };
                }
                self.fields.insert(response.trailers, map);
            }
        }
        response.body = self.streams_id_base;
        self.streams_id_base = self.streams_id_base + 1;
        self.streams.insert(response.body, buf.freeze().into());
        self.responses.insert(response_id, response);
        Ok(response_id)
    }
}

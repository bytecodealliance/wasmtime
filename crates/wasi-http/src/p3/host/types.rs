use super::{
    delete_fields, delete_request, delete_response, get_fields, get_fields_mut, get_request,
    get_request_mut, get_response, get_response_mut, push_fields, push_request, push_response,
};
use crate::p3::bindings::clocks::monotonic_clock::Duration;
use crate::p3::bindings::http::types::{
    ErrorCode, FieldName, FieldValue, Fields, HeaderError, Headers, Host, HostFields, HostRequest,
    HostRequestOptions, HostRequestWithStore, HostResponse, HostResponseWithStore, Method, Request,
    RequestOptions, RequestOptionsError, Response, Scheme, StatusCode, Trailers,
};
use crate::p3::body::Body;
use crate::p3::{HttpError, WasiHttp, WasiHttpCtxView};
use anyhow::Context as _;
use bytes::Bytes;
use core::mem;
use core::num::NonZeroUsize;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use http::header::CONTENT_LENGTH;
use http_body::Body as _;
use http_body_util::combinators::BoxBody;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::oneshot;
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Accessor, Destination, FutureProducer, FutureReader, Resource, StreamProducer, StreamReader,
    StreamResult,
};
use wasmtime_wasi::ResourceTable;
use wasmtime_wasi::p3::FutureOneshotProducer;
use wasmtime_wasi::p3::StreamEmptyProducer;

fn get_request_options<'a>(
    table: &'a ResourceTable,
    opts: &Resource<RequestOptions>,
) -> wasmtime::Result<&'a RequestOptions> {
    table
        .get(opts)
        .context("failed to get request options from table")
}

fn get_request_options_mut<'a>(
    table: &'a mut ResourceTable,
    opts: &Resource<RequestOptions>,
) -> wasmtime::Result<&'a mut RequestOptions> {
    table
        .get_mut(opts)
        .context("failed to get request options from table")
}

fn push_request_options(
    table: &mut ResourceTable,
    opts: RequestOptions,
) -> wasmtime::Result<Resource<RequestOptions>> {
    table
        .push(opts)
        .context("failed to push request options to table")
}

fn delete_request_options(
    table: &mut ResourceTable,
    opts: Resource<RequestOptions>,
) -> wasmtime::Result<RequestOptions> {
    table
        .delete(opts)
        .context("failed to delete request options from table")
}

fn parse_header_value(
    name: &http::HeaderName,
    value: impl AsRef<[u8]>,
) -> Result<http::HeaderValue, HeaderError> {
    if name == CONTENT_LENGTH {
        let s = str::from_utf8(value.as_ref()).or(Err(HeaderError::InvalidSyntax))?;
        let v: u64 = s.parse().or(Err(HeaderError::InvalidSyntax))?;
        Ok(v.into())
    } else {
        http::HeaderValue::from_bytes(value.as_ref()).or(Err(HeaderError::InvalidSyntax))
    }
}

struct GuestBodyResultProducer(
    oneshot::Receiver<Box<dyn Future<Output = Result<(), ErrorCode>> + Send>>,
);

impl<D> FutureProducer<D> for GuestBodyResultProducer {
    type Item = Result<(), ErrorCode>;

    async fn produce(self, _: &Accessor<D>) -> wasmtime::Result<Self::Item> {
        let Ok(fut) = self.0.await else {
            return Ok(Ok(()));
        };
        Ok(Box::into_pin(fut).await)
    }
}

struct HostBodyStreamProducer<T> {
    body: BoxBody<Bytes, ErrorCode>,
    trailers: Option<oneshot::Sender<Result<Option<Resource<Trailers>>, ErrorCode>>>,
    getter: for<'a> fn(&'a mut T) -> WasiHttpCtxView<'a>,
}

impl<T> Drop for HostBodyStreamProducer<T> {
    fn drop(&mut self) {
        self.close(Ok(None))
    }
}

impl<T> HostBodyStreamProducer<T> {
    fn close(&mut self, res: Result<Option<Resource<Trailers>>, ErrorCode>) {
        if let Some(tx) = self.trailers.take() {
            _ = tx.send(res);
        }
    }
}

impl<D> StreamProducer<D> for HostBodyStreamProducer<D>
where
    D: 'static,
{
    type Item = u8;
    type Buffer = Cursor<Bytes>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let res = 'result: {
            let cap = match dst.remaining(&mut store).map(NonZeroUsize::new) {
                Some(Some(cap)) => Some(cap),
                Some(None) => {
                    if self.body.is_end_stream() {
                        break 'result Ok(None);
                    } else {
                        return Poll::Ready(Ok(StreamResult::Completed));
                    }
                }
                None => None,
            };
            match Pin::new(&mut self.body).poll_frame(cx) {
                Poll::Ready(Some(Ok(frame))) => {
                    match frame.into_data().map_err(http_body::Frame::into_trailers) {
                        Ok(mut frame) => {
                            if let Some(cap) = cap {
                                let n = frame.len();
                                let cap = cap.into();
                                if n > cap {
                                    dst.set_buffer(Cursor::new(frame.split_off(cap)));
                                    let mut dst = dst.as_direct(store, cap);
                                    dst.remaining().copy_from_slice(&frame);
                                    dst.mark_written(cap);
                                } else {
                                    let mut dst = dst.as_direct(store, n);
                                    dst.remaining()[..n].copy_from_slice(&frame);
                                    dst.mark_written(n);
                                }
                            } else {
                                dst.set_buffer(Cursor::new(frame));
                            }
                            return Poll::Ready(Ok(StreamResult::Completed));
                        }
                        Err(Ok(trailers)) => {
                            let trailers = push_fields(
                                (self.getter)(store.data_mut()).table,
                                Fields::new_mutable(trailers),
                            )?;
                            break 'result Ok(Some(trailers));
                        }
                        Err(Err(..)) => break 'result Err(ErrorCode::HttpProtocolError),
                    }
                }
                Poll::Ready(Some(Err(err))) => break 'result Err(err),
                Poll::Ready(None) => break 'result Ok(None),
                Poll::Pending if finish => return Poll::Ready(Ok(StreamResult::Cancelled)),
                Poll::Pending => return Poll::Pending,
            }
        };
        self.close(res);
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

impl HostFields for WasiHttpCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<Fields>> {
        push_fields(self.table, Fields::new_mutable_default())
    }

    fn from_list(
        &mut self,
        entries: Vec<(FieldName, FieldValue)>,
    ) -> wasmtime::Result<Result<Resource<Fields>, HeaderError>> {
        let mut fields = http::HeaderMap::default();
        for (name, value) in entries {
            let Ok(name) = name.parse() else {
                return Ok(Err(HeaderError::InvalidSyntax));
            };
            if self.ctx.is_forbidden_header(&name) {
                return Ok(Err(HeaderError::Forbidden));
            }
            match parse_header_value(&name, value) {
                Ok(value) => {
                    fields.append(name, value);
                }
                Err(err) => return Ok(Err(err)),
            }
        }
        let fields = push_fields(self.table, Fields::new_mutable(fields))?;
        Ok(Ok(fields))
    }

    fn get(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
    ) -> wasmtime::Result<Vec<FieldValue>> {
        let fields = get_fields(self.table, &fields)?;
        Ok(fields
            .get_all(name)
            .into_iter()
            .map(|val| val.as_bytes().into())
            .collect())
    }

    fn has(&mut self, fields: Resource<Fields>, name: FieldName) -> wasmtime::Result<bool> {
        let fields = get_fields(self.table, &fields)?;
        Ok(fields.contains_key(name))
    }

    fn set(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
        value: Vec<FieldValue>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        let Ok(name) = name.parse() else {
            return Ok(Err(HeaderError::InvalidSyntax));
        };
        if self.ctx.is_forbidden_header(&name) {
            return Ok(Err(HeaderError::Forbidden));
        }
        let mut values = Vec::with_capacity(value.len());
        for value in value {
            match parse_header_value(&name, value) {
                Ok(value) => {
                    values.push(value);
                }
                Err(err) => return Ok(Err(err)),
            }
        }
        let fields = get_fields_mut(self.table, &fields)?;
        let Some(fields) = fields.get_mut() else {
            return Ok(Err(HeaderError::Immutable));
        };
        fields.remove(&name);
        for value in values {
            fields.append(&name, value);
        }
        Ok(Ok(()))
    }

    fn delete(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        let header = match http::HeaderName::from_bytes(name.as_bytes()) {
            Ok(header) => header,
            Err(_) => return Ok(Err(HeaderError::InvalidSyntax)),
        };
        if self.ctx.is_forbidden_header(&header) {
            return Ok(Err(HeaderError::Forbidden));
        }
        let fields = get_fields_mut(self.table, &fields)?;
        let Some(fields) = fields.get_mut() else {
            return Ok(Err(HeaderError::Immutable));
        };
        fields.remove(&name);
        Ok(Ok(()))
    }

    fn get_and_delete(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
    ) -> wasmtime::Result<Result<Vec<FieldValue>, HeaderError>> {
        let Ok(header) = http::header::HeaderName::from_bytes(name.as_bytes()) else {
            return Ok(Err(HeaderError::InvalidSyntax));
        };
        if self.ctx.is_forbidden_header(&header) {
            return Ok(Err(HeaderError::Forbidden));
        }
        let fields = get_fields_mut(self.table, &fields)?;
        let Some(fields) = fields.get_mut() else {
            return Ok(Err(HeaderError::Immutable));
        };
        let http::header::Entry::Occupied(entry) = fields.entry(header) else {
            return Ok(Ok(vec![]));
        };
        let (.., values) = entry.remove_entry_mult();
        Ok(Ok(values.map(|header| header.as_bytes().into()).collect()))
    }

    fn append(
        &mut self,
        fields: Resource<Fields>,
        name: FieldName,
        value: FieldValue,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        let Ok(name) = name.parse() else {
            return Ok(Err(HeaderError::InvalidSyntax));
        };
        if self.ctx.is_forbidden_header(&name) {
            return Ok(Err(HeaderError::Forbidden));
        }
        let value = match parse_header_value(&name, value) {
            Ok(value) => value,
            Err(err) => return Ok(Err(err)),
        };
        let fields = get_fields_mut(self.table, &fields)?;
        let Some(fields) = fields.get_mut() else {
            return Ok(Err(HeaderError::Immutable));
        };
        fields.append(name, value);
        Ok(Ok(()))
    }

    fn copy_all(
        &mut self,
        fields: Resource<Fields>,
    ) -> wasmtime::Result<Vec<(FieldName, FieldValue)>> {
        let fields = get_fields(self.table, &fields)?;
        let fields = fields
            .iter()
            .map(|(name, value)| (name.as_str().into(), value.as_bytes().into()))
            .collect();
        Ok(fields)
    }

    fn clone(&mut self, fields: Resource<Fields>) -> wasmtime::Result<Resource<Fields>> {
        let fields = get_fields(self.table, &fields)?;
        push_fields(self.table, Fields::new_mutable(Arc::clone(fields)))
    }

    fn drop(&mut self, fields: Resource<Fields>) -> wasmtime::Result<()> {
        delete_fields(self.table, fields)?;
        Ok(())
    }
}

impl HostRequestWithStore for WasiHttp {
    async fn new<T>(
        store: &Accessor<T, Self>,
        headers: Resource<Headers>,
        contents: Option<StreamReader<u8>>,
        trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
        options: Option<Resource<RequestOptions>>,
    ) -> wasmtime::Result<(Resource<Request>, FutureReader<Result<(), ErrorCode>>)> {
        let instance = store.instance();
        store.with(|mut store| {
            let (result_tx, result_rx) = oneshot::channel();
            let WasiHttpCtxView { table, .. } = store.get();
            let headers = delete_fields(table, headers)?;
            let options = options
                .map(|options| delete_request_options(table, options))
                .transpose()?;
            let body = Body::Guest {
                contents_rx: contents,
                trailers_rx: trailers,
                result_tx,
            };
            let req = Request {
                method: http::Method::GET,
                scheme: None,
                authority: None,
                path_with_query: None,
                headers: headers.into(),
                options: options.map(Into::into),
                body,
            };
            let req = push_request(table, req)?;
            Ok((
                req,
                FutureReader::new(instance, &mut store, GuestBodyResultProducer(result_rx)),
            ))
        })
    }

    async fn consume_body<T>(
        store: &Accessor<T, Self>,
        req: Resource<Request>,
    ) -> wasmtime::Result<
        Result<
            (
                StreamReader<u8>,
                FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
            ),
            (),
        >,
    > {
        let getter = store.getter();
        store.with(|mut store| {
            let req = get_request_mut(store.get().table, &req)?;
            match mem::replace(&mut req.body, Body::Consumed) {
                Body::Guest {
                    contents_rx: Some(contents_rx),
                    trailers_rx,
                    result_tx,
                } => {
                    // TODO: Use a result specified by the caller
                    // https://github.com/WebAssembly/wasi-http/issues/176
                    _ = result_tx.send(Box::new(async { Ok(()) }));
                    Ok(Ok((contents_rx, trailers_rx)))
                }
                Body::Guest {
                    contents_rx: None,
                    trailers_rx,
                    result_tx,
                } => {
                    let instance = store.instance();
                    // TODO: Use a result specified by the caller
                    // https://github.com/WebAssembly/wasi-http/issues/176
                    _ = result_tx.send(Box::new(async { Ok(()) }));
                    Ok(Ok((
                        StreamReader::new(instance, &mut store, StreamEmptyProducer::default()),
                        trailers_rx,
                    )))
                }
                Body::Host { body, result_tx } => {
                    let instance = store.instance();
                    // TODO: Use a result specified by the caller
                    // https://github.com/WebAssembly/wasi-http/issues/176
                    _ = result_tx.send(Box::new(async { Ok(()) }));
                    let (trailers_tx, trailers_rx) = oneshot::channel();
                    Ok(Ok((
                        StreamReader::new(
                            instance,
                            &mut store,
                            HostBodyStreamProducer {
                                body,
                                trailers: Some(trailers_tx),
                                getter,
                            },
                        ),
                        FutureReader::new(instance, &mut store, FutureOneshotProducer(trailers_rx)),
                    )))
                }
                Body::Consumed => Ok(Err(())),
            }
        })
    }
}

impl HostRequest for WasiHttpCtxView<'_> {
    fn get_method(&mut self, req: Resource<Request>) -> wasmtime::Result<Method> {
        let Request { method, .. } = get_request(self.table, &req)?;
        Ok(method.into())
    }

    fn set_method(
        &mut self,
        req: Resource<Request>,
        method: Method,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = get_request_mut(self.table, &req)?;
        let Ok(method) = method.try_into() else {
            return Ok(Err(()));
        };
        req.method = method;
        Ok(Ok(()))
    }

    fn get_path_with_query(&mut self, req: Resource<Request>) -> wasmtime::Result<Option<String>> {
        let Request {
            path_with_query, ..
        } = get_request(self.table, &req)?;
        Ok(path_with_query.as_ref().map(|pq| pq.as_str().into()))
    }

    fn set_path_with_query(
        &mut self,
        req: Resource<Request>,
        path_with_query: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = get_request_mut(self.table, &req)?;
        let Some(path_with_query) = path_with_query else {
            req.path_with_query = None;
            return Ok(Ok(()));
        };
        let Ok(path_with_query) = path_with_query.try_into() else {
            return Ok(Err(()));
        };
        req.path_with_query = Some(path_with_query);
        Ok(Ok(()))
    }

    fn get_scheme(&mut self, req: Resource<Request>) -> wasmtime::Result<Option<Scheme>> {
        let Request { scheme, .. } = get_request(self.table, &req)?;
        Ok(scheme.as_ref().map(Into::into))
    }

    fn set_scheme(
        &mut self,
        req: Resource<Request>,
        scheme: Option<Scheme>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = get_request_mut(self.table, &req)?;
        let Some(scheme) = scheme else {
            req.scheme = None;
            return Ok(Ok(()));
        };
        let Ok(scheme) = scheme.try_into() else {
            return Ok(Err(()));
        };
        req.scheme = Some(scheme);
        Ok(Ok(()))
    }

    fn get_authority(&mut self, req: Resource<Request>) -> wasmtime::Result<Option<String>> {
        let Request { authority, .. } = get_request(self.table, &req)?;
        Ok(authority.as_ref().map(|auth| auth.as_str().into()))
    }

    fn set_authority(
        &mut self,
        req: Resource<Request>,
        authority: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        let req = get_request_mut(self.table, &req)?;
        let Some(authority) = authority else {
            req.authority = None;
            return Ok(Ok(()));
        };
        let has_port = authority.contains(':');
        let Ok(authority) = http::uri::Authority::try_from(authority) else {
            return Ok(Err(()));
        };
        if has_port && authority.port_u16().is_none() {
            return Ok(Err(()));
        }
        req.authority = Some(authority);
        Ok(Ok(()))
    }

    fn get_options(
        &mut self,
        req: Resource<Request>,
    ) -> wasmtime::Result<Option<Resource<RequestOptions>>> {
        let Request { options, .. } = get_request(self.table, &req)?;
        if let Some(options) = options {
            let options = push_request_options(
                self.table,
                RequestOptions::new_immutable(Arc::clone(options)),
            )?;
            Ok(Some(options))
        } else {
            Ok(None)
        }
    }

    fn get_headers(&mut self, req: Resource<Request>) -> wasmtime::Result<Resource<Headers>> {
        let Request { headers, .. } = get_request(self.table, &req)?;
        push_fields(self.table, Fields::new_immutable(Arc::clone(headers)))
    }

    fn drop(&mut self, req: Resource<Request>) -> wasmtime::Result<()> {
        delete_request(self.table, req)?;
        Ok(())
    }
}

impl HostRequestOptions for WasiHttpCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<RequestOptions>> {
        push_request_options(self.table, RequestOptions::new_mutable_default())
    }

    fn get_connect_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        let opts = get_request_options(self.table, &opts)?;
        let Some(connect_timeout) = opts.connect_timeout else {
            return Ok(None);
        };
        let ns = connect_timeout.as_nanos();
        let ns = ns
            .try_into()
            .context("connect timeout duration nanoseconds do not fit in u64")?;
        Ok(Some(ns))
    }

    fn set_connect_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), RequestOptionsError>> {
        let opts = get_request_options_mut(self.table, &opts)?;
        let Some(opts) = opts.get_mut() else {
            return Ok(Err(RequestOptionsError::Immutable));
        };
        opts.connect_timeout = duration.map(core::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn get_first_byte_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        let opts = get_request_options(self.table, &opts)?;
        let Some(first_byte_timeout) = opts.first_byte_timeout else {
            return Ok(None);
        };
        let ns = first_byte_timeout.as_nanos();
        let ns = ns
            .try_into()
            .context("first byte timeout duration nanoseconds do not fit in u64")?;
        Ok(Some(ns))
    }

    fn set_first_byte_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), RequestOptionsError>> {
        let opts = get_request_options_mut(self.table, &opts)?;
        let Some(opts) = opts.get_mut() else {
            return Ok(Err(RequestOptionsError::Immutable));
        };
        opts.first_byte_timeout = duration.map(core::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn get_between_bytes_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<Duration>> {
        let opts = get_request_options(self.table, &opts)?;
        let Some(between_bytes_timeout) = opts.between_bytes_timeout else {
            return Ok(None);
        };
        let ns = between_bytes_timeout.as_nanos();
        let ns = ns
            .try_into()
            .context("between bytes timeout duration nanoseconds do not fit in u64")?;
        Ok(Some(ns))
    }

    fn set_between_bytes_timeout(
        &mut self,
        opts: Resource<RequestOptions>,
        duration: Option<Duration>,
    ) -> wasmtime::Result<Result<(), RequestOptionsError>> {
        let opts = get_request_options_mut(self.table, &opts)?;
        let Some(opts) = opts.get_mut() else {
            return Ok(Err(RequestOptionsError::Immutable));
        };
        opts.between_bytes_timeout = duration.map(core::time::Duration::from_nanos);
        Ok(Ok(()))
    }

    fn clone(
        &mut self,
        opts: Resource<RequestOptions>,
    ) -> wasmtime::Result<Resource<RequestOptions>> {
        let opts = get_request_options(self.table, &opts)?;
        push_request_options(self.table, RequestOptions::new_mutable(Arc::clone(opts)))
    }

    fn drop(&mut self, opts: Resource<RequestOptions>) -> wasmtime::Result<()> {
        delete_request_options(self.table, opts)?;
        Ok(())
    }
}

impl HostResponseWithStore for WasiHttp {
    async fn new<T>(
        store: &Accessor<T, Self>,
        headers: Resource<Headers>,
        contents: Option<StreamReader<u8>>,
        trailers: FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
    ) -> wasmtime::Result<(Resource<Response>, FutureReader<Result<(), ErrorCode>>)> {
        let instance = store.instance();
        store.with(|mut store| {
            let (result_tx, result_rx) = oneshot::channel();
            let WasiHttpCtxView { table, .. } = store.get();
            let headers = delete_fields(table, headers)?;
            let body = Body::Guest {
                contents_rx: contents,
                trailers_rx: trailers,
                result_tx,
            };
            let res = Response {
                status: http::StatusCode::OK,
                headers: headers.into(),
                body,
            };
            let res = push_response(table, res)?;
            Ok((
                res,
                FutureReader::new(instance, &mut store, GuestBodyResultProducer(result_rx)),
            ))
        })
    }

    async fn consume_body<T>(
        store: &Accessor<T, Self>,
        res: Resource<Response>,
    ) -> wasmtime::Result<
        Result<
            (
                StreamReader<u8>,
                FutureReader<Result<Option<Resource<Trailers>>, ErrorCode>>,
            ),
            (),
        >,
    > {
        let getter = store.getter();
        store.with(|mut store| {
            let res = get_response_mut(store.get().table, &res)?;
            match mem::replace(&mut res.body, Body::Consumed) {
                Body::Guest {
                    contents_rx: Some(contents_rx),
                    trailers_rx,
                    result_tx,
                } => {
                    // TODO: Use a result specified by the caller
                    // https://github.com/WebAssembly/wasi-http/issues/176
                    _ = result_tx.send(Box::new(async { Ok(()) }));
                    Ok(Ok((contents_rx, trailers_rx)))
                }
                Body::Guest {
                    contents_rx: None,
                    trailers_rx,
                    result_tx,
                } => {
                    let instance = store.instance();
                    // TODO: Use a result specified by the caller
                    // https://github.com/WebAssembly/wasi-http/issues/176
                    _ = result_tx.send(Box::new(async { Ok(()) }));
                    Ok(Ok((
                        StreamReader::new(instance, &mut store, StreamEmptyProducer::default()),
                        trailers_rx,
                    )))
                }
                Body::Host { body, result_tx } => {
                    let instance = store.instance();
                    // TODO: Use a result specified by the caller
                    // https://github.com/WebAssembly/wasi-http/issues/176
                    _ = result_tx.send(Box::new(async { Ok(()) }));
                    let (trailers_tx, trailers_rx) = oneshot::channel();
                    Ok(Ok((
                        StreamReader::new(
                            instance,
                            &mut store,
                            HostBodyStreamProducer {
                                body,
                                trailers: Some(trailers_tx),
                                getter,
                            },
                        ),
                        FutureReader::new(instance, &mut store, FutureOneshotProducer(trailers_rx)),
                    )))
                }
                Body::Consumed => Ok(Err(())),
            }
        })
    }
}

impl HostResponse for WasiHttpCtxView<'_> {
    fn get_status_code(&mut self, res: Resource<Response>) -> wasmtime::Result<StatusCode> {
        let res = get_response(self.table, &res)?;
        Ok(res.status.into())
    }

    fn set_status_code(
        &mut self,
        res: Resource<Response>,
        status_code: StatusCode,
    ) -> wasmtime::Result<Result<(), ()>> {
        let res = get_response_mut(self.table, &res)?;
        let Ok(status) = http::StatusCode::from_u16(status_code) else {
            return Ok(Err(()));
        };
        res.status = status;
        Ok(Ok(()))
    }

    fn get_headers(&mut self, res: Resource<Response>) -> wasmtime::Result<Resource<Headers>> {
        let Response { headers, .. } = get_response(self.table, &res)?;
        push_fields(self.table, Fields::new_immutable(Arc::clone(headers)))
    }

    fn drop(&mut self, res: Resource<Response>) -> wasmtime::Result<()> {
        delete_response(self.table, res)?;
        Ok(())
    }
}

impl Host for WasiHttpCtxView<'_> {
    fn convert_error_code(&mut self, error: HttpError) -> wasmtime::Result<ErrorCode> {
        error.downcast()
    }
}

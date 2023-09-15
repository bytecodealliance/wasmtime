//! Implements the base structure (i.e. [WasiHttpCtx]) that will provide the
//! implementation of the wasi-http API.

use crate::bindings::http::types::{Headers, Method, Scheme};
use bytes::Bytes;
use std::collections::HashMap;
use std::pin::Pin;
use std::task;
use wasmtime_wasi::preview2::{
    self, pipe::AsyncReadStream, AbortOnDropJoinHandle, HostInputStream, StreamState, Table,
    TableError,
};

const MAX_BUF_SIZE: usize = 65_536;

/// Capture the state necessary for use in the wasi-http API implementation.
pub struct WasiHttpCtx;

pub trait WasiHttpView: Send {
    fn ctx(&mut self) -> &mut WasiHttpCtx;
    fn table(&mut self) -> &mut Table;
}

pub type FieldsMap = HashMap<String, Vec<Vec<u8>>>;

pub struct HostOutgoingRequest {
    pub method: Method,
    pub scheme: Option<Scheme>,
    pub path_with_query: String,
    pub authority: String,
    pub headers: HostFields,
    pub body: Option<AsyncReadStream>,
}

// TODO:
//
// 1. HostIncomingBody needs a new method to spawn a task and return two things:
//   a. a channel that streams `Bytes` out for the response body
//   b. a one-shot channel that writes the trailer fields out
// 2. In incoming_response_consume, we need to consume channel 1.a, sticking it in the table as
//    something that implementes HostInputStream
// 3. In either of the trailers methods, we need to consume 1.b and decide the return value based
//    on its state.
//
// The method defined in 1 needs to be called in both 2 and 3 if either encounter the unspawned
// task state.

pub struct HostIncomingResponse {
    pub status: u16,
    pub headers: HeadersRef,
    pub body: Option<Box<HostIncomingBody>>,
}

pub enum HeadersRef {
    Value(hyper::HeaderMap),
    Resource(Headers),
}

pub type DataReceiver = tokio::sync::mpsc::Receiver<bytes::Bytes>;

pub type HostFutureTrailers = tokio::sync::oneshot::Receiver<hyper::HeaderMap>;

pub struct HostIncomingBody {
    pub body: hyper::body::Incoming,
    pub worker: AbortOnDropJoinHandle<anyhow::Result<()>>,
}

impl HostIncomingBody {
    pub fn new(
        body: hyper::body::Incoming,
        worker: AbortOnDropJoinHandle<anyhow::Result<()>>,
    ) -> Self {
        Self { body, worker }
    }

    /// Consume the state held in the [`HostIncomingBody`] to spawn a task that will drive the
    /// streaming body to completion. Data segments will be communicated out over the
    /// [`DataReceiver`] channel, and a [`HostFutureTrailers`] gives a way to block on/retrieve the
    /// trailers.
    pub fn spawn(
        mut self,
    ) -> (
        AbortOnDropJoinHandle<anyhow::Result<()>>,
        DataReceiver,
        HostFutureTrailers,
    ) {
        use hyper::body::{Body, Frame};

        struct FrameFut<'a> {
            body: &'a mut hyper::body::Incoming,
        }

        impl<'a> FrameFut<'a> {
            fn new(body: &'a mut hyper::body::Incoming) -> Self {
                Self { body }
            }
        }

        impl<'a> std::future::Future for FrameFut<'a> {
            type Output = Option<Result<Frame<bytes::Bytes>, hyper::Error>>;

            fn poll(
                mut self: Pin<&mut Self>,
                cx: &mut task::Context<'_>,
            ) -> task::Poll<Self::Output> {
                if self.body.is_end_stream() {
                    return task::Poll::Ready(None);
                }

                Pin::new(&mut self.body).poll_frame(cx)
            }
        }

        let (writer, reader) = tokio::sync::mpsc::channel(1);
        let (trailer_writer, trailer_reader) = tokio::sync::oneshot::channel();

        let handle = preview2::spawn(async move {
            while let Some(frame) = FrameFut::new(&mut self.body).await {
                // TODO: we need to actually handle errors here, right now we'll exit the loop
                // early without signaling properly to either channel that we're done.
                let frame = frame?;

                if frame.is_trailers() {
                    // We know we're not going to write any more data frames at this point, so we
                    // explicitly drop the writer so that anything waiting on the read end returns
                    // immediately.
                    drop(writer);

                    let trailers = frame.into_trailers().unwrap();

                    // TODO: this will fail in two cases:
                    // 1. we've already used the channel once, which should be imposible,
                    // 2. the read end is closed.
                    // I'm not sure how to differentiate between these two cases, or really
                    // if we need to do anything to handle either.
                    let _ = trailer_writer.send(trailers);

                    break;
                }

                assert!(frame.is_data());

                let data = frame.into_data().unwrap();

                // TODO: we need to handle send errors here. In particular, if we fail to write
                // because the reader has been dropped, we need to continue around the loop to
                // drain data frames so that we can ultimately deliver the trailers.
                let _ = writer.send(data);
            }

            Ok(())
        });

        (handle, reader, trailer_reader)
    }
}

#[async_trait::async_trait]
impl HostInputStream for HostIncomingBody {
    fn read(&mut self, _size: usize) -> anyhow::Result<(Bytes, StreamState)> {
        todo!()
    }

    async fn ready(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct HostFields(pub HashMap<String, Vec<Vec<u8>>>);

impl HostFields {
    pub fn new() -> Self {
        Self(FieldsMap::new())
    }
}

impl From<hyper::HeaderMap> for HostFields {
    fn from(headers: hyper::HeaderMap) -> Self {
        use std::collections::hash_map::Entry;

        let mut res: HashMap<String, Vec<Vec<u8>>> = HashMap::new();

        for (k, v) in headers.iter() {
            let v = v.as_bytes().to_vec();
            match res.entry(k.as_str().to_string()) {
                Entry::Occupied(mut vs) => vs.get_mut().push(v),
                Entry::Vacant(e) => {
                    e.insert(vec![v]);
                }
            }
        }

        Self(res)
    }
}

pub struct IncomingResponseInternal {
    pub resp: hyper::Response<hyper::body::Incoming>,
    pub worker: AbortOnDropJoinHandle<anyhow::Result<()>>,
}

type FutureIncomingResponseHandle = AbortOnDropJoinHandle<anyhow::Result<IncomingResponseInternal>>;

pub enum HostFutureIncomingResponse {
    Pending(FutureIncomingResponseHandle),
    Ready(anyhow::Result<IncomingResponseInternal>),
}

impl HostFutureIncomingResponse {
    pub fn new(handle: FutureIncomingResponseHandle) -> Self {
        Self::Pending(handle)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    pub fn unwrap_ready(self) -> anyhow::Result<IncomingResponseInternal> {
        match self {
            Self::Ready(res) => res,
            Self::Pending(_) => {
                panic!("unwrap_ready called on a pending HostFutureIncomingResponse")
            }
        }
    }
}

impl std::future::Future for HostFutureIncomingResponse {
    type Output = anyhow::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let s = self.get_mut();
        match s {
            Self::Pending(ref mut handle) => match Pin::new(handle).poll(cx) {
                task::Poll::Pending => task::Poll::Pending,
                task::Poll::Ready(r) => {
                    *s = Self::Ready(r);
                    task::Poll::Ready(Ok(()))
                }
            },

            Self::Ready(_) => task::Poll::Ready(Ok(())),
        }
    }
}

#[async_trait::async_trait]
pub trait TableHttpExt {
    fn push_outgoing_response(&mut self, request: HostOutgoingRequest) -> Result<u32, TableError>;
    fn get_outgoing_request(&self, id: u32) -> Result<&HostOutgoingRequest, TableError>;
    fn get_outgoing_request_mut(&mut self, id: u32)
        -> Result<&mut HostOutgoingRequest, TableError>;
    fn delete_outgoing_request(&mut self, id: u32) -> Result<HostOutgoingRequest, TableError>;

    fn push_incoming_response(&mut self, response: HostIncomingResponse)
        -> Result<u32, TableError>;
    fn get_incoming_response(&self, id: u32) -> Result<&HostIncomingResponse, TableError>;
    fn get_incoming_response_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostIncomingResponse, TableError>;
    fn delete_incoming_response(&mut self, id: u32) -> Result<HostIncomingResponse, TableError>;

    fn push_fields(&mut self, fields: HostFields) -> Result<u32, TableError>;
    fn get_fields(&self, id: u32) -> Result<&HostFields, TableError>;
    fn get_fields_mut(&mut self, id: u32) -> Result<&mut HostFields, TableError>;
    fn delete_fields(&mut self, id: u32) -> Result<HostFields, TableError>;

    fn push_future_incoming_response(
        &mut self,
        response: HostFutureIncomingResponse,
    ) -> Result<u32, TableError>;
    fn get_future_incoming_response(
        &self,
        id: u32,
    ) -> Result<&HostFutureIncomingResponse, TableError>;
    fn get_future_incoming_response_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostFutureIncomingResponse, TableError>;
    fn delete_future_incoming_response(
        &mut self,
        id: u32,
    ) -> Result<HostFutureIncomingResponse, TableError>;
}

#[async_trait::async_trait]
impl TableHttpExt for Table {
    fn push_outgoing_response(&mut self, request: HostOutgoingRequest) -> Result<u32, TableError> {
        self.push(Box::new(request))
    }
    fn get_outgoing_request(&self, id: u32) -> Result<&HostOutgoingRequest, TableError> {
        self.get::<HostOutgoingRequest>(id)
    }
    fn get_outgoing_request_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostOutgoingRequest, TableError> {
        self.get_mut::<HostOutgoingRequest>(id)
    }
    fn delete_outgoing_request(&mut self, id: u32) -> Result<HostOutgoingRequest, TableError> {
        let req = self.delete::<HostOutgoingRequest>(id)?;
        Ok(req)
    }

    fn push_incoming_response(
        &mut self,
        response: HostIncomingResponse,
    ) -> Result<u32, TableError> {
        self.push(Box::new(response))
    }
    fn get_incoming_response(&self, id: u32) -> Result<&HostIncomingResponse, TableError> {
        self.get::<HostIncomingResponse>(id)
    }
    fn get_incoming_response_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostIncomingResponse, TableError> {
        self.get_mut::<HostIncomingResponse>(id)
    }
    fn delete_incoming_response(&mut self, id: u32) -> Result<HostIncomingResponse, TableError> {
        let resp = self.delete::<HostIncomingResponse>(id)?;
        Ok(resp)
    }

    fn push_fields(&mut self, fields: HostFields) -> Result<u32, TableError> {
        self.push(Box::new(fields))
    }
    fn get_fields(&self, id: u32) -> Result<&HostFields, TableError> {
        self.get::<HostFields>(id)
    }
    fn get_fields_mut(&mut self, id: u32) -> Result<&mut HostFields, TableError> {
        self.get_mut::<HostFields>(id)
    }
    fn delete_fields(&mut self, id: u32) -> Result<HostFields, TableError> {
        let fields = self.delete::<HostFields>(id)?;
        Ok(fields)
    }

    fn push_future_incoming_response(
        &mut self,
        response: HostFutureIncomingResponse,
    ) -> Result<u32, TableError> {
        self.push(Box::new(response))
    }
    fn get_future_incoming_response(
        &self,
        id: u32,
    ) -> Result<&HostFutureIncomingResponse, TableError> {
        self.get::<HostFutureIncomingResponse>(id)
    }
    fn get_future_incoming_response_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut HostFutureIncomingResponse, TableError> {
        self.get_mut::<HostFutureIncomingResponse>(id)
    }
    fn delete_future_incoming_response(
        &mut self,
        id: u32,
    ) -> Result<HostFutureIncomingResponse, TableError> {
        self.delete(id)
    }
}

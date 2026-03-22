//! p3 host implementation for `wasi:tls`.

use crate::p3::util::{AsyncReadProducer, AsyncWriteConsumer, Closed, Deferred, Shared, pipe};
use crate::p3::{WasiTls, WasiTlsCtxView, bindings};
use crate::{BoxFutureTlStream, Error, TlsStream};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::{io::AsyncWriteExt as _, sync::oneshot};
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Access, Accessor, AccessorTask, FutureProducer, FutureReader, HasData, Resource, StreamReader,
};

/// Host-side state stored for `wasi:tls/client` `connector` resources.
pub struct Connector {
    connection: Shared<Deferred<Box<dyn TlsStream>>>,
    send: Option<pipe::Writer>,
    recv: Option<pipe::Reader>,
}

impl<'a> bindings::tls::client::Host for WasiTlsCtxView<'a> {}
impl<'a> bindings::tls::types::Host for WasiTlsCtxView<'a> {}

impl<'a> bindings::tls::types::HostError for WasiTlsCtxView<'a> {
    fn to_debug_string(&mut self, this: Resource<Error>) -> wasmtime::Result<String> {
        Ok(self.table.get(&this)?.to_string())
    }

    fn drop(&mut self, rep: Resource<Error>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl<'a> bindings::tls::client::HostConnector for WasiTlsCtxView<'a> {
    fn new(&mut self) -> wasmtime::Result<Resource<Connector>> {
        Ok(self.table.push(Connector {
            connection: Shared::new(Deferred::pending()),
            send: None,
            recv: None,
        })?)
    }

    fn drop(&mut self, rep: Resource<Connector>) -> wasmtime::Result<()> {
        self.table.delete(rep)?;
        Ok(())
    }
}

impl bindings::tls::client::HostConnectorWithStore for WasiTls {
    fn send<T: 'static>(
        mut store: Access<'_, T, Self>,
        this: Resource<Connector>,
        mut cleartext: StreamReader<u8>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)> {
        let getter = store.getter();

        {
            let ctx = store.get();
            let connector = ctx.table.get(&this)?;
            if connector.send.is_some() {
                cleartext.close(&mut store)?;

                let err = Error::msg("send already configured");
                let ciphertext = Closed(err.clone());
                let result = ResultProducer::ready(getter, Err(err));

                return Ok((
                    StreamReader::new(&mut store, ciphertext)?,
                    FutureReader::new(&mut store, result)?,
                ));
            }
        }

        let (ciphertext_reader, ciphertext_writer) = pipe::pipe();
        let (ciphertext_result_tx, ciphertext_result_rx) = oneshot::channel();
        let (cleartext_result_tx, cleartext_result_rx) = oneshot::channel();
        let (send_result_tx, send_result_rx) = oneshot::channel();

        let connection = {
            let ctx = store.get();
            let connector = ctx.table.get_mut(&this)?;
            connector.send = Some(ciphertext_writer);
            connector.connection.clone()
        };

        cleartext.pipe(
            &mut store,
            AsyncWriteConsumer::new(connection, cleartext_result_tx),
        )?;

        let ciphertext = AsyncReadProducer::new(ciphertext_reader, ciphertext_result_tx);
        store.spawn(FnTask(async move || {
            let cleartext_result = match cleartext_result_rx.await? {
                Ok(mut inner) => inner.shutdown().await, // Drive the close_notify sequence
                Err(e) => Err(e),
            };
            let ciphertext_result = ciphertext_result_rx.await?.map(drop);
            let combined_result = cleartext_result
                .and(ciphertext_result)
                .map_err(|e| Error::from(e));
            _ = send_result_tx.send(combined_result);
            Ok(())
        }));
        let result = ResultProducer::new(getter, send_result_rx);

        Ok((
            StreamReader::new(&mut store, ciphertext)?,
            FutureReader::new(&mut store, result)?,
        ))
    }

    fn receive<T: 'static>(
        mut store: Access<'_, T, Self>,
        this: Resource<Connector>,
        mut ciphertext: StreamReader<u8>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)> {
        let getter = store.getter();

        {
            let ctx = store.get();
            let connector = ctx.table.get(&this)?;
            if connector.recv.is_some() {
                ciphertext.close(&mut store)?;

                let err = Error::msg("receive already configured");
                let cleartext = Closed(err.clone());
                let result = ResultProducer::ready(getter, Err(err));

                return Ok((
                    StreamReader::new(&mut store, cleartext)?,
                    FutureReader::new(&mut store, result)?,
                ));
            }
        }

        let (ciphertext_reader, ciphertext_writer) = pipe::pipe();
        let (ciphertext_result_tx, ciphertext_result_rx) = oneshot::channel();
        let (cleartext_result_tx, cleartext_result_rx) = oneshot::channel();
        let (recv_result_tx, recv_result_rx) = oneshot::channel();

        let connection = {
            let ctx = store.get();
            let connector = ctx.table.get_mut(&this)?;
            connector.recv = Some(ciphertext_reader);
            connector.connection.clone()
        };

        ciphertext.pipe(
            &mut store,
            AsyncWriteConsumer::new(ciphertext_writer, ciphertext_result_tx),
        )?;

        let cleartext = AsyncReadProducer::new(connection, cleartext_result_tx);
        store.spawn(FnTask(async move || {
            let ciphertext_result = match ciphertext_result_rx.await? {
                // Let the TLS implementation know the transport is closed.
                // Most likely, `shutdown` will be entirely synchronous and
                // complete immediately, but awaiting it anyway to adhere to
                // the AsyncWrite contract:
                Ok(mut inner) => inner.shutdown().await,
                Err(e) => Err(e),
            };
            let cleartext_result = cleartext_result_rx.await?.map(drop);
            let combined_result = cleartext_result
                .and(ciphertext_result)
                .map_err(|e| Error::from(e));
            _ = recv_result_tx.send(combined_result);
            Ok(())
        }));
        let result = ResultProducer::new(getter, recv_result_rx);

        Ok((
            StreamReader::new(&mut store, cleartext)?,
            FutureReader::new(&mut store, result)?,
        ))
    }

    async fn connect<T: Send>(
        accessor: &Accessor<T, Self>,
        this: Resource<Connector>,
        server_name: String,
    ) -> wasmtime::Result<Result<(), Resource<Error>>> {
        fn connect_err(msg: &'static str) -> BoxFutureTlStream {
            Box::pin(async move { Err(Error::msg(msg).into()) })
        }
        let (fut, connection) = accessor.with(
            move |mut access| -> wasmtime::Result<(BoxFutureTlStream, _)> {
                let WasiTlsCtxView { table, ctx } = access.get();
                let connector = table.delete(this)?;
                let connection = connector.connection;

                let Some(ciphertext_writer) = connector.send else {
                    return Ok((
                        connect_err("send() must be called before connect()"),
                        connection,
                    ));
                };
                let Some(ciphertext_reader) = connector.recv else {
                    return Ok((
                        connect_err("receive() must be called before connect()"),
                        connection,
                    ));
                };

                let transport = Box::new(tokio::io::join(ciphertext_reader, ciphertext_writer));
                let fut = ctx.provider.connect(server_name, transport);

                Ok((fut, connection))
            },
        )?;

        match fut.await {
            Ok(tls_stream) => {
                connection.lock().resolve(tls_stream);
                Ok(Ok(()))
            }
            Err(e) => {
                connection.lock().resolve(Box::new(Closed(e.clone())));
                let resource = accessor.with(|mut access| access.get().table.push(e))?;
                Ok(Err(resource))
            }
        }
    }
}

pub(crate) struct FnTask<Fn>(pub(crate) Fn);
impl<Fn, Fut, T, D> AccessorTask<T, D> for FnTask<Fn>
where
    Fn: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = wasmtime::Result<()>> + Send + 'static,
    D: HasData + ?Sized,
{
    fn run(
        self,
        _accessor: &wasmtime::component::Accessor<T, D>,
    ) -> impl Future<Output = wasmtime::Result<()>> + Send {
        self.0()
    }
}

pub(crate) struct ResultProducer<D> {
    result: oneshot::Receiver<Result<(), Error>>,
    getter: for<'a> fn(&'a mut D) -> WasiTlsCtxView<'a>,
}
impl<D> ResultProducer<D> {
    pub(crate) fn new(
        getter: for<'a> fn(&'a mut D) -> WasiTlsCtxView<'a>,
        result: oneshot::Receiver<Result<(), Error>>,
    ) -> Self {
        Self { result, getter }
    }

    pub(crate) fn ready(
        getter: for<'a> fn(&'a mut D) -> WasiTlsCtxView<'a>,
        result: Result<(), Error>,
    ) -> Self {
        let (sender, receiver) = oneshot::channel();
        sender.send(result).expect("receiver dropped");
        Self {
            result: receiver,
            getter,
        }
    }
}
impl<D> FutureProducer<D> for ResultProducer<D>
where
    D: 'static,
{
    type Item = Result<(), Resource<Error>>;

    fn poll_produce(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<D>,
        finish: bool,
    ) -> Poll<wasmtime::error::Result<Option<Self::Item>>> {
        match Pin::new(&mut self.result).poll(cx) {
            Poll::Ready(Ok(Ok(()))) => Poll::Ready(Ok(Some(Ok(())))),
            Poll::Ready(Ok(Err(err))) => {
                let WasiTlsCtxView { table, .. } = (self.getter)(store.data_mut());
                let err = table.push(err)?;
                Poll::Ready(Ok(Some(Err(err))))
            }
            Poll::Ready(Err(_)) => Poll::Ready(Err(wasmtime::format_err!("sender dropped"))),
            Poll::Pending if finish => Poll::Ready(Ok(None)),
            Poll::Pending => Poll::Pending,
        }
    }
}

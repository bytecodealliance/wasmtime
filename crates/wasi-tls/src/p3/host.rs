//! p3 host implementation for `wasi:tls`.

use crate::p3::util::{AsyncReadProducer, AsyncWriteConsumer, Closed, Deferred, Shared, pipe};
use crate::p3::{WasiTls, bindings};
use crate::{BoxFutureTlsStream, Error, TlsStream, WasiTlsCtxView};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::{io::AsyncWriteExt as _, sync::oneshot};
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Access, Accessor, FutureProducer, FutureReader, HasData, Resource, StreamReader,
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

impl<T> bindings::tls::client::HostConnectorWithStore<T> for WasiTls {
    fn send(
        store: Access<'_, T, Self>,
        this: Resource<Connector>,
        cleartext: StreamReader<u8>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)> {
        let getter = store.getter();
        send(store, getter, this, cleartext)
    }

    fn receive(
        store: Access<'_, T, Self>,
        this: Resource<Connector>,
        ciphertext: StreamReader<u8>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)> {
        let getter = store.getter();
        receive(store, getter, this, ciphertext)
    }

    async fn connect(
        accessor: &Accessor<T, Self>,
        this: Resource<Connector>,
        server_name: String,
    ) -> wasmtime::Result<Result<(), Resource<Error>>> {
        let getter = accessor.getter();
        connect(accessor, getter, this, server_name).await
    }
}

// The functions below are generic over how to get a `WasiTlsCtxView` so they
// can be shared with named imports, whose getter closes over a `NamedId`.

fn send<T, D>(
    mut store: Access<'_, T, D>,
    getter: impl Fn(&mut T) -> WasiTlsCtxView<'_> + Copy + Send + Unpin + 'static,
    this: Resource<Connector>,
    mut cleartext: StreamReader<u8>,
) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)>
where
    T: 'static,
    D: HasData,
{
    {
        let ctx = getter(store.data_mut());
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
        let ctx = getter(store.data_mut());
        let connector = ctx.table.get_mut(&this)?;
        connector.send = Some(ciphertext_writer);
        connector.connection.clone()
    };

    cleartext.pipe(
        &mut store,
        AsyncWriteConsumer::new(connection, cleartext_result_tx),
    )?;

    let ciphertext = AsyncReadProducer::new(ciphertext_reader, ciphertext_result_tx);
    store.spawn(async move |_| {
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
    })?;
    let result = ResultProducer::new(getter, send_result_rx);

    Ok((
        StreamReader::new(&mut store, ciphertext)?,
        FutureReader::new(&mut store, result)?,
    ))
}

fn receive<T, D>(
    mut store: Access<'_, T, D>,
    getter: impl Fn(&mut T) -> WasiTlsCtxView<'_> + Copy + Send + Unpin + 'static,
    this: Resource<Connector>,
    mut ciphertext: StreamReader<u8>,
) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)>
where
    T: 'static,
    D: HasData,
{
    {
        let ctx = getter(store.data_mut());
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
        let ctx = getter(store.data_mut());
        let connector = ctx.table.get_mut(&this)?;
        connector.recv = Some(ciphertext_reader);
        connector.connection.clone()
    };

    ciphertext.pipe(
        &mut store,
        AsyncWriteConsumer::new(ciphertext_writer, ciphertext_result_tx),
    )?;

    let cleartext = AsyncReadProducer::new(connection, cleartext_result_tx);
    store.spawn(async move |_| {
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
    })?;
    let result = ResultProducer::new(getter, recv_result_rx);

    Ok((
        StreamReader::new(&mut store, cleartext)?,
        FutureReader::new(&mut store, result)?,
    ))
}

async fn connect<T, D>(
    accessor: &Accessor<T, D>,
    getter: impl Fn(&mut T) -> WasiTlsCtxView<'_> + Copy + Send + 'static,
    this: Resource<Connector>,
    server_name: String,
) -> wasmtime::Result<Result<(), Resource<Error>>>
where
    T: 'static,
    D: HasData,
{
    fn connect_err(msg: &'static str) -> BoxFutureTlsStream {
        Box::pin(async move { Err(Error::msg(msg)) })
    }
    let (fut, connection) = accessor.with(
        move |mut access| -> wasmtime::Result<(BoxFutureTlsStream, _)> {
            let WasiTlsCtxView { table, ctx } = getter(access.data_mut());
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
            let resource = accessor.with(|mut access| getter(access.data_mut()).table.push(e))?;
            Ok(Err(resource))
        }
    }
}

pub(crate) struct ResultProducer<G> {
    result: oneshot::Receiver<Result<(), Error>>,
    getter: G,
}
impl<G> ResultProducer<G> {
    pub(crate) fn new(getter: G, result: oneshot::Receiver<Result<(), Error>>) -> Self {
        Self { result, getter }
    }

    pub(crate) fn ready(getter: G, result: Result<(), Error>) -> Self {
        let (sender, receiver) = oneshot::channel();
        sender.send(result).expect("receiver dropped");
        Self {
            result: receiver,
            getter,
        }
    }
}
impl<D, G> FutureProducer<D> for ResultProducer<G>
where
    D: 'static,
    G: Fn(&mut D) -> WasiTlsCtxView<'_> + Send + Unpin + 'static,
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

mod named {
    use super::Connector;
    use crate::p3::bindings::named_imports::wasi::tls::{client, types};
    use crate::p3::bindings::tls::{client as unnamed_client, types as unnamed_types};
    use crate::{Error, WasiTlsNamed, WasiTlsNamedView};
    use wasmtime::component::{Access, Accessor, FutureReader, Resource, StreamReader};
    use wasmtime_wasi::{NamedId, WasiCtxNamedView};

    impl<T: WasiTlsNamedView> client::Host for WasiCtxNamedView<'_, T> {}
    impl<T: WasiTlsNamedView> types::Host for WasiCtxNamedView<'_, T> {}

    impl<T: WasiTlsNamedView> types::HostError for WasiCtxNamedView<'_, T> {
        fn to_debug_string(
            &mut self,
            id: NamedId,
            this: Resource<Error>,
        ) -> wasmtime::Result<String> {
            unnamed_types::HostError::to_debug_string(&mut self.0.tls(id), this)
        }

        fn drop(&mut self, id: NamedId, rep: Resource<Error>) -> wasmtime::Result<()> {
            unnamed_types::HostError::drop(&mut self.0.tls(id), rep)
        }
    }

    impl<T: WasiTlsNamedView> client::HostConnector for WasiCtxNamedView<'_, T> {
        fn new(&mut self, id: NamedId) -> wasmtime::Result<Resource<Connector>> {
            unnamed_client::HostConnector::new(&mut self.0.tls(id))
        }

        fn drop(&mut self, id: NamedId, rep: Resource<Connector>) -> wasmtime::Result<()> {
            unnamed_client::HostConnector::drop(&mut self.0.tls(id), rep)
        }
    }

    impl<T, U> client::HostConnectorWithStore<U> for WasiTlsNamed<T>
    where
        T: WasiTlsNamedView,
        U: 'static,
    {
        fn send(
            store: Access<'_, U, Self>,
            id: NamedId,
            this: Resource<Connector>,
            cleartext: StreamReader<u8>,
        ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)>
        {
            let getter = store.getter();
            super::send(store, move |data| getter(data).0.tls(id), this, cleartext)
        }

        fn receive(
            store: Access<'_, U, Self>,
            id: NamedId,
            this: Resource<Connector>,
            ciphertext: StreamReader<u8>,
        ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)>
        {
            let getter = store.getter();
            super::receive(store, move |data| getter(data).0.tls(id), this, ciphertext)
        }

        async fn connect(
            accessor: &Accessor<U, Self>,
            id: NamedId,
            this: Resource<Connector>,
            server_name: String,
        ) -> wasmtime::Result<Result<(), Resource<Error>>> {
            let getter = accessor.getter();
            super::connect(
                accessor,
                move |data| getter(data).0.tls(id),
                this,
                server_name,
            )
            .await
        }
    }
}

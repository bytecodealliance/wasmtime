use super::{
    CiphertextConsumer, CiphertextProducer, Pending, PlaintextConsumer, PlaintextProducer,
    mk_delete, mk_get_mut, mk_push, push_error,
};
use crate::p3::bindings::tls::client::{Connector, Host, HostConnector, HostConnectorWithStore};
use crate::p3::bindings::tls::types::Error;
use crate::p3::host::ResultProducer;
use crate::p3::{TlsStream, WasiTls, WasiTlsCtxView};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use wasmtime::component::{Access, Accessor, FutureReader, Resource, StreamReader};

mk_push!(Connector, push_connector, "client connector");
mk_get_mut!(Connector, get_connector_mut, "client connector");
mk_delete!(Connector, delete_connector, "client connector");

impl Host for WasiTlsCtxView<'_> {}

impl HostConnector for WasiTlsCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<Connector>> {
        push_connector(&mut self.table, Connector::default())
    }

    fn drop(&mut self, conn: Resource<Connector>) -> wasmtime::Result<()> {
        delete_connector(&mut self.table, conn)?;
        Ok(())
    }
}

impl HostConnectorWithStore for WasiTls {
    fn send<T>(
        mut store: Access<T, Self>,
        conn: Resource<Connector>,
        cleartext: StreamReader<u8>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)>
    where
        T: 'static,
    {
        let conn @ Connector { send_tx: None, .. } = get_connector_mut(store.get().table, &conn)?
        else {
            return Err(wasmtime::Error::msg("`send` was already called"));
        };

        let (cons_tx, cons_rx) = oneshot::channel();
        let (prod_tx, prod_rx) = oneshot::channel();
        let (err_tx, err_rx) = oneshot::channel();

        conn.send_tx = Some((prod_tx, cons_tx, err_tx));

        let rx = StreamReader::new(&mut store, Pending::from(prod_rx));
        cleartext.pipe(&mut store, Pending::from(cons_rx));
        let getter = store.getter();
        Ok((
            rx,
            FutureReader::new(store, ResultProducer { rx: err_rx, getter }),
        ))
    }

    fn receive<T>(
        mut store: Access<T, Self>,
        conn: Resource<Connector>,
        ciphertext: StreamReader<u8>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), Resource<Error>>>)>
    where
        T: 'static,
    {
        let conn @ Connector {
            receive_tx: None, ..
        } = get_connector_mut(store.get().table, &conn)?
        else {
            return Err(wasmtime::Error::msg("`receive` was already called"));
        };

        let (cons_tx, cons_rx) = oneshot::channel();
        let (prod_tx, prod_rx) = oneshot::channel();
        let (err_tx, err_rx) = oneshot::channel();

        conn.receive_tx = Some((prod_tx, cons_tx, err_tx));

        let rx = StreamReader::new(&mut store, Pending::from(prod_rx));
        ciphertext.pipe(&mut store, Pending::from(cons_rx));
        let getter = store.getter();
        Ok((
            rx,
            FutureReader::new(store, ResultProducer { rx: err_rx, getter }),
        ))
    }

    async fn connect<T>(
        store: &Accessor<T, Self>,
        conn: Resource<Connector>,
        server_name: String,
    ) -> wasmtime::Result<Result<(), Resource<Error>>>
    where
        T: 'static,
    {
        store.with(|mut store| {
            let server_name = match server_name.try_into() {
                Ok(name) => name,
                Err(err) => {
                    let err = push_error(store.get().table, format!("{err}"))?;
                    return Ok(Err(err));
                }
            };

            let Connector {
                receive_tx: Some((receive_prod_tx, receive_cons_tx, receive_err_tx)),
                send_tx: Some((send_prod_tx, send_cons_tx, send_err_tx)),
            } = delete_connector(store.get().table, conn)?
            else {
                let err = push_error(
                    store.get().table,
                    format!("`send` and `receive` must be called prior to calling `connect`"),
                )?;
                return Ok(Err(err));
            };

            let roots = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            };
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();

            let conn = match rustls::ClientConnection::new(Arc::from(config), server_name) {
                Ok(conn) => conn,
                Err(err) => {
                    let err = push_error(store.get().table, format!("{err}"))?;
                    return Ok(Err(err));
                }
            };

            let stream = Arc::new(Mutex::new(TlsStream::new(conn)));

            let send_err_tx = Arc::new(Mutex::new(Some(send_err_tx)));
            let _ = send_cons_tx.send(PlaintextConsumer {
                stream: Arc::clone(&stream),
                error_tx: Arc::clone(&send_err_tx),
            });
            let _ = send_prod_tx.send(CiphertextProducer {
                stream: Arc::clone(&stream),
                error_tx: send_err_tx,
            });

            let receive_err_tx = Arc::new(Mutex::new(Some(receive_err_tx)));
            let _ = receive_cons_tx.send(CiphertextConsumer {
                stream: Arc::clone(&stream),
                error_tx: Arc::clone(&receive_err_tx),
            });
            let _ = receive_prod_tx.send(PlaintextProducer {
                stream,
                error_tx: receive_err_tx,
            });

            Ok(Ok(()))
        })
    }
}

#![expect(unused, reason = "WIP")]

use super::{PlaintextConsumer, PlaintextProducer, ResultProducer, mk_delete, mk_get, mk_push};
use crate::p3::bindings::tls::server::{
    Handshake, Host, HostHandshake, HostHandshakeWithStore, HostWithStore,
};
use crate::p3::bindings::tls::types::Certificate;
use crate::p3::{TlsStream, TlsStreamServerArc, WasiTls, WasiTlsCtxView};
use anyhow::{Context as _, anyhow};
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll};
use rustls::server::ResolvesServerCert;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Access, Accessor, Destination, FutureReader, Resource, Source, StreamConsumer, StreamProducer,
    StreamReader, StreamResult,
};

mk_delete!(Handshake, delete_handshake, "server handshake");
mk_get!(Handshake, get_handshake, "server handshake");
mk_push!(Handshake, push_handshake, "server handshake");

enum CiphertextConsumer {
    Pending {
        acceptor: rustls::server::Acceptor,
        tx: oneshot::Sender<
            Result<
                (
                    rustls::server::Accepted,
                    oneshot::Sender<TlsStreamServerArc>,
                ),
                rustls::Error,
            >,
        >,
    },
    Accepted(oneshot::Receiver<TlsStreamServerArc>),
    Active(super::CiphertextConsumer<rustls::ServerConnection>),
    Corrupted,
}

impl<D> StreamConsumer<D> for CiphertextConsumer {
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let this = self.get_mut();
        match mem::replace(this, Self::Corrupted) {
            Self::Pending { mut acceptor, tx } => {
                let mut src = src.as_direct(store);
                if src.remaining().is_empty() {
                    return Poll::Ready(Ok(StreamResult::Completed));
                }
                acceptor.read_tls(&mut src)?;
                match acceptor.accept() {
                    Ok(None) => {
                        *this = Self::Pending { acceptor, tx };
                        Poll::Ready(Ok(StreamResult::Completed))
                    }
                    Ok(Some(accepted)) => {
                        let (stream_tx, stream_rx) = oneshot::channel();
                        _ = tx.send(Ok((accepted, stream_tx)));
                        *this = Self::Accepted(stream_rx);
                        Poll::Ready(Ok(StreamResult::Completed))
                    }
                    Err(err) => {
                        _ = tx.send(Err(err));
                        Poll::Ready(Ok(StreamResult::Dropped))
                    }
                }
            }
            Self::Accepted(mut rx) => match Pin::new(&mut rx).poll(cx) {
                Poll::Ready(Ok(stream)) => {
                    *this = Self::Active(super::CiphertextConsumer(stream));
                    Poll::Ready(Ok(StreamResult::Completed))
                }
                Poll::Ready(Err(..)) => Poll::Ready(Ok(StreamResult::Dropped)),
                Poll::Pending if finish => {
                    *this = Self::Accepted(rx);
                    Poll::Ready(Ok(StreamResult::Cancelled))
                }
                Poll::Pending => {
                    *this = Self::Accepted(rx);
                    Poll::Ready(Ok(StreamResult::Cancelled))
                }
            },
            Self::Active(ref mut conn) => Pin::new(conn).poll_consume(cx, store, src, finish),
            Self::Corrupted => Poll::Ready(Err(anyhow!("corrupted stream consumer state"))),
        }
    }
}

enum CiphertextProducer {
    Pending(oneshot::Receiver<TlsStreamServerArc>),
    Active(super::CiphertextProducer<rustls::ServerConnection>),
    Corrupted,
}

impl<D> StreamProducer<D> for CiphertextProducer {
    type Item = u8;
    type Buffer = Option<u8>; // unused

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let this = self.get_mut();
        match mem::replace(this, Self::Corrupted) {
            Self::Pending(mut rx) => match Pin::new(&mut rx).poll(cx) {
                Poll::Ready(Ok(stream)) => {
                    *this = Self::Active(super::CiphertextProducer(stream));
                    Poll::Ready(Ok(StreamResult::Completed))
                }
                Poll::Ready(Err(..)) => Poll::Ready(Ok(StreamResult::Dropped)),
                Poll::Pending if finish => {
                    *this = Self::Pending(rx);
                    Poll::Ready(Ok(StreamResult::Cancelled))
                }
                Poll::Pending => {
                    *this = Self::Pending(rx);
                    Poll::Ready(Ok(StreamResult::Cancelled))
                }
            },
            Self::Active(ref mut conn) => Pin::new(conn).poll_produce(cx, store, dst, finish),
            Self::Corrupted => Poll::Ready(Err(anyhow!("corrupted stream producer state"))),
        }
    }
}

#[derive(Debug)]
struct CertificateResolver;

impl ResolvesServerCert for CertificateResolver {
    fn resolve(
        &self,
        hello: rustls::server::ClientHello,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        // TODO: Implement
        None
    }
}

impl Host for WasiTlsCtxView<'_> {}

impl HostWithStore for WasiTls {
    async fn accept<T>(
        store: &Accessor<T, Self>,
        incoming: StreamReader<u8>,
    ) -> wasmtime::Result<Result<(StreamReader<u8>, Resource<Handshake>), ()>> {
        let (accept_tx, accept_rx) = oneshot::channel();
        store.with(|store| {
            incoming.pipe(
                store,
                CiphertextConsumer::Pending {
                    acceptor: rustls::server::Acceptor::default(),
                    tx: accept_tx,
                },
            );
        });
        let (accepted, consumer_tx) = match accept_rx
            .await
            .context("oneshot sender dropped unexpectedly")?
        {
            Ok((accepted, consumer_tx)) => (accepted, consumer_tx),
            Err(_err) => return Ok(Err(())),
        };
        let (producer_tx, producer_rx) = oneshot::channel();
        store.with(|mut store| {
            let handshake = push_handshake(
                store.get().table,
                Handshake {
                    accepted,
                    consumer_tx,
                    producer_tx,
                },
            )?;
            Ok(Ok((
                StreamReader::new(&mut store, CiphertextProducer::Pending(producer_rx)),
                handshake,
            )))
        })
    }
}

impl HostHandshake for WasiTlsCtxView<'_> {
    fn set_server_certificate(
        &mut self,
        handshake: Resource<Handshake>,
        cert: Resource<Certificate>,
    ) -> wasmtime::Result<()> {
        todo!()
    }

    fn get_client_certificate(
        &mut self,
        handshake: Resource<Handshake>,
    ) -> wasmtime::Result<FutureReader<Result<Resource<Certificate>, ()>>> {
        todo!()
    }

    fn get_server_name(
        &mut self,
        handshake: Resource<Handshake>,
    ) -> wasmtime::Result<Option<String>> {
        let handshake = get_handshake(&self.table, &handshake)?;
        let hello = handshake.accepted.client_hello();
        let server_name = hello.server_name().map(Into::into);
        Ok(server_name)
    }

    fn get_alpn_ids(
        &mut self,
        handshake: Resource<Handshake>,
    ) -> wasmtime::Result<Option<Vec<Vec<u8>>>> {
        let handshake = get_handshake(&self.table, &handshake)?;
        let hello = handshake.accepted.client_hello();
        let alpn = hello.alpn().map(|alpn| alpn.map(Into::into).collect());
        Ok(alpn)
    }

    fn get_cipher_suites(&mut self, handshake: Resource<Handshake>) -> wasmtime::Result<Vec<u16>> {
        let handshake = get_handshake(&self.table, &handshake)?;
        let hello = handshake.accepted.client_hello();
        let cipher_suites = hello
            .cipher_suites()
            .into_iter()
            .map(rustls::CipherSuite::get_u16)
            .collect();
        Ok(cipher_suites)
    }

    fn set_cipher_suite(
        &mut self,
        handshake: Resource<Handshake>,
        cipher_suite: u16,
    ) -> wasmtime::Result<()> {
        todo!()
    }

    fn drop(&mut self, handshake: Resource<Handshake>) -> wasmtime::Result<()> {
        delete_handshake(&mut self.table, handshake)?;
        Ok(())
    }
}

impl HostHandshakeWithStore for WasiTls {
    fn finish<T>(
        mut store: Access<T, Self>,
        handshake: Resource<Handshake>,
        data: StreamReader<u8>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ()>>)> {
        let Handshake {
            accepted,
            consumer_tx,
            producer_tx,
        } = delete_handshake(&mut store.get().table, handshake)?;
        // TODO: configure
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(CertificateResolver));
        let conn = accepted
            .into_connection(Arc::from(config))
            .context("failed to construct rustls server connection")?;
        let (error_tx, error_rx) = oneshot::channel();
        let stream = Arc::new(Mutex::new(TlsStream::new(conn, error_tx)));
        data.pipe(&mut store, PlaintextConsumer(Arc::clone(&stream)));
        _ = consumer_tx.send(Arc::clone(&stream));
        _ = producer_tx.send(Arc::clone(&stream));
        Ok((
            StreamReader::new(&mut store, PlaintextProducer(stream)),
            FutureReader::new(&mut store, ResultProducer(error_rx)),
        ))
    }
}

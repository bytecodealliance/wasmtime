use super::{
    CiphertextConsumer, CiphertextProducer, PlaintextConsumer, PlaintextProducer, ResultProducer,
    mk_delete, mk_get, mk_get_mut, mk_push,
};
use crate::p3::bindings::tls::client::{
    Handshake, Hello, Host, HostHandshake, HostHandshakeWithStore, HostHello, HostWithStore,
};
use crate::p3::bindings::tls::types::Certificate;
use crate::p3::{TlsStream, TlsStreamClientArc, WasiTls, WasiTlsCtxView};
use anyhow::{Context as _, anyhow, bail};
use core::mem;
use core::net::{IpAddr, Ipv4Addr};
use core::pin::{Pin, pin};
use core::task::{Context, Poll};
use rustls::client::ResolvesClientCert;
use rustls::pki_types::ServerName;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use wasmtime::StoreContextMut;
use wasmtime::component::{Access, FutureProducer, FutureReader, Resource, StreamReader};

mk_push!(Hello, push_hello, "client hello");
mk_get_mut!(Hello, get_hello_mut, "client hello");
mk_delete!(Hello, delete_hello, "client hello");

mk_push!(Handshake, push_handshake, "client handshake");
mk_get!(Handshake, get_handshake, "client handshake");
mk_delete!(Handshake, delete_handshake, "client handshake");

#[derive(Default)]
enum ConnectProducer<T> {
    Pending {
        stream: TlsStreamClientArc,
        error_rx: oneshot::Receiver<rustls::Error>,
        getter: fn(&mut T) -> WasiTlsCtxView<'_>,
    },
    #[default]
    Exhausted,
}

impl<D> FutureProducer<D> for ConnectProducer<D>
where
    D: 'static,
{
    type Item = Result<Resource<Handshake>, ()>;

    fn poll_produce(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<D>,
        finish: bool,
    ) -> Poll<anyhow::Result<Option<Self::Item>>> {
        let this = self.get_mut();
        let Self::Pending {
            stream,
            mut error_rx,
            getter,
        } = mem::take(this)
        else {
            return Poll::Ready(Err(anyhow!("polled after ready")));
        };
        if let Poll::Ready(..) = pin!(&mut error_rx).poll(cx) {
            return Poll::Ready(Ok(Some(Err(()))));
        }

        {
            let mut stream_lock = stream.lock();
            let TlsStream { conn, read_tls, .. } = stream_lock.as_deref_mut().unwrap();
            if conn.peer_certificates().is_none() || conn.negotiated_cipher_suite().is_none() {
                if !finish {
                    *read_tls = Some(cx.waker().clone());
                }
                drop(stream_lock);
                *this = Self::Pending {
                    stream,
                    error_rx,
                    getter,
                };
                if finish {
                    return Poll::Ready(Ok(None));
                }
                return Poll::Pending;
            }
        };

        let WasiTlsCtxView { table, .. } = getter(store.data_mut());

        let handshake = Handshake { stream, error_rx };
        let handshake = push_handshake(table, handshake)?;

        Poll::Ready(Ok(Some(Ok(handshake))))
    }
}

#[derive(Debug)]
struct CertificateResolver;

impl ResolvesClientCert for CertificateResolver {
    fn resolve(
        &self,
        _root_hint_subjects: &[&[u8]],
        _sigschemes: &[rustls::SignatureScheme],
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        // TODO: implement
        None
    }

    fn has_certs(&self) -> bool {
        false
    }
}

impl Host for WasiTlsCtxView<'_> {}

impl HostHello for WasiTlsCtxView<'_> {
    fn new(&mut self) -> wasmtime::Result<Resource<Hello>> {
        push_hello(&mut self.table, Hello::default())
    }

    fn set_server_name(
        &mut self,
        hello: Resource<Hello>,
        server_name: String,
    ) -> wasmtime::Result<Result<(), ()>> {
        let hello = get_hello_mut(&mut self.table, &hello)?;
        let Ok(server_name) = server_name.try_into() else {
            return Ok(Err(()));
        };
        hello.server_name = Some(server_name);
        Ok(Ok(()))
    }

    fn set_alpn_ids(
        &mut self,
        hello: Resource<Hello>,
        alpn_ids: Vec<Vec<u8>>,
    ) -> wasmtime::Result<()> {
        let hello = get_hello_mut(&mut self.table, &hello)?;
        hello.alpn_ids = Some(alpn_ids);
        Ok(())
    }

    fn set_cipher_suites(
        &mut self,
        hello: Resource<Hello>,
        cipher_suites: Vec<u16>,
    ) -> wasmtime::Result<()> {
        let hello = get_hello_mut(&mut self.table, &hello)?;
        hello.cipher_suites = cipher_suites;
        Ok(())
    }

    fn drop(&mut self, hello: Resource<Hello>) -> wasmtime::Result<()> {
        delete_hello(&mut self.table, hello)?;
        Ok(())
    }
}

impl HostWithStore for WasiTls {
    fn connect<T>(
        mut store: Access<T, Self>,
        hello: Resource<Hello>,
        incoming: StreamReader<u8>,
    ) -> wasmtime::Result<(
        StreamReader<u8>,
        FutureReader<Result<Resource<Handshake>, ()>>,
    )> {
        let Hello {
            server_name,
            alpn_ids,
            cipher_suites,
        } = delete_hello(store.get().table, hello)?;

        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.into(),
        };
        if !cipher_suites.is_empty() {
            // TODO: implement
            bail!("custom cipher suites not supported yet")
        }
        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_client_cert_resolver(Arc::new(CertificateResolver));
        if let Some(alpn_ids) = alpn_ids {
            config.alpn_protocols = alpn_ids;
        }
        let server_name = if let Some(server_name) = server_name {
            server_name
        } else {
            config.enable_sni = false;
            ServerName::IpAddress(IpAddr::V4(Ipv4Addr::UNSPECIFIED).into())
        };
        let conn = rustls::ClientConnection::new(Arc::from(config), server_name)
            .context("failed to construct rustls client connection")?;
        let (error_tx, error_rx) = oneshot::channel();
        let stream = Arc::new(Mutex::new(TlsStream::new(conn, error_tx)));

        incoming.pipe(&mut store, CiphertextConsumer(Arc::clone(&stream)));
        let getter = store.getter();
        Ok((
            StreamReader::new(&mut store, CiphertextProducer(Arc::clone(&stream))),
            FutureReader::new(
                &mut store,
                ConnectProducer::Pending {
                    stream,
                    error_rx,
                    getter,
                },
            ),
        ))
    }
}

impl HostHandshake for WasiTlsCtxView<'_> {
    fn set_client_certificate(
        &mut self,
        _handshake: Resource<Handshake>,
        _cert: Resource<Certificate>,
    ) -> wasmtime::Result<()> {
        todo!()
    }

    fn get_server_certificate(
        &mut self,
        _handshake: Resource<Handshake>,
    ) -> wasmtime::Result<Option<Resource<Certificate>>> {
        todo!()
    }

    fn get_cipher_suite(&mut self, handshake: Resource<Handshake>) -> wasmtime::Result<u16> {
        let Handshake { stream, .. } = get_handshake(&self.table, &handshake)?;
        let mut stream = stream.lock();
        let TlsStream { conn, .. } = stream.as_deref_mut().unwrap();
        let cipher_suite = conn
            .negotiated_cipher_suite()
            .context("cipher suite not available")?;
        Ok(cipher_suite.suite().get_u16())
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
        let Handshake { stream, error_rx } = delete_handshake(&mut store.get().table, handshake)?;
        data.pipe(&mut store, PlaintextConsumer(Arc::clone(&stream)));
        Ok((
            StreamReader::new(&mut store, PlaintextProducer(stream)),
            FutureReader::new(&mut store, ResultProducer(error_rx)),
        ))
    }
}

use crate::p3::{TlsStream, TlsStreamArc};
use anyhow::Context as _;
use core::ops::DerefMut;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use std::io::{Read as _, Write as _};
use tokio::sync::oneshot;
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Destination, FutureProducer, Source, StreamConsumer, StreamProducer, StreamResult,
};

mod client;
mod server;
mod types;

macro_rules! mk_push {
    ($t:ty, $f:ident, $desc:literal) => {
        #[track_caller]
        #[inline]
        pub fn $f(
            table: &mut wasmtime::component::ResourceTable,
            value: $t,
        ) -> wasmtime::Result<wasmtime::component::Resource<$t>> {
            use anyhow::Context as _;

            table
                .push(value)
                .context(concat!("failed to push ", $desc, " resource to table"))
        }
    };
}

macro_rules! mk_get {
    ($t:ty, $f:ident, $desc:literal) => {
        #[track_caller]
        #[inline]
        pub fn $f<'a>(
            table: &'a wasmtime::component::ResourceTable,
            resource: &'a wasmtime::component::Resource<$t>,
        ) -> wasmtime::Result<&'a $t> {
            use anyhow::Context as _;

            table
                .get(resource)
                .context(concat!("failed to get ", $desc, " resource from table"))
        }
    };
}

macro_rules! mk_get_mut {
    ($t:ty, $f:ident, $desc:literal) => {
        #[track_caller]
        #[inline]
        pub fn $f<'a>(
            table: &'a mut wasmtime::component::ResourceTable,
            resource: &'a wasmtime::component::Resource<$t>,
        ) -> wasmtime::Result<&'a mut $t> {
            use anyhow::Context as _;

            table.get_mut(resource).context(concat!(
                "failed to get ",
                $desc,
                " resource from table"
            ))
        }
    };
}

macro_rules! mk_delete {
    ($t:ty, $f:ident, $desc:literal) => {
        #[track_caller]
        #[inline]
        pub fn $f(
            table: &mut wasmtime::component::ResourceTable,
            resource: wasmtime::component::Resource<$t>,
        ) -> wasmtime::Result<$t> {
            use anyhow::Context as _;

            table.delete(resource).context(concat!(
                "failed to delete ",
                $desc,
                " resource from table"
            ))
        }
    };
}

pub(crate) use {mk_delete, mk_get, mk_get_mut, mk_push};

struct CiphertextConsumer<T>(TlsStreamArc<T>);

impl<T, U, D> StreamConsumer<D> for CiphertextConsumer<T>
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static,
{
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut stream = self.0.lock();
        let TlsStream {
            conn,
            error_tx,
            read_tls,
            ciphertext_consumer,
            ciphertext_producer,
            plaintext_consumer,
            plaintext_producer,
            ..
        } = stream.as_deref_mut().unwrap();
        if error_tx.is_none() {
            return Poll::Ready(Ok(StreamResult::Dropped));
        }

        if !conn.wants_read() {
            if finish {
                return Poll::Ready(Ok(StreamResult::Cancelled));
            }
            *ciphertext_consumer = Some(cx.waker().clone());
            return Poll::Pending;
        }

        let mut src = src.as_direct(store);
        if src.remaining().is_empty() {
            return Poll::Ready(Ok(StreamResult::Completed));
        }
        let n = conn.read_tls(&mut src)?;
        debug_assert_ne!(n, 0);
        read_tls.take().map(Waker::wake);

        let state = match conn.process_new_packets() {
            Ok(state) => state,
            Err(err) => {
                _ = error_tx.take().unwrap().send(err);
                ciphertext_producer.take().map(Waker::wake);
                plaintext_consumer.take().map(Waker::wake);
                plaintext_producer.take().map(Waker::wake);
                return Poll::Ready(Ok(StreamResult::Dropped));
            }
        };

        if state.plaintext_bytes_to_read() > 0 {
            plaintext_producer.take().map(Waker::wake);
        }

        if state.tls_bytes_to_write() > 0 {
            ciphertext_producer.take().map(Waker::wake);
        }

        if state.peer_has_closed() {
            // even if there are no bytes to read, the producer may be pending
            plaintext_producer.take().map(Waker::wake);
            return Poll::Ready(Ok(StreamResult::Dropped));
        }

        Poll::Ready(Ok(StreamResult::Completed))
    }
}

struct PlaintextProducer<T>(TlsStreamArc<T>);

impl<T, U, D> StreamProducer<D> for PlaintextProducer<T>
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static,
{
    type Item = u8;
    type Buffer = Option<u8>; // unused

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut stream = self.0.lock();
        let TlsStream {
            conn,
            error_tx,
            ciphertext_consumer,
            plaintext_producer,
            ..
        } = stream.as_deref_mut().unwrap();
        if error_tx.is_none() {
            return Poll::Ready(Ok(StreamResult::Dropped));
        }

        let state = conn.process_new_packets().context("unhandled TLS error")?;
        if state.plaintext_bytes_to_read() == 0 {
            if state.peer_has_closed() {
                return Poll::Ready(Ok(StreamResult::Dropped));
            } else if finish {
                return Poll::Ready(Ok(StreamResult::Cancelled));
            }
            *plaintext_producer = Some(cx.waker().clone());
            return Poll::Pending;
        }

        let mut dst = dst.as_direct(store, state.plaintext_bytes_to_read());
        let buf = dst.remaining();
        if buf.is_empty() {
            return Poll::Ready(Ok(StreamResult::Completed));
        }
        let n = conn.reader().read(buf)?;
        debug_assert_ne!(n, 0);
        dst.mark_written(n);
        if conn.wants_read() {
            ciphertext_consumer.take().map(Waker::wake);
        }
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

struct PlaintextConsumer<T, U>(TlsStreamArc<T>)
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static;

impl<T, U> Drop for PlaintextConsumer<T, U>
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static,
{
    fn drop(&mut self) {
        let mut stream = self.0.lock();
        let TlsStream {
            conn,
            close_notify,
            ciphertext_producer,
            ..
        } = stream.as_deref_mut().unwrap();
        conn.send_close_notify();
        *close_notify = true;
        ciphertext_producer.take().map(Waker::wake);
    }
}

impl<T, U, D> StreamConsumer<D> for PlaintextConsumer<T, U>
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static,
    U: 'static,
{
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut stream = self.0.lock();
        let TlsStream {
            conn,
            error_tx,
            ciphertext_producer,
            plaintext_consumer,
            ..
        } = stream.as_deref_mut().unwrap();
        if error_tx.is_none() {
            return Poll::Ready(Ok(StreamResult::Dropped));
        }

        let mut src = src.as_direct(store);
        if src.remaining().is_empty() {
            return Poll::Ready(Ok(StreamResult::Completed));
        }

        let mut dst = conn.writer();
        let n = dst.write(src.remaining())?;
        if n == 0 {
            if finish {
                return Poll::Ready(Ok(StreamResult::Cancelled));
            }
            *plaintext_consumer = Some(cx.waker().clone());
            return Poll::Pending;
        }
        src.mark_read(n);
        dst.flush()?;
        if conn.wants_write() {
            ciphertext_producer.take().map(Waker::wake);
        }
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

struct CiphertextProducer<T>(TlsStreamArc<T>);

impl<T, U, D> StreamProducer<D> for CiphertextProducer<T>
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static,
{
    type Item = u8;
    type Buffer = Option<u8>; // unused

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut stream = self.0.lock();
        let TlsStream {
            conn,
            error_tx,
            close_notify,
            ciphertext_consumer,
            ciphertext_producer,
            plaintext_consumer,
            ..
        } = stream.as_deref_mut().unwrap();
        if error_tx.is_none() {
            return Poll::Ready(Ok(StreamResult::Dropped));
        }

        if !conn.wants_write() {
            if *close_notify {
                return Poll::Ready(Ok(StreamResult::Dropped));
            } else if finish {
                return Poll::Ready(Ok(StreamResult::Cancelled));
            }
            *ciphertext_producer = Some(cx.waker().clone());
            plaintext_consumer.take().map(Waker::wake);
            return Poll::Pending;
        }

        let state = conn.process_new_packets().context("unhandled TLS error")?;
        let mut dst = dst.as_direct(store, state.tls_bytes_to_write());
        if dst.remaining().is_empty() {
            return Poll::Ready(Ok(StreamResult::Completed));
        }
        let n = conn.write_tls(&mut dst)?;
        debug_assert_ne!(n, 0);
        if conn.wants_read() {
            ciphertext_consumer.take().map(Waker::wake);
        }
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

struct ResultProducer(oneshot::Receiver<rustls::Error>);

impl<D> FutureProducer<D> for ResultProducer {
    type Item = Result<(), ()>;

    fn poll_produce(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        _store: StoreContextMut<D>,
        finish: bool,
    ) -> Poll<anyhow::Result<Option<Self::Item>>> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Ready(Ok(_err)) => Poll::Ready(Ok(Some(Err(())))),
            Poll::Ready(Err(..)) => Poll::Ready(Ok(Some(Ok(())))),
            Poll::Pending if finish => Poll::Ready(Ok(None)),
            Poll::Pending => Poll::Pending,
        }
    }
}

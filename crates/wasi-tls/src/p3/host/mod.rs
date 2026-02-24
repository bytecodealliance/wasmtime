use crate::p3::bindings::tls::client::Error;
use crate::p3::{TlsStream, TlsStreamArc, WasiTlsCtxView};
use core::ops::DerefMut;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use std::io::{Read as _, Write as _};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Destination, FutureProducer, Resource, Source, StreamConsumer, StreamProducer, StreamResult,
};

mod client;
mod types;

// Default buffer capacity to use for reads of byte-sized values.
const DEFAULT_BUFFER_CAPACITY: usize = 8192;

struct Pending<T> {
    inner_rx: oneshot::Receiver<T>,
    inner: Option<T>,
}

impl<T> From<oneshot::Receiver<T>> for Pending<T> {
    fn from(rx: oneshot::Receiver<T>) -> Self {
        Self {
            inner_rx: rx,
            inner: None,
        }
    }
}

impl<T, D> StreamProducer<D> for Pending<T>
where
    T: StreamProducer<D> + Unpin,
{
    type Item = <T as StreamProducer<D>>::Item;
    type Buffer = <T as StreamProducer<D>>::Buffer;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        if let Some(ref mut inner) = self.inner {
            return Pin::new(inner).poll_produce(cx, store, dst, finish);
        }
        match Pin::new(&mut self.inner_rx).poll(cx) {
            Poll::Ready(Ok(inner)) => {
                self.inner = Some(inner);
                return self.poll_produce(cx, store, dst, finish);
            }
            Poll::Ready(Err(..)) => Poll::Ready(Ok(StreamResult::Dropped)),
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T, D> StreamConsumer<D> for Pending<T>
where
    T: StreamConsumer<D> + Unpin,
{
    type Item = <T as StreamConsumer<D>>::Item;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        if let Some(ref mut inner) = self.inner {
            return Pin::new(inner).poll_consume(cx, store, src, finish);
        }
        match Pin::new(&mut self.inner_rx).poll(cx) {
            Poll::Ready(Ok(inner)) => {
                self.inner = Some(inner);
                return self.poll_consume(cx, store, src, finish);
            }
            Poll::Ready(Err(..)) => Poll::Ready(Ok(StreamResult::Dropped)),
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct CiphertextConsumer<T> {
    stream: TlsStreamArc<T>,
    error_tx: Arc<Mutex<Option<oneshot::Sender<String>>>>,
    handshake_tx: Option<oneshot::Sender<()>>,
}

impl<T> Drop for CiphertextConsumer<T> {
    fn drop(&mut self) {
        let mut stream = self.stream.lock();
        let TlsStream {
            ciphertext_consumer_dropped,
            plaintext_producer,
            ciphertext_producer,
            ..
        } = stream.as_deref_mut().unwrap();
        *ciphertext_consumer_dropped = true;
        plaintext_producer.take().map(Waker::wake);
        ciphertext_producer.take().map(Waker::wake);
    }
}

impl<T, U, D> StreamConsumer<D> for CiphertextConsumer<T>
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static,
{
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut error_tx = self.error_tx.lock().unwrap();
        if error_tx.is_none() {
            return Poll::Ready(Ok(StreamResult::Dropped));
        }

        let mut stream = self.stream.lock();
        let TlsStream {
            conn,
            ciphertext_consumer,
            ciphertext_producer,
            plaintext_producer,
            ..
        } = stream.as_deref_mut().unwrap();

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

        let state = match conn.process_new_packets() {
            Ok(state) => state,
            Err(err) => {
                _ = error_tx.take().unwrap().send(format!("{err}"));
                ciphertext_producer.take().map(Waker::wake);
                return Poll::Ready(Ok(StreamResult::Dropped));
            }
        };
        if state.plaintext_bytes_to_read() > 0 {
            plaintext_producer.take().map(Waker::wake);
        }
        if state.tls_bytes_to_write() > 0 {
            ciphertext_producer.take().map(Waker::wake);
        }
        let is_handshaking = conn.is_handshaking();
        drop(stream);
        drop(error_tx);
        if !is_handshaking {
            if let Some(tx) = self.handshake_tx.take() {
                _ = tx.send(());
            };
        }
        if state.peer_has_closed() {
            Poll::Ready(Ok(StreamResult::Dropped))
        } else {
            Poll::Ready(Ok(StreamResult::Completed))
        }
    }
}

pub struct PlaintextProducer<T> {
    stream: TlsStreamArc<T>,
    error_tx: Arc<Mutex<Option<oneshot::Sender<String>>>>,
}

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
        let mut error_tx = self.error_tx.lock().unwrap();
        if error_tx.is_none() {
            return Poll::Ready(Ok(StreamResult::Dropped));
        }

        let mut stream = self.stream.lock();
        let TlsStream {
            conn,
            ciphertext_consumer_dropped,
            ciphertext_consumer,
            plaintext_producer,
            ..
        } = stream.as_deref_mut().unwrap();

        let mut dst = dst.as_direct(store, DEFAULT_BUFFER_CAPACITY);
        let buf = dst.remaining();
        if buf.is_empty() {
            return Poll::Ready(Ok(StreamResult::Completed));
        }
        match conn.reader().read(buf) {
            Ok(0) => Poll::Ready(Ok(StreamResult::Dropped)),
            Ok(n) => {
                dst.mark_written(n);
                if conn.wants_read() {
                    ciphertext_consumer.take().map(Waker::wake);
                }
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if *ciphertext_consumer_dropped {
                    return Poll::Ready(Ok(StreamResult::Dropped));
                }
                if finish {
                    return Poll::Ready(Ok(StreamResult::Cancelled));
                }
                *plaintext_producer = Some(cx.waker().clone());
                Poll::Pending
            }
            Err(err) => {
                _ = error_tx.take().unwrap().send(format!("{err}"));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
        }
    }
}

pub struct PlaintextConsumer<T, U>
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static,
{
    stream: TlsStreamArc<T>,
}

impl<T, U> Drop for PlaintextConsumer<T, U>
where
    T: DerefMut<Target = rustls::ConnectionCommon<U>> + Send + 'static,
{
    fn drop(&mut self) {
        let mut stream = self.stream.lock();
        let TlsStream {
            conn,
            plaintext_consumer_dropped,
            ciphertext_producer,
            ..
        } = stream.as_deref_mut().unwrap();
        *plaintext_consumer_dropped = true;
        ciphertext_producer.take().map(Waker::wake);
        conn.send_close_notify();
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
        let mut stream = self.stream.lock();
        let TlsStream {
            conn,
            ciphertext_producer,
            plaintext_consumer,
            ..
        } = stream.as_deref_mut().unwrap();

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

pub struct CiphertextProducer<T> {
    stream: TlsStreamArc<T>,
}

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
        let mut stream = self.stream.lock();
        let TlsStream {
            conn,
            plaintext_consumer_dropped,
            ciphertext_consumer_dropped,
            ciphertext_consumer,
            ciphertext_producer,
            plaintext_consumer,
            ..
        } = stream.as_deref_mut().unwrap();

        if !conn.wants_write() {
            if *plaintext_consumer_dropped && *ciphertext_consumer_dropped {
                return Poll::Ready(Ok(StreamResult::Dropped));
            } else if finish {
                return Poll::Ready(Ok(StreamResult::Cancelled));
            }
            *ciphertext_producer = Some(cx.waker().clone());
            plaintext_consumer.take().map(Waker::wake);
            return Poll::Pending;
        }
        let mut dst = dst.as_direct(store, DEFAULT_BUFFER_CAPACITY);
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

pub struct ResultProducer<T> {
    rx: oneshot::Receiver<String>,
    getter: for<'a> fn(&'a mut T) -> WasiTlsCtxView<'a>,
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
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(err)) => {
                let WasiTlsCtxView { table, .. } = (self.getter)(store.data_mut());
                let err = table.push(err)?;
                Poll::Ready(Ok(Some(Err(err))))
            }
            Poll::Ready(Err(..)) => Poll::Ready(Ok(Some(Ok(())))),
            Poll::Pending if finish => Poll::Ready(Ok(None)),
            Poll::Pending => Poll::Pending,
        }
    }
}

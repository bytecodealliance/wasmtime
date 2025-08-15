use anyhow::Result;
use futures::{
    SinkExt, StreamExt,
    channel::{mpsc, oneshot},
};
use std::thread;
use wasmtime::component::{
    Accessor, Destination, FutureConsumer, FutureProducer, Lift, Lower, Source, StreamConsumer,
    StreamProducer, StreamState,
};

pub async fn sleep(duration: std::time::Duration) {
    if cfg!(miri) {
        // TODO: We should be able to use `tokio::time::sleep` here, but as of
        // this writing the miri-compatible version of `wasmtime-fiber` uses
        // threads behind the scenes, which means thread-local storage is not
        // preserved when we switch fibers, and that confuses Tokio.  If we ever
        // fix that we can stop using our own, special version of `sleep` and
        // switch back to the Tokio version.

        let (tx, rx) = oneshot::channel();
        let handle = thread::spawn(move || {
            thread::sleep(duration);
            _ = tx.send(());
        });
        _ = rx.await;
        _ = handle.join();
    } else {
        tokio::time::sleep(duration).await;
    }
}

pub struct MpscProducer<T> {
    rx: mpsc::Receiver<T>,
    closed: bool,
}

impl<T: Send + Sync + 'static> MpscProducer<T> {
    pub fn new(rx: mpsc::Receiver<T>) -> Self {
        Self { rx, closed: false }
    }

    fn state(&self) -> StreamState {
        if self.closed {
            StreamState::Closed
        } else {
            StreamState::Open
        }
    }
}

impl<D, T: Send + Sync + Lower + 'static> StreamProducer<D, T> for MpscProducer<T> {
    async fn produce(
        &mut self,
        accessor: &Accessor<D>,
        destination: &mut Destination<T>,
    ) -> Result<StreamState> {
        if let Some(item) = self.rx.next().await {
            let item = destination.write(accessor, Some(item)).await?;
            assert!(item.is_none());
        } else {
            self.closed = true;
        }

        Ok(self.state())
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> Result<StreamState> {
        Ok(self.state())
    }
}

pub struct MpscConsumer<T> {
    tx: mpsc::Sender<T>,
}

impl<T> MpscConsumer<T> {
    pub fn new(tx: mpsc::Sender<T>) -> Self {
        Self { tx }
    }

    fn state(&self) -> StreamState {
        if self.tx.is_closed() {
            StreamState::Closed
        } else {
            StreamState::Open
        }
    }
}

impl<D, T: Lift + 'static> StreamConsumer<D, T> for MpscConsumer<T> {
    async fn consume(
        &mut self,
        accessor: &Accessor<D>,
        source: &mut Source<'_, T>,
    ) -> Result<StreamState> {
        let item = &mut None;
        accessor.with(|access| source.read(access, item))?;
        _ = self.tx.send(item.take().unwrap()).await;
        Ok(self.state())
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> Result<StreamState> {
        Ok(self.state())
    }
}

pub struct OneshotProducer<T> {
    rx: oneshot::Receiver<T>,
}

impl<T> OneshotProducer<T> {
    pub fn new(rx: oneshot::Receiver<T>) -> Self {
        Self { rx }
    }
}

impl<D, T: Send + 'static> FutureProducer<D, T> for OneshotProducer<T> {
    async fn produce(self, _: &Accessor<D>) -> Result<T> {
        Ok(self.rx.await?)
    }
}

pub struct OneshotConsumer<T> {
    tx: oneshot::Sender<T>,
}

impl<T> OneshotConsumer<T> {
    pub fn new(tx: oneshot::Sender<T>) -> Self {
        Self { tx }
    }
}

impl<D, T: Send + 'static> FutureConsumer<D, T> for OneshotConsumer<T> {
    async fn consume(self, _: &Accessor<D>, value: T) -> Result<()> {
        _ = self.tx.send(value);
        Ok(())
    }
}

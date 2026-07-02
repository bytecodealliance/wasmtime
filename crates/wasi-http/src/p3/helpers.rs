use std::pin::Pin;
use std::task::{Context, Poll};
use wasmtime::component::{FutureConsumer, FutureReader, Lift, Source};
use wasmtime::error::Context as _;
use wasmtime::{AsContextMut, StoreContextMut};

/// Extension methosd for `FutureReader`
pub trait FutureReaderExt<T> {
    /// Get the underlying `FutureReader`.
    fn as_future_reader(self) -> FutureReader<T>;

    /// Run `cb` with the result of this future when it's ready.
    ///
    /// The `cb` is given the store's data-at-the-time, the result of the
    /// future, and can produce a trapping error if so desirable.
    fn pipe_cb<S>(
        self,
        store: S,
        cb: impl FnOnce(&mut S::Data, T) -> wasmtime::Result<()> + Unpin + Send + 'static,
    ) -> wasmtime::Result<()>
    where
        Self: Sized,
        S: AsContextMut,
        T: Lift + 'static,
    {
        struct Consumer<F, D, T> {
            cb: Option<F>,
            _marker: std::marker::PhantomData<fn(D, T)>,
        }

        impl<T, D, F> FutureConsumer<D> for Consumer<F, D, T>
        where
            T: Lift + 'static,
            F: FnOnce(&mut D, T) -> wasmtime::Result<()> + Send + Unpin + 'static,
            D: 'static,
        {
            type Item = T;

            fn poll_consume(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
                mut store: StoreContextMut<D>,
                mut src: Source<'_, Self::Item>,
                _: bool,
            ) -> Poll<wasmtime::Result<()>> {
                let mut res = None;
                src.read(&mut store, &mut res)
                    .context("failed to read result")?;
                let res = res.context("result value missing")?;
                let cb = self.cb.take().context("polled after returning `Ready`")?;
                cb(store.data_mut(), res)?;
                Poll::Ready(Ok(()))
            }
        }

        self.as_future_reader().pipe(
            store,
            Consumer {
                cb: Some(cb),
                _marker: std::marker::PhantomData,
            },
        )
    }
}

impl<T> FutureReaderExt<T> for FutureReader<T> {
    fn as_future_reader(self) -> FutureReader<T> {
        self
    }
}

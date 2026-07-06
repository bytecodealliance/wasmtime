use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use wasmtime::component::{FutureConsumer, FutureProducer, FutureReader, Lift, Lower, Source};
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

    /// Creates a new `FutureReader<T>` which is the combination of waiting for
    /// `future` to resolved followed by invoking the `cb` provided.
    ///
    /// Note that `cb` gets access to the store's data.
    fn new_cb<U, S, E>(
        store: S,
        future: impl Future<Output = Result<U, E>> + Send + 'static,
        cb: impl FnOnce(&mut S::Data, U) -> T + Send + 'static,
    ) -> wasmtime::Result<FutureReader<T>>
    where
        S: AsContextMut,
        E: Into<wasmtime::Error>,
        T: Lift + Lower + 'static,
    {
        pin_project! {
            struct Producer<F, C, D, T> {
                cb: Option<C>,
                #[pin]
                future: F,
                _marker: std::marker::PhantomData<fn(D, T)>,
            }
        }

        impl<F, C, D, T, U, E> FutureProducer<D> for Producer<F, C, D, T>
        where
            F: Future<Output = Result<U, E>> + Send + 'static,
            C: FnOnce(&mut D, U) -> T + Send + 'static,
            D: 'static,
            T: 'static,
            E: Into<wasmtime::Error>,
        {
            type Item = T;

            fn poll_produce(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                mut store: StoreContextMut<D>,
                finish: bool,
            ) -> Poll<wasmtime::Result<Option<T>>> {
                let this = self.project();
                let res = match this.future.poll(cx) {
                    Poll::Ready(Ok(res)) => res,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                    Poll::Pending => {
                        return if finish {
                            Poll::Ready(Ok(None))
                        } else {
                            Poll::Pending
                        };
                    }
                };
                let cb = this.cb.take().context("polled after returning `Ready`")?;
                let res = cb(store.data_mut(), res);
                Poll::Ready(Ok(Some(res)))
            }
        }

        FutureReader::new(
            store,
            Producer::<_, _, S::Data, T> {
                cb: Some(cb),
                future,
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

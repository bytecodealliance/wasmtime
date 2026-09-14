use crate::I32Exit;
use crate::cli::{IsTerminal, WasiCli, WasiCliCtxView};
use crate::p3::DEFAULT_BUFFER_CAPACITY;
use crate::p3::bindings::cli::types::ErrorCode;
use crate::p3::bindings::cli::{
    environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
    terminal_stdin, terminal_stdout,
};
use crate::p3::cli::{TerminalInput, TerminalOutput};
use bytes::BytesMut;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::io;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::oneshot;
use wasmtime::AsContextMut;
use wasmtime::component::{
    Access, Destination, FutureReader, Resource, Source, StreamConsumer, StreamProducer,
    StreamReader, StreamResult,
};
use wasmtime::{StoreContextMut, error::Context as _, format_err};

struct InputStreamProducer {
    rx: Pin<Box<dyn AsyncRead + Send + Sync>>,
    result_tx: Option<oneshot::Sender<ErrorCode>>,
}

fn io_error_to_error_code(err: io::Error) -> ErrorCode {
    match err.kind() {
        io::ErrorKind::BrokenPipe => ErrorCode::Pipe,
        other => {
            tracing::warn!("stdio error: {other}");
            ErrorCode::Io
        }
    }
}

impl<D> StreamProducer<D> for InputStreamProducer {
    type Item = u8;
    type Buffer = BytesMut;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        // If the destination buffer is empty then this is a request on
        // behalf of the guest to wait for this input stream to be readable.
        // The `AsyncRead` trait abstraction does not provide the ability to
        // await this event so we're forced to basically just lie here and
        // say we're ready read data later.
        //
        // See WebAssembly/component-model#561 for some more information.
        if dst.remaining(store.as_context_mut()) == Some(0) {
            return Poll::Ready(Ok(StreamResult::Completed));
        }

        let mut dst = dst.as_direct(store, DEFAULT_BUFFER_CAPACITY);
        let mut buf = ReadBuf::new(dst.remaining());
        match self.rx.as_mut().poll_read(cx, &mut buf) {
            Poll::Ready(Ok(())) if buf.filled().is_empty() => {
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Ready(Ok(())) => {
                let n = buf.filled().len();
                dst.mark_written(n);
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(e)) => {
                let _ = self
                    .result_tx
                    .take()
                    .unwrap()
                    .send(io_error_to_error_code(e));
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct OutputStreamConsumer {
    tx: Pin<Box<dyn AsyncWrite + Send + Sync>>,
    result_tx: Option<oneshot::Sender<ErrorCode>>,
    flush_pending: bool,
}

impl OutputStreamConsumer {
    fn poll_flush(
        &mut self,
        cx: &mut Context<'_>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        match self.tx.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => {
                self.flush_pending = false;
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(e)) => self.dropped(e),
            Poll::Pending => {
                if finish {
                    self.flush_pending = false;
                    Poll::Ready(Ok(StreamResult::Cancelled))
                } else {
                    self.flush_pending = true;
                    Poll::Pending
                }
            }
        }
    }

    fn dropped(&mut self, err: io::Error) -> Poll<wasmtime::Result<StreamResult>> {
        if let Some(tx) = self.result_tx.take() {
            let _ = tx.send(io_error_to_error_code(err));
        }
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

impl<D> StreamConsumer<D> for OutputStreamConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        if self.flush_pending {
            return self.poll_flush(cx, finish);
        }

        let mut src = src.as_direct(store);
        let buf = src.remaining();

        // If the source buffer is empty then this is a request on behalf of
        // the guest to wait for this output stream to be writable. The
        // `AsyncWrite` trait abstraction does not provide the ability to await
        // this event so we're forced to basically just lie here and say we're
        // ready write data later.
        //
        // See WebAssembly/component-model#561 for some more information.
        if buf.len() == 0 {
            return Poll::Ready(Ok(StreamResult::Completed));
        }
        match self.tx.as_mut().poll_write(cx, buf) {
            Poll::Ready(Ok(0)) => self.dropped(io::ErrorKind::WriteZero.into()),
            Poll::Ready(Ok(n)) => {
                src.mark_read(n);
                self.poll_flush(cx, finish)
            }
            Poll::Ready(Err(e)) => self.dropped(e),
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl terminal_input::Host for WasiCliCtxView<'_> {}
impl terminal_output::Host for WasiCliCtxView<'_> {}

impl terminal_input::HostTerminalInput for WasiCliCtxView<'_> {
    fn drop(&mut self, rep: Resource<TerminalInput>) -> wasmtime::Result<()> {
        self.table
            .delete(rep)
            .context("failed to delete terminal input resource from table")?;
        Ok(())
    }
}

impl terminal_output::HostTerminalOutput for WasiCliCtxView<'_> {
    fn drop(&mut self, rep: Resource<TerminalOutput>) -> wasmtime::Result<()> {
        self.table
            .delete(rep)
            .context("failed to delete terminal output resource from table")?;
        Ok(())
    }
}

impl terminal_stdin::Host for WasiCliCtxView<'_> {
    fn get_terminal_stdin(&mut self) -> wasmtime::Result<Option<Resource<TerminalInput>>> {
        if self.ctx.stdin.is_terminal() {
            let fd = self
                .table
                .push(TerminalInput)
                .context("failed to push terminal stdin resource to table")?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

impl terminal_stdout::Host for WasiCliCtxView<'_> {
    fn get_terminal_stdout(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx.stdout.is_terminal() {
            let fd = self
                .table
                .push(TerminalOutput)
                .context("failed to push terminal stdout resource to table")?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

impl terminal_stderr::Host for WasiCliCtxView<'_> {
    fn get_terminal_stderr(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx.stderr.is_terminal() {
            let fd = self
                .table
                .push(TerminalOutput)
                .context("failed to push terminal stderr resource to table")?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

fn read_stdin(
    mut store: impl AsContextMut,
    stdin: Box<dyn AsyncRead + Send + Sync>,
) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
    let mut store = store.as_context_mut();
    let (result_tx, result_rx) = oneshot::channel();
    let stream = StreamReader::new(
        &mut store,
        InputStreamProducer {
            rx: Box::into_pin(stdin),
            result_tx: Some(result_tx),
        },
    )?;
    let future = FutureReader::new(&mut store, async {
        wasmtime::error::Ok(match result_rx.await {
            Ok(err) => Err(err),
            Err(_) => Ok(()),
        })
    })?;
    Ok((stream, future))
}

impl<U> stdin::HostWithStore<U> for WasiCli {
    fn read_via_stream(
        mut store: Access<U, Self>,
    ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
        let rx = store.get().ctx.stdin.async_stream();
        read_stdin(&mut store, rx)
    }
}

impl stdin::Host for WasiCliCtxView<'_> {}

fn write_output(
    mut store: impl AsContextMut,
    data: StreamReader<u8>,
    writer: Box<dyn AsyncWrite + Send + Sync>,
) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
    let (result_tx, result_rx) = oneshot::channel();
    data.pipe(
        &mut store,
        OutputStreamConsumer {
            tx: Box::into_pin(writer),
            result_tx: Some(result_tx),
            flush_pending: false,
        },
    )?;
    FutureReader::new(&mut store, async {
        wasmtime::error::Ok(match result_rx.await {
            Ok(err) => Err(err),
            Err(_) => Ok(()),
        })
    })
}

impl<U> stdout::HostWithStore<U> for WasiCli {
    fn write_via_stream(
        mut store: Access<'_, U, Self>,
        data: StreamReader<u8>,
    ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
        let tx = store.get().ctx.stdout.async_stream();
        write_output(store, data, tx)
    }
}

impl stdout::Host for WasiCliCtxView<'_> {}

impl<U> stderr::HostWithStore<U> for WasiCli {
    fn write_via_stream(
        mut store: Access<'_, U, Self>,
        data: StreamReader<u8>,
    ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
        let tx = store.get().ctx.stderr.async_stream();
        write_output(store, data, tx)
    }
}

impl stderr::Host for WasiCliCtxView<'_> {}

impl environment::Host for WasiCliCtxView<'_> {
    fn get_environment(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
        Ok(self.ctx.environment.clone())
    }

    fn get_arguments(&mut self) -> wasmtime::Result<Vec<String>> {
        Ok(self.ctx.arguments.clone())
    }

    fn get_initial_cwd(&mut self) -> wasmtime::Result<Option<String>> {
        Ok(self.ctx.initial_cwd.clone())
    }
}

impl exit::Host for WasiCliCtxView<'_> {
    fn exit(&mut self, status: Result<(), ()>) -> wasmtime::Result<()> {
        let status = match status {
            Ok(()) => 0,
            Err(()) => 1,
        };
        Err(format_err!(I32Exit(status)))
    }

    fn exit_with_code(&mut self, status_code: u8) -> wasmtime::Result<()> {
        Err(format_err!(I32Exit(status_code.into())))
    }
}

mod named {
    use crate::cli::{WasiCliNamed, WasiCliNamedView};
    use crate::p3::bindings::cli::types::ErrorCode;
    use crate::p3::bindings::named_imports::wasi::cli::{
        environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
        terminal_stdin, terminal_stdout,
    };
    use crate::p3::cli::{TerminalInput, TerminalOutput};
    use crate::{NamedId, WasiCtxNamedView};
    use wasmtime::component::{Access, FutureReader, Resource, StreamReader};

    impl<T> exit::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn exit(&mut self, id: NamedId, status: Result<(), ()>) -> wasmtime::Result<()> {
            super::exit::Host::exit(&mut self.0.cli(id), status)
        }

        fn exit_with_code(&mut self, id: NamedId, status_code: u8) -> wasmtime::Result<()> {
            super::exit::Host::exit_with_code(&mut self.0.cli(id), status_code)
        }
    }

    impl<T> terminal_input::Host for WasiCtxNamedView<'_, T> where T: WasiCliNamedView {}
    impl<T> terminal_output::Host for WasiCtxNamedView<'_, T> where T: WasiCliNamedView {}

    impl<T> terminal_input::HostTerminalInput for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn drop(&mut self, id: NamedId, rep: Resource<TerminalInput>) -> wasmtime::Result<()> {
            super::terminal_input::HostTerminalInput::drop(&mut self.0.cli(id), rep)
        }
    }

    impl<T> terminal_output::HostTerminalOutput for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn drop(&mut self, id: NamedId, rep: Resource<TerminalOutput>) -> wasmtime::Result<()> {
            super::terminal_output::HostTerminalOutput::drop(&mut self.0.cli(id), rep)
        }
    }

    impl<T> terminal_stdin::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_terminal_stdin(
            &mut self,
            id: NamedId,
        ) -> wasmtime::Result<Option<Resource<TerminalInput>>> {
            super::terminal_stdin::Host::get_terminal_stdin(&mut self.0.cli(id))
        }
    }

    impl<T> terminal_stdout::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_terminal_stdout(
            &mut self,
            id: NamedId,
        ) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
            super::terminal_stdout::Host::get_terminal_stdout(&mut self.0.cli(id))
        }
    }

    impl<T> terminal_stderr::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_terminal_stderr(
            &mut self,
            id: NamedId,
        ) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
            super::terminal_stderr::Host::get_terminal_stderr(&mut self.0.cli(id))
        }
    }

    impl<T, U> stdin::HostWithStore<U> for WasiCliNamed<T>
    where
        T: WasiCliNamedView,
    {
        fn read_via_stream(
            mut store: Access<U, Self>,
            id: NamedId,
        ) -> wasmtime::Result<(StreamReader<u8>, FutureReader<Result<(), ErrorCode>>)> {
            let rx = store.get().0.cli(id).ctx.stdin.async_stream();
            super::read_stdin(&mut store, rx)
        }
    }

    impl<T> stdin::Host for WasiCtxNamedView<'_, T> where T: WasiCliNamedView {}

    impl<T, U> stdout::HostWithStore<U> for WasiCliNamed<T>
    where
        T: WasiCliNamedView,
    {
        fn write_via_stream(
            mut store: Access<'_, U, Self>,
            id: NamedId,
            data: StreamReader<u8>,
        ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
            let tx = store.get().0.cli(id).ctx.stdout.async_stream();
            super::write_output(store, data, tx)
        }
    }

    impl<T> stdout::Host for WasiCtxNamedView<'_, T> where T: WasiCliNamedView {}

    impl<T, U> stderr::HostWithStore<U> for WasiCliNamed<T>
    where
        T: WasiCliNamedView,
    {
        fn write_via_stream(
            mut store: Access<'_, U, Self>,
            id: NamedId,
            data: StreamReader<u8>,
        ) -> wasmtime::Result<FutureReader<Result<(), ErrorCode>>> {
            let tx = store.get().0.cli(id).ctx.stderr.async_stream();
            super::write_output(store, data, tx)
        }
    }

    impl<T> stderr::Host for WasiCtxNamedView<'_, T> where T: WasiCliNamedView {}

    impl<T> environment::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiCliNamedView,
    {
        fn get_environment(&mut self, id: NamedId) -> wasmtime::Result<Vec<(String, String)>> {
            super::environment::Host::get_environment(&mut self.0.cli(id))
        }

        fn get_arguments(&mut self, id: NamedId) -> wasmtime::Result<Vec<String>> {
            super::environment::Host::get_arguments(&mut self.0.cli(id))
        }

        fn get_initial_cwd(&mut self, id: NamedId) -> wasmtime::Result<Option<String>> {
            super::environment::Host::get_initial_cwd(&mut self.0.cli(id))
        }
    }
}

use crate::I32Exit;
use crate::cli::{IsTerminal, WasiCli, WasiCliCtxView};
use crate::p3::bindings::cli::{
    environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
    terminal_stdin, terminal_stdout,
};
use crate::p3::cli::{TerminalInput, TerminalOutput};
use crate::p3::{DEFAULT_BUFFER_CAPACITY, MAX_BUFFER_CAPACITY};
use anyhow::{Context as _, anyhow};
use bytes::BytesMut;
use std::io::Cursor;
use std::pin::Pin;
use std::task::{self, Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, ReadBuf};
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Accessor, Destination, Resource, Source, StreamConsumer, StreamProducer, StreamReader,
    StreamResult,
};

struct InputStreamProducer<T> {
    rx: T,
}

impl<T, D> StreamProducer<D> for InputStreamProducer<T>
where
    T: AsyncRead + Send + Unpin + 'static,
{
    type Item = u8;
    type Buffer = Cursor<BytesMut>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        destination: &'a mut Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        if finish {
            return Poll::Ready(Ok(StreamResult::Cancelled));
        }

        let me = self.get_mut();

        Poll::Ready(Ok(
            if let Some(mut destination) = destination.as_direct_destination(store)
                && !destination.remaining().is_empty()
            {
                let mut buffer = ReadBuf::new(destination.remaining());
                match task::ready!(Pin::new(&mut me.rx).poll_read(cx, &mut buffer)) {
                    Ok(()) => {
                        if buffer.filled().is_empty() {
                            StreamResult::Dropped
                        } else {
                            let count = buffer.filled().len();
                            destination.mark_written(count);
                            StreamResult::Completed
                        }
                    }
                    Err(_) => {
                        // TODO: Report the error to the guest
                        StreamResult::Dropped
                    }
                }
            } else {
                let capacity = destination
                    .remaining(store)
                    .unwrap_or(DEFAULT_BUFFER_CAPACITY)
                    // In the case of small or zero-length reads, we read more than
                    // was asked for; this will save the runtime from having to
                    // block or call `poll_produce` on subsequent reads.  See the
                    // documentation for `StreamProducer::poll_produce` for details.
                    .max(DEFAULT_BUFFER_CAPACITY)
                    .min(MAX_BUFFER_CAPACITY);

                let mut buffer = destination.take_buffer().into_inner();
                buffer.clear();
                buffer.reserve(capacity);

                let mut readbuf = ReadBuf::uninit(buffer.spare_capacity_mut());
                let result = Pin::new(&mut me.rx).poll_read(cx, &mut readbuf);
                let count = readbuf.filled().len();
                // SAFETY: `ReadyBuf::filled` promised us `count` bytes have
                // been initialized.
                unsafe {
                    buffer.set_len(count);
                }

                destination.set_buffer(Cursor::new(buffer));

                match task::ready!(result) {
                    Ok(()) => {
                        if count == 0 {
                            StreamResult::Dropped
                        } else {
                            StreamResult::Completed
                        }
                    }
                    Err(_) => {
                        // TODO: Report the error to the guest
                        StreamResult::Dropped
                    }
                }
            },
        ))
    }
}

struct OutputStreamConsumer<T> {
    tx: T,
}

impl<T, D> StreamConsumer<D> for OutputStreamConsumer<T>
where
    T: AsyncWrite + Send + Unpin + 'static,
{
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<D>,
        source: &mut Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let me = self.get_mut();

        let mut source = source.as_direct_source(store);

        let (mut count, mut result) = if !source.remaining().is_empty() {
            match task::ready!(Pin::new(&mut me.tx).poll_write(cx, source.remaining())) {
                Ok(count) => (count, StreamResult::Completed),
                Err(_) => {
                    // TODO: Report the error to the guest
                    (0, StreamResult::Dropped)
                }
            }
        } else {
            (0, StreamResult::Completed)
        };

        if task::ready!(Pin::new(&mut me.tx).poll_flush(cx)).is_err() {
            // TODO: Report the error to the guest
            count = 0;
            result = StreamResult::Dropped;
        }

        if count > 0 {
            source.mark_read(count);
        }

        Poll::Ready(Ok(result))
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

impl stdin::HostWithStore for WasiCli {
    async fn get_stdin<U>(store: &Accessor<U, Self>) -> wasmtime::Result<StreamReader<u8>> {
        let instance = store.instance();
        store.with(|mut store| {
            let rx = store.get().ctx.stdin.async_stream();
            Ok(StreamReader::new(
                instance,
                &mut store,
                InputStreamProducer {
                    rx: Box::into_pin(rx),
                },
            ))
        })
    }
}

impl stdin::Host for WasiCliCtxView<'_> {}

impl stdout::HostWithStore for WasiCli {
    async fn set_stdout<U>(
        store: &Accessor<U, Self>,
        data: StreamReader<u8>,
    ) -> wasmtime::Result<()> {
        store.with(|mut store| {
            let tx = store.get().ctx.stdout.async_stream();
            data.pipe(
                store,
                OutputStreamConsumer {
                    tx: Box::into_pin(tx),
                },
            );
            Ok(())
        })
    }
}

impl stdout::Host for WasiCliCtxView<'_> {}

impl stderr::HostWithStore for WasiCli {
    async fn set_stderr<U>(
        store: &Accessor<U, Self>,
        data: StreamReader<u8>,
    ) -> wasmtime::Result<()> {
        store.with(|mut store| {
            let tx = store.get().ctx.stderr.async_stream();
            data.pipe(
                store,
                OutputStreamConsumer {
                    tx: Box::into_pin(tx),
                },
            );
            Ok(())
        })
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
        Err(anyhow!(I32Exit(status)))
    }

    fn exit_with_code(&mut self, status_code: u8) -> wasmtime::Result<()> {
        Err(anyhow!(I32Exit(status_code.into())))
    }
}

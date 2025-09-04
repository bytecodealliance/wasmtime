use crate::I32Exit;
use crate::cli::{IsTerminal, WasiCli, WasiCliCtxView};
use crate::p3::DEFAULT_BUFFER_CAPACITY;
use crate::p3::bindings::cli::{
    environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
    terminal_stdin, terminal_stdout,
};
use crate::p3::cli::{TerminalInput, TerminalOutput};
use anyhow::{Context as _, anyhow};
use bytes::BytesMut;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::io::Cursor;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use wasmtime::StoreContextMut;
use wasmtime::component::{
    Accessor, Destination, Resource, Source, StreamConsumer, StreamProducer, StreamReader,
    StreamResult,
};

struct InputStreamProducer {
    rx: Pin<Box<dyn AsyncRead + Send + Sync>>,
}

impl<D> StreamProducer<D> for InputStreamProducer {
    type Item = u8;
    type Buffer = Cursor<BytesMut>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        dst: &'a mut Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        if let Some(mut dst) = dst.as_direct_destination(store) {
            if !dst.remaining().is_empty() {
                let mut buf = ReadBuf::new(dst.remaining());
                match self.rx.as_mut().poll_read(cx, &mut buf) {
                    Poll::Ready(Ok(())) if buf.filled().is_empty() => {
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    }
                    Poll::Ready(Ok(())) => {
                        let n = buf.filled().len();
                        dst.mark_written(n);
                        return Poll::Ready(Ok(StreamResult::Completed));
                    }
                    Poll::Ready(Err(..)) => {
                        // TODO: Report the error to the guest
                        return Poll::Ready(Ok(StreamResult::Dropped));
                    }
                    Poll::Pending if finish => return Poll::Ready(Ok(StreamResult::Cancelled)),
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
        let mut buf = dst.take_buffer().into_inner();
        buf.clear();
        buf.reserve(DEFAULT_BUFFER_CAPACITY);
        let mut rbuf = ReadBuf::uninit(buf.spare_capacity_mut());
        match self.rx.as_mut().poll_read(cx, &mut rbuf) {
            Poll::Ready(Ok(())) if rbuf.filled().is_empty() => {
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Ready(Ok(())) => {
                let n = rbuf.filled().len();
                // SAFETY: `ReadyBuf::filled` promised us `count` bytes have
                // been initialized.
                unsafe { buf.set_len(n) };
                dst.set_buffer(Cursor::new(buf));
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(..)) => {
                // TODO: Report the error to the guest
                Poll::Ready(Ok(StreamResult::Dropped))
            }
            Poll::Pending if finish => Poll::Ready(Ok(StreamResult::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct OutputStreamConsumer {
    tx: Pin<Box<dyn AsyncWrite + Send + Sync>>,
}

impl<D> StreamConsumer<D> for OutputStreamConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        src: &mut Source<Self::Item>,
        finish: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut src = src.as_direct_source(store);
        let buf = src.remaining();
        match self.tx.as_mut().poll_write(cx, buf) {
            Poll::Ready(Ok(n)) if buf.is_empty() => {
                debug_assert_eq!(n, 0);
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Ok(n)) => {
                src.mark_read(n);
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(Err(..)) => {
                // TODO: Report the error to the guest
                Poll::Ready(Ok(StreamResult::Dropped))
            }
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

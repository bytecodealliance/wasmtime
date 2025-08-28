use crate::I32Exit;
use crate::cli::{IsTerminal, WasiCli, WasiCliCtxView};
use crate::p3::bindings::cli::{
    environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
    terminal_stdin, terminal_stdout,
};
use crate::p3::cli::{TerminalInput, TerminalOutput};
use crate::p3::write_buffered_bytes;
use crate::p3::{DEFAULT_BUFFER_CAPACITY, MAX_BUFFER_CAPACITY};
use anyhow::{Context as _, anyhow};
use bytes::BytesMut;
use std::io::Cursor;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use wasmtime::component::{
    Accessor, Destination, Resource, Source, StreamConsumer, StreamProducer, StreamReader,
    StreamState,
};

struct InputStreamProducer<T> {
    rx: T,
    buffer: Cursor<BytesMut>,
}

impl<T> InputStreamProducer<T>
where
    T: AsyncRead + Send + Unpin,
{
    async fn read(&mut self, n: usize) -> StreamState {
        self.buffer.get_mut().reserve(n);
        match self.rx.read_buf(self.buffer.get_mut()).await {
            Ok(0) => StreamState::Closed,
            Ok(_) => StreamState::Open,
            Err(_err) => {
                // TODO: Report the error to the guest
                StreamState::Closed
            }
        }
    }
}

impl<T, D> StreamProducer<D, u8> for InputStreamProducer<T>
where
    T: AsyncRead + Send + Unpin + 'static,
{
    async fn produce(
        &mut self,
        store: &Accessor<D>,
        dst: &mut Destination<u8>,
    ) -> wasmtime::Result<StreamState> {
        if !self.buffer.get_ref().is_empty() {
            write_buffered_bytes(store, &mut self.buffer, dst).await?;
            return Ok(StreamState::Open);
        }
        let n = store
            .with(|store| dst.remaining(store))
            .unwrap_or(DEFAULT_BUFFER_CAPACITY)
            .min(MAX_BUFFER_CAPACITY);
        match self.read(n).await {
            StreamState::Open => {
                write_buffered_bytes(store, &mut self.buffer, dst).await?;
                Ok(StreamState::Open)
            }
            StreamState::Closed => Ok(StreamState::Closed),
        }
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if !self.buffer.get_ref().is_empty() {
            return Ok(StreamState::Open);
        }
        Ok(self.read(DEFAULT_BUFFER_CAPACITY).await)
    }
}

struct OutputStreamConsumer<T> {
    tx: T,
    buffer: BytesMut,
}

impl<T> OutputStreamConsumer<T>
where
    T: AsyncWrite + Send + Unpin + 'static,
{
    async fn flush(&mut self) -> StreamState {
        match self.tx.write_all(&self.buffer).await {
            Ok(()) => {
                self.buffer.clear();
                StreamState::Open
            }
            Err(_err) => {
                // TODO: Report the error to the guest
                StreamState::Closed
            }
        }
    }
}

impl<T, D> StreamConsumer<D, u8> for OutputStreamConsumer<T>
where
    T: AsyncWrite + Send + Unpin + 'static,
{
    async fn consume(
        &mut self,
        store: &Accessor<D>,
        src: &mut Source<'_, u8>,
    ) -> wasmtime::Result<StreamState> {
        store.with(|mut store| {
            let n = src.remaining(&mut store).min(MAX_BUFFER_CAPACITY);
            self.buffer.reserve(n);
            src.read(&mut store, &mut self.buffer)
        })?;
        Ok(self.flush().await)
    }

    async fn when_ready(&mut self, _: &Accessor<D>) -> wasmtime::Result<StreamState> {
        if !self.buffer.is_empty() {
            return Ok(self.flush().await);
        }
        Ok(StreamState::Open)
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
                    buffer: Cursor::default(),
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
                    buffer: BytesMut::default(),
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
                    buffer: BytesMut::default(),
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

use crate::cli::{IsTerminal, WasiCliImpl, WasiCliView};
use crate::p3::bindings::cli::{
    environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
    terminal_stdin, terminal_stdout,
};
use crate::p3::cli::{InputStream, OutputStream, TerminalInput, TerminalOutput, WasiCli};
use crate::{I32Exit, ResourceView as _};
use anyhow::{Context as _, anyhow};
use bytes::BytesMut;
use std::io::Cursor;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use wasmtime::component::{
    Accessor, AccessorTask, HostStream, Resource, StreamReader, StreamWriter,
};

struct InputTask<T> {
    rx: T,
    tx: StreamWriter<Cursor<BytesMut>>,
}

impl<T, U, V> AccessorTask<T, WasiCli<U>, wasmtime::Result<()>> for InputTask<V>
where
    U: 'static,
    V: AsyncRead + Send + Sync + Unpin + 'static,
{
    async fn run(mut self, store: &Accessor<T, WasiCli<U>>) -> wasmtime::Result<()> {
        let mut buf = BytesMut::with_capacity(8192);
        let mut tx = self.tx;
        loop {
            match self.rx.read_buf(&mut buf).await {
                Ok(0) => return Ok(()),
                Ok(_) => {
                    let (Some(tx_next), buf_next) = tx.write_all(store, Cursor::new(buf)).await
                    else {
                        break Ok(());
                    };
                    tx = tx_next;
                    buf = buf_next.into_inner();
                    buf.clear();
                }
                Err(_err) => {
                    // TODO: Report the error to the guest
                    return Ok(());
                }
            }
        }
    }
}

struct OutputTask<T> {
    rx: StreamReader<BytesMut>,
    tx: T,
}

impl<T, U, V> AccessorTask<T, WasiCli<U>, wasmtime::Result<()>> for OutputTask<V>
where
    U: 'static,
    V: AsyncWrite + Send + Sync + Unpin + 'static,
{
    async fn run(mut self, store: &Accessor<T, WasiCli<U>>) -> wasmtime::Result<()> {
        let mut buf = BytesMut::with_capacity(8192);
        let mut rx = self.rx;
        while let (Some(rx_next), buf_next) = rx.read(store, buf).await {
            buf = buf_next;
            rx = rx_next;
            match self.tx.write_all(&buf).await {
                Ok(()) => {
                    buf.clear();
                    continue;
                }
                Err(_err) => {
                    // TODO: Report the error to the guest
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

impl<T> terminal_input::Host for WasiCliImpl<T> where T: WasiCliView {}
impl<T> terminal_output::Host for WasiCliImpl<T> where T: WasiCliView {}

impl<T> terminal_input::HostTerminalInput for WasiCliImpl<T>
where
    T: WasiCliView,
{
    fn drop(&mut self, rep: Resource<TerminalInput>) -> wasmtime::Result<()> {
        self.table()
            .delete(rep)
            .context("failed to delete terminal input resource from table")?;
        Ok(())
    }
}

impl<T> terminal_output::HostTerminalOutput for WasiCliImpl<T>
where
    T: WasiCliView,
{
    fn drop(&mut self, rep: Resource<TerminalOutput>) -> wasmtime::Result<()> {
        self.table()
            .delete(rep)
            .context("failed to delete terminal output resource from table")?;
        Ok(())
    }
}

impl<T> terminal_stdin::Host for WasiCliImpl<T>
where
    T: WasiCliView,
{
    fn get_terminal_stdin(&mut self) -> wasmtime::Result<Option<Resource<TerminalInput>>> {
        if self.cli().stdin.is_terminal() {
            let fd = self
                .table()
                .push(TerminalInput)
                .context("failed to push terminal stdin resource to table")?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

impl<T> terminal_stdout::Host for WasiCliImpl<T>
where
    T: WasiCliView,
{
    fn get_terminal_stdout(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
        if self.cli().stdout.is_terminal() {
            let fd = self
                .table()
                .push(TerminalOutput)
                .context("failed to push terminal stdout resource to table")?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

impl<T> terminal_stderr::Host for WasiCliImpl<T>
where
    T: WasiCliView,
{
    fn get_terminal_stderr(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
        if self.cli().stderr.is_terminal() {
            let fd = self
                .table()
                .push(TerminalOutput)
                .context("failed to push terminal stderr resource to table")?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

impl<T> stdin::HostConcurrent for WasiCli<T>
where
    T: WasiCliView + 'static,
    T::InputStream: InputStream,
{
    async fn get_stdin<U>(store: &Accessor<U, Self>) -> wasmtime::Result<HostStream<u8>> {
        store.with(|mut view| {
            let instance = view.instance();
            let (tx, rx) = instance
                .stream::<_, _, BytesMut>(&mut view)
                .context("failed to create stream")?;
            let stdin = view.get().cli().stdin.reader();
            view.spawn(InputTask { rx: stdin, tx });
            Ok(rx.into())
        })
    }
}

impl<T> stdin::Host for WasiCliImpl<T> where T: WasiCliView {}

impl<T> stdout::HostConcurrent for WasiCli<T>
where
    T: WasiCliView + 'static,
    T::OutputStream: OutputStream,
{
    async fn set_stdout<U>(
        store: &Accessor<U, Self>,
        data: HostStream<u8>,
    ) -> wasmtime::Result<()> {
        store.with(|mut view| {
            let stdout = data.into_reader(&mut view);
            let tx = view.get().cli().stdout.writer();
            view.spawn(OutputTask { rx: stdout, tx });
            Ok(())
        })
    }
}

impl<T> stdout::Host for WasiCliImpl<T> where T: WasiCliView {}

impl<T> stderr::HostConcurrent for WasiCli<T>
where
    T: WasiCliView + 'static,
    T::OutputStream: OutputStream,
{
    async fn set_stderr<U>(
        store: &Accessor<U, Self>,
        data: HostStream<u8>,
    ) -> wasmtime::Result<()> {
        store.with(|mut view| {
            let stderr = data.into_reader(&mut view);
            let tx = view.get().cli().stderr.writer();
            view.spawn(OutputTask { rx: stderr, tx });
            Ok(())
        })
    }
}

impl<T> stderr::Host for WasiCliImpl<T> where T: WasiCliView {}

impl<T> environment::Host for WasiCliImpl<T>
where
    T: WasiCliView,
{
    fn get_environment(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
        Ok(self.cli().environment.clone())
    }

    fn get_arguments(&mut self) -> wasmtime::Result<Vec<String>> {
        Ok(self.cli().arguments.clone())
    }

    fn initial_cwd(&mut self) -> wasmtime::Result<Option<String>> {
        Ok(self.cli().initial_cwd.clone())
    }
}

impl<T> exit::Host for WasiCliImpl<T>
where
    T: WasiCliView,
{
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

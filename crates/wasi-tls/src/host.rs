use anyhow::Result;
use wasmtime::component::Resource;
use wasmtime_wasi::async_trait;
use wasmtime_wasi::p2::Pollable;
use wasmtime_wasi::p2::{DynInputStream, DynOutputStream, DynPollable, IoError};

use crate::{
    TlsStream, TlsTransport, WasiTls, bindings,
    io::{
        AsyncReadStream, AsyncWriteStream, FutureOutput, WasiFuture, WasiStreamReader,
        WasiStreamWriter,
    },
};

impl<'a> bindings::types::Host for WasiTls<'a> {}

/// Represents the ClientHandshake which will be used to configure the handshake
pub struct HostClientHandshake {
    server_name: String,
    transport: Box<dyn TlsTransport>,
}

impl<'a> bindings::types::HostClientHandshake for WasiTls<'a> {
    fn new(
        &mut self,
        server_name: String,
        input: Resource<DynInputStream>,
        output: Resource<DynOutputStream>,
    ) -> wasmtime::Result<Resource<HostClientHandshake>> {
        let input = self.table.delete(input)?;
        let output = self.table.delete(output)?;

        let reader = WasiStreamReader::new(input);
        let writer = WasiStreamWriter::new(output);
        let transport = tokio::io::join(reader, writer);

        Ok(self.table.push(HostClientHandshake {
            server_name,
            transport: Box::new(transport) as Box<dyn TlsTransport>,
        })?)
    }

    fn finish(
        &mut self,
        this: Resource<HostClientHandshake>,
    ) -> wasmtime::Result<Resource<HostFutureClientStreams>> {
        let handshake = self.table.delete(this)?;

        let connect = self
            .ctx
            .provider
            .connect(handshake.server_name, handshake.transport);

        let future = HostFutureClientStreams(WasiFuture::spawn(async move {
            let tls_stream = connect.await?;

            let (rx, tx) = tokio::io::split(tls_stream);
            let write_stream = AsyncWriteStream::new(tx);
            let client = HostClientConnection(write_stream.clone());

            let input = Box::new(AsyncReadStream::new(rx)) as DynInputStream;
            let output = Box::new(write_stream) as DynOutputStream;

            Ok((client, input, output))
        }));

        Ok(self.table.push(future)?)
    }

    fn drop(&mut self, this: Resource<HostClientHandshake>) -> wasmtime::Result<()> {
        self.table.delete(this)?;
        Ok(())
    }
}

/// Future streams provides the tls streams after the handshake is completed
pub struct HostFutureClientStreams(
    WasiFuture<Result<(HostClientConnection, DynInputStream, DynOutputStream), IoError>>,
);

#[async_trait]
impl Pollable for HostFutureClientStreams {
    async fn ready(&mut self) {
        self.0.ready().await
    }
}

impl<'a> bindings::types::HostFutureClientStreams for WasiTls<'a> {
    fn subscribe(
        &mut self,
        this: Resource<HostFutureClientStreams>,
    ) -> wasmtime::Result<Resource<DynPollable>> {
        wasmtime_wasi::p2::subscribe(self.table, this)
    }

    fn get(
        &mut self,
        this: Resource<HostFutureClientStreams>,
    ) -> wasmtime::Result<
        Option<
            Result<
                Result<
                    (
                        Resource<HostClientConnection>,
                        Resource<DynInputStream>,
                        Resource<DynOutputStream>,
                    ),
                    Resource<IoError>,
                >,
                (),
            >,
        >,
    > {
        let future = self.table.get_mut(&this)?;

        let result = match future.0.get() {
            FutureOutput::Ready(Ok((client, input, output))) => {
                let client = self.table.push(client)?;
                let input = self.table.push_child(input, &client)?;
                let output = self.table.push_child(output, &client)?;

                Some(Ok(Ok((client, input, output))))
            }
            FutureOutput::Ready(Err(io_error)) => {
                let io_error = self.table.push(io_error)?;

                Some(Ok(Err(io_error)))
            }
            FutureOutput::Consumed => Some(Err(())),
            FutureOutput::Pending => None,
        };

        Ok(result)
    }

    fn drop(&mut self, this: Resource<HostFutureClientStreams>) -> wasmtime::Result<()> {
        self.table.delete(this)?;
        Ok(())
    }
}

/// Represents the client connection and used to shut down the tls stream
pub struct HostClientConnection(
    crate::io::AsyncWriteStream<tokio::io::WriteHalf<Box<dyn TlsStream>>>,
);

impl<'a> bindings::types::HostClientConnection for WasiTls<'a> {
    fn close_output(&mut self, this: Resource<HostClientConnection>) -> wasmtime::Result<()> {
        self.table.get_mut(&this)?.0.close()
    }

    fn drop(&mut self, this: Resource<HostClientConnection>) -> wasmtime::Result<()> {
        self.table.delete(this)?;
        Ok(())
    }
}

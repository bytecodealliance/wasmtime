use crate::{
    wasi_tcp::{self, BytesResult, Socket, WasiFuture, WasiTcp},
    WasiCtx,
};
use anyhow::Result;
use wit_bindgen_host_wasmtime_rust::Error;

#[async_trait::async_trait]
impl WasiTcp for WasiCtx {
    async fn bytes_readable(
        &mut self,
        socket: Socket,
    ) -> Result<BytesResult, Error<wasi_tcp::Error>> {
        drop(socket);
        todo!()
    }

    async fn bytes_writable(
        &mut self,
        socket: Socket,
    ) -> Result<BytesResult, Error<wasi_tcp::Error>> {
        drop(socket);
        todo!()
    }

    async fn subscribe_read(&mut self, socket: Socket) -> Result<WasiFuture> {
        drop(socket);
        todo!()
    }

    async fn subscribe_write(&mut self, socket: Socket) -> Result<WasiFuture> {
        drop(socket);
        todo!()
    }
}

use crate::{
    wasi_tcp::{self, BytesResult, Socket, WasiFuture, WasiTcp},
    WasiCtx,
};
use anyhow::Result;
use wit_bindgen_host_wasmtime_rust::Error;

impl WasiTcp for WasiCtx {
    fn bytes_readable(&mut self, socket: Socket) -> Result<BytesResult, Error<wasi_tcp::Error>> {
        drop(socket);
        todo!()
    }

    fn bytes_writable(&mut self, socket: Socket) -> Result<BytesResult, Error<wasi_tcp::Error>> {
        drop(socket);
        todo!()
    }

    fn subscribe_read(&mut self, socket: Socket) -> Result<WasiFuture> {
        drop(socket);
        todo!()
    }

    fn subscribe_write(&mut self, socket: Socket) -> Result<WasiFuture> {
        drop(socket);
        todo!()
    }
}

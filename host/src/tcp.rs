use crate::{
    wasi_tcp::{self, BytesResult, Socket, WasiTcp},
    HostResult, WasiCtx,
};

#[async_trait::async_trait]
impl WasiTcp for WasiCtx {
    async fn bytes_readable(&mut self, socket: Socket) -> HostResult<BytesResult, wasi_tcp::Error> {
        drop(socket);
        todo!()
    }

    async fn bytes_writable(&mut self, socket: Socket) -> HostResult<BytesResult, wasi_tcp::Error> {
        drop(socket);
        todo!()
    }
}

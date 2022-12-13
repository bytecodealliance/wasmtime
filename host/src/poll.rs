#![allow(unused_variables)]

use crate::{
    wasi_poll::{WasiFuture, WasiPoll},
    WasiCtx,
};
use anyhow::Result;

#[async_trait::async_trait]
impl WasiPoll for WasiCtx {
    async fn drop_future(&mut self, future: WasiFuture) -> Result<()> {
        todo!()
    }

    async fn poll_oneoff(&mut self, futures: Vec<WasiFuture>) -> Result<Vec<u8>> {
        todo!()
    }
}

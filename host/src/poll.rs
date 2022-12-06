#![allow(unused_variables)]

use crate::{
    wasi_poll::{WasiFuture, WasiPoll},
    WasiCtx,
};
use anyhow::Result;

impl WasiPoll for WasiCtx {
    fn drop_future(&mut self, future: WasiFuture) -> Result<()> {
        todo!()
    }

    fn poll_oneoff(&mut self, futures: Vec<WasiFuture>) -> Result<Vec<u8>> {
        todo!()
    }
}

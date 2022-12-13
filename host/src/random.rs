#![allow(unused_variables)]
use crate::{wasi_random, WasiCtx};

#[async_trait::async_trait]
impl wasi_random::WasiRandom for WasiCtx {
    async fn getrandom(&mut self, len: u32) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}

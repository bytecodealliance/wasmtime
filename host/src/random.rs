#![allow(unused_variables)]
use crate::{wasi_random, WasiCtx};

impl wasi_random::WasiRandom for WasiCtx {
    fn getrandom(&mut self, len: u32) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}

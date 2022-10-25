#![allow(unused_variables)]

use crate::{wasi_poll, WasiCtx};

impl wasi_poll::WasiPoll for WasiCtx {
    fn poll_oneoff(
        &mut self,
        subs: Vec<wasi_poll::Subscription>,
    ) -> anyhow::Result<Vec<wasi_poll::Event>> {
        todo!()
    }
}

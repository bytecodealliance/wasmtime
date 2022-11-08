#![allow(unused_variables)]

use crate::{wasi_poll, WasiCtx};

impl wasi_poll::WasiPoll for WasiCtx {
    fn drop_future(&mut self, future: wasi_poll::WasiFuture) -> anyhow::Result<()> {
        todo!()
    }

    fn bytes_readable(
        &mut self,
        fd: wasi_poll::Descriptor,
    ) -> anyhow::Result<wasi_poll::BytesResult> {
        todo!()
    }

    fn bytes_writable(
        &mut self,
        fd: wasi_poll::Descriptor,
    ) -> anyhow::Result<wasi_poll::BytesResult> {
        todo!()
    }

    fn subscribe_read(
        &mut self,
        fd: wasi_poll::Descriptor,
    ) -> anyhow::Result<wasi_poll::WasiFuture> {
        todo!()
    }

    fn subscribe_write(
        &mut self,
        fd: wasi_poll::Descriptor,
    ) -> anyhow::Result<wasi_poll::WasiFuture> {
        todo!()
    }

    fn subscribe_wall_clock(
        &mut self,
        when: wasi_poll::Datetime,
        absolute: bool,
    ) -> anyhow::Result<wasi_poll::WasiFuture> {
        todo!()
    }

    fn subscribe_monotonic_clock(
        &mut self,
        when: wasi_poll::Instant,
        absolute: bool,
    ) -> anyhow::Result<wasi_poll::WasiFuture> {
        todo!()
    }

    fn poll_oneoff(&mut self, futures: Vec<wasi_poll::WasiFuture>) -> anyhow::Result<Vec<bool>> {
        todo!()
    }
}

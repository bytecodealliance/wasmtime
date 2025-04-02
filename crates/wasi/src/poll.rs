use crate::runtime::in_tokio;
use wasmtime_wasi_io::{bindings::wasi::io::poll as async_poll, poll::DynPollable, IoImpl, IoView};

use anyhow::Result;
use wasmtime::component::Resource;

impl<T> crate::bindings::sync::io::poll::Host for IoImpl<T>
where
    T: IoView,
{
    fn poll(&mut self, pollables: Vec<Resource<DynPollable>>) -> Result<Vec<u32>> {
        in_tokio(async { async_poll::Host::poll(self, pollables).await })
    }
}

impl<T> crate::bindings::sync::io::poll::HostPollable for IoImpl<T>
where
    T: IoView,
{
    fn ready(&mut self, pollable: Resource<DynPollable>) -> Result<bool> {
        in_tokio(async { async_poll::HostPollable::ready(self, pollable).await })
    }
    fn block(&mut self, pollable: Resource<DynPollable>) -> Result<()> {
        in_tokio(async { async_poll::HostPollable::block(self, pollable).await })
    }
    fn drop(&mut self, pollable: Resource<DynPollable>) -> Result<()> {
        async_poll::HostPollable::drop(self, pollable)
    }
}

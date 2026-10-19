use crate::runtime::in_tokio;
use wasmtime_wasi_io::{bindings::wasi::io::poll as async_poll, poll::DynPollable};

use wasmtime::Result;
use wasmtime::component::{Resource, ResourceTable};

impl crate::p2::bindings::sync::io::poll::Host for ResourceTable {
    fn poll(&mut self, pollables: Vec<Resource<DynPollable>>) -> Result<Vec<u32>> {
        in_tokio(async { async_poll::Host::poll(self, pollables).await })
    }
}

impl crate::p2::bindings::sync::io::poll::HostPollable for ResourceTable {
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

mod named {
    use super::*;
    use crate::p2::bindings::sync::named_imports::wasi::io::poll as named_poll;
    use crate::{NamedId, WasiCtxNamedView, WasiNamedView};

    impl<T> named_poll::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        fn poll(&mut self, id: NamedId, pollables: Vec<Resource<DynPollable>>) -> Result<Vec<u32>> {
            crate::p2::bindings::sync::io::poll::Host::poll(self.0.ctx(id).table, pollables)
        }
    }

    impl<T> named_poll::HostPollable for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        fn ready(&mut self, id: NamedId, pollable: Resource<DynPollable>) -> Result<bool> {
            crate::p2::bindings::sync::io::poll::HostPollable::ready(self.0.ctx(id).table, pollable)
        }

        fn block(&mut self, id: NamedId, pollable: Resource<DynPollable>) -> Result<()> {
            crate::p2::bindings::sync::io::poll::HostPollable::block(self.0.ctx(id).table, pollable)
        }

        fn drop(&mut self, id: NamedId, pollable: Resource<DynPollable>) -> Result<()> {
            crate::p2::bindings::sync::io::poll::HostPollable::drop(self.0.ctx(id).table, pollable)
        }
    }
}

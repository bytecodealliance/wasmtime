pub mod sync {
    use crate::{bindings::io::poll as async_poll, runtime::in_tokio, IoImpl, IoView};
    use anyhow::Result;
    use wasmtime::component::Resource;

    impl<T> poll::Host for IoImpl<T>
    where
        T: IoView,
    {
        fn poll(&mut self, pollables: Vec<Resource<Pollable>>) -> Result<Vec<u32>> {
            in_tokio(async { async_poll::Host::poll(self, pollables).await })
        }
    }

    impl<T> crate::bindings::sync::io::poll::HostPollable for IoImpl<T>
    where
        T: IoView,
    {
        fn ready(&mut self, pollable: Resource<Pollable>) -> Result<bool> {
            in_tokio(async { async_poll::HostPollable::ready(self, pollable).await })
        }
        fn block(&mut self, pollable: Resource<Pollable>) -> Result<()> {
            in_tokio(async { async_poll::HostPollable::block(self, pollable).await })
        }
        fn drop(&mut self, pollable: Resource<Pollable>) -> Result<()> {
            async_poll::HostPollable::drop(self, pollable)
        }
    }
}

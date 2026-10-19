use crate::p2::{
    StreamError, StreamResult,
    bindings::sync::io::poll::Pollable,
    bindings::sync::io::streams::{self, InputStream, OutputStream},
};
use crate::runtime::in_tokio;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi_io::bindings::wasi::io::streams::{
    self as async_streams, Host as AsyncHost, HostInputStream as AsyncHostInputStream,
    HostOutputStream as AsyncHostOutputStream,
};

impl From<async_streams::StreamError> for streams::StreamError {
    fn from(other: async_streams::StreamError) -> Self {
        match other {
            async_streams::StreamError::LastOperationFailed(e) => Self::LastOperationFailed(e),
            async_streams::StreamError::Closed => Self::Closed,
        }
    }
}

impl streams::Host for ResourceTable {
    fn convert_stream_error(&mut self, err: StreamError) -> wasmtime::Result<streams::StreamError> {
        Ok(AsyncHost::convert_stream_error(self, err)?.into())
    }
}

impl streams::HostOutputStream for ResourceTable {
    fn drop(&mut self, stream: Resource<OutputStream>) -> wasmtime::Result<()> {
        in_tokio(async { AsyncHostOutputStream::drop(self, stream).await })
    }

    fn check_write(&mut self, stream: Resource<OutputStream>) -> StreamResult<u64> {
        Ok(AsyncHostOutputStream::check_write(self, stream)?)
    }

    fn write(&mut self, stream: Resource<OutputStream>, bytes: Vec<u8>) -> StreamResult<()> {
        Ok(AsyncHostOutputStream::write(self, stream, bytes)?)
    }

    fn blocking_write_and_flush(
        &mut self,
        stream: Resource<OutputStream>,
        bytes: Vec<u8>,
    ) -> StreamResult<()> {
        in_tokio(async {
            AsyncHostOutputStream::blocking_write_and_flush(self, stream, bytes).await
        })
    }

    fn blocking_write_zeroes_and_flush(
        &mut self,
        stream: Resource<OutputStream>,
        len: u64,
    ) -> StreamResult<()> {
        in_tokio(async {
            AsyncHostOutputStream::blocking_write_zeroes_and_flush(self, stream, len).await
        })
    }

    fn subscribe(
        &mut self,
        stream: Resource<OutputStream>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        Ok(AsyncHostOutputStream::subscribe(self, stream)?)
    }

    fn write_zeroes(&mut self, stream: Resource<OutputStream>, len: u64) -> StreamResult<()> {
        Ok(AsyncHostOutputStream::write_zeroes(self, stream, len)?)
    }

    fn flush(&mut self, stream: Resource<OutputStream>) -> StreamResult<()> {
        Ok(AsyncHostOutputStream::flush(
            self,
            Resource::new_borrow(stream.rep()),
        )?)
    }

    fn blocking_flush(&mut self, stream: Resource<OutputStream>) -> StreamResult<()> {
        in_tokio(async {
            AsyncHostOutputStream::blocking_flush(self, Resource::new_borrow(stream.rep())).await
        })
    }

    fn splice(
        &mut self,
        dst: Resource<OutputStream>,
        src: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        AsyncHostOutputStream::splice(self, dst, src, len)
    }

    fn blocking_splice(
        &mut self,
        dst: Resource<OutputStream>,
        src: Resource<InputStream>,
        len: u64,
    ) -> StreamResult<u64> {
        in_tokio(async { AsyncHostOutputStream::blocking_splice(self, dst, src, len).await })
    }
}

impl streams::HostInputStream for ResourceTable {
    fn drop(&mut self, stream: Resource<InputStream>) -> wasmtime::Result<()> {
        in_tokio(async { AsyncHostInputStream::drop(self, stream).await })
    }

    fn read(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<Vec<u8>> {
        AsyncHostInputStream::read(self, stream, len)
    }

    fn blocking_read(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<Vec<u8>> {
        in_tokio(async { AsyncHostInputStream::blocking_read(self, stream, len).await })
    }

    fn skip(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<u64> {
        AsyncHostInputStream::skip(self, stream, len)
    }

    fn blocking_skip(&mut self, stream: Resource<InputStream>, len: u64) -> StreamResult<u64> {
        in_tokio(async { AsyncHostInputStream::blocking_skip(self, stream, len).await })
    }

    fn subscribe(&mut self, stream: Resource<InputStream>) -> wasmtime::Result<Resource<Pollable>> {
        AsyncHostInputStream::subscribe(self, stream)
    }
}

mod named {
    use crate::p2::bindings::named_imports::wasi::io::{error, poll, streams};
    use crate::{NamedId, WasiCtxNamedView, WasiNamedView};
    use wasmtime::component::Resource;
    use wasmtime_wasi_io::bindings::wasi::io::{
        error as async_error, poll as async_poll, streams as async_streams,
    };
    use wasmtime_wasi_io::poll::DynPollable;
    use wasmtime_wasi_io::streams::{
        DynInputStream, DynOutputStream, Error, StreamError, StreamResult,
    };

    // `wasi:io` has no state other than resources in the table, so named
    // imports only need to locate the table to use.

    impl<T> error::Host for WasiCtxNamedView<'_, T> where T: WasiNamedView {}

    impl<T> error::HostError for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        fn to_debug_string(
            &mut self,
            id: NamedId,
            err: Resource<Error>,
        ) -> wasmtime::Result<String> {
            async_error::HostError::to_debug_string(self.0.ctx(id).table, err)
        }

        fn drop(&mut self, id: NamedId, err: Resource<Error>) -> wasmtime::Result<()> {
            async_error::HostError::drop(self.0.ctx(id).table, err)
        }
    }

    impl<T> poll::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        async fn poll(
            &mut self,
            id: NamedId,
            pollables: Vec<Resource<DynPollable>>,
        ) -> wasmtime::Result<Vec<u32>> {
            async_poll::Host::poll(self.0.ctx(id).table, pollables).await
        }
    }

    impl<T> poll::HostPollable for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        async fn block(
            &mut self,
            id: NamedId,
            pollable: Resource<DynPollable>,
        ) -> wasmtime::Result<()> {
            async_poll::HostPollable::block(self.0.ctx(id).table, pollable).await
        }

        async fn ready(
            &mut self,
            id: NamedId,
            pollable: Resource<DynPollable>,
        ) -> wasmtime::Result<bool> {
            async_poll::HostPollable::ready(self.0.ctx(id).table, pollable).await
        }

        fn drop(&mut self, id: NamedId, pollable: Resource<DynPollable>) -> wasmtime::Result<()> {
            async_poll::HostPollable::drop(self.0.ctx(id).table, pollable)
        }
    }

    impl<T> streams::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        fn convert_stream_error(
            &mut self,
            id: NamedId,
            err: StreamError,
        ) -> wasmtime::Result<async_streams::StreamError> {
            async_streams::Host::convert_stream_error(self.0.ctx(id).table, err)
        }
    }

    impl<T> streams::HostOutputStream for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        async fn drop(
            &mut self,
            id: NamedId,
            stream: Resource<DynOutputStream>,
        ) -> wasmtime::Result<()> {
            async_streams::HostOutputStream::drop(self.0.ctx(id).table, stream).await
        }

        fn check_write(
            &mut self,
            id: NamedId,
            stream: Resource<DynOutputStream>,
        ) -> StreamResult<u64> {
            async_streams::HostOutputStream::check_write(self.0.ctx(id).table, stream)
        }

        fn write(
            &mut self,
            id: NamedId,
            stream: Resource<DynOutputStream>,
            bytes: Vec<u8>,
        ) -> StreamResult<()> {
            async_streams::HostOutputStream::write(self.0.ctx(id).table, stream, bytes)
        }

        fn subscribe(
            &mut self,
            id: NamedId,
            stream: Resource<DynOutputStream>,
        ) -> wasmtime::Result<Resource<DynPollable>> {
            async_streams::HostOutputStream::subscribe(self.0.ctx(id).table, stream)
        }

        async fn blocking_write_and_flush(
            &mut self,
            id: NamedId,
            stream: Resource<DynOutputStream>,
            bytes: Vec<u8>,
        ) -> StreamResult<()> {
            async_streams::HostOutputStream::blocking_write_and_flush(
                self.0.ctx(id).table,
                stream,
                bytes,
            )
            .await
        }

        async fn blocking_write_zeroes_and_flush(
            &mut self,
            id: NamedId,
            stream: Resource<DynOutputStream>,
            len: u64,
        ) -> StreamResult<()> {
            async_streams::HostOutputStream::blocking_write_zeroes_and_flush(
                self.0.ctx(id).table,
                stream,
                len,
            )
            .await
        }

        fn write_zeroes(
            &mut self,
            id: NamedId,
            stream: Resource<DynOutputStream>,
            len: u64,
        ) -> StreamResult<()> {
            async_streams::HostOutputStream::write_zeroes(self.0.ctx(id).table, stream, len)
        }

        fn flush(&mut self, id: NamedId, stream: Resource<DynOutputStream>) -> StreamResult<()> {
            async_streams::HostOutputStream::flush(self.0.ctx(id).table, stream)
        }

        async fn blocking_flush(
            &mut self,
            id: NamedId,
            stream: Resource<DynOutputStream>,
        ) -> StreamResult<()> {
            async_streams::HostOutputStream::blocking_flush(self.0.ctx(id).table, stream).await
        }

        fn splice(
            &mut self,
            id: NamedId,
            dst: Resource<DynOutputStream>,
            src: Resource<DynInputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            async_streams::HostOutputStream::splice(self.0.ctx(id).table, dst, src, len)
        }

        async fn blocking_splice(
            &mut self,
            id: NamedId,
            dst: Resource<DynOutputStream>,
            src: Resource<DynInputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            async_streams::HostOutputStream::blocking_splice(self.0.ctx(id).table, dst, src, len)
                .await
        }
    }

    impl<T> streams::HostInputStream for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        async fn drop(
            &mut self,
            id: NamedId,
            stream: Resource<DynInputStream>,
        ) -> wasmtime::Result<()> {
            async_streams::HostInputStream::drop(self.0.ctx(id).table, stream).await
        }

        fn read(
            &mut self,
            id: NamedId,
            stream: Resource<DynInputStream>,
            len: u64,
        ) -> StreamResult<Vec<u8>> {
            async_streams::HostInputStream::read(self.0.ctx(id).table, stream, len)
        }

        async fn blocking_read(
            &mut self,
            id: NamedId,
            stream: Resource<DynInputStream>,
            len: u64,
        ) -> StreamResult<Vec<u8>> {
            async_streams::HostInputStream::blocking_read(self.0.ctx(id).table, stream, len).await
        }

        fn skip(
            &mut self,
            id: NamedId,
            stream: Resource<DynInputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            async_streams::HostInputStream::skip(self.0.ctx(id).table, stream, len)
        }

        async fn blocking_skip(
            &mut self,
            id: NamedId,
            stream: Resource<DynInputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            async_streams::HostInputStream::blocking_skip(self.0.ctx(id).table, stream, len).await
        }

        fn subscribe(
            &mut self,
            id: NamedId,
            stream: Resource<DynInputStream>,
        ) -> wasmtime::Result<Resource<DynPollable>> {
            async_streams::HostInputStream::subscribe(self.0.ctx(id).table, stream)
        }
    }
}

mod sync_named {
    use super::*;
    use crate::p2::bindings::sync::named_imports::wasi::io::streams as named_streams;
    use crate::{NamedId, WasiCtxNamedView, WasiNamedView};

    impl<T> named_streams::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        fn convert_stream_error(
            &mut self,
            id: NamedId,
            err: StreamError,
        ) -> wasmtime::Result<streams::StreamError> {
            streams::Host::convert_stream_error(self.0.ctx(id).table, err)
        }
    }

    impl<T> named_streams::HostOutputStream for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        fn drop(&mut self, id: NamedId, stream: Resource<OutputStream>) -> wasmtime::Result<()> {
            streams::HostOutputStream::drop(self.0.ctx(id).table, stream)
        }

        fn check_write(
            &mut self,
            id: NamedId,
            stream: Resource<OutputStream>,
        ) -> StreamResult<u64> {
            streams::HostOutputStream::check_write(self.0.ctx(id).table, stream)
        }

        fn write(
            &mut self,
            id: NamedId,
            stream: Resource<OutputStream>,
            bytes: Vec<u8>,
        ) -> StreamResult<()> {
            streams::HostOutputStream::write(self.0.ctx(id).table, stream, bytes)
        }

        fn blocking_write_and_flush(
            &mut self,
            id: NamedId,
            stream: Resource<OutputStream>,
            bytes: Vec<u8>,
        ) -> StreamResult<()> {
            streams::HostOutputStream::blocking_write_and_flush(self.0.ctx(id).table, stream, bytes)
        }

        fn blocking_write_zeroes_and_flush(
            &mut self,
            id: NamedId,
            stream: Resource<OutputStream>,
            len: u64,
        ) -> StreamResult<()> {
            streams::HostOutputStream::blocking_write_zeroes_and_flush(
                self.0.ctx(id).table,
                stream,
                len,
            )
        }

        fn subscribe(
            &mut self,
            id: NamedId,
            stream: Resource<OutputStream>,
        ) -> wasmtime::Result<Resource<Pollable>> {
            streams::HostOutputStream::subscribe(self.0.ctx(id).table, stream)
        }

        fn write_zeroes(
            &mut self,
            id: NamedId,
            stream: Resource<OutputStream>,
            len: u64,
        ) -> StreamResult<()> {
            streams::HostOutputStream::write_zeroes(self.0.ctx(id).table, stream, len)
        }

        fn flush(&mut self, id: NamedId, stream: Resource<OutputStream>) -> StreamResult<()> {
            streams::HostOutputStream::flush(self.0.ctx(id).table, stream)
        }

        fn blocking_flush(
            &mut self,
            id: NamedId,
            stream: Resource<OutputStream>,
        ) -> StreamResult<()> {
            streams::HostOutputStream::blocking_flush(self.0.ctx(id).table, stream)
        }

        fn splice(
            &mut self,
            id: NamedId,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            streams::HostOutputStream::splice(self.0.ctx(id).table, dst, src, len)
        }

        fn blocking_splice(
            &mut self,
            id: NamedId,
            dst: Resource<OutputStream>,
            src: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            streams::HostOutputStream::blocking_splice(self.0.ctx(id).table, dst, src, len)
        }
    }

    impl<T> named_streams::HostInputStream for WasiCtxNamedView<'_, T>
    where
        T: WasiNamedView,
    {
        fn drop(&mut self, id: NamedId, stream: Resource<InputStream>) -> wasmtime::Result<()> {
            streams::HostInputStream::drop(self.0.ctx(id).table, stream)
        }

        fn read(
            &mut self,
            id: NamedId,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<Vec<u8>> {
            streams::HostInputStream::read(self.0.ctx(id).table, stream, len)
        }

        fn blocking_read(
            &mut self,
            id: NamedId,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<Vec<u8>> {
            streams::HostInputStream::blocking_read(self.0.ctx(id).table, stream, len)
        }

        fn skip(
            &mut self,
            id: NamedId,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            streams::HostInputStream::skip(self.0.ctx(id).table, stream, len)
        }

        fn blocking_skip(
            &mut self,
            id: NamedId,
            stream: Resource<InputStream>,
            len: u64,
        ) -> StreamResult<u64> {
            streams::HostInputStream::blocking_skip(self.0.ctx(id).table, stream, len)
        }

        fn subscribe(
            &mut self,
            id: NamedId,
            stream: Resource<InputStream>,
        ) -> wasmtime::Result<Resource<Pollable>> {
            streams::HostInputStream::subscribe(self.0.ctx(id).table, stream)
        }
    }
}

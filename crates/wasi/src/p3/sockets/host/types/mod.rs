use crate::p3::bindings::sockets::types::{ErrorCode, Host};
use crate::p3::sockets::SocketError;
use crate::sockets::WasiSocketsCtxView;

mod tcp;
mod udp;

impl Host for WasiSocketsCtxView<'_> {
    fn convert_error_code(&mut self, error: SocketError) -> wasmtime::Result<ErrorCode> {
        error.downcast()
    }
}

mod named {
    use crate::WasiCtxNamedView;
    use crate::p3::bindings::named_imports::wasi::sockets::types::Host;
    use crate::p3::bindings::sockets::types::ErrorCode;
    use crate::p3::sockets::SocketError;
    use crate::sockets::WasiSocketsNamedView;

    impl<T> Host for WasiCtxNamedView<'_, T>
    where
        T: WasiSocketsNamedView,
    {
        fn convert_error_code(&mut self, error: SocketError) -> wasmtime::Result<ErrorCode> {
            error.downcast()
        }
    }
}

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

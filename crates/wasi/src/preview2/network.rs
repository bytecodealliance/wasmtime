use crate::preview2::bindings::wasi::sockets::network::ErrorCode;
use crate::preview2::{TableError, TrappableError};
use cap_std::net::Pool;

pub struct Network {
    pub pool: Pool,
    pub allow_ip_name_lookup: bool,
}

pub type SocketResult<T> = Result<T, SocketError>;

pub type SocketError = TrappableError<ErrorCode>;

impl From<TableError> for SocketError {
    fn from(error: TableError) -> Self {
        Self::trap(error)
    }
}

impl From<std::io::Error> for SocketError {
    fn from(error: std::io::Error) -> Self {
        ErrorCode::from(error).into()
    }
}

impl From<rustix::io::Errno> for SocketError {
    fn from(error: rustix::io::Errno) -> Self {
        ErrorCode::from(error).into()
    }
}

use crate::TrappableError;
use crate::p2::bindings::sockets::network::ErrorCode;
use crate::p2::bindings::sockets::tcp::ShutdownType;

pub type SocketResult<T> = Result<T, SocketError>;

pub type SocketError = TrappableError<ErrorCode>;

impl From<wasmtime::component::ResourceTableError> for SocketError {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
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

impl From<crate::sockets::util::ErrorCode> for SocketError {
    fn from(error: crate::sockets::util::ErrorCode) -> Self {
        ErrorCode::from(error).into()
    }
}

impl From<crate::sockets::util::ErrorCode> for ErrorCode {
    fn from(error: crate::sockets::util::ErrorCode) -> Self {
        match error {
            crate::sockets::util::ErrorCode::Other => Self::Unknown,
            crate::sockets::util::ErrorCode::AccessDenied => Self::AccessDenied,
            crate::sockets::util::ErrorCode::NotSupported => Self::NotSupported,
            crate::sockets::util::ErrorCode::InvalidArgument => Self::InvalidArgument,
            crate::sockets::util::ErrorCode::OutOfMemory => Self::OutOfMemory,
            crate::sockets::util::ErrorCode::Timeout => Self::Timeout,
            crate::sockets::util::ErrorCode::InvalidState => Self::InvalidState,
            crate::sockets::util::ErrorCode::AddressNotBindable => Self::AddressNotBindable,
            crate::sockets::util::ErrorCode::AddressInUse => Self::AddressInUse,
            crate::sockets::util::ErrorCode::RemoteUnreachable => Self::RemoteUnreachable,
            crate::sockets::util::ErrorCode::ConnectionRefused => Self::ConnectionRefused,
            crate::sockets::util::ErrorCode::ConnectionBroken => Self::Unknown,
            crate::sockets::util::ErrorCode::ConnectionReset => Self::ConnectionReset,
            crate::sockets::util::ErrorCode::ConnectionAborted => Self::ConnectionAborted,
            crate::sockets::util::ErrorCode::DatagramTooLarge => Self::DatagramTooLarge,
        }
    }
}

impl From<ShutdownType> for std::net::Shutdown {
    fn from(value: ShutdownType) -> Self {
        match value {
            ShutdownType::Receive => Self::Read,
            ShutdownType::Send => Self::Write,
            ShutdownType::Both => Self::Both,
        }
    }
}

/// A network resource representing the capability to use the networking
/// APIs in WASI 0.2. The concept of network handles has been removed in
/// WASI 0.3.
///
/// Note that in Wasmtime all permissions are checked through the `WasiCtx` and
/// not through the `Network` resource itself. The `Network` resource is just a
/// placeholder capability handle.
pub struct Network {
    pub(crate) _priv: (),
}

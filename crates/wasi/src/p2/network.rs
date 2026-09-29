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

impl From<crate::sockets::ErrorCode> for SocketError {
    fn from(error: crate::sockets::ErrorCode) -> Self {
        ErrorCode::from(error).into()
    }
}

impl From<crate::sockets::ErrorCode> for ErrorCode {
    fn from(error: crate::sockets::ErrorCode) -> Self {
        match error {
            crate::sockets::ErrorCode::Other => Self::Unknown,
            crate::sockets::ErrorCode::AccessDenied => Self::AccessDenied,
            crate::sockets::ErrorCode::NotSupported => Self::NotSupported,
            crate::sockets::ErrorCode::InvalidArgument => Self::InvalidArgument,
            crate::sockets::ErrorCode::OutOfMemory => Self::OutOfMemory,
            crate::sockets::ErrorCode::Timeout => Self::Timeout,
            crate::sockets::ErrorCode::InvalidState => Self::InvalidState,
            crate::sockets::ErrorCode::AddressNotBindable => Self::AddressNotBindable,
            crate::sockets::ErrorCode::AddressInUse => Self::AddressInUse,
            crate::sockets::ErrorCode::RemoteUnreachable => Self::RemoteUnreachable,
            crate::sockets::ErrorCode::ConnectionRefused => Self::ConnectionRefused,
            crate::sockets::ErrorCode::ConnectionBroken => Self::Unknown,
            crate::sockets::ErrorCode::ConnectionReset => Self::ConnectionReset,
            crate::sockets::ErrorCode::ConnectionAborted => Self::ConnectionAborted,
            crate::sockets::ErrorCode::DatagramTooLarge => Self::DatagramTooLarge,
        }
    }
}

impl From<crate::sockets::ip_name_lookup::ErrorCode> for ErrorCode {
    fn from(code: crate::sockets::ip_name_lookup::ErrorCode) -> Self {
        match code {
            crate::sockets::ip_name_lookup::ErrorCode::AccessDenied => Self::AccessDenied,
            crate::sockets::ip_name_lookup::ErrorCode::InvalidArgument => Self::InvalidArgument,
            crate::sockets::ip_name_lookup::ErrorCode::NameUnresolvable => Self::NameUnresolvable,
            crate::sockets::ip_name_lookup::ErrorCode::TemporaryResolverFailure => {
                Self::TemporaryResolverFailure
            }
            crate::sockets::ip_name_lookup::ErrorCode::PermanentResolverFailure => {
                Self::PermanentResolverFailure
            }
            crate::sockets::ip_name_lookup::ErrorCode::Other => Self::Unknown,
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

use std::time::Duration;

/// The concrete type behind a `wasi:http/types.request-options` resource.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RequestOptions {
    /// How long to wait for a connection to be established.
    pub connect_timeout: Option<Duration>,
    /// How long to wait for the first byte of the response body.
    pub first_byte_timeout: Option<Duration>,
    /// How long to wait between frames of the response body.
    pub between_bytes_timeout: Option<Duration>,
}

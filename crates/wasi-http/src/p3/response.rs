use http::{HeaderMap, StatusCode};
use std::sync::Arc;

/// The concrete type behind a `wasi:http/types/response` resource.
pub struct Response {
    /// The status of the response.
    pub status: StatusCode,
    /// The headers of the response.
    pub headers: Arc<HeaderMap>,
}

impl Response {
    /// Construct a new [Response]
    pub fn new(status: StatusCode, headers: impl Into<Arc<HeaderMap>>) -> Self {
        Self {
            status,
            headers: headers.into(),
        }
    }
}

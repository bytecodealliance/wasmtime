use anyhow::Error;
use bytes::Bytes;
use futures::executor::block_on;
use http::{header::HeaderName, HeaderMap, HeaderValue};
use reqwest::{Client, Method};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, PoisonError, RwLock},
};
use tokio::runtime::Handle;
use url::Url;
use wasmtime::*;

const MEMORY: &str = "memory";
const ALLOW_ALL_HOSTS: &str = "insecure:allow-all";

pub type WasiHttpHandle = u32;

/// Response body for HTTP requests, consumed by guest modules.
struct Body {
    bytes: Bytes,
    pos: usize,
}

/// An HTTP response abstraction that is persisted across multiple
/// host calls.
struct Response {
    headers: HeaderMap,
    body: Body,
}

/// Host state for the responses of the instance.
#[derive(Default)]
struct State {
    responses: HashMap<WasiHttpHandle, Response>,
    current_handle: WasiHttpHandle,
}

#[derive(Debug, thiserror::Error)]
enum HttpError {
    #[error("Invalid handle: [{0}]")]
    InvalidHandle(WasiHttpHandle),
    #[error("Memory not found")]
    MemoryNotFound,
    #[error("Memory access error")]
    MemoryAccessError(#[from] wasmtime::MemoryAccessError),
    #[error("Buffer too small")]
    BufferTooSmall,
    #[error("Header not found")]
    HeaderNotFound,
    #[error("UTF-8 error")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("Destination not allowed")]
    DestinationNotAllowed(String),
    #[error("Invalid method")]
    InvalidMethod,
    #[error("Invalid encoding")]
    InvalidEncoding,
    #[error("Invalid URL")]
    InvalidUrl,
    #[error("HTTP error")]
    RequestError(#[from] reqwest::Error),
    #[error("Runtime error")]
    RuntimeError,
    #[error("Too many sessions")]
    TooManySessions,
}

impl From<HttpError> for u32 {
    fn from(e: HttpError) -> u32 {
        match e {
            HttpError::InvalidHandle(_) => 1,
            HttpError::MemoryNotFound => 2,
            HttpError::MemoryAccessError(_) => 3,
            HttpError::BufferTooSmall => 4,
            HttpError::HeaderNotFound => 5,
            HttpError::Utf8Error(_) => 6,
            HttpError::DestinationNotAllowed(_) => 7,
            HttpError::InvalidMethod => 8,
            HttpError::InvalidEncoding => 9,
            HttpError::InvalidUrl => 10,
            HttpError::RequestError(_) => 11,
            HttpError::RuntimeError => 12,
            HttpError::TooManySessions => 13,
        }
    }
}

impl From<PoisonError<std::sync::RwLockReadGuard<'_, State>>> for HttpError {
    fn from(_: PoisonError<std::sync::RwLockReadGuard<'_, State>>) -> Self {
        HttpError::RuntimeError
    }
}

impl From<PoisonError<std::sync::RwLockWriteGuard<'_, State>>> for HttpError {
    fn from(_: PoisonError<std::sync::RwLockWriteGuard<'_, State>>) -> Self {
        HttpError::RuntimeError
    }
}

impl From<PoisonError<&mut State>> for HttpError {
    fn from(_: PoisonError<&mut State>) -> Self {
        HttpError::RuntimeError
    }
}

struct HostCalls;

impl HostCalls {
    /// Remove the current handle from the state.
    /// Depending on the implementation, guest modules might
    /// have to manually call `close`.
    // TODO (@radu-matei)
    // Fix the clippy warning.
    #[allow(clippy::unnecessary_wraps)]
    fn close(st: Arc<RwLock<State>>, handle: WasiHttpHandle) -> Result<(), HttpError> {
        let mut st = st.write()?;
        st.responses.remove(&handle);
        Ok(())
    }

    /// Read `buf_len` bytes from the response of `handle` and
    /// write them into `buf_ptr`.
    fn body_read(
        st: Arc<RwLock<State>>,
        memory: Memory,
        mut store: impl AsContextMut,
        handle: WasiHttpHandle,
        buf_ptr: u32,
        buf_len: u32,
        buf_read_ptr: u32,
    ) -> Result<(), HttpError> {
        let mut st = st.write()?;

        let mut body = &mut st.responses.get_mut(&handle).unwrap().body;
        let mut context = store.as_context_mut();

        // Write at most either the remaining of the response body, or the entire
        // length requested by the guest.
        let available = std::cmp::min(buf_len as _, body.bytes.len() - body.pos);
        memory.write(
            &mut context,
            buf_ptr as _,
            &body.bytes[body.pos..body.pos + available],
        )?;
        body.pos += available;
        // Write the number of bytes written back to the guest.
        memory.write(
            &mut context,
            buf_read_ptr as _,
            &(available as u32).to_le_bytes(),
        )?;
        Ok(())
    }

    /// Get a response header value given a key.
    #[allow(clippy::too_many_arguments)]
    fn header_get(
        st: Arc<RwLock<State>>,
        memory: Memory,
        mut store: impl AsContextMut,
        handle: WasiHttpHandle,
        name_ptr: u32,
        name_len: u32,
        value_ptr: u32,
        value_len: u32,
        value_written_ptr: u32,
    ) -> Result<(), HttpError> {
        let st = st.read()?;

        // Get the current response headers.
        let headers = &st
            .responses
            .get(&handle)
            .ok_or(HttpError::InvalidHandle(handle))?
            .headers;

        let mut store = store.as_context_mut();

        // Read the header key from the module's memory.
        let key = string_from_memory(&memory, &mut store, name_ptr, name_len)?.to_ascii_lowercase();
        // Attempt to get the corresponding value from the resposne headers.
        let value = headers.get(key).ok_or(HttpError::HeaderNotFound)?;
        if value.len() > value_len as _ {
            return Err(HttpError::BufferTooSmall);
        }
        // Write the header value and its length.
        memory.write(&mut store, value_ptr as _, value.as_bytes())?;
        memory.write(
            &mut store,
            value_written_ptr as _,
            &(value.len() as u32).to_le_bytes(),
        )?;
        Ok(())
    }

    fn headers_get_all(
        st: Arc<RwLock<State>>,
        memory: Memory,
        mut store: impl AsContextMut,
        handle: WasiHttpHandle,
        buf_ptr: u32,
        buf_len: u32,
        buf_written_ptr: u32,
    ) -> Result<(), HttpError> {
        let st = st.read()?;

        let headers = &st
            .responses
            .get(&handle)
            .ok_or(HttpError::InvalidHandle(handle))?
            .headers;

        let headers = match header_map_to_string(headers) {
            Ok(res) => res,
            Err(_) => return Err(HttpError::RuntimeError),
        };

        if headers.len() > buf_len as _ {
            return Err(HttpError::BufferTooSmall);
        }

        let mut store = store.as_context_mut();

        memory.write(&mut store, buf_ptr as _, headers.as_bytes())?;
        memory.write(
            &mut store,
            buf_written_ptr as _,
            &(headers.len() as u32).to_le_bytes(),
        )?;
        Ok(())
    }

    /// Execute a request for a guest module, given
    /// the request data.
    #[allow(clippy::too_many_arguments)]
    fn req(
        st: Arc<RwLock<State>>,
        allowed_hosts: Option<&[String]>,
        max_concurrent_requests: Option<u32>,
        memory: Memory,
        mut store: impl AsContextMut,
        url_ptr: u32,
        url_len: u32,
        method_ptr: u32,
        method_len: u32,
        req_headers_ptr: u32,
        req_headers_len: u32,
        req_body_ptr: u32,
        req_body_len: u32,
        status_code_ptr: u32,
        res_handle_ptr: u32,
    ) -> Result<(), HttpError> {
        let span = tracing::trace_span!("req");
        let _enter = span.enter();

        let mut st = st.write()?;

        if let Some(max) = max_concurrent_requests {
            if st.responses.len() > (max - 1) as usize {
                return Err(HttpError::TooManySessions);
            }
        };
        let mut store = store.as_context_mut();

        // Read the request parts from the module's linear memory and check early if
        // the guest is allowed to make a request to the given URL.
        let url = string_from_memory(&memory, &mut store, url_ptr, url_len)?;
        if !is_allowed(url.as_str(), allowed_hosts)? {
            return Err(HttpError::DestinationNotAllowed(url));
        }

        let method = Method::from_str(
            string_from_memory(&memory, &mut store, method_ptr, method_len)?.as_str(),
        )
        .map_err(|_| HttpError::InvalidMethod)?;
        let req_body = slice_from_memory(&memory, &mut store, req_body_ptr, req_body_len)?;
        let headers = string_to_header_map(
            string_from_memory(&memory, &mut store, req_headers_ptr, req_headers_len)?.as_str(),
        )
        .map_err(|_| HttpError::InvalidEncoding)?;

        // Send the request.
        let (status, resp_headers, resp_body) =
            request(url.as_str(), headers, method, req_body.as_slice())?;
        tracing::debug!(
            status,
            ?resp_headers,
            body_len = resp_body.as_ref().len(),
            "got HTTP response, writing back to memory"
        );

        // Write the status code to the guest.
        memory.write(&mut store, status_code_ptr as _, &status.to_le_bytes())?;

        // Construct the response, add it to the current state, and write
        // the handle to the guest.
        let response = Response {
            headers: resp_headers,
            body: Body {
                bytes: resp_body,
                pos: 0,
            },
        };

        let initial_handle = st.current_handle;
        while st.responses.get(&st.current_handle).is_some() {
            st.current_handle += 1;
            if st.current_handle == initial_handle {
                return Err(HttpError::TooManySessions);
            }
        }
        let handle = st.current_handle;
        st.responses.insert(handle, response);
        memory.write(&mut store, res_handle_ptr as _, &handle.to_le_bytes())?;

        Ok(())
    }
}

pub fn add_to_linker<T>(
    linker: &mut wasmtime::Linker<T>,
    get_cx: impl Fn(&T) -> &HttpCtx + Send + Sync + 'static,
) -> anyhow::Result<()> {
    let http = HttpState::new()?;
    http.add_to_linker(linker, get_cx)?;
    Ok(())
}

/// Per-instance context data used to control whether the guest
/// is allowed to make an outbound HTTP request.
#[derive(Clone)]
pub struct HttpCtx {
    pub allowed_hosts: Option<Vec<String>>,
    pub max_concurrent_requests: Option<u32>,
}

/// Experimental HTTP extension object for Wasmtime.
pub struct HttpState {
    state: Arc<RwLock<State>>,
}

impl HttpState {
    /// Module the HTTP extension is going to be defined as.
    pub const MODULE: &'static str = "wasi_experimental_http";

    /// Create a new HTTP extension object.
    /// `allowed_hosts` may be `None` (no outbound connections allowed)
    /// or a list of allowed host names.
    pub fn new() -> Result<Self, Error> {
        let state = Arc::new(RwLock::new(State::default()));
        Ok(HttpState { state })
    }

    pub fn add_to_linker<T>(
        &self,
        linker: &mut Linker<T>,
        get_cx: impl Fn(&T) -> &HttpCtx + Send + Sync + 'static,
    ) -> Result<(), Error> {
        let st = self.state.clone();
        linker.func_wrap(
            Self::MODULE,
            "close",
            move |handle: WasiHttpHandle| -> u32 {
                match HostCalls::close(st.clone(), handle) {
                    Ok(()) => 0,
                    Err(e) => e.into(),
                }
            },
        )?;

        let st = self.state.clone();
        linker.func_wrap(
            Self::MODULE,
            "body_read",
            move |mut caller: Caller<'_, T>,
                  handle: WasiHttpHandle,
                  buf_ptr: u32,
                  buf_len: u32,
                  buf_read_ptr: u32|
                  -> u32 {
                let memory = match memory_get(&mut caller) {
                    Ok(m) => m,
                    Err(e) => return e.into(),
                };

                let ctx = caller.as_context_mut();

                match HostCalls::body_read(
                    st.clone(),
                    memory,
                    ctx,
                    handle,
                    buf_ptr,
                    buf_len,
                    buf_read_ptr,
                ) {
                    Ok(()) => 0,
                    Err(e) => e.into(),
                }
            },
        )?;

        let st = self.state.clone();
        linker.func_wrap(
            Self::MODULE,
            "header_get",
            move |mut caller: Caller<'_, T>,
                  handle: WasiHttpHandle,
                  name_ptr: u32,
                  name_len: u32,
                  value_ptr: u32,
                  value_len: u32,
                  value_written_ptr: u32|
                  -> u32 {
                let memory = match memory_get(&mut caller) {
                    Ok(m) => m,
                    Err(e) => return e.into(),
                };

                let ctx = caller.as_context_mut();

                match HostCalls::header_get(
                    st.clone(),
                    memory,
                    ctx,
                    handle,
                    name_ptr,
                    name_len,
                    value_ptr,
                    value_len,
                    value_written_ptr,
                ) {
                    Ok(()) => 0,
                    Err(e) => e.into(),
                }
            },
        )?;

        let st = self.state.clone();
        linker.func_wrap(
            Self::MODULE,
            "headers_get_all",
            move |mut caller: Caller<'_, T>,
                  handle: WasiHttpHandle,
                  buf_ptr: u32,
                  buf_len: u32,
                  buf_read_ptr: u32|
                  -> u32 {
                let memory = match memory_get(&mut caller) {
                    Ok(m) => m,
                    Err(e) => return e.into(),
                };

                let ctx = caller.as_context_mut();

                match HostCalls::headers_get_all(
                    st.clone(),
                    memory,
                    ctx,
                    handle,
                    buf_ptr,
                    buf_len,
                    buf_read_ptr,
                ) {
                    Ok(()) => 0,
                    Err(e) => e.into(),
                }
            },
        )?;

        let st = self.state.clone();
        linker.func_wrap(
            Self::MODULE,
            "req",
            move |mut caller: Caller<'_, T>,
                  url_ptr: u32,
                  url_len: u32,
                  method_ptr: u32,
                  method_len: u32,
                  req_headers_ptr: u32,
                  req_headers_len: u32,
                  req_body_ptr: u32,
                  req_body_len: u32,
                  status_code_ptr: u32,
                  res_handle_ptr: u32|
                  -> u32 {
                let memory = match memory_get(&mut caller) {
                    Ok(m) => m,
                    Err(e) => return e.into(),
                };

                let ctx = caller.as_context_mut();
                let http_ctx = get_cx(ctx.data());
                let max_concurrent_requests = http_ctx.max_concurrent_requests;
                // TODO: There is probably a way to avoid this copy.
                let allowed_hosts = http_ctx.allowed_hosts.clone();

                match HostCalls::req(
                    st.clone(),
                    allowed_hosts.as_deref(),
                    max_concurrent_requests,
                    memory,
                    ctx,
                    url_ptr,
                    url_len,
                    method_ptr,
                    method_len,
                    req_headers_ptr,
                    req_headers_len,
                    req_body_ptr,
                    req_body_len,
                    status_code_ptr,
                    res_handle_ptr,
                ) {
                    Ok(()) => 0,
                    Err(e) => e.into(),
                }
            },
        )?;

        Ok(())
    }
}

#[tracing::instrument]
fn request(
    url: &str,
    headers: HeaderMap,
    method: Method,
    body: &[u8],
) -> Result<(u16, HeaderMap<HeaderValue>, Bytes), HttpError> {
    tracing::debug!(
        %url,
        ?headers,
        ?method,
        body_len = body.len(),
        "performing request"
    );
    let url: Url = url.parse().map_err(|_| HttpError::InvalidUrl)?;
    let body = body.to_vec();
    match Handle::try_current() {
        Ok(r) => {
            // If running in a Tokio runtime, spawn a new blocking executor
            // that will send the HTTP request, and block on its execution.
            // This attempts to avoid any deadlocks from other operations
            // already executing on the same executor (compared with just
            // blocking on the current one).
            //
            // This should only be a temporary workaround, until we take
            // advantage of async functions in Wasmtime.
            tracing::trace!("tokio runtime available, spawning request on tokio thread");
            block_on(r.spawn_blocking(move || {
                let client = Client::builder().build().unwrap();
                let res = block_on(
                    client
                        .request(method, url)
                        .headers(headers)
                        .body(body)
                        .send(),
                )?;
                Ok((
                    res.status().as_u16(),
                    res.headers().clone(),
                    block_on(res.bytes())?,
                ))
            }))
            .map_err(|_| HttpError::RuntimeError)?
        }
        Err(_) => {
            tracing::trace!("no tokio runtime available, using blocking request");
            let res = reqwest::blocking::Client::new()
                .request(method, url)
                .headers(headers)
                .body(body)
                .send()?;
            return Ok((res.status().as_u16(), res.headers().clone(), res.bytes()?));
        }
    }
}

/// Get the exported memory block called `memory`.
/// This will return an `HttpError::MemoryNotFound` if the module does
/// not export a memory block.
fn memory_get<T>(caller: &mut Caller<'_, T>) -> Result<Memory, HttpError> {
    if let Some(Extern::Memory(mem)) = caller.get_export(MEMORY) {
        Ok(mem)
    } else {
        Err(HttpError::MemoryNotFound)
    }
}

/// Get a slice of length `len` from `memory`, starting at `offset`.
/// This will return an `HttpError::BufferTooSmall` if the size of the
/// requested slice is larger than the memory size.
fn slice_from_memory(
    memory: &Memory,
    mut ctx: impl AsContextMut,
    offset: u32,
    len: u32,
) -> Result<Vec<u8>, HttpError> {
    let required_memory_size = offset.checked_add(len).ok_or(HttpError::BufferTooSmall)? as usize;

    if required_memory_size > memory.data_size(&mut ctx) {
        return Err(HttpError::BufferTooSmall);
    }

    let mut buf = vec![0u8; len as usize];
    memory.read(&mut ctx, offset as usize, buf.as_mut_slice())?;
    Ok(buf)
}

/// Read a string of byte length `len` from `memory`, starting at `offset`.
fn string_from_memory(
    memory: &Memory,
    ctx: impl AsContextMut,
    offset: u32,
    len: u32,
) -> Result<String, HttpError> {
    let slice = slice_from_memory(memory, ctx, offset, len)?;
    Ok(std::str::from_utf8(&slice)?.to_string())
}

/// Check if guest module is allowed to send request to URL, based on the list of
/// allowed hosts defined by the runtime.
/// If `None` is passed, the guest module is not allowed to send the request.
fn is_allowed(url: &str, allowed_hosts: Option<&[String]>) -> Result<bool, HttpError> {
    let url_host = Url::parse(url)
        .map_err(|_| HttpError::InvalidUrl)?
        .host_str()
        .ok_or(HttpError::InvalidUrl)?
        .to_owned();
    match allowed_hosts {
        Some(domains) => {
            // check domains has any "insecure:allow-all" wildcard
            if domains.iter().any(|domain| domain == ALLOW_ALL_HOSTS) {
                Ok(true)
            } else {
                let allowed: Result<Vec<_>, _> = domains.iter().map(|d| Url::parse(d)).collect();
                let allowed = allowed.map_err(|_| HttpError::InvalidUrl)?;

                Ok(allowed
                    .iter()
                    .map(|u| u.host_str().unwrap())
                    .any(|x| x == url_host.as_str()))
            }
        }
        None => Ok(false),
    }
}

// The following two functions are copied from the `wasi_experimental_http`
// crate, because the Windows linker apparently cannot handle unresolved
// symbols from a crate, even when the caller does not actually use any of the
// external symbols.
//
// https://github.com/rust-lang/rust/issues/86125

/// Decode a header map from a string.
fn string_to_header_map(s: &str) -> Result<HeaderMap, Error> {
    let mut headers = HeaderMap::new();
    for entry in s.lines() {
        let mut parts = entry.splitn(2, ':');
        #[allow(clippy::or_fun_call)]
        let k = parts.next().ok_or(anyhow::format_err!(
            "Invalid serialized header: [{}]",
            entry
        ))?;
        let v = parts.next().unwrap();
        headers.insert(HeaderName::from_str(k)?, HeaderValue::from_str(v)?);
    }
    Ok(headers)
}

/// Encode a header map as a string.
fn header_map_to_string(hm: &HeaderMap) -> Result<String, Error> {
    let mut res = String::new();
    for (name, value) in hm
        .iter()
        .map(|(name, value)| (name.as_str(), std::str::from_utf8(value.as_bytes())))
    {
        let value = value?;
        anyhow::ensure!(
            !name
                .chars()
                .any(|x| x.is_control() || "(),/:;<=>?@[\\]{}".contains(x)),
            "Invalid header name"
        );
        anyhow::ensure!(
            !value.chars().any(|x| x.is_control()),
            "Invalid header value"
        );
        res.push_str(&format!("{}:{}\n", name, value));
    }
    Ok(res)
}

#[test]
#[allow(clippy::bool_assert_comparison)]
fn test_allowed_domains() {
    let allowed_domains = vec![
        "https://api.brigade.sh".to_string(),
        "https://example.com".to_string(),
        "http://192.168.0.1".to_string(),
    ];

    assert_eq!(
        true,
        is_allowed(
            "https://api.brigade.sh/healthz",
            Some(allowed_domains.as_ref())
        )
        .unwrap()
    );
    assert_eq!(
        true,
        is_allowed(
            "https://example.com/some/path/with/more/paths",
            Some(allowed_domains.as_ref())
        )
        .unwrap()
    );
    assert_eq!(
        true,
        is_allowed("http://192.168.0.1/login", Some(allowed_domains.as_ref())).unwrap()
    );
    assert_eq!(
        false,
        is_allowed("https://test.brigade.sh", Some(allowed_domains.as_ref())).unwrap()
    );
}

#[test]
#[allow(clippy::bool_assert_comparison)]
fn test_allowed_domains_with_wildcard() {
    let allowed_domains = vec![
        "https://example.com".to_string(),
        ALLOW_ALL_HOSTS.to_string(),
        "http://192.168.0.1".to_string(),
    ];

    assert_eq!(
        true,
        is_allowed(
            "https://api.brigade.sh/healthz",
            Some(allowed_domains.as_ref())
        )
        .unwrap()
    );
    assert_eq!(
        true,
        is_allowed(
            "https://example.com/some/path/with/more/paths",
            Some(allowed_domains.as_ref())
        )
        .unwrap()
    );
    assert_eq!(
        true,
        is_allowed("http://192.168.0.1/login", Some(allowed_domains.as_ref())).unwrap()
    );
    assert_eq!(
        true,
        is_allowed("https://test.brigade.sh", Some(allowed_domains.as_ref())).unwrap()
    );
}

#[test]
#[should_panic]
#[allow(clippy::bool_assert_comparison)]
fn test_url_parsing() {
    let allowed_domains = vec![ALLOW_ALL_HOSTS.to_string()];

    is_allowed("not even a url", Some(allowed_domains.as_ref())).unwrap();
}

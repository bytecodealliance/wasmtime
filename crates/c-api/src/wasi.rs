//! The WASI embedding API definitions for Wasmtime.

use crate::wasm_byte_vec_t;
use bytes::Bytes;
use std::ffi::{CStr, c_char, c_void};
use std::fs::File;
use std::path::Path;
use std::pin::Pin;
use std::slice;
use std::task::{Context, Poll};
use tokio::io::{self, AsyncWrite};
use wasmtime::Result;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi_io::streams::StreamError;

unsafe fn cstr_to_path<'a>(path: *const c_char) -> Option<&'a Path> {
    CStr::from_ptr(path).to_str().map(Path::new).ok()
}

unsafe fn cstr_to_str<'a>(s: *const c_char) -> Option<&'a str> {
    CStr::from_ptr(s).to_str().ok()
}

unsafe fn open_file(path: *const c_char) -> Option<File> {
    File::open(cstr_to_path(path)?).ok()
}

unsafe fn create_file(path: *const c_char) -> Option<File> {
    File::create(cstr_to_path(path)?).ok()
}

#[repr(C)]
pub struct wasi_config_t {
    builder: WasiCtxBuilder,
}

wasmtime_c_api_macros::declare_own!(wasi_config_t);

impl wasi_config_t {
    pub fn into_wasi_ctx(mut self) -> Result<WasiP1Ctx> {
        Ok(self.builder.build_p1())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasi_config_new() -> Box<wasi_config_t> {
    Box::new(wasi_config_t {
        builder: WasiCtxBuilder::new(),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasi_config_set_argv(
    config: &mut wasi_config_t,
    argc: usize,
    argv: *const *const c_char,
) -> bool {
    for arg in slice::from_raw_parts(argv, argc) {
        let arg = match CStr::from_ptr(*arg).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        };
        config.builder.arg(arg);
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn wasi_config_inherit_argv(config: &mut wasi_config_t) {
    config.builder.inherit_args();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasi_config_set_env(
    config: &mut wasi_config_t,
    envc: usize,
    names: *const *const c_char,
    values: *const *const c_char,
) -> bool {
    let names = slice::from_raw_parts(names, envc);
    let values = slice::from_raw_parts(values, envc);

    for (k, v) in names.iter().zip(values) {
        let k = match cstr_to_str(*k) {
            Some(s) => s,
            None => return false,
        };
        let v = match cstr_to_str(*v) {
            Some(s) => s,
            None => return false,
        };
        config.builder.env(k, v);
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn wasi_config_inherit_env(config: &mut wasi_config_t) {
    config.builder.inherit_env();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasi_config_set_stdin_file(
    config: &mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match open_file(path) {
        Some(f) => f,
        None => return false,
    };

    let file = tokio::fs::File::from_std(file);
    let stdin_stream = wasmtime_wasi::cli::AsyncStdinStream::new(file);
    config.builder.stdin(stdin_stream);

    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasi_config_set_stdin_bytes(
    config: &mut wasi_config_t,
    binary: &mut wasm_byte_vec_t,
) {
    let binary = binary.take();
    let binary = wasmtime_wasi::p2::pipe::MemoryInputPipe::new(binary);
    config.builder.stdin(binary);
}

#[unsafe(no_mangle)]
pub extern "C" fn wasi_config_inherit_stdin(config: &mut wasi_config_t) {
    config.builder.inherit_stdin();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasi_config_set_stdout_file(
    config: &mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match create_file(path) {
        Some(f) => f,
        None => return false,
    };

    config
        .builder
        .stdout(wasmtime_wasi::cli::OutputFile::new(file));

    true
}

#[unsafe(no_mangle)]
pub extern "C" fn wasi_config_inherit_stdout(config: &mut wasi_config_t) {
    config.builder.inherit_stdout();
}

struct CustomOutputStreamInner {
    foreign_data: crate::ForeignData,
    callback: extern "C" fn(*mut c_void, *const u8, usize) -> isize,
}

impl CustomOutputStreamInner {
    pub fn raw_write(&self, buf: &[u8]) -> io::Result<usize> {
        let wrote = (self.callback)(self.foreign_data.data, buf.as_ptr(), buf.len());

        if wrote >= 0 {
            Ok(wrote as _)
        } else {
            Err(io::Error::from_raw_os_error(wrote.abs() as _))
        }
    }
}

#[derive(Clone)]
pub struct CustomOutputStream {
    inner: std::sync::Arc<CustomOutputStreamInner>,
}

impl CustomOutputStream {
    pub fn new(
        foreign_data: crate::ForeignData,
        callback: extern "C" fn(*mut c_void, *const u8, usize) -> isize,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(CustomOutputStreamInner {
                foreign_data,
                callback,
            }),
        }
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi::p2::Pollable for CustomOutputStream {
    async fn ready(&mut self) {}
}

#[async_trait::async_trait]
impl wasmtime_wasi::p2::OutputStream for CustomOutputStream {
    fn write(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        let wrote = self
            .inner
            .raw_write(&bytes)
            .map_err(|e| StreamError::LastOperationFailed(e.into()))?;

        if wrote != bytes.len() {
            return Err(StreamError::LastOperationFailed(wasmtime::format_err!(
                "Partial writes in wasip2 implementation are not allowed"
            )));
        }

        Ok(())
    }
    fn flush(&mut self) -> Result<(), StreamError> {
        Ok(())
    }
    fn check_write(&mut self) -> Result<usize, StreamError> {
        Ok(usize::MAX)
    }
}

impl AsyncWrite for CustomOutputStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.inner.raw_write(buf))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl wasmtime_wasi::cli::IsTerminal for CustomOutputStream {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl wasmtime_wasi::cli::StdoutStream for CustomOutputStream {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasi_config_set_stdout_custom(
    config: &mut wasi_config_t,
    callback: extern "C" fn(*mut c_void, *const u8, usize) -> isize,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    config.builder.stdout(CustomOutputStream::new(
        crate::ForeignData { data, finalizer },
        callback,
    ));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasi_config_set_stderr_file(
    config: &mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match create_file(path) {
        Some(f) => f,
        None => return false,
    };

    config
        .builder
        .stderr(wasmtime_wasi::cli::OutputFile::new(file));

    true
}

#[unsafe(no_mangle)]
pub extern "C" fn wasi_config_inherit_stderr(config: &mut wasi_config_t) {
    config.builder.inherit_stderr();
}

#[unsafe(no_mangle)]
pub extern "C" fn wasi_config_set_stderr_custom(
    config: &mut wasi_config_t,
    callback: extern "C" fn(*mut c_void, *const u8, usize) -> isize,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    config.builder.stderr(CustomOutputStream::new(
        crate::ForeignData { data, finalizer },
        callback,
    ));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasi_config_preopen_dir(
    config: &mut wasi_config_t,
    path: *const c_char,
    guest_path: *const c_char,
    dir_perms: usize,
    file_perms: usize,
) -> bool {
    let guest_path = match cstr_to_str(guest_path) {
        Some(p) => p,
        None => return false,
    };

    let host_path = match cstr_to_path(path) {
        Some(p) => p,
        None => return false,
    };

    let dir_perms = match wasmtime_wasi::DirPerms::from_bits(dir_perms) {
        Some(p) => p,
        None => return false,
    };

    let file_perms = match wasmtime_wasi::FilePerms::from_bits(file_perms) {
        Some(p) => p,
        None => return false,
    };

    config
        .builder
        .preopened_dir(host_path, guest_path, dir_perms, file_perms)
        .is_ok()
}

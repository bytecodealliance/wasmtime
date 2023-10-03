use crate::preview2::filesystem::FileInputStream;
use crate::preview2::poll::Subscribe;
use anyhow::Error;
use anyhow::Result;
use bytes::Bytes;
use std::fmt;

/// An error which should be reported to Wasm as a runtime error, rather than
/// an error which should trap Wasm execution. The definition for runtime
/// stream errors is the empty type, so the contents of this error will only
/// be available via a `tracing`::event` at `Level::DEBUG`.
pub struct StreamRuntimeError(anyhow::Error);
impl From<anyhow::Error> for StreamRuntimeError {
    fn from(e: anyhow::Error) -> Self {
        StreamRuntimeError(e)
    }
}
impl fmt::Debug for StreamRuntimeError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Stream runtime error: {:?}", self.0)
    }
}
impl fmt::Display for StreamRuntimeError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Stream runtime error")
    }
}
impl std::error::Error for StreamRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamState {
    Open,
    Closed,
}

impl StreamState {
    pub fn is_closed(&self) -> bool {
        *self == Self::Closed
    }
}

/// Host trait for implementing the `wasi:io/streams.input-stream` resource: A
/// bytestream which can be read from.
#[async_trait::async_trait]
pub trait HostInputStream: Subscribe {
    /// Read bytes. On success, returns a pair holding the number of bytes
    /// read and a flag indicating whether the end of the stream was reached.
    /// Important: this read must be non-blocking!
    /// Returning an Err which downcasts to a [`StreamRuntimeError`] will be
    /// reported to Wasm as the empty error result. Otherwise, errors will trap.
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error>;

    /// Read bytes from a stream and discard them. Important: this method must
    /// be non-blocking!
    /// Returning an Error which downcasts to a StreamRuntimeError will be
    /// reported to Wasm as the empty error result. Otherwise, errors will trap.
    fn skip(&mut self, nelem: usize) -> Result<(usize, StreamState), Error> {
        let mut nread = 0;
        let mut state = StreamState::Open;

        let (bs, read_state) = self.read(nelem)?;
        // TODO: handle the case where `bs.len()` is less than `nelem`
        nread += bs.len();
        if read_state.is_closed() {
            state = read_state;
        }

        Ok((nread, state))
    }
}

#[derive(Debug)]
pub enum OutputStreamError {
    Closed,
    LastOperationFailed(anyhow::Error),
    Trap(anyhow::Error),
}
impl std::fmt::Display for OutputStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputStreamError::Closed => write!(f, "closed"),
            OutputStreamError::LastOperationFailed(e) => write!(f, "last operation failed: {e}"),
            OutputStreamError::Trap(e) => write!(f, "trap: {e}"),
        }
    }
}
impl std::error::Error for OutputStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OutputStreamError::Closed => None,
            OutputStreamError::LastOperationFailed(e) | OutputStreamError::Trap(e) => e.source(),
        }
    }
}

/// Host trait for implementing the `wasi:io/streams.output-stream` resource:
/// A bytestream which can be written to.
#[async_trait::async_trait]
pub trait HostOutputStream: Subscribe {
    /// Write bytes after obtaining a permit to write those bytes
    /// Prior to calling [`write`](Self::write)
    /// the caller must call [`write_ready`](Self::write_ready),
    /// which resolves to a non-zero permit
    ///
    /// This method must never block.
    /// [`write_ready`](Self::write_ready) permit indicates the maximum amount of bytes that are
    /// permitted to be written in a single [`write`](Self::write) following the
    /// [`write_ready`](Self::write_ready) resolution
    ///
    /// # Errors
    ///
    /// Returns an [OutputStreamError] if:
    /// - stream is closed
    /// - prior operation ([`write`](Self::write) or [`flush`](Self::flush)) failed
    /// - caller performed an illegal operation (e.g. wrote more bytes than were permitted)
    fn write(&mut self, bytes: Bytes) -> Result<(), OutputStreamError>;

    /// Trigger a flush of any bytes buffered in this stream implementation.
    ///
    /// This method may be called at any time and must never block.
    ///
    /// After this method is called, [`write_ready`](Self::write_ready) must pend until flush is
    /// complete.
    /// When [`write_ready`](Self::write_ready) becomes ready after a flush, that guarantees that
    /// all prior writes have been flushed from the implementation successfully, or that any error
    /// associated with those writes is reported in the return value of [`flush`](Self::flush) or
    /// [`write_ready`](Self::write_ready)
    ///
    /// # Errors
    ///
    /// Returns an [OutputStreamError] if:
    /// - stream is closed
    /// - prior operation ([`write`](Self::write) or [`flush`](Self::flush)) failed
    /// - caller performed an illegal operation (e.g. wrote more bytes than were permitted)
    fn flush(&mut self) -> Result<(), OutputStreamError>;

    /// Returns the number of bytes that are ready to be written to this stream.
    ///
    /// Zero bytes indicates that this stream is not currently ready for writing
    /// and `ready()` must be awaited first.
    ///
    /// # Errors
    ///
    /// Returns an [OutputStreamError] if:
    /// - stream is closed
    /// - prior operation ([`write`](Self::write) or [`flush`](Self::flush)) failed
    fn check_write(&mut self) -> Result<usize, OutputStreamError>;

    /// Repeatedly write a byte to a stream.
    /// Important: this write must be non-blocking!
    /// Returning an Err which downcasts to a [`StreamRuntimeError`] will be
    /// reported to Wasm as the empty error result. Otherwise, errors will trap.
    fn write_zeroes(&mut self, nelem: usize) -> Result<(), OutputStreamError> {
        // TODO: We could optimize this to not allocate one big zeroed buffer, and instead write
        // repeatedly from a 'static buffer of zeros.
        let bs = Bytes::from_iter(core::iter::repeat(0 as u8).take(nelem));
        self.write(bs)?;
        Ok(())
    }

    /// Simultaneously waits for this stream to be writable and then returns how
    /// much may be written or the last error that happened.
    async fn write_ready(&mut self) -> Result<usize, OutputStreamError> {
        self.ready().await;
        self.check_write()
    }
}

#[async_trait::async_trait]
impl Subscribe for Box<dyn HostOutputStream> {
    async fn ready(&mut self) {
        (**self).ready().await
    }
}

pub enum InputStream {
    Host(Box<dyn HostInputStream>),
    File(FileInputStream),
}

#[async_trait::async_trait]
impl Subscribe for InputStream {
    async fn ready(&mut self) {
        match self {
            InputStream::Host(stream) => stream.ready().await,
            // Files are always ready
            InputStream::File(_) => {}
        }
    }
}

pub type OutputStream = Box<dyn HostOutputStream>;

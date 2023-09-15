use crate::preview2::bindings::io::streams::{InputStream, OutputStream};
use crate::preview2::filesystem::FileInputStream;
use crate::preview2::{Table, TableError};
use anyhow::Error;
use bytes::Bytes;
use std::fmt;
use wasmtime::component::Resource;

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
pub trait HostInputStream: Send + Sync {
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

    /// Check for read readiness: this method blocks until the stream is ready
    /// for reading.
    /// Returning an error will trap execution.
    async fn ready(&mut self) -> Result<(), Error>;
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
pub trait HostOutputStream: Send + Sync {
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

    /// Returns a future, which:
    /// - when pending, indicates 0 bytes are permitted for writing
    /// - when ready, returns a non-zero number of bytes permitted to write
    ///
    /// # Errors
    ///
    /// Returns an [OutputStreamError] if:
    /// - stream is closed
    /// - prior operation ([`write`](Self::write) or [`flush`](Self::flush)) failed
    async fn write_ready(&mut self) -> Result<usize, OutputStreamError>;

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
}

pub(crate) enum InternalInputStream {
    Host(Box<dyn HostInputStream>),
    File(FileInputStream),
}

pub(crate) trait InternalTableStreamExt {
    fn push_internal_input_stream(
        &mut self,
        istream: InternalInputStream,
    ) -> Result<Resource<InputStream>, TableError>;
    fn push_internal_input_stream_child<T: 'static>(
        &mut self,
        istream: InternalInputStream,
        parent: Resource<T>,
    ) -> Result<Resource<InputStream>, TableError>;
    fn get_internal_input_stream_mut(
        &mut self,
        fd: &Resource<InputStream>,
    ) -> Result<&mut InternalInputStream, TableError>;
    fn delete_internal_input_stream(
        &mut self,
        fd: Resource<InputStream>,
    ) -> Result<InternalInputStream, TableError>;
}
impl InternalTableStreamExt for Table {
    fn push_internal_input_stream(
        &mut self,
        istream: InternalInputStream,
    ) -> Result<Resource<InputStream>, TableError> {
        Ok(Resource::new_own(self.push(Box::new(istream))?))
    }
    fn push_internal_input_stream_child<T: 'static>(
        &mut self,
        istream: InternalInputStream,
        parent: Resource<T>,
    ) -> Result<Resource<InputStream>, TableError> {
        Ok(Resource::new_own(
            self.push_child(Box::new(istream), parent.rep())?,
        ))
    }
    fn get_internal_input_stream_mut(
        &mut self,
        fd: &Resource<InputStream>,
    ) -> Result<&mut InternalInputStream, TableError> {
        self.get_mut(fd.rep())
    }
    fn delete_internal_input_stream(
        &mut self,
        fd: Resource<InputStream>,
    ) -> Result<InternalInputStream, TableError> {
        self.delete(fd.rep())
    }
}

/// Extension trait for managing [`HostInputStream`]s and [`HostOutputStream`]s in the [`Table`].
pub trait TableStreamExt {
    /// Push a [`HostInputStream`] into a [`Table`], returning the table index.
    fn push_input_stream(
        &mut self,
        istream: Box<dyn HostInputStream>,
    ) -> Result<Resource<InputStream>, TableError>;
    /// Same as [`push_input_stream`](Self::push_output_stream) except assigns a parent resource to
    /// the input-stream created.
    fn push_input_stream_child<T: 'static>(
        &mut self,
        istream: Box<dyn HostInputStream>,
        parent: Resource<T>,
    ) -> Result<Resource<InputStream>, TableError>;
    /// Get a mutable reference to a [`HostInputStream`] in a [`Table`].
    fn get_input_stream_mut(
        &mut self,
        fd: &Resource<InputStream>,
    ) -> Result<&mut dyn HostInputStream, TableError>;
    /// Remove [`HostInputStream`] from table:
    fn delete_input_stream(
        &mut self,
        fd: Resource<InputStream>,
    ) -> Result<Box<dyn HostInputStream>, TableError>;

    /// Push a [`HostOutputStream`] into a [`Table`], returning the table index.
    fn push_output_stream(
        &mut self,
        ostream: Box<dyn HostOutputStream>,
    ) -> Result<Resource<OutputStream>, TableError>;
    /// Same as [`push_output_stream`](Self::push_output_stream) except assigns a parent resource
    /// to the output-stream created.
    fn push_output_stream_child<T: 'static>(
        &mut self,
        ostream: Box<dyn HostOutputStream>,
        parent: Resource<T>,
    ) -> Result<Resource<OutputStream>, TableError>;
    /// Get a mutable reference to a [`HostOutputStream`] in a [`Table`].
    fn get_output_stream_mut(
        &mut self,
        fd: &Resource<OutputStream>,
    ) -> Result<&mut dyn HostOutputStream, TableError>;

    /// Remove [`HostOutputStream`] from table:
    fn delete_output_stream(
        &mut self,
        fd: Resource<OutputStream>,
    ) -> Result<Box<dyn HostOutputStream>, TableError>;
}
impl TableStreamExt for Table {
    fn push_input_stream(
        &mut self,
        istream: Box<dyn HostInputStream>,
    ) -> Result<Resource<InputStream>, TableError> {
        self.push_internal_input_stream(InternalInputStream::Host(istream))
    }
    fn push_input_stream_child<T: 'static>(
        &mut self,
        istream: Box<dyn HostInputStream>,
        parent: Resource<T>,
    ) -> Result<Resource<InputStream>, TableError> {
        self.push_internal_input_stream_child(InternalInputStream::Host(istream), parent)
    }
    fn get_input_stream_mut(
        &mut self,
        fd: &Resource<InputStream>,
    ) -> Result<&mut dyn HostInputStream, TableError> {
        match self.get_internal_input_stream_mut(fd)? {
            InternalInputStream::Host(ref mut h) => Ok(h.as_mut()),
            _ => Err(TableError::WrongType),
        }
    }
    fn delete_input_stream(
        &mut self,
        fd: Resource<InputStream>,
    ) -> Result<Box<dyn HostInputStream>, TableError> {
        let occ = self.entry(fd.rep())?;
        match occ.get().downcast_ref::<InternalInputStream>() {
            Some(InternalInputStream::Host(_)) => {
                let any = occ.remove_entry()?;
                match *any.downcast().expect("downcast checked above") {
                    InternalInputStream::Host(h) => Ok(h),
                    _ => unreachable!("variant checked above"),
                }
            }
            _ => Err(TableError::WrongType),
        }
    }

    fn push_output_stream(
        &mut self,
        ostream: Box<dyn HostOutputStream>,
    ) -> Result<Resource<OutputStream>, TableError> {
        Ok(Resource::new_own(self.push(Box::new(ostream))?))
    }
    fn push_output_stream_child<T: 'static>(
        &mut self,
        ostream: Box<dyn HostOutputStream>,
        parent: Resource<T>,
    ) -> Result<Resource<OutputStream>, TableError> {
        Ok(Resource::new_own(
            self.push_child(Box::new(ostream), parent.rep())?,
        ))
    }
    fn get_output_stream_mut(
        &mut self,
        fd: &Resource<OutputStream>,
    ) -> Result<&mut dyn HostOutputStream, TableError> {
        let boxed: &mut Box<dyn HostOutputStream> = self.get_mut(fd.rep())?;
        Ok(boxed.as_mut())
    }
    fn delete_output_stream(
        &mut self,
        fd: Resource<OutputStream>,
    ) -> Result<Box<dyn HostOutputStream>, TableError> {
        self.delete(fd.rep())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn input_stream_in_table() {
        let dummy = crate::preview2::pipe::ClosedInputStream;
        let mut table = Table::new();
        // Put it into the table:
        let ix = table.push_input_stream(Box::new(dummy)).unwrap();
        // Get a mut ref to it:
        let _ = table.get_input_stream_mut(ix).unwrap();
        // Fails at wrong type:
        assert!(matches!(
            table.get_output_stream_mut(ix),
            Err(TableError::WrongType)
        ));
        // Delete it:
        let _ = table.delete_input_stream(ix).unwrap();
        // Now absent from table:
        assert!(matches!(
            table.get_input_stream_mut(ix),
            Err(TableError::NotPresent)
        ));
    }

    #[test]
    fn output_stream_in_table() {
        let dummy = crate::preview2::pipe::SinkOutputStream;
        let mut table = Table::new();
        // Put it in the table:
        let ix = table.push_output_stream(Box::new(dummy)).unwrap();
        // Get a mut ref to it:
        let _ = table.get_output_stream_mut(ix).unwrap();
        // Fails at wrong type:
        assert!(matches!(
            table.get_input_stream_mut(ix),
            Err(TableError::WrongType)
        ));
        // Delete it:
        let _ = table.delete_output_stream(ix).unwrap();
        // Now absent:
        assert!(matches!(
            table.get_output_stream_mut(ix),
            Err(TableError::NotPresent)
        ));
    }
}

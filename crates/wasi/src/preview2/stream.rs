use crate::preview2::filesystem::{FileInputStream, FileOutputStream};
use crate::preview2::{Table, TableError};
use anyhow::Error;
use bytes::Bytes;

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
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error>;

    /// Read bytes from a stream and discard them. Important: this method must
    /// be non-blocking!
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
    async fn ready(&mut self) -> Result<(), Error>;
}

/// Host trait for implementing the `wasi:io/streams.output-stream` resource:
/// A bytestream which can be written to.
#[async_trait::async_trait]
pub trait HostOutputStream: Send + Sync {
    /// Write bytes. On success, returns the number of bytes written.
    /// Important: this write must be non-blocking!
    fn write(&mut self, bytes: Bytes) -> Result<(usize, StreamState), Error>;

    /// Transfer bytes directly from an input stream to an output stream.
    /// Important: this splice must be non-blocking!
    fn splice(
        &mut self,
        src: &mut dyn HostInputStream,
        nelem: usize,
    ) -> Result<(usize, StreamState), Error> {
        let mut nspliced = 0;
        let mut state = StreamState::Open;

        // TODO: handle the case where `bs.len()` is less than `nelem`
        let (bs, read_state) = src.read(nelem)?;
        // TODO: handle the case where write returns less than `bs.len()`
        let (nwritten, _write_state) = self.write(bs)?;
        nspliced += nwritten;
        if read_state.is_closed() {
            state = read_state;
        }

        Ok((nspliced, state))
    }

    /// Repeatedly write a byte to a stream. Important: this write must be
    /// non-blocking!
    fn write_zeroes(&mut self, nelem: usize) -> Result<(usize, StreamState), Error> {
        // TODO: We could optimize this to not allocate one big zeroed buffer, and instead write
        // repeatedly from a 'static buffer of zeros.
        let bs = Bytes::from_iter(core::iter::repeat(0 as u8).take(nelem));
        let r = self.write(bs)?;
        Ok(r)
    }

    /// Check for write readiness: this method blocks until the stream is
    /// ready for writing.
    async fn ready(&mut self) -> Result<(), Error>;
}

pub(crate) enum InternalInputStream {
    Host(Box<dyn HostInputStream>),
    File(FileInputStream),
}

pub(crate) enum InternalOutputStream {
    Host(Box<dyn HostOutputStream>),
    File(FileOutputStream),
}

pub(crate) trait InternalTableStreamExt {
    fn push_internal_input_stream(
        &mut self,
        istream: InternalInputStream,
    ) -> Result<u32, TableError>;
    fn get_internal_input_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut InternalInputStream, TableError>;
    fn delete_internal_input_stream(&mut self, fd: u32) -> Result<InternalInputStream, TableError>;

    fn push_internal_output_stream(
        &mut self,
        ostream: InternalOutputStream,
    ) -> Result<u32, TableError>;
    fn get_internal_output_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut InternalOutputStream, TableError>;
    fn delete_internal_output_stream(
        &mut self,
        fd: u32,
    ) -> Result<InternalOutputStream, TableError>;
}
impl InternalTableStreamExt for Table {
    fn push_internal_input_stream(
        &mut self,
        istream: InternalInputStream,
    ) -> Result<u32, TableError> {
        self.push(Box::new(istream))
    }
    fn get_internal_input_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut InternalInputStream, TableError> {
        self.get_mut(fd)
    }
    fn delete_internal_input_stream(&mut self, fd: u32) -> Result<InternalInputStream, TableError> {
        self.delete(fd)
    }

    fn push_internal_output_stream(
        &mut self,
        ostream: InternalOutputStream,
    ) -> Result<u32, TableError> {
        self.push(Box::new(ostream))
    }
    fn get_internal_output_stream_mut(
        &mut self,
        fd: u32,
    ) -> Result<&mut InternalOutputStream, TableError> {
        self.get_mut(fd)
    }
    fn delete_internal_output_stream(
        &mut self,
        fd: u32,
    ) -> Result<InternalOutputStream, TableError> {
        self.delete(fd)
    }
}

/// Extension trait for managing [`HostInputStream`]s and [`HostOutputStream`]s in the [`Table`].
pub trait TableStreamExt {
    /// Push a [`HostInputStream`] into a [`Table`], returning the table index.
    fn push_input_stream(&mut self, istream: Box<dyn HostInputStream>) -> Result<u32, TableError>;
    /// Get a mutable reference to a [`HostInputStream`] in a [`Table`].
    fn get_input_stream_mut(&mut self, fd: u32) -> Result<&mut dyn HostInputStream, TableError>;
    /// Remove [`HostInputStream`] from table:
    fn delete_input_stream(&mut self, fd: u32) -> Result<Box<dyn HostInputStream>, TableError>;

    /// Push a [`HostOutputStream`] into a [`Table`], returning the table index.
    fn push_output_stream(&mut self, ostream: Box<dyn HostOutputStream>)
        -> Result<u32, TableError>;
    /// Get a mutable reference to a [`HostOutputStream`] in a [`Table`].
    fn get_output_stream_mut(&mut self, fd: u32) -> Result<&mut dyn HostOutputStream, TableError>;

    /// Remove [`HostOutputStream`] from table:
    fn delete_output_stream(&mut self, fd: u32) -> Result<Box<dyn HostOutputStream>, TableError>;
}
impl TableStreamExt for Table {
    fn push_input_stream(&mut self, istream: Box<dyn HostInputStream>) -> Result<u32, TableError> {
        self.push_internal_input_stream(InternalInputStream::Host(istream))
    }
    fn get_input_stream_mut(&mut self, fd: u32) -> Result<&mut dyn HostInputStream, TableError> {
        match self.get_internal_input_stream_mut(fd)? {
            InternalInputStream::Host(ref mut h) => Ok(h.as_mut()),
            _ => Err(TableError::WrongType),
        }
    }
    fn delete_input_stream(&mut self, fd: u32) -> Result<Box<dyn HostInputStream>, TableError> {
        let occ = self.entry(fd)?;
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
    ) -> Result<u32, TableError> {
        self.push_internal_output_stream(InternalOutputStream::Host(ostream))
    }
    fn get_output_stream_mut(&mut self, fd: u32) -> Result<&mut dyn HostOutputStream, TableError> {
        match self.get_internal_output_stream_mut(fd)? {
            InternalOutputStream::Host(ref mut h) => Ok(h.as_mut()),
            _ => Err(TableError::WrongType),
        }
    }
    fn delete_output_stream(&mut self, fd: u32) -> Result<Box<dyn HostOutputStream>, TableError> {
        let occ = self.entry(fd)?;
        match occ.get().downcast_ref::<InternalOutputStream>() {
            Some(InternalOutputStream::Host(_)) => {
                let any = occ.remove_entry()?;
                match *any.downcast().expect("downcast checked above") {
                    InternalOutputStream::Host(h) => Ok(h),
                    _ => unreachable!("variant checked above"),
                }
            }
            _ => Err(TableError::WrongType),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn input_stream_in_table() {
        struct DummyInputStream;
        #[async_trait::async_trait]
        impl HostInputStream for DummyInputStream {
            fn read(&mut self, _size: usize) -> Result<(Bytes, StreamState), Error> {
                unimplemented!();
            }
            async fn ready(&mut self) -> Result<(), Error> {
                unimplemented!();
            }
        }

        let dummy = DummyInputStream;
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
        struct DummyOutputStream;
        #[async_trait::async_trait]
        impl HostOutputStream for DummyOutputStream {
            fn write(&mut self, _: Bytes) -> Result<(usize, StreamState), Error> {
                unimplemented!();
            }
            async fn ready(&mut self) -> Result<(), Error> {
                unimplemented!();
            }
        }

        let dummy = DummyOutputStream;
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

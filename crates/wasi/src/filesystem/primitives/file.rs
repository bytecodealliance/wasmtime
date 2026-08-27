use std::io;

/// Unix-specific extensions to [`fs::File`].
#[cfg(unix)]
pub trait FileExt {
    /// Reads a number of bytes starting from a given offset.
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize>;

    /// Reads the exact number of bytes required to fill `buf` from the given offset.
    fn read_exact_at(&self, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
        while !buf.is_empty() {
            match self.read_at(buf, offset) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                    offset += n as u64;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        } else {
            Ok(())
        }
    }

    /// Writes a number of bytes starting from a given offset.
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize>;

    /// Attempts to write an entire buffer starting from a given offset.
    fn write_all_at(&self, mut buf: &[u8], mut offset: u64) -> io::Result<()> {
        while !buf.is_empty() {
            match self.write_at(buf, offset) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => {
                    buf = &buf[n..];
                    offset += n as u64
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

/// WASI-specific extensions to [`fs::File`].
#[cfg(target_os = "wasi")]
pub trait FileExt {
    /// Reads a number of bytes starting from a given offset.
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        let bufs = &mut [io::IoSliceMut::new(buf)];
        self.read_vectored_at(bufs, offset)
    }

    /// Reads a number of bytes starting from a given offset.
    fn read_vectored_at(&self, bufs: &mut [io::IoSliceMut<'_>], offset: u64) -> io::Result<usize>;

    /// Reads the exact number of byte required to fill `buf` from the given offset.
    fn read_exact_at(&self, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
        while !buf.is_empty() {
            match self.read_at(buf, offset) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                    offset += n as u64;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        } else {
            Ok(())
        }
    }

    /// Writes a number of bytes starting from a given offset.
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        let bufs = &[io::IoSlice::new(buf)];
        self.write_vectored_at(bufs, offset)
    }

    /// Writes a number of bytes starting from a given offset.
    fn write_vectored_at(&self, bufs: &[io::IoSlice<'_>], offset: u64) -> io::Result<usize>;

    /// Attempts to write an entire buffer starting from a given offset.
    fn write_all_at(&self, mut buf: &[u8], mut offset: u64) -> io::Result<()> {
        while !buf.is_empty() {
            match self.write_at(buf, offset) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => {
                    buf = &buf[n..];
                    offset += n as u64
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Adjust the flags associated with this file.
    fn fdstat_set_flags(&self, flags: u16) -> io::Result<()>;

    /// Adjust the rights associated with this file.
    fn fdstat_set_rights(&self, rights: u64, inheriting: u64) -> io::Result<()>;

    /// Provide file advisory information on a file descriptor.
    fn advise(&self, offset: u64, len: u64, advice: u8) -> io::Result<()>;

    /// Force the allocation of space in a file.
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()>;

    /// Create a directory.
    fn create_directory<P: AsRef<std::path::Path>>(&self, dir: P) -> io::Result<()>;

    /// Unlink a file.
    fn remove_file<P: AsRef<std::path::Path>>(&self, path: P) -> io::Result<()>;

    /// Remove a directory.
    fn remove_directory<P: AsRef<std::path::Path>>(&self, path: P) -> io::Result<()>;
}

/// Windows-specific extensions to [`fs::File`].
#[cfg(windows)]
pub trait FileExt {
    /// Seeks to a given position and reads a number of bytes.
    fn seek_read(&self, buf: &mut [u8], offset: u64) -> io::Result<usize>;

    /// Seeks to a given position and writes a number of bytes.
    fn seek_write(&self, buf: &[u8], offset: u64) -> io::Result<usize>;
}

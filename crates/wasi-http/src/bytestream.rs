use crate::common::{Error, ErrorExt, InputStream, OutputStream};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::vec::Vec;

#[derive(Clone, Debug)]
pub struct ByteStream {
    inner: Bytes,
    is_readable: bool,
    is_writable: bool,
}

impl ByteStream {
    pub fn new() -> Self {
        Self {
            inner: Bytes::new(),
            is_readable: true,
            is_writable: true,
        }
    }

    pub fn new_readable() -> Self {
        Self {
            inner: Bytes::new(),
            is_readable: true,
            is_writable: false,
        }
    }

    pub fn new_writable() -> Self {
        Self {
            inner: Bytes::new(),
            is_readable: false,
            is_writable: true,
        }
    }
}

impl InputStream for ByteStream {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        let len = buf.len();
        let mut s = BytesMut::from(self.inner.as_ref());
        let size = s.len();
        if len == 0 || size == 0 {
            Ok((0, true))
        } else if size > len {
            s.advance(len);
            buf.copy_from_slice(&s);
            Ok((len.try_into()?, false))
        } else {
            buf[..size].copy_from_slice(&s);
            Ok((size.try_into()?, true))
        }
    }

    fn readable(&self) -> Result<(), Error> {
        if self.is_readable {
            Ok(())
        } else {
            Err(self::Error::badf().context("stream is not readable"))
        }
    }

    fn writable(&self) -> Result<(), Error> {
        if self.is_writable {
            Ok(())
        } else {
            Err(self::Error::badf().context("stream is not writable"))
        }
    }
}

impl OutputStream for ByteStream {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        let len = buf.len();
        let mut s = BytesMut::from(self.inner.as_ref());
        let size = s.len();
        if len == 0 || size == 0 {
            Ok((0, true))
        } else if size > len {
            s.advance(len);
            buf.copy_from_slice(&s);
            Ok((len.try_into()?, false))
        } else {
            buf[..size].copy_from_slice(&s);
            Ok((size.try_into()?, true))
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        let data = &self.inner;
        let buf_len = buf.len();
        let len = data.len() + buf_len;
        if len > 0 {
            let mut new = BytesMut::with_capacity(len);
            new.put(Bytes::from(data.clone()));
            new.put(buf);
            self.inner = new.freeze().into();
        }
        Ok(buf_len.try_into()?)
    }

    fn readable(&self) -> Result<(), Error> {
        if self.is_readable {
            Ok(())
        } else {
            Err(self::Error::badf().context("stream is not readable"))
        }
    }

    fn writable(&self) -> Result<(), Error> {
        if self.is_writable {
            Ok(())
        } else {
            Err(self::Error::badf().context("stream is not writable"))
        }
    }
}

impl From<Bytes> for ByteStream {
    fn from(buf: Bytes) -> ByteStream {
        ByteStream {
            inner: buf,
            is_readable: true,
            is_writable: true,
        }
    }
}

impl Into<Bytes> for ByteStream {
    fn into(self) -> Bytes {
        self.inner
    }
}

impl From<Vec<u8>> for ByteStream {
    fn from(vec: Vec<u8>) -> ByteStream {
        ByteStream {
            inner: Bytes::from(vec),
            is_readable: true,
            is_writable: true,
        }
    }
}

impl Deref for ByteStream {
    type Target = Bytes;
    fn deref(&self) -> &Bytes {
        &self.inner
    }
}

impl DerefMut for ByteStream {
    fn deref_mut(&mut self) -> &mut Bytes {
        &mut self.inner
    }
}

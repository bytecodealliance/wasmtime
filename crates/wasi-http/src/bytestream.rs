use crate::common::{Error, InputStream, OutputStream};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::vec::Vec;

#[derive(Clone, Debug)]
pub struct ByteStream(Bytes);

impl ByteStream {
    pub fn new() -> Self {
        Self(Bytes::new())
    }
}

impl InputStream for ByteStream {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        let len = buf.len();
        let mut s = BytesMut::from(self.0.as_ref());
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
        Ok(())
    }
}

impl OutputStream for ByteStream {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        let data = &self.0;
        let buf_len = buf.len();
        let len = data.len() + buf_len;
        if len > 0 {
            let mut new = BytesMut::with_capacity(len);
            new.put(Bytes::from(data.clone()));
            new.put(buf);
            self.0 = new.freeze().into();
        }
        Ok(buf_len.try_into()?)
    }

    fn writable(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl From<Bytes> for ByteStream {
    fn from(buf: Bytes) -> ByteStream {
        ByteStream(buf)
    }
}

impl From<Vec<u8>> for ByteStream {
    fn from(vec: Vec<u8>) -> ByteStream {
        ByteStream(Bytes::from(vec))
    }
}

impl Deref for ByteStream {
    type Target = Bytes;
    fn deref(&self) -> &Bytes {
        &self.0
    }
}

impl DerefMut for ByteStream {
    fn deref_mut(&mut self) -> &mut Bytes {
        &mut self.0
    }
}

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::vec::Vec;
use wasmtime_wasi::preview2::{
    pipe::{AsyncReadStream, AsyncWriteStream},
    HostInputStream, HostOutputStream, StreamState,
};

const MAX_BUF_SIZE: usize = 65_536;

pub struct ByteStream {
    // inner: BytesMut,
    reader: AsyncReadStream,
    writer: AsyncWriteStream,
}

// pub trait HostInputOutputStream: HostInputStream + HostOutputStream {
//     fn as_any(&self) -> &dyn Any;
//     fn as_boxed_any(self) -> Box<dyn Any + Send + Sync + 'static>
//     where
//         Self: Sized + 'static;
//     // fn as_input(self) -> Box<dyn HostInputStream>;
//     // fn as_output(self) -> Box<dyn HostOutputStream>;
// }

impl ByteStream {
    pub fn new() -> Self {
        let (read_stream, write_stream) = tokio::io::duplex(MAX_BUF_SIZE);
        // let (_read_half, write_half) = tokio::io::split(a);
        // let (read_half, _write_half) = tokio::io::split(b);
        Self {
            reader: AsyncReadStream::new(read_stream),
            writer: AsyncWriteStream::new(write_stream),
        }
        // Self {
        //     inner: BytesMut::new(),
        // }
    }

    pub(crate) fn reader(mut self) -> impl HostInputStream {
        self.reader
    }

    pub(crate) fn writer(mut self) -> impl HostOutputStream {
        self.writer
    }
}

// impl HostInputOutputStream for ByteStream {
//     fn as_any(&self) -> &dyn Any
// // where
//     //     Self: Sized + 'static,
//     {
//         self
//     }

//     fn as_boxed_any(self) -> Box<dyn Any + Send + Sync + 'static>
//     where
//         Self: Sized + 'static,
//     {
//         Box::new(self)
//     }

//     // fn as_input(self) -> Box<dyn HostInputStream + 'static> {
//     //     Box::new(
//     //         self.as_boxed_any()
//     //             .downcast::<dyn HostInputStream + 'static>()
//     //             .unwrap()
//     //             .to_owned(),
//     //     )
//     // }

//     // fn as_output(self) -> Box<dyn HostOutputStream> {
//     //     Box::new(
//     //         self.as_any()
//     //             .downcast_ref::<dyn HostOutputStream>()
//     //             .unwrap()
//     //             .to_owned(),
//     //     )
//     // }
// }

#[async_trait::async_trait]
impl HostInputStream for ByteStream {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), anyhow::Error> {
        println!("read input {}", size);
        self.reader.read(size)
        // let mut s = &mut self.inner; //BytesMut::from(self.inner.as_ref());
        // let mut bytes = BytesMut::with_capacity(size);
        // if size == 0 || s.len() == 0 {
        //     println!("read input 1");
        //     Ok((Bytes::new(), StreamState::Closed))
        // } else if s.len() > size {
        //     println!("read input 2");
        //     s.advance(size);
        //     bytes.copy_from_slice(&s);
        //     Ok((bytes.into(), StreamState::Closed))
        // } else {
        //     println!("read input 3");
        //     bytes[..s.len()].copy_from_slice(&s);
        //     Ok((bytes.into(), StreamState::Open))
        // }
    }

    fn skip(&mut self, nelem: usize) -> Result<(usize, StreamState), anyhow::Error> {
        self.reader.skip(nelem)
    }

    async fn ready(&mut self) -> Result<(), anyhow::Error> {
        self.reader.ready().await
        // Ok(())
    }
}

#[async_trait::async_trait]
impl HostOutputStream for ByteStream {
    fn write(&mut self, bytes: Bytes) -> Result<(usize, StreamState), anyhow::Error> {
        println!("write input {}", bytes.len());
        self.writer.write(bytes)

        // let data = &self.inner;
        // let size = bytes.len();
        // let len = data.len() + size;
        // if len > 0 {
        //     let mut new = BytesMut::with_capacity(len);
        //     new.put(Bytes::from(data.clone()));
        //     new.put(bytes);
        //     self.inner = new.freeze().into();
        // }
        // Ok((size, StreamState::Open))
        // let mut buf = &mut self.inner; //BytesMut::from(self.inner.as_ref());
        // buf.extend_from_slice(bytes.as_ref());
        // Ok((bytes.len(), StreamState::Open))
    }

    fn splice(
        &mut self,
        src: &mut dyn HostInputStream,
        nelem: usize,
    ) -> Result<(usize, StreamState), anyhow::Error> {
        self.writer.splice(src, nelem)
    }

    fn write_zeroes(&mut self, nelem: usize) -> Result<(usize, StreamState), anyhow::Error> {
        self.writer.write_zeroes(nelem)
    }

    async fn ready(&mut self) -> Result<(), anyhow::Error> {
        self.writer.ready().await
        // Ok(())
    }
}

// impl Into<Box<dyn HostInputStream>> for dyn HostInputOutputStream {
//     fn into(self) -> Box<dyn HostInputStream>
//     where
//         Self: Sized + 'static,
//     {
//         (Box::new(self) as Box<dyn Any>)
//             .downcast::<dyn HostInputStream>()
//             .unwrap()
//     }
// }

// impl From<Bytes> for ByteStream {
//     fn from(buf: Bytes) -> ByteStream {
//         ByteStream {
//             inner: buf,
//             is_readable: true,
//             is_writable: true,
//         }
//     }
// }

// impl Into<Bytes> for ByteStream {
//     fn into(self) -> Bytes {
//         let mut stream = BytesMut::new();
//         // let mut reader = &mut self.reader;
//         // let mut eof = StreamState::Open;
//         // while eof != StreamState::Closed {
//         //     let (mut body_chunk, stream_status) = reader.read(u64::MAX as usize).unwrap();
//         //     eof = stream_status;
//         //     stream.extend_from_slice(&body_chunk[..]);
//         // }
//         stream.freeze()
//         // self.inner
//     }
// }

impl From<Vec<u8>> for ByteStream {
    fn from(vec: Vec<u8>) -> ByteStream {
        let mut stream: ByteStream = ByteStream::new();
        stream.writer.write(Bytes::from(vec)).unwrap();
        stream
        // ByteStream {
        //     inner: BytesMut::from(Bytes::from(vec).as_ref()),
        // }
    }
}

// impl Deref for ByteStream {
//     type Target = Bytes;
//     fn deref(&self) -> &Bytes {
//         &self.inner
//     }
// }

// impl DerefMut for ByteStream {
//     fn deref_mut(&mut self) -> &mut Bytes {
//         &mut self.inner
//     }
// }

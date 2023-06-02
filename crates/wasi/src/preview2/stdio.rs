use anyhow::Error;
use std::convert::TryInto;
use std::io::{self, Read, Write};

use crate::preview2::{HostInputStream, HostOutputStream};

pub struct Stdin(std::io::Stdin);

pub fn stdin() -> Stdin {
    Stdin(std::io::stdin())
}

#[async_trait::async_trait]
impl HostInputStream for Stdin {
    async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        match Read::read(&mut self.0, buf) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    async fn read_vectored<'a>(
        &mut self,
        bufs: &mut [io::IoSliceMut<'a>],
    ) -> Result<(u64, bool), Error> {
        match Read::read_vectored(&mut self.0, bufs) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    #[cfg(can_vector)]
    fn is_read_vectored(&self) {
        Read::is_read_vectored(&mut self.0)
    }

    async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
        let num = io::copy(&mut io::Read::take(&mut self.0, nelem), &mut io::sink())?;
        Ok((num, num < nelem))
    }

    async fn readable(&self) -> Result<(), Error> {
        Err(anyhow::anyhow!("idk"))
    }
}

macro_rules! wasi_output_stream_impl {
    ($ty:ty, $ident:ident) => {
        #[async_trait::async_trait]
        impl HostOutputStream for $ty {
            async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
                let n = Write::write(&mut self.0, buf)?;
                Ok(n.try_into()?)
            }
            async fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                let n = Write::write_vectored(&mut self.0, bufs)?;
                Ok(n.try_into()?)
            }
            #[cfg(can_vector)]
            fn is_write_vectored(&self) {
                Write::is_write_vectored(&mut self.0)
            }
            // TODO: Optimize for stdio streams.
            /*
            async fn splice(
                &mut self,
                src: &mut dyn InputStream,
                nelem: u64,
            ) -> Result<u64, Error> {
                todo!()
            }
            */

            async fn write_zeroes(&mut self, nelem: u64) -> Result<u64, Error> {
                let num = io::copy(&mut io::Read::take(io::repeat(0), nelem), &mut self.0)?;
                Ok(num)
            }

            async fn writable(&self) -> Result<(), Error> {
                Ok(())
            }
        }
    };
}

pub struct Stdout(std::io::Stdout);

pub fn stdout() -> Stdout {
    Stdout(std::io::stdout())
}
wasi_output_stream_impl!(Stdout, Stdout);

pub struct Stderr(std::io::Stderr);

pub fn stderr() -> Stderr {
    Stderr(std::io::stderr())
}
wasi_output_stream_impl!(Stderr, Stderr);

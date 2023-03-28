use crate::poll::Pollable;
use crate::streams::{InputStream, OutputStream, StreamError};
use crate::WasiHttp;
use anyhow::{anyhow, bail};
use std::vec::Vec;
use bytes::BufMut;

impl crate::streams::Host for WasiHttp {
    fn read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> wasmtime::Result<Result<(Vec<u8>, bool), StreamError>> {
        let s = self
            .streams
            .get_mut(&stream)
            .ok_or_else(|| anyhow!("stream not found: {stream}"))?;
        if len == 0 {
            Ok(Ok((bytes::Bytes::new().to_vec(), s.len() > 0)))
        } else if s.len() > len.try_into()? {
            let result = s.split_to(len.try_into()?);
            Ok(Ok((result.to_vec(), false)))
        } else {
            s.truncate(s.len());
            Ok(Ok((s.clone().to_vec(), true)))
        }
    }

    fn skip(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> wasmtime::Result<Result<(u64, bool), StreamError>> {
        let s = self
            .streams
            .get_mut(&stream)
            .ok_or_else(|| anyhow!("stream not found: {stream}"))?;
        if len == 0 {
            Ok(Ok((0, s.len() > 0)))
        } else if s.len() > len.try_into()? {
            s.truncate(len.try_into()?);
            Ok(Ok((len, false)))
        } else {
            let bytes = s.len();
            s.truncate(s.len());
            Ok(Ok((bytes.try_into()?, true)))
        }
    }

    fn subscribe_to_input_stream(&mut self, _this: InputStream) -> wasmtime::Result<Pollable> {
        bail!("unimplemented: subscribe_to_input_stream");
    }

    fn drop_input_stream(&mut self, stream: InputStream) -> wasmtime::Result<()> {
        self.streams.remove(&stream);
        Ok(())
    }

    fn write(
        &mut self,
        this: OutputStream,
        buf: Vec<u8>,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        match self.streams.get(&this) {
            Some(data) => {
                let mut new = bytes::BytesMut::with_capacity(data.len() + buf.len());
                new.put(data.clone());
                new.put(bytes::Bytes::from(buf.clone()));
                self.streams.insert(this, new.freeze());
            },
            None => {
                self.streams.insert(this, bytes::Bytes::from(buf.clone()));
            },
        }
        Ok(Ok(buf.len().try_into()?))
    }

    fn write_zeroes(
        &mut self,
        this: OutputStream,
        len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let mut data = Vec::with_capacity(len.try_into()?);
        let mut i = 0;
        while i < len {
            data.push(0);
            i = i + 1;
        }
        self.write(this, data)
    }

    fn splice(
        &mut self,
        _this: OutputStream,
        _src: InputStream,
        _len: u64,
    ) -> wasmtime::Result<Result<(u64, bool), StreamError>> {
        bail!("unimplemented: splice");
    }

    fn forward(
        &mut self,
        _this: OutputStream,
        _src: InputStream,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        bail!("unimplemented: forward");
    }

    fn subscribe_to_output_stream(&mut self, _this: OutputStream) -> wasmtime::Result<Pollable> {
        bail!("unimplemented: subscribe_to_output_stream");
    }

    fn drop_output_stream(&mut self, stream: OutputStream) -> wasmtime::Result<()> {
        self.streams.remove(&stream);
        Ok(())
    }
}

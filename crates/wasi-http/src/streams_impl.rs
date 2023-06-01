use crate::wasi::io::streams::{Host, InputStream, OutputStream, Pollable, StreamError};
use crate::WasiHttp;
use anyhow::{anyhow, bail};
use std::vec::Vec;

impl Host for WasiHttp {
    fn read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> wasmtime::Result<Result<(Vec<u8>, bool), StreamError>> {
        let st = self
            .streams
            .get_mut(&stream)
            .ok_or_else(|| anyhow!("stream not found: {stream}"))?;
        if st.closed {
            bail!("stream is dropped!");
        }
        let s = &mut st.data;
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
        let st = self
            .streams
            .get_mut(&stream)
            .ok_or_else(|| anyhow!("stream not found: {stream}"))?;
        if st.closed {
            bail!("stream is dropped!");
        }
        let s = &mut st.data;
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
        let st = self
            .streams
            .get_mut(&stream)
            .ok_or_else(|| anyhow!("stream not found: {stream}"))?;
        st.closed = true;
        Ok(())
    }

    fn write(
        &mut self,
        this: OutputStream,
        buf: Vec<u8>,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let len = buf.len();
        let st = self.streams.entry(this).or_default();
        if st.closed {
            bail!("cannot write to closed stream");
        }
        st.data.extend_from_slice(buf.as_slice());
        Ok(Ok(len.try_into()?))
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
        let st = self
            .streams
            .get_mut(&stream)
            .ok_or_else(|| anyhow!("stream not found: {stream}"))?;
        st.closed = true;
        Ok(())
    }

    fn blocking_read(
        &mut self,
        _: InputStream,
        _: u64,
    ) -> wasmtime::Result<Result<(Vec<u8>, bool), StreamError>> {
        bail!("unimplemented")
    }

    fn blocking_skip(
        &mut self,
        _: InputStream,
        _: u64,
    ) -> wasmtime::Result<Result<(u64, bool), StreamError>> {
        bail!("unimplemented")
    }

    fn blocking_write(
        &mut self,
        _: OutputStream,
        _: Vec<u8>,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        bail!("unimplemented")
    }

    fn blocking_write_zeroes(
        &mut self,
        _: OutputStream,
        _: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        bail!("unimplemented")
    }

    fn blocking_splice(
        &mut self,
        _: OutputStream,
        _: InputStream,
        _: u64,
    ) -> wasmtime::Result<Result<(u64, bool), StreamError>> {
        bail!("unimplemented")
    }
}

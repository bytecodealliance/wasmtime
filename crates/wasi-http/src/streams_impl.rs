use crate::common::stream::TableStreamExt;
use crate::wasi::io::streams::{InputStream, OutputStream, StreamError};
use crate::wasi::poll::poll::Pollable;
use crate::WasiHttpCtx;
use anyhow::bail;
use std::vec::Vec;

fn convert(error: crate::common::Error) -> anyhow::Error {
    // if let Some(errno) = error.downcast_ref() {
    //     anyhow::Error::new(StreamError {})
    // } else {
    error.into()
    // }
}

impl crate::wasi::io::streams::Host for WasiHttpCtx {
    fn read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> wasmtime::Result<Result<(Vec<u8>, bool), StreamError>> {
        let s = self
            .table_mut()
            .get_input_stream_mut(stream)
            .map_err(convert)?;
        // if s.closed {
        //     bail!("stream is dropped!");
        // }

        // Len could be any `u64` value, but we don't want to
        // allocate too much up front, so make a wild guess
        // of an upper bound for the buffer size.
        let buffer_len = std::cmp::min(len, 0x400000) as _;
        let mut buffer = vec![0; buffer_len];

        let (bytes_read, end) = s.read(&mut buffer).map_err(convert)?;

        buffer.truncate(bytes_read as usize);

        Ok(Ok((buffer, end)))
    }

    fn skip(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> wasmtime::Result<Result<(u64, bool), StreamError>> {
        let s = self
            .table_mut()
            .get_input_stream_mut(stream)
            .map_err(convert)?;
        // if s.closed {
        //     bail!("stream is dropped!");
        // }

        let (bytes_skipped, end) = s.skip(len).map_err(convert)?;

        Ok(Ok((bytes_skipped, end)))
    }

    fn subscribe_to_input_stream(&mut self, _this: InputStream) -> wasmtime::Result<Pollable> {
        bail!("unimplemented: subscribe_to_input_stream");
    }

    fn drop_input_stream(&mut self, stream: InputStream) -> wasmtime::Result<()> {
        // let st = self
        //     .streams
        //     .get_mut(&stream)
        //     .ok_or_else(|| anyhow!("stream not found: {stream}"))?;
        // st.closed = true;
        self.table_mut()
            .delete_input_stream(stream)
            .map_err(convert)?;
        Ok(())
    }

    fn write(
        &mut self,
        stream: OutputStream,
        buf: Vec<u8>,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let s = self
            .table_mut()
            .get_output_stream_mut(stream)
            .map_err(convert)?;
        // if s.closed {
        //     bail!("cannot write to closed stream");
        // }

        let bytes_written = s.write(&buf).map_err(convert)?;

        Ok(Ok(u64::try_from(bytes_written)?))
    }

    fn write_zeroes(
        &mut self,
        stream: OutputStream,
        len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let s = self
            .table_mut()
            .get_output_stream_mut(stream)
            .map_err(convert)?;

        let bytes_written: u64 = s.write_zeroes(len).map_err(convert)?;

        Ok(Ok(bytes_written))
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
        // let st = self
        //     .streams
        //     .get_mut(&stream)
        //     .ok_or_else(|| anyhow!("stream not found: {stream}"))?;
        // st.closed = true;
        self.table_mut()
            .delete_output_stream(stream)
            .map_err(convert)?;
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

use crate::poll::Pollable;
use crate::streams::{InputStream, OutputStream, StreamError};
use crate::WasiHttp;
use anyhow::bail;
use std::vec::Vec;

impl crate::streams::Host for WasiHttp {
    fn read(
        &mut self,
        stream: InputStream,
        _len: u64,
    ) -> wasmtime::Result<Result<(Vec<u8>, bool), StreamError>> {
        let response = match self.responses.get(&stream) {
            Some(r) => r,
            None => bail!("not found"),
        };
        let bytes = response.body.clone();
        Ok(Ok((bytes.to_vec(), true)))
    }

    fn skip(
        &mut self,
        _this: InputStream,
        _len: u64,
    ) -> wasmtime::Result<Result<(u64, bool), StreamError>> {
        todo!();
    }

    fn subscribe_to_input_stream(&mut self, _this: InputStream) -> wasmtime::Result<Pollable> {
        todo!();
    }

    fn drop_input_stream(&mut self, stream: InputStream) -> wasmtime::Result<()> {
        match self.streams.get_mut(&stream) {
            Some(r) => r.truncate(0),
            None => {}
        }
        Ok(())
    }

    fn write(
        &mut self,
        this: OutputStream,
        buf: Vec<u8>,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        // TODO: Make this a real write not a replace.
        self.streams.insert(this, bytes::Bytes::from(buf.clone()));
        Ok(Ok(buf.len().try_into().unwrap()))
    }

    fn write_zeroes(
        &mut self,
        _this: OutputStream,
        _len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!();
    }

    fn splice(
        &mut self,
        _this: OutputStream,
        _src: InputStream,
        _len: u64,
    ) -> wasmtime::Result<Result<(u64, bool), StreamError>> {
        todo!();
    }

    fn forward(
        &mut self,
        _this: OutputStream,
        _src: InputStream,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!();
    }

    fn subscribe_to_output_stream(&mut self, _this: OutputStream) -> wasmtime::Result<Pollable> {
        todo!();
    }

    fn drop_output_stream(&mut self, _this: OutputStream) -> wasmtime::Result<()> {
        todo!();
    }
}

use crate::{
    wasi_io::{InputStream, OutputStream, StreamError, WasiIo},
    HostResult, WasiCtx,
};
use wasi_common::stream::TableStreamExt;

fn convert(error: wasi_common::Error) -> anyhow::Error {
    if let Some(_errno) = error.downcast_ref() {
        anyhow::Error::new(StreamError {})
    } else {
        error.into()
    }
}

#[async_trait::async_trait]
impl WasiIo for WasiCtx {
    async fn drop_input_stream(&mut self, stream: InputStream) -> anyhow::Result<()> {
        self.table_mut()
            .delete::<Box<dyn wasi_common::InputStream>>(stream)
            .map_err(convert)?;
        Ok(())
    }

    async fn drop_output_stream(&mut self, stream: OutputStream) -> anyhow::Result<()> {
        self.table_mut()
            .delete::<Box<dyn wasi_common::OutputStream>>(stream)
            .map_err(convert)?;
        Ok(())
    }

    async fn read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> HostResult<(Vec<u8>, bool), StreamError> {
        let s: &mut Box<dyn wasi_common::InputStream> = self
            .table_mut()
            .get_input_stream_mut(stream)
            .map_err(convert)?;

        let mut buffer = vec![0; len.try_into().unwrap()];

        let (bytes_read, end) = s.read(&mut buffer).await.map_err(convert)?;

        buffer.truncate(bytes_read as usize);

        Ok(Ok((buffer, end)))
    }

    async fn write(
        &mut self,
        stream: OutputStream,
        bytes: Vec<u8>,
    ) -> HostResult<u64, StreamError> {
        let s: &mut Box<dyn wasi_common::OutputStream> = self
            .table_mut()
            .get_output_stream_mut(stream)
            .map_err(convert)?;

        let bytes_written: u64 = s.write(&bytes).await.map_err(convert)?;

        Ok(Ok(u64::try_from(bytes_written).unwrap()))
    }

    async fn skip(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> HostResult<(u64, bool), StreamError> {
        let s: &mut Box<dyn wasi_common::InputStream> = self
            .table_mut()
            .get_input_stream_mut(stream)
            .map_err(convert)?;

        let (bytes_skipped, end) = s.skip(len).await.map_err(convert)?;

        Ok(Ok((bytes_skipped, end)))
    }

    async fn write_repeated(
        &mut self,
        stream: OutputStream,
        byte: u8,
        len: u64,
    ) -> HostResult<u64, StreamError> {
        let s: &mut Box<dyn wasi_common::OutputStream> = self
            .table_mut()
            .get_output_stream_mut(stream)
            .map_err(convert)?;

        let bytes_written: u64 = s.write_repeated(byte, len).await.map_err(convert)?;

        Ok(Ok(bytes_written))
    }

    async fn splice(
        &mut self,
        _src: InputStream,
        _dst: OutputStream,
        _len: u64,
    ) -> HostResult<(u64, bool), StreamError> {
        // TODO: We can't get two streams at the same time because they both
        // carry the exclusive lifetime of `self`. When [`get_many_mut`] is
        // stabilized, that could allow us to add a `get_many_stream_mut` or
        // so which lets us do this.
        //
        // [`get_many_mut`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.get_many_mut
        /*
        let s: &mut Box<dyn wasi_common::InputStream> = self
            .table_mut()
            .get_input_stream_mut(src)
            .map_err(convert)?;
        let d: &mut Box<dyn wasi_common::OutputStream> = self
            .table_mut()
            .get_output_stream_mut(dst)
            .map_err(convert)?;

        let bytes_spliced: u64 = s.splice(&mut **d, len).await.map_err(convert)?;

        Ok(bytes_spliced)
        */

        todo!()
    }

    async fn forward(
        &mut self,
        _src: InputStream,
        _dst: OutputStream,
    ) -> HostResult<u64, StreamError> {
        // TODO: We can't get two streams at the same time because they both
        // carry the exclusive lifetime of `self`. When [`get_many_mut`] is
        // stabilized, that could allow us to add a `get_many_stream_mut` or
        // so which lets us do this.
        //
        // [`get_many_mut`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.get_many_mut
        /*
        let s: &mut Box<dyn wasi_common::InputStream> = self
            .table_mut()
            .get_input_stream_mut(src)
            .map_err(convert)?;
        let d: &mut Box<dyn wasi_common::OutputStream> = self
            .table_mut()
            .get_output_stream_mut(dst)
            .map_err(convert)?;

        let bytes_spliced: u64 = s.splice(&mut **d, len).await.map_err(convert)?;

        Ok(bytes_spliced)
        */

        todo!()
    }
}

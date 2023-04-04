use crate::{
    command,
    command::wasi::poll::Pollable,
    command::wasi::streams::{self, InputStream, OutputStream, StreamError},
    poll::PollableEntry,
    proxy, WasiCtx,
};
use anyhow::anyhow;
use wasi_common::stream::TableStreamExt;

impl From<wasi_common::Error> for streams::Error {
    fn from(error: wasi_common::Error) -> streams::Error {
        if let Some(_) = error.downcast_ref() {
            StreamError {}.into()
        } else {
            streams::Error::trap(anyhow!(error))
        }
    }
}

impl From<streams::Error> for proxy::wasi::streams::Error {
    fn from(error: streams::Error) -> proxy::wasi::streams::Error {
        if let Some(_) = error.downcast_ref() {
            proxy::wasi::streams::StreamError {}.into()
        } else {
            proxy::wasi::streams::Error::trap(anyhow!(error))
        }
    }
}

async fn drop_input_stream(ctx: &mut WasiCtx, stream: InputStream) -> anyhow::Result<()> {
    ctx.table_mut()
        .delete::<Box<dyn wasi_common::InputStream>>(stream)?;
    Ok(())
}

async fn drop_output_stream(ctx: &mut WasiCtx, stream: OutputStream) -> anyhow::Result<()> {
    ctx.table_mut()
        .delete::<Box<dyn wasi_common::OutputStream>>(stream)?;
    Ok(())
}

async fn read(
    ctx: &mut WasiCtx,
    stream: InputStream,
    len: u64,
) -> Result<(Vec<u8>, bool), streams::Error> {
    let s: &mut Box<dyn wasi_common::InputStream> = ctx.table_mut().get_input_stream_mut(stream)?;

    // Len could be any `u64` value, but we don't want to
    // allocate too much up front, so make a wild guess
    // of an upper bound for the buffer size.
    let buffer_len = std::cmp::min(len, 0x400000) as _;
    let mut buffer = vec![0; buffer_len];

    let (bytes_read, end) = s.read(&mut buffer).await?;

    buffer.truncate(bytes_read as usize);

    Ok((buffer, end))
}

async fn blocking_read(
    ctx: &mut WasiCtx,
    stream: InputStream,
    len: u64,
) -> Result<(Vec<u8>, bool), streams::Error> {
    // TODO: When this is really async make this block.
    read(ctx, stream, len).await
}

async fn write(
    ctx: &mut WasiCtx,
    stream: OutputStream,
    bytes: Vec<u8>,
) -> Result<u64, streams::Error> {
    let s: &mut Box<dyn wasi_common::OutputStream> =
        ctx.table_mut().get_output_stream_mut(stream)?;

    let bytes_written: u64 = s.write(&bytes).await?;

    Ok(u64::try_from(bytes_written).unwrap())
}

async fn blocking_write(
    ctx: &mut WasiCtx,
    stream: OutputStream,
    bytes: Vec<u8>,
) -> Result<u64, streams::Error> {
    // TODO: When this is really async make this block.
    write(ctx, stream, bytes).await
}

async fn skip(
    ctx: &mut WasiCtx,
    stream: InputStream,
    len: u64,
) -> Result<(u64, bool), streams::Error> {
    let s: &mut Box<dyn wasi_common::InputStream> = ctx.table_mut().get_input_stream_mut(stream)?;

    let (bytes_skipped, end) = s.skip(len).await?;

    Ok((bytes_skipped, end))
}

async fn blocking_skip(
    ctx: &mut WasiCtx,
    stream: InputStream,
    len: u64,
) -> Result<(u64, bool), streams::Error> {
    // TODO: When this is really async make this block.
    skip(ctx, stream, len).await
}

async fn write_zeroes(
    ctx: &mut WasiCtx,
    stream: OutputStream,
    len: u64,
) -> Result<u64, streams::Error> {
    let s: &mut Box<dyn wasi_common::OutputStream> =
        ctx.table_mut().get_output_stream_mut(stream)?;

    let bytes_written: u64 = s.write_zeroes(len).await?;

    Ok(bytes_written)
}

async fn blocking_write_zeroes(
    ctx: &mut WasiCtx,
    stream: OutputStream,
    len: u64,
) -> Result<u64, streams::Error> {
    // TODO: When this is really async make this block.
    write_zeroes(ctx, stream, len).await
}

async fn splice(
    _ctx: &mut WasiCtx,
    _src: InputStream,
    _dst: OutputStream,
    _len: u64,
) -> Result<(u64, bool), streams::Error> {
    // TODO: We can't get two streams at the same time because they both
    // carry the exclusive lifetime of `ctx`. When [`get_many_mut`] is
    // stabilized, that could allow us to add a `get_many_stream_mut` or
    // so which lets us do this.
    //
    // [`get_many_mut`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.get_many_mut
    /*
    let s: &mut Box<dyn wasi_common::InputStream> = ctx
        .table_mut()
        .get_input_stream_mut(src)
        ?;
    let d: &mut Box<dyn wasi_common::OutputStream> = ctx
        .table_mut()
        .get_output_stream_mut(dst)
        ?;

    let bytes_spliced: u64 = s.splice(&mut **d, len).await?;

    Ok(bytes_spliced)
    */

    todo!()
}

async fn blocking_splice(
    ctx: &mut WasiCtx,
    src: InputStream,
    dst: OutputStream,
    len: u64,
) -> Result<(u64, bool), streams::Error> {
    // TODO: When this is really async make this block.
    splice(ctx, src, dst, len).await
}

async fn forward(
    _ctx: &mut WasiCtx,
    _src: InputStream,
    _dst: OutputStream,
) -> Result<u64, streams::Error> {
    // TODO: We can't get two streams at the same time because they both
    // carry the exclusive lifetime of `ctx`. When [`get_many_mut`] is
    // stabilized, that could allow us to add a `get_many_stream_mut` or
    // so which lets us do this.
    //
    // [`get_many_mut`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.get_many_mut
    /*
    let s: &mut Box<dyn wasi_common::InputStream> = ctx
        .table_mut()
        .get_input_stream_mut(src)
        ?;
    let d: &mut Box<dyn wasi_common::OutputStream> = ctx
        .table_mut()
        .get_output_stream_mut(dst)
        ?;

    let bytes_spliced: u64 = s.splice(&mut **d, len).await?;

    Ok(bytes_spliced)
    */

    todo!()
}

async fn subscribe_to_input_stream(
    ctx: &mut WasiCtx,
    stream: InputStream,
) -> anyhow::Result<Pollable> {
    Ok(ctx
        .table_mut()
        .push(Box::new(PollableEntry::Read(stream)))?)
}

async fn subscribe_to_output_stream(
    ctx: &mut WasiCtx,
    stream: OutputStream,
) -> anyhow::Result<Pollable> {
    Ok(ctx
        .table_mut()
        .push(Box::new(PollableEntry::Write(stream)))?)
}

// Implementatations of the traits for both the command and proxy worlds.
// The bodies have been pulled out into functions above to allow them to
// be shared between the two. Ideally, we should add features to the
// bindings to facilitate this kind of sharing.

#[async_trait::async_trait]
impl command::wasi::streams::Host for WasiCtx {
    async fn drop_input_stream(&mut self, stream: InputStream) -> anyhow::Result<()> {
        drop_input_stream(self, stream).await
    }

    async fn drop_output_stream(&mut self, stream: OutputStream) -> anyhow::Result<()> {
        drop_output_stream(self, stream).await
    }

    async fn read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(Vec<u8>, bool), streams::Error> {
        read(self, stream, len).await
    }

    async fn blocking_read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(Vec<u8>, bool), streams::Error> {
        blocking_read(self, stream, len).await
    }

    async fn write(&mut self, stream: OutputStream, bytes: Vec<u8>) -> Result<u64, streams::Error> {
        write(self, stream, bytes).await
    }

    async fn blocking_write(
        &mut self,
        stream: OutputStream,
        bytes: Vec<u8>,
    ) -> Result<u64, streams::Error> {
        blocking_write(self, stream, bytes).await
    }

    async fn skip(&mut self, stream: InputStream, len: u64) -> Result<(u64, bool), streams::Error> {
        skip(self, stream, len).await
    }

    async fn blocking_skip(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(u64, bool), streams::Error> {
        blocking_skip(self, stream, len).await
    }

    async fn write_zeroes(
        &mut self,
        stream: OutputStream,
        len: u64,
    ) -> Result<u64, streams::Error> {
        write_zeroes(self, stream, len).await
    }

    async fn blocking_write_zeroes(
        &mut self,
        stream: OutputStream,
        len: u64,
    ) -> Result<u64, streams::Error> {
        blocking_write_zeroes(self, stream, len).await
    }

    async fn splice(
        &mut self,
        src: InputStream,
        dst: OutputStream,
        len: u64,
    ) -> Result<(u64, bool), streams::Error> {
        splice(self, src, dst, len).await
    }

    async fn blocking_splice(
        &mut self,
        src: InputStream,
        dst: OutputStream,
        len: u64,
    ) -> Result<(u64, bool), streams::Error> {
        blocking_splice(self, src, dst, len).await
    }

    async fn forward(
        &mut self,
        src: InputStream,
        dst: OutputStream,
    ) -> Result<u64, streams::Error> {
        forward(self, src, dst).await
    }

    async fn subscribe_to_input_stream(&mut self, stream: InputStream) -> anyhow::Result<Pollable> {
        subscribe_to_input_stream(self, stream).await
    }

    async fn subscribe_to_output_stream(
        &mut self,
        stream: OutputStream,
    ) -> anyhow::Result<Pollable> {
        subscribe_to_output_stream(self, stream).await
    }
}

#[async_trait::async_trait]
impl proxy::wasi::streams::Host for WasiCtx {
    async fn drop_input_stream(&mut self, stream: InputStream) -> anyhow::Result<()> {
        drop_input_stream(self, stream).await
    }

    async fn drop_output_stream(&mut self, stream: OutputStream) -> anyhow::Result<()> {
        drop_output_stream(self, stream).await
    }

    async fn read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(Vec<u8>, bool), proxy::wasi::streams::Error> {
        read(self, stream, len).await.map_err(|e| e.into())
    }

    async fn blocking_read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(Vec<u8>, bool), proxy::wasi::streams::Error> {
        blocking_read(self, stream, len).await.map_err(|e| e.into())
    }

    async fn write(
        &mut self,
        stream: OutputStream,
        bytes: Vec<u8>,
    ) -> Result<u64, proxy::wasi::streams::Error> {
        write(self, stream, bytes).await.map_err(|e| e.into())
    }

    async fn blocking_write(
        &mut self,
        stream: OutputStream,
        bytes: Vec<u8>,
    ) -> Result<u64, proxy::wasi::streams::Error> {
        blocking_write(self, stream, bytes)
            .await
            .map_err(|e| e.into())
    }

    async fn skip(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(u64, bool), proxy::wasi::streams::Error> {
        skip(self, stream, len).await.map_err(|e| e.into())
    }

    async fn blocking_skip(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(u64, bool), proxy::wasi::streams::Error> {
        blocking_skip(self, stream, len).await.map_err(|e| e.into())
    }

    async fn write_zeroes(
        &mut self,
        stream: OutputStream,
        len: u64,
    ) -> Result<u64, proxy::wasi::streams::Error> {
        write_zeroes(self, stream, len).await.map_err(|e| e.into())
    }

    async fn blocking_write_zeroes(
        &mut self,
        stream: OutputStream,
        len: u64,
    ) -> Result<u64, proxy::wasi::streams::Error> {
        blocking_write_zeroes(self, stream, len)
            .await
            .map_err(|e| e.into())
    }

    async fn splice(
        &mut self,
        src: InputStream,
        dst: OutputStream,
        len: u64,
    ) -> Result<(u64, bool), proxy::wasi::streams::Error> {
        splice(self, src, dst, len).await.map_err(|e| e.into())
    }

    async fn blocking_splice(
        &mut self,
        src: InputStream,
        dst: OutputStream,
        len: u64,
    ) -> Result<(u64, bool), proxy::wasi::streams::Error> {
        blocking_splice(self, src, dst, len)
            .await
            .map_err(|e| e.into())
    }

    async fn forward(
        &mut self,
        src: InputStream,
        dst: OutputStream,
    ) -> Result<u64, proxy::wasi::streams::Error> {
        forward(self, src, dst).await.map_err(|e| e.into())
    }

    async fn subscribe_to_input_stream(&mut self, stream: InputStream) -> anyhow::Result<Pollable> {
        subscribe_to_input_stream(self, stream).await
    }

    async fn subscribe_to_output_stream(
        &mut self,
        stream: OutputStream,
    ) -> anyhow::Result<Pollable> {
        subscribe_to_output_stream(self, stream).await
    }
}

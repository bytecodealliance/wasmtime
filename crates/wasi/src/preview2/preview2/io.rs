use crate::preview2::{
    bindings::io::streams::{self, InputStream, OutputStream, StreamError},
    bindings::poll::poll::Pollable,
    poll::PollableFuture,
    stream::{HostInputStream, HostOutputStream, TableStreamExt},
    HostPollable, TableError, TablePollableExt, WasiView,
};
use anyhow::anyhow;
use std::any::Any;

impl From<anyhow::Error> for streams::Error {
    fn from(error: anyhow::Error) -> streams::Error {
        tracing::trace!(
            "turning anyhow::Error in the streams interface into the empty error result: {error:?}"
        );
        StreamError {}.into()
    }
}

impl From<TableError> for streams::Error {
    fn from(error: TableError) -> streams::Error {
        match error {
            TableError::Full => streams::Error::trap(anyhow!(error)),
            TableError::NotPresent | TableError::WrongType => {
                // wit definition needs to define a badf-equiv variant:
                StreamError {}.into()
            }
        }
    }
}

#[async_trait::async_trait]
impl<T: WasiView> streams::Host for T {
    async fn drop_input_stream(&mut self, stream: InputStream) -> anyhow::Result<()> {
        self.table_mut()
            .delete::<Box<dyn HostInputStream>>(stream)?;
        Ok(())
    }

    async fn drop_output_stream(&mut self, stream: OutputStream) -> anyhow::Result<()> {
        self.table_mut()
            .delete::<Box<dyn HostOutputStream>>(stream)?;
        Ok(())
    }

    async fn read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(Vec<u8>, bool), streams::Error> {
        // let s = self.table_mut().get_input_stream_mut(stream)?;
        //
        // // Len could be any `u64` value, but we don't want to
        // // allocate too much up front, so make a wild guess
        // // of an upper bound for the buffer size.
        // let buffer_len = std::cmp::min(len, 0x400000) as _;
        // let mut buffer = vec![0; buffer_len];
        //
        // let (bytes_read, state) = HostInputStream::read(s.as_mut(), &mut buffer)?;
        //
        // buffer.truncate(bytes_read as usize);
        //
        // Ok((buffer, state.is_closed()))
        todo!()
    }

    async fn blocking_read(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(Vec<u8>, bool), streams::Error> {
        self.table_mut()
            .get_input_stream_mut(stream)?
            .ready()
            .await?;
        self.read(stream, len).await
    }

    async fn write(&mut self, stream: OutputStream, bytes: Vec<u8>) -> Result<u64, streams::Error> {
        // let s = self.table_mut().get_output_stream_mut(stream)?;
        //
        // let bytes_written: u64 = HostOutputStream::write(s.as_mut(), &bytes)?;
        //
        // Ok(u64::try_from(bytes_written).unwrap())
        todo!()
    }

    async fn blocking_write(
        &mut self,
        stream: OutputStream,
        bytes: Vec<u8>,
    ) -> Result<u64, streams::Error> {
        let written = self.write(stream, bytes).await?;
        self.table_mut()
            .get_output_stream_mut(stream)?
            .ready()
            .await?;
        Ok(written)
    }

    async fn skip(&mut self, stream: InputStream, len: u64) -> Result<(u64, bool), streams::Error> {
        let s = self.table_mut().get_input_stream_mut(stream)?;

        let (bytes_skipped, end) = s.skip(len)?;

        Ok((bytes_skipped, end.is_closed()))
    }

    async fn blocking_skip(
        &mut self,
        stream: InputStream,
        len: u64,
    ) -> Result<(u64, bool), streams::Error> {
        let r = self.skip(stream, len).await?;
        self.table_mut()
            .get_input_stream_mut(stream)?
            .ready()
            .await?;
        Ok(r)
    }

    async fn write_zeroes(
        &mut self,
        stream: OutputStream,
        len: u64,
    ) -> Result<u64, streams::Error> {
        let s = self.table_mut().get_output_stream_mut(stream)?;

        let bytes_written: u64 = s.write_zeroes(len)?;

        Ok(bytes_written)
    }

    async fn blocking_write_zeroes(
        &mut self,
        stream: OutputStream,
        len: u64,
    ) -> Result<u64, streams::Error> {
        let r = self.write_zeroes(stream, len).await?;
        self.table_mut()
            .get_output_stream_mut(stream)?
            .ready()
            .await?;
        Ok(r)
    }

    async fn splice(
        &mut self,
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
        let s: &mut Box<dyn crate::InputStream> = ctx
            .table_mut()
            .get_input_stream_mut(src)
            ?;
        let d: &mut Box<dyn crate::OutputStream> = ctx
            .table_mut()
            .get_output_stream_mut(dst)
            ?;

        let bytes_spliced: u64 = s.splice(&mut **d, len).await?;

        Ok(bytes_spliced)
        */
        todo!("stream splice is not implemented")
    }

    async fn blocking_splice(
        &mut self,
        _src: InputStream,
        _dst: OutputStream,
        _len: u64,
    ) -> Result<(u64, bool), streams::Error> {
        // TODO: once splice is implemented, figure out what the blocking semantics are for waiting
        // on src and dest here. probably just delete these blocking_ funcs altogether and defer
        // that decision to userland, though.
        todo!("stream splice is not implemented")
    }

    async fn forward(
        &mut self,
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
        let s: &mut Box<dyn crate::InputStream> = ctx
            .table_mut()
            .get_input_stream_mut(src)
            ?;
        let d: &mut Box<dyn crate::OutputStream> = ctx
            .table_mut()
            .get_output_stream_mut(dst)
            ?;

        let bytes_spliced: u64 = s.splice(&mut **d, len).await?;

        Ok(bytes_spliced)
        */

        todo!("stream forward is not implemented")
    }

    async fn subscribe_to_input_stream(&mut self, stream: InputStream) -> anyhow::Result<Pollable> {
        // Ensure that table element is an input-stream:
        let _ = self.table_mut().get_input_stream_mut(stream)?;

        fn input_stream_ready<'a>(stream: &'a mut dyn Any) -> PollableFuture<'a> {
            let stream = stream
                .downcast_mut::<Box<dyn HostInputStream>>()
                // Should be impossible because we made sure this will downcast to a
                // HostImputStream with table check above.
                .expect("downcast to HostInputStream failed");
            stream.ready()
        }

        let pollable = HostPollable::TableEntry {
            index: stream,
            make_future: input_stream_ready,
        };
        Ok(self.table_mut().push_host_pollable(pollable)?)
    }

    async fn subscribe_to_output_stream(
        &mut self,
        stream: OutputStream,
    ) -> anyhow::Result<Pollable> {
        // Ensure that table element is an output-stream:
        let _ = self.table_mut().get_output_stream_mut(stream)?;

        fn output_stream_ready<'a>(stream: &'a mut dyn Any) -> PollableFuture<'a> {
            let stream = stream
                .downcast_mut::<Box<dyn HostOutputStream>>()
                // Should be impossible because we made sure this will downcast to a
                // HostOutputStream with table check above.
                .expect("downcast to HostOutputStream failed");
            stream.ready()
        }

        let pollable = HostPollable::TableEntry {
            index: stream,
            make_future: output_stream_ready,
        };

        Ok(self.table_mut().push_host_pollable(pollable)?)
    }
}

pub mod sync {
    use crate::preview2::{
        bindings::io::streams::Host as AsyncHost,
        bindings::sync_io::io::streams::{self, InputStream, OutputStream},
        bindings::sync_io::poll::poll::Pollable,
        block_on, WasiView,
    };

    impl<T: WasiView> streams::Host for T {
        fn drop_input_stream(&mut self, stream: InputStream) -> anyhow::Result<()> {
            block_on(async { AsyncHost::drop_input_stream(self, stream).await })
        }

        fn drop_output_stream(&mut self, stream: OutputStream) -> anyhow::Result<()> {
            block_on(async { AsyncHost::drop_output_stream(self, stream).await })
        }

        fn read(
            &mut self,
            stream: InputStream,
            len: u64,
        ) -> Result<(Vec<u8>, bool), streams::Error> {
            block_on(async { AsyncHost::read(self, stream, len).await })
                .map_err(streams::Error::from)
        }

        fn blocking_read(
            &mut self,
            stream: InputStream,
            len: u64,
        ) -> Result<(Vec<u8>, bool), streams::Error> {
            block_on(async { AsyncHost::blocking_read(self, stream, len).await })
                .map_err(streams::Error::from)
        }

        fn write(&mut self, stream: OutputStream, bytes: Vec<u8>) -> Result<u64, streams::Error> {
            block_on(async { AsyncHost::write(self, stream, bytes).await })
                .map_err(streams::Error::from)
        }

        fn blocking_write(
            &mut self,
            stream: OutputStream,
            bytes: Vec<u8>,
        ) -> Result<u64, streams::Error> {
            block_on(async { AsyncHost::write(self, stream, bytes).await })
                .map_err(streams::Error::from)
        }

        fn skip(&mut self, stream: InputStream, len: u64) -> Result<(u64, bool), streams::Error> {
            block_on(async { AsyncHost::skip(self, stream, len).await })
                .map_err(streams::Error::from)
        }

        fn blocking_skip(
            &mut self,
            stream: InputStream,
            len: u64,
        ) -> Result<(u64, bool), streams::Error> {
            block_on(async { AsyncHost::blocking_skip(self, stream, len).await })
                .map_err(streams::Error::from)
        }

        fn write_zeroes(&mut self, stream: OutputStream, len: u64) -> Result<u64, streams::Error> {
            block_on(async { AsyncHost::write_zeroes(self, stream, len).await })
                .map_err(streams::Error::from)
        }

        fn blocking_write_zeroes(
            &mut self,
            stream: OutputStream,
            len: u64,
        ) -> Result<u64, streams::Error> {
            block_on(async { AsyncHost::blocking_write_zeroes(self, stream, len).await })
                .map_err(streams::Error::from)
        }

        fn splice(
            &mut self,
            src: InputStream,
            dst: OutputStream,
            len: u64,
        ) -> Result<(u64, bool), streams::Error> {
            block_on(async { AsyncHost::splice(self, src, dst, len).await })
                .map_err(streams::Error::from)
        }

        fn blocking_splice(
            &mut self,
            src: InputStream,
            dst: OutputStream,
            len: u64,
        ) -> Result<(u64, bool), streams::Error> {
            block_on(async { AsyncHost::blocking_splice(self, src, dst, len).await })
                .map_err(streams::Error::from)
        }

        fn forward(&mut self, src: InputStream, dst: OutputStream) -> Result<u64, streams::Error> {
            block_on(async { AsyncHost::forward(self, src, dst).await })
                .map_err(streams::Error::from)
        }

        fn subscribe_to_input_stream(&mut self, stream: InputStream) -> anyhow::Result<Pollable> {
            block_on(async { AsyncHost::subscribe_to_input_stream(self, stream).await })
        }

        fn subscribe_to_output_stream(&mut self, stream: OutputStream) -> anyhow::Result<Pollable> {
            block_on(async { AsyncHost::subscribe_to_output_stream(self, stream).await })
        }
    }
}

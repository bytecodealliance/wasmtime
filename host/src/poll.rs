use crate::{
    wasi_clocks,
    wasi_io::{InputStream, OutputStream, StreamError},
    wasi_poll::{Pollable, WasiPoll},
    WasiCtx,
};
use wasi_common::clocks::TableMonotonicClockExt;
use wasi_common::stream::TableStreamExt;

fn convert(error: wasi_common::Error) -> anyhow::Error {
    if let Some(_errno) = error.downcast_ref() {
        anyhow::Error::new(StreamError {})
    } else {
        error.into()
    }
}

/// A pollable resource table entry.
#[derive(Copy, Clone)]
enum PollableEntry {
    /// Poll for read events.
    Read(InputStream),
    /// Poll for write events.
    Write(OutputStream),
    /// Poll for a monotonic-clock timer.
    MonotonicClock(wasi_clocks::MonotonicClock, wasi_clocks::Instant, bool),
}

#[async_trait::async_trait]
impl WasiPoll for WasiCtx {
    async fn drop_pollable(&mut self, pollable: Pollable) -> anyhow::Result<()> {
        self.table_mut()
            .delete::<PollableEntry>(pollable)
            .map_err(convert)?;
        Ok(())
    }

    async fn poll_oneoff(&mut self, futures: Vec<Pollable>) -> anyhow::Result<Vec<u8>> {
        use wasi_common::sched::{Poll, Userdata};

        // Convert `futures` into `Poll` subscriptions.
        let mut poll = Poll::new();
        let len = futures.len();
        for (index, future) in futures.into_iter().enumerate() {
            match *self.table().get(future).map_err(convert)? {
                PollableEntry::Read(stream) => {
                    let wasi_stream: &dyn wasi_common::InputStream =
                        self.table().get_input_stream(stream).map_err(convert)?;
                    poll.subscribe_read(wasi_stream, Userdata::from(index as u64));
                }
                PollableEntry::Write(stream) => {
                    let wasi_stream: &dyn wasi_common::OutputStream =
                        self.table().get_output_stream(stream).map_err(convert)?;
                    poll.subscribe_write(wasi_stream, Userdata::from(index as u64));
                }
                PollableEntry::MonotonicClock(clock, when, absolute) => {
                    let wasi_clock = self.table().get_monotonic_clock(clock).map_err(convert)?;
                    poll.subscribe_monotonic_clock(
                        wasi_clock,
                        when,
                        absolute,
                        Userdata::from(index as u64),
                    );
                }
            }
        }

        // Do the poll.
        self.sched.poll_oneoff(&mut poll).await?;

        // Convert the results into a list of `u8` to return.
        let mut results = vec![0_u8; len];
        for (_result, data) in poll.results() {
            results[u64::from(data) as usize] = u8::from(true);
        }
        Ok(results)
    }

    async fn subscribe_read(&mut self, stream: InputStream) -> anyhow::Result<Pollable> {
        Ok(self
            .table_mut()
            .push(Box::new(PollableEntry::Read(stream)))?)
    }

    async fn subscribe_write(&mut self, stream: OutputStream) -> anyhow::Result<Pollable> {
        Ok(self
            .table_mut()
            .push(Box::new(PollableEntry::Write(stream)))?)
    }

    async fn subscribe_monotonic_clock(
        &mut self,
        clock: wasi_clocks::MonotonicClock,
        when: wasi_clocks::Instant,
        absolute: bool,
    ) -> anyhow::Result<Pollable> {
        Ok(self
            .table_mut()
            .push(Box::new(PollableEntry::MonotonicClock(
                clock, when, absolute,
            )))?)
    }
}

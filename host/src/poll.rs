use crate::{
    wasi::monotonic_clock::{Instant, MonotonicClock},
    wasi::poll::{self, Pollable},
    wasi::streams::{InputStream, OutputStream, StreamError},
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
pub(crate) enum PollableEntry {
    /// Poll for read events.
    Read(InputStream),
    /// Poll for write events.
    Write(OutputStream),
    /// Poll for a monotonic-clock timer.
    MonotonicClock(MonotonicClock, Instant, bool),
}

#[async_trait::async_trait]
impl poll::Host for WasiCtx {
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
}

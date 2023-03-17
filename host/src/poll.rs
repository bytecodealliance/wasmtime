use crate::{
    command,
    command::wasi::monotonic_clock::{Instant, MonotonicClock},
    command::wasi::poll::Pollable,
    command::wasi::streams::{InputStream, OutputStream, StreamError},
    proxy, WasiCtx,
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

async fn drop_pollable(ctx: &mut WasiCtx, pollable: Pollable) -> anyhow::Result<()> {
    ctx.table_mut()
        .delete::<PollableEntry>(pollable)
        .map_err(convert)?;
    Ok(())
}

async fn poll_oneoff(ctx: &mut WasiCtx, futures: Vec<Pollable>) -> anyhow::Result<Vec<u8>> {
    use wasi_common::sched::{Poll, Userdata};

    // Convert `futures` into `Poll` subscriptions.
    let mut poll = Poll::new();
    let len = futures.len();
    for (index, future) in futures.into_iter().enumerate() {
        match *ctx.table().get(future).map_err(convert)? {
            PollableEntry::Read(stream) => {
                let wasi_stream: &dyn wasi_common::InputStream =
                    ctx.table().get_input_stream(stream).map_err(convert)?;
                poll.subscribe_read(wasi_stream, Userdata::from(index as u64));
            }
            PollableEntry::Write(stream) => {
                let wasi_stream: &dyn wasi_common::OutputStream =
                    ctx.table().get_output_stream(stream).map_err(convert)?;
                poll.subscribe_write(wasi_stream, Userdata::from(index as u64));
            }
            PollableEntry::MonotonicClock(clock, when, absolute) => {
                let wasi_clock = ctx.table().get_monotonic_clock(clock).map_err(convert)?;
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
    ctx.sched.poll_oneoff(&mut poll).await?;

    // Convert the results into a list of `u8` to return.
    let mut results = vec![0_u8; len];
    for (_result, data) in poll.results() {
        results[u64::from(data) as usize] = u8::from(true);
    }
    Ok(results)
}

// Implementatations of the traits for both the command and proxy worlds.
// The bodies have been pulled out into functions above to allow them to
// be shared between the two. Ideally, we should add features to the
// bindings to facilitate this kind of sharing.

#[async_trait::async_trait]
impl command::wasi::poll::Host for WasiCtx {
    async fn drop_pollable(&mut self, pollable: Pollable) -> anyhow::Result<()> {
        drop_pollable(self, pollable).await
    }

    async fn poll_oneoff(&mut self, futures: Vec<Pollable>) -> anyhow::Result<Vec<u8>> {
        poll_oneoff(self, futures).await
    }
}

#[async_trait::async_trait]
impl proxy::wasi::poll::Host for WasiCtx {
    async fn drop_pollable(&mut self, pollable: Pollable) -> anyhow::Result<()> {
        drop_pollable(self, pollable).await
    }

    async fn poll_oneoff(&mut self, futures: Vec<Pollable>) -> anyhow::Result<Vec<u8>> {
        poll_oneoff(self, futures).await
    }
}

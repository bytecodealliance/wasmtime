use crate::{
    command,
    command::wasi::monotonic_clock::Instant,
    command::wasi::poll::Pollable,
    command::wasi::streams::{InputStream, OutputStream, StreamError},
    command::wasi::tcp::TcpSocket,
    proxy, WasiCtx,
};
use wasi_common::stream::TableStreamExt;
use wasi_common::tcp_socket::TableTcpSocketExt;

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
    MonotonicClock(Instant, bool),
    /// Poll for a tcp-socket.
    TcpSocket(TcpSocket),
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
        let userdata = Userdata::from(index as u64);

        match *ctx.table().get(future).map_err(convert)? {
            PollableEntry::Read(stream) => {
                let wasi_stream: &dyn wasi_common::InputStream =
                    ctx.table().get_input_stream(stream).map_err(convert)?;
                poll.subscribe_read(wasi_stream, userdata);
            }
            PollableEntry::Write(stream) => {
                let wasi_stream: &dyn wasi_common::OutputStream =
                    ctx.table().get_output_stream(stream).map_err(convert)?;
                poll.subscribe_write(wasi_stream, userdata);
            }
            PollableEntry::MonotonicClock(when, absolute) => {
                poll.subscribe_monotonic_clock(&*ctx.clocks.monotonic, when, absolute, userdata);
            }
            PollableEntry::TcpSocket(tcp_socket) => {
                let wasi_tcp_socket: &dyn wasi_common::WasiTcpSocket =
                    ctx.table().get_tcp_socket(tcp_socket).map_err(convert)?;
                poll.subscribe_tcp_socket(wasi_tcp_socket, userdata);
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

use crate::{
    stream::TableStreamExt,
    wasi,
    wasi::monotonic_clock::Instant,
    wasi::poll::Pollable,
    wasi::streams::{InputStream, OutputStream, StreamError},
    WasiView,
};

fn convert(error: crate::Error) -> anyhow::Error {
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
    /* FIXME: need to rebuild the poll interface to let pollables be created in different crates.
    /// Poll for a tcp-socket.
    TcpSocket(TcpSocket),
    */
}

// Implementatations of the interface. The bodies had been pulled out into
// functions above to allow them to be shared between the two worlds, which
// used to require different traits . Features have been added to facilitate
// sharing between worlds, but I want to avoid the huge whitespace diff on
// this PR.

#[async_trait::async_trait]
impl<T: WasiView> wasi::poll::Host for T {
    async fn drop_pollable(&mut self, pollable: Pollable) -> anyhow::Result<()> {
        self.table_mut()
            .delete::<PollableEntry>(pollable)
            .map_err(convert)?;
        Ok(())
    }

    async fn poll_oneoff(&mut self, futures: Vec<Pollable>) -> anyhow::Result<Vec<u8>> {
        use crate::sched::{Poll, Userdata};

        // Convert `futures` into `Poll` subscriptions.
        let mut poll = Poll::new();
        let len = futures.len();
        for (index, future) in futures.into_iter().enumerate() {
            let userdata = Userdata::from(index as u64);

            match *self.table().get(future).map_err(convert)? {
                PollableEntry::Read(stream) => {
                    let wasi_stream: &dyn crate::InputStream =
                        self.table().get_input_stream(stream).map_err(convert)?;
                    poll.subscribe_read(wasi_stream, userdata);
                }
                PollableEntry::Write(stream) => {
                    let wasi_stream: &dyn crate::OutputStream =
                        self.table().get_output_stream(stream).map_err(convert)?;
                    poll.subscribe_write(wasi_stream, userdata);
                }
                PollableEntry::MonotonicClock(when, absolute) => {
                    poll.subscribe_monotonic_clock(
                        &*self.ctx().clocks.monotonic,
                        when,
                        absolute,
                        userdata,
                    );
                } /*
                  PollableEntry::TcpSocket(tcp_socket) => {
                      let wasi_tcp_socket: &dyn crate::WasiTcpSocket =
                          self.table().get_tcp_socket(tcp_socket).map_err(convert)?;
                      poll.subscribe_tcp_socket(wasi_tcp_socket, userdata);
                  }
                  */
            }
        }

        let ctx = self.ctx();
        // Do the poll.
        ctx.sched.poll_oneoff(&mut poll).await?;

        // Convert the results into a list of `u8` to return.
        let mut results = vec![0_u8; len];
        for (_result, data) in poll.results() {
            results[u64::from(data) as usize] = u8::from(true);
        }
        Ok(results)
    }
}

use crate::preview2::{
    stream::TableStreamExt,
    wasi::clocks::monotonic_clock::Instant,
    wasi::io::streams::{InputStream, OutputStream},
    wasi::poll::poll::{self, Pollable},
    WasiView,
};

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
impl<T: WasiView> poll::Host for T {
    async fn drop_pollable(&mut self, pollable: Pollable) -> anyhow::Result<()> {
        self.table_mut().delete::<PollableEntry>(pollable)?;
        Ok(())
    }

    async fn poll_oneoff(&mut self, futures: Vec<Pollable>) -> anyhow::Result<Vec<bool>> {
        use crate::preview2::sched::{sync::SyncSched, Poll, Userdata, WasiSched};

        // Convert `futures` into `Poll` subscriptions.
        let mut poll = Poll::new();
        let len = futures.len();
        for (index, future) in futures.into_iter().enumerate() {
            let userdata = Userdata::from(index as u64);

            match *self.table().get(future)? {
                PollableEntry::Read(stream) => {
                    let wasi_stream: &dyn crate::preview2::InputStream =
                        self.table().get_input_stream(stream)?;
                    poll.subscribe_read(wasi_stream, userdata);
                }
                PollableEntry::Write(stream) => {
                    let wasi_stream: &dyn crate::preview2::OutputStream =
                        self.table().get_output_stream(stream)?;
                    poll.subscribe_write(wasi_stream, userdata);
                }
                PollableEntry::MonotonicClock(when, absolute) => {
                    poll.subscribe_monotonic_clock(
                        &*self.ctx().monotonic_clock,
                        when,
                        absolute,
                        userdata,
                    );
                } /*
                  PollableEntry::TcpSocket(tcp_socket) => {
                      let wasi_tcp_socket: &dyn crate::WasiTcpSocket =
                          self.table().get_tcp_socket(tcp_socket)?;
                      poll.subscribe_tcp_socket(wasi_tcp_socket, userdata);
                  }
                  */
            }
        }

        // Do the poll.
        SyncSched.poll_oneoff(&mut poll).await?;

        let mut results = vec![false; len];
        for (_result, data) in poll.results() {
            results[u64::from(data) as usize] = true;
        }
        Ok(results)
    }
}

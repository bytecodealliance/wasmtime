use rustix::io::{PollFd, PollFlags};
use std::thread;
use std::time::Duration;
use wasi_common::sched::subscription::{RwEventFlags, RwStream};
use wasi_common::{
    sched::{Poll, WasiSched},
    Error, ErrorExt,
};

pub async fn poll_oneoff<'a>(poll: &mut Poll<'a>) -> Result<(), Error> {
    // Collect all stream I/O subscriptions. Clock subscriptions are handled
    // separately below.
    let mut ready = false;
    let mut pollfds = Vec::new();
    for rwsub in poll.rw_subscriptions() {
        match rwsub.stream {
            RwStream::Read(stream) => {
                // Poll things that can be polled.
                if let Some(fd) = stream.pollable_read() {
                    #[cfg(unix)]
                    {
                        pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::IN));
                        continue;
                    }

                    #[cfg(windows)]
                    {
                        if let Some(fd) = fd.as_socket() {
                            pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::IN));
                            continue;
                        }
                    }
                }

                // Allow in-memory buffers or other immediately-available
                // sources to complete successfully.
                if let Ok(nbytes) = stream.num_ready_bytes().await {
                    if nbytes != 0 {
                        rwsub.complete(RwEventFlags::empty());
                        ready = true;
                        continue;
                    }
                }

                return Err(Error::invalid_argument().context("stream is not pollable for reading"));
            }

            RwStream::Write(stream) => {
                let fd = stream.pollable_write().ok_or(
                    Error::invalid_argument().context("stream is not pollable for writing"),
                )?;

                #[cfg(unix)]
                {
                    pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::OUT));
                }

                #[cfg(windows)]
                {
                    if let Some(fd) = fd.as_socket() {
                        pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::OUT));
                    } else {
                        return Err(Error::trap(anyhow::anyhow!(
                            "unimplemented: polling for writing to non-OS resources"
                        )));
                    }
                }
            }

            RwStream::TcpSocket(tcp_socket) => {
                let fd = tcp_socket.pollable();
                pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::IN | PollFlags::PRI));
            }
        }
    }

    // If we didn't have any streams that are immediately available, do an OS
    // `poll` to wait for streams to become available.
    if !ready {
        loop {
            let poll_timeout = if let Some(t) = poll.earliest_clock_deadline() {
                // Convert the timeout to milliseconds for `poll`, rounding up.
                //
                // TODO: On Linux and FreeBSD, we could use `ppoll` instead
                // which takes a `timespec.`
                ((t.deadline + 999_999) / 1_000_000)
                    .try_into()
                    .map_err(|_| Error::overflow().context("poll timeout"))?
            } else {
                // A negative value requests an infinite timeout.
                -1
            };
            tracing::debug!(
                poll_timeout = tracing::field::debug(poll_timeout),
                poll_fds = tracing::field::debug(&pollfds),
                "poll"
            );
            match rustix::io::poll(&mut pollfds, poll_timeout) {
                Ok(_num_ready) => {
                    ready = true;
                    break;
                }
                Err(rustix::io::Errno::INTR) => continue,
                Err(err) => return Err(std::io::Error::from(err).into()),
            }
        }

        assert_eq!(poll.rw_subscriptions().count(), pollfds.len());

        // If the OS `poll` returned events, record them.
        if ready {
            // Iterate through the stream subscriptions, skipping those that
            // were already completed due to being immediately available.
            for (rwsub, pollfd) in poll.rw_subscriptions().zip(pollfds.into_iter()) {
                let revents = pollfd.revents();
                if revents.contains(PollFlags::NVAL) {
                    rwsub.error(Error::badf());
                } else if revents.contains(PollFlags::ERR) {
                    rwsub.error(Error::io());
                } else if revents.contains(PollFlags::HUP) {
                    rwsub.complete(RwEventFlags::HANGUP);
                } else {
                    rwsub.complete(RwEventFlags::empty());
                };
            }
        }
    };

    // If we had no immediately-available events and no events becoming
    // available in a `poll`, it means we timed out. Report that event.
    if !ready {
        poll.earliest_clock_deadline()
            .expect("timed out")
            .result()
            .expect("timer deadline is past")
            .unwrap()
    }

    Ok(())
}
pub struct SyncSched {}
impl SyncSched {
    pub fn new() -> Self {
        Self {}
    }
}
#[async_trait::async_trait]
impl WasiSched for SyncSched {
    async fn poll_oneoff<'a>(&self, poll: &mut Poll<'a>) -> Result<(), Error> {
        poll_oneoff(poll).await
    }
    async fn sched_yield(&self) -> Result<(), Error> {
        thread::yield_now();
        Ok(())
    }
    async fn sleep(&self, duration: Duration) -> Result<(), Error> {
        std::thread::sleep(duration);
        Ok(())
    }
}
pub fn sched_ctx() -> Box<dyn WasiSched> {
    Box::new(SyncSched::new())
}

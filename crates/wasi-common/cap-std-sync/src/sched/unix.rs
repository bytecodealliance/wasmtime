use cap_std::time::Duration;
use std::convert::TryInto;
use std::ops::Deref;
use std::os::unix::io::{AsRawFd, RawFd};
use wasi_common::{
    file::WasiFile,
    sched::{
        subscription::{RwEventFlags, Subscription},
        Poll, WasiSched,
    },
    Error, ErrorExt,
};

use poll::{PollFd, PollFlags};

pub struct SyncSched;

impl SyncSched {
    pub fn new() -> Self {
        SyncSched
    }
}

#[wiggle::async_trait]
impl WasiSched for SyncSched {
    async fn poll_oneoff<'a>(&self, poll: &'_ Poll<'a>) -> Result<(), Error> {
        if poll.is_empty() {
            return Ok(());
        }
        let mut pollfds = Vec::new();
        let timeout = poll.earliest_clock_deadline();
        for s in poll.rw_subscriptions() {
            match s {
                Subscription::Read(f) => {
                    let raw_fd = wasi_file_raw_fd(f.file.deref()).ok_or(
                        Error::invalid_argument().context("read subscription fd downcast failed"),
                    )?;
                    pollfds.push(unsafe { PollFd::new(raw_fd, PollFlags::POLLIN) });
                }

                Subscription::Write(f) => {
                    let raw_fd = wasi_file_raw_fd(f.file.deref()).ok_or(
                        Error::invalid_argument().context("write subscription fd downcast failed"),
                    )?;
                    pollfds.push(unsafe { PollFd::new(raw_fd, PollFlags::POLLOUT) });
                }
                Subscription::MonotonicClock { .. } => unreachable!(),
            }
        }

        let ready = loop {
            let poll_timeout = if let Some(t) = timeout {
                let duration = t.duration_until().unwrap_or(Duration::from_secs(0));
                (duration.as_millis() + 1) // XXX try always rounding up?
                    .try_into()
                    .map_err(|_| Error::overflow().context("poll timeout"))?
            } else {
                libc::c_int::max_value()
            };
            tracing::debug!(
                poll_timeout = tracing::field::debug(poll_timeout),
                poll_fds = tracing::field::debug(&pollfds),
                "poll"
            );
            match poll::poll(&mut pollfds, poll_timeout) {
                Ok(ready) => break ready,
                Err(_) => {
                    let last_err = std::io::Error::last_os_error();
                    if last_err.raw_os_error().unwrap() == libc::EINTR {
                        continue;
                    } else {
                        return Err(last_err.into());
                    }
                }
            }
        };
        if ready > 0 {
            for (rwsub, pollfd) in poll.rw_subscriptions().zip(pollfds.into_iter()) {
                if let Some(revents) = pollfd.revents() {
                    let (nbytes, rwsub) = match rwsub {
                        Subscription::Read(sub) => {
                            let ready = sub.file.num_ready_bytes().await?;
                            (std::cmp::max(ready, 1), sub)
                        }
                        Subscription::Write(sub) => (0, sub),
                        _ => unreachable!(),
                    };
                    if revents.contains(PollFlags::POLLNVAL) {
                        rwsub.error(Error::badf());
                    } else if revents.contains(PollFlags::POLLERR) {
                        rwsub.error(Error::io());
                    } else if revents.contains(PollFlags::POLLHUP) {
                        rwsub.complete(nbytes, RwEventFlags::HANGUP);
                    } else {
                        rwsub.complete(nbytes, RwEventFlags::empty());
                    };
                }
            }
        } else {
            timeout
                .expect("timed out")
                .result()
                .expect("timer deadline is past")
                .unwrap()
        }
        Ok(())
    }
    async fn sched_yield(&self) -> Result<(), Error> {
        std::thread::yield_now();
        Ok(())
    }
    async fn sleep(&self, duration: Duration) -> Result<(), Error> {
        std::thread::sleep(duration);
        Ok(())
    }
}

fn wasi_file_raw_fd(f: &dyn WasiFile) -> Option<RawFd> {
    let a = f.as_any();
    if a.is::<crate::file::File>() {
        Some(a.downcast_ref::<crate::file::File>().unwrap().as_raw_fd())
    } else if a.is::<crate::stdio::Stdin>() {
        Some(a.downcast_ref::<crate::stdio::Stdin>().unwrap().as_raw_fd())
    } else if a.is::<crate::stdio::Stdout>() {
        Some(
            a.downcast_ref::<crate::stdio::Stdout>()
                .unwrap()
                .as_raw_fd(),
        )
    } else if a.is::<crate::stdio::Stderr>() {
        Some(
            a.downcast_ref::<crate::stdio::Stderr>()
                .unwrap()
                .as_raw_fd(),
        )
    } else {
        None
    }
}

mod poll {
    use bitflags::bitflags;
    use std::convert::TryInto;
    use std::os::unix::io::RawFd;

    bitflags! {
        pub struct PollFlags: libc::c_short {
            const POLLIN = libc::POLLIN;
            const POLLPRI = libc::POLLPRI;
            const POLLOUT = libc::POLLOUT;
            const POLLRDNORM = libc::POLLRDNORM;
            const POLLWRNORM = libc::POLLWRNORM;
            const POLLRDBAND = libc::POLLRDBAND;
            const POLLWRBAND = libc::POLLWRBAND;
            const POLLERR = libc::POLLERR;
            const POLLHUP = libc::POLLHUP;
            const POLLNVAL = libc::POLLNVAL;
        }
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[repr(C)]
    pub struct PollFd(libc::pollfd);

    impl PollFd {
        pub unsafe fn new(fd: RawFd, events: PollFlags) -> Self {
            Self(libc::pollfd {
                fd,
                events: events.bits(),
                revents: PollFlags::empty().bits(),
            })
        }

        pub fn revents(self) -> Option<PollFlags> {
            PollFlags::from_bits(self.0.revents)
        }
    }

    pub fn poll(fds: &mut [PollFd], timeout: libc::c_int) -> Result<usize, std::io::Error> {
        let nready = unsafe {
            libc::poll(
                fds.as_mut_ptr() as *mut libc::pollfd,
                fds.len() as libc::nfds_t,
                timeout,
            )
        };
        if nready == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            // When poll doesn't fail, its return value is a non-negative int, which will
            // always be convertable to usize, so we can unwrap() here.
            Ok(nready.try_into().unwrap())
        }
    }
}

use crate::handle::{Filesize, Filetype};
use crate::sched::{Eventrwflags, RwSubscription, Subscription, SubscriptionSet};
use crate::sys::AsFile;
use crate::Error;
use std::time::SystemTime;
use std::{convert::TryInto, os::unix::prelude::AsRawFd};
use yanix::file::fionread;
use yanix::poll::{poll, PollFd, PollFlags};

pub fn oneoff(ss: SubscriptionSet) -> Result<(), Error> {
    let earliest_deadline = ss.earliest_deadline();

    let mut poll_fd_error = false;
    let mut poll_fds = Vec::new();
    let mut rw_subs = Vec::new();

    for s in ss.subs.iter() {
        match s {
            Subscription::Timer { .. } => {}
            Subscription::Read(s) => match s.as_poll_fd(PollFlags::POLLIN) {
                Ok(poll_fd) => {
                    poll_fds.push(poll_fd);
                    rw_subs.push(RW::Read(&s));
                }
                Err(e) => {
                    s.error(e);
                    poll_fd_error = true;
                }
            },
            Subscription::Write(s) => match s.as_poll_fd(PollFlags::POLLOUT) {
                Ok(poll_fd) => {
                    poll_fds.push(poll_fd);
                    rw_subs.push(RW::Write(&s));
                }
                Err(e) => {
                    s.error(e);
                    poll_fd_error = true;
                }
            },
        }
    }

    if poll_fd_error {
        return Err(Error::Inval);
    }

    let ready = loop {
        let poll_timeout = poll_timeout_until(&earliest_deadline);
        match poll(&mut poll_fds, poll_timeout) {
            Err(_) => {
                let last_err = std::io::Error::last_os_error();
                if last_err.raw_os_error().unwrap() == libc::EINTR {
                    continue;
                } else {
                    return Err(last_err.into());
                }
            }
            Ok(ready) => break ready,
        }
    };

    if ready > 0 {
        for (rw_sub, poll_fd) in rw_subs.into_iter().zip(poll_fds.into_iter()) {
            let revents = match poll_fd.revents() {
                Some(revents) => revents,
                None => continue,
            };

            if revents.contains(PollFlags::POLLNVAL) {
                rw_sub.error(Error::Badf)
            } else if revents.contains(PollFlags::POLLERR) {
                rw_sub.error(Error::Io)
            } else if revents.contains(PollFlags::POLLHUP) {
                rw_sub.complete(0, Eventrwflags::FD_READWRITE_HANGUP)
            } else if revents.contains(PollFlags::POLLIN) | revents.contains(PollFlags::POLLOUT) {
                let nbytes = rw_sub.query_nbytes()?;
                rw_sub.complete(nbytes.try_into()?, Eventrwflags::empty())
            }
        }
    }

    Ok(())
}

fn poll_timeout_until(deadline: &Option<SystemTime>) -> i32 {
    if let Some(deadline) = deadline {
        let now = SystemTime::now();
        match deadline.duration_since(now) {
            Ok(duration_into_future) => duration_into_future
                .as_millis()
                .try_into()
                .unwrap_or(libc::c_int::max_value()),
            Err(_) => 0,
        }
    } else {
        -1
    }
}

trait AsPollFd {
    fn as_poll_fd(&self, flags: PollFlags) -> Result<PollFd, Error>;
}

impl AsPollFd for RwSubscription {
    fn as_poll_fd(&self, flags: PollFlags) -> Result<PollFd, Error> {
        let file = self.handle.as_file()?;
        unsafe { Ok(PollFd::new(file.as_raw_fd(), flags)) }
    }
}

enum RW<'a> {
    Read(&'a RwSubscription),
    Write(&'a RwSubscription),
}

impl<'a> RW<'a> {
    fn complete(&self, size: Filesize, flags: Eventrwflags) {
        match self {
            Self::Read(s) => s.complete(size, flags),
            Self::Write(s) => s.complete(size, flags),
        }
    }
    fn error(&self, error: Error) {
        match self {
            Self::Read(s) => s.error(error),
            Self::Write(s) => s.error(error),
        }
    }
    fn query_nbytes(&self) -> Result<u64, Error> {
        match self {
            Self::Read(s) => {
                let file = s.handle.as_file()?;
                if s.handle.get_file_type() == Filetype::RegularFile {
                    // fionread may overflow for large files, so use another way for regular files.
                    use yanix::file::tell;
                    let meta = file.metadata()?;
                    let len = meta.len();
                    let host_offset = unsafe { tell(file.as_raw_fd())? };
                    return Ok(len - host_offset);
                }
                Ok(unsafe { fionread(file.as_raw_fd())?.into() })
            }
            Self::Write { .. } => Ok(0),
        }
    }
}

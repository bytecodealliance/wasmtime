use crate::{Errno, Result};
use bitflags::bitflags;
use std::{convert::TryInto, os::unix::prelude::*};

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

pub fn poll(fds: &mut [PollFd], timeout: i32) -> Result<usize> {
    Errno::from_result(unsafe {
        libc::poll(
            fds.as_mut_ptr() as *mut libc::pollfd,
            fds.len() as libc::nfds_t,
            timeout,
        )
    })
    .and_then(|nready| nready.try_into().map_err(Into::into))
}

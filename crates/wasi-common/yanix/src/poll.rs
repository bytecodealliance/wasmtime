use super::{errno::Errno, Result};
use bitflags::bitflags;
use std::os::unix::prelude::*;

bitflags! {
    /// These flags define the different events that can be monitored by `poll` and `ppoll`
    pub struct PollFlags: libc::c_short {
        /// There is data to read.
        const POLLIN = libc::POLLIN;
        /// There is some exceptional condition on the file descriptor.
        ///
        /// Possibilities include:
        ///
        /// *  There is out-of-band data on a TCP socket (see
        ///    [tcp(7)](http://man7.org/linux/man-pages/man7/tcp.7.html)).
        /// *  A pseudoterminal master in packet mode has seen a state
        ///    change on the slave (see
        ///    [ioctl_tty(2)](http://man7.org/linux/man-pages/man2/ioctl_tty.2.html)).
        /// *  A cgroup.events file has been modified (see
        ///    [cgroups(7)](http://man7.org/linux/man-pages/man7/cgroups.7.html)).
        const POLLPRI = libc::POLLPRI;
        /// Writing is now possible, though a write larger that the
        /// available space in a socket or pipe will still block (unless
        /// `O_NONBLOCK` is set).
        const POLLOUT = libc::POLLOUT;
        /// Equivalent to [`POLLIN`](constant.POLLIN.html)
        const POLLRDNORM = libc::POLLRDNORM;
        /// Equivalent to [`POLLOUT`](constant.POLLOUT.html)
        const POLLWRNORM = libc::POLLWRNORM;
        /// Priority band data can be read (generally unused on Linux).
        const POLLRDBAND = libc::POLLRDBAND;
        /// Priority data may be written.
        const POLLWRBAND = libc::POLLWRBAND;
        /// Error condition (only returned in
        /// [`PollFd::revents`](struct.PollFd.html#method.revents);
        /// ignored in [`PollFd::new`](struct.PollFd.html#method.new)).
        /// This bit is also set for a file descriptor referring to the
        /// write end of a pipe when the read end has been closed.
        const POLLERR = libc::POLLERR;
        /// Hang up (only returned in [`PollFd::revents`](struct.PollFd.html#method.revents);
        /// ignored in [`PollFd::new`](struct.PollFd.html#method.new)).
        /// Note that when reading from a channel such as a pipe or a stream
        /// socket, this event merely indicates that the peer closed its
        /// end of the channel.  Subsequent reads from the channel will
        /// return 0 (end of file) only after all outstanding data in the
        /// channel has been consumed.
        const POLLHUP = libc::POLLHUP;
        /// Invalid request: `fd` not open (only returned in
        /// [`PollFd::revents`](struct.PollFd.html#method.revents);
        /// ignored in [`PollFd::new`](struct.PollFd.html#method.new)).
        const POLLNVAL = libc::POLLNVAL;
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct PollFd(libc::pollfd);

impl PollFd {
    pub fn new(fd: RawFd, events: PollFlags) -> Self {
        Self(libc::pollfd {
            fd,
            events: events.bits(),
            revents: PollFlags::empty().bits(),
        })
    }

    /// Returns the events that occured in the last call to `poll` or `ppoll`.
    pub fn revents(self) -> Option<PollFlags> {
        PollFlags::from_bits(self.0.revents)
    }
}

pub fn poll(fds: &mut [PollFd], timeout: i32) -> Result<usize> {
    use std::convert::TryInto;
    Errno::from_result(unsafe {
        libc::poll(
            fds.as_mut_ptr() as *mut libc::pollfd,
            fds.len() as libc::nfds_t,
            timeout,
        )
    })
    .and_then(|nready| nready.try_into().map_err(Into::into))
}

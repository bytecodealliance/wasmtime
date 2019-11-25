use super::{errno::Errno, libc_bitflags, Result};
use std::os::unix::prelude::*;

/// This is a wrapper around `libc::pollfd`.
///
/// It's meant to be used as an argument to the [`poll`](fn.poll.html) and
/// [`ppoll`](fn.ppoll.html) functions to specify the events of interest
/// for a specific file descriptor.
///
/// After a call to `poll` or `ppoll`, the events that occured can be
/// retrieved by calling [`revents()`](#method.revents) on the `PollFd`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PollFd {
    pollfd: libc::pollfd,
}

impl PollFd {
    /// Creates a new `PollFd` specifying the events of interest
    /// for a given file descriptor.
    pub fn new(fd: RawFd, events: PollFlags) -> PollFd {
        PollFd {
            pollfd: libc::pollfd {
                fd,
                events: events.bits(),
                revents: PollFlags::empty().bits(),
            },
        }
    }

    /// Returns the events that occured in the last call to `poll` or `ppoll`.
    pub fn revents(self) -> Option<PollFlags> {
        PollFlags::from_bits(self.pollfd.revents)
    }
}

libc_bitflags! {
    /// These flags define the different events that can be monitored by `poll` and `ppoll`
    pub struct PollFlags: libc::c_short {
        /// There is data to read.
        POLLIN;
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
        POLLPRI;
        /// Writing is now possible, though a write larger that the
        /// available space in a socket or pipe will still block (unless
        /// `O_NONBLOCK` is set).
        POLLOUT;
        /// Equivalent to [`POLLIN`](constant.POLLIN.html)
        POLLRDNORM;
        /// Equivalent to [`POLLOUT`](constant.POLLOUT.html)
        POLLWRNORM;
        /// Priority band data can be read (generally unused on Linux).
        POLLRDBAND;
        /// Priority data may be written.
        POLLWRBAND;
        /// Error condition (only returned in
        /// [`PollFd::revents`](struct.PollFd.html#method.revents);
        /// ignored in [`PollFd::new`](struct.PollFd.html#method.new)).
        /// This bit is also set for a file descriptor referring to the
        /// write end of a pipe when the read end has been closed.
        POLLERR;
        /// Hang up (only returned in [`PollFd::revents`](struct.PollFd.html#method.revents);
        /// ignored in [`PollFd::new`](struct.PollFd.html#method.new)).
        /// Note that when reading from a channel such as a pipe or a stream
        /// socket, this event merely indicates that the peer closed its
        /// end of the channel.  Subsequent reads from the channel will
        /// return 0 (end of file) only after all outstanding data in the
        /// channel has been consumed.
        POLLHUP;
        /// Invalid request: `fd` not open (only returned in
        /// [`PollFd::revents`](struct.PollFd.html#method.revents);
        /// ignored in [`PollFd::new`](struct.PollFd.html#method.new)).
        POLLNVAL;
    }
}

/// `poll` waits for one of a set of file descriptors to become ready to perform I/O.
/// ([`poll(2)`](http://pubs.opengroup.org/onlinepubs/9699919799/functions/poll.html))
///
/// `fds` contains all [`PollFd`](struct.PollFd.html) to poll.
/// The function will return as soon as any event occur for any of these `PollFd`s.
///
/// The `timeout` argument specifies the number of milliseconds that `poll()`
/// should block waiting for a file descriptor to become ready.  The call
/// will block until either:
///
/// *  a file descriptor becomes ready;
/// *  the call is interrupted by a signal handler; or
/// *  the timeout expires.
///
/// Note that the timeout interval will be rounded up to the system clock
/// granularity, and kernel scheduling delays mean that the blocking
/// interval may overrun by a small amount.  Specifying a negative value
/// in timeout means an infinite timeout.  Specifying a timeout of zero
/// causes `poll()` to return immediately, even if no file descriptors are
/// ready.
pub fn poll(fds: &mut [PollFd], timeout: libc::c_int) -> Result<libc::c_int> {
    Errno::from_result(unsafe {
        libc::poll(
            fds.as_mut_ptr() as *mut libc::pollfd,
            fds.len() as libc::nfds_t,
            timeout,
        )
    })
}

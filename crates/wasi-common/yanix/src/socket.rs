use super::{errno::Errno, Result};
use std::os::unix::prelude::*;

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum SockType {
    /// Provides sequenced, reliable, two-way, connection-
    /// based byte streams.  An out-of-band data transmission
    /// mechanism may be supported.
    Stream = libc::SOCK_STREAM,
    /// Supports datagrams (connectionless, unreliable
    /// messages of a fixed maximum length).
    Datagram = libc::SOCK_DGRAM,
    /// Provides a sequenced, reliable, two-way connection-
    /// based data transmission path for datagrams of fixed
    /// maximum length; a consumer is required to read an
    /// entire packet with each input system call.
    SeqPacket = libc::SOCK_SEQPACKET,
    /// Provides raw network protocol access.
    Raw = libc::SOCK_RAW,
    /// Provides a reliable datagram layer that does not
    /// guarantee ordering.
    Rdm = libc::SOCK_RDM,
}

pub fn get_socket_type(fd: RawFd) -> Result<SockType> {
    use std::mem::{self, MaybeUninit};
    let mut buffer = unsafe { MaybeUninit::<SockType>::zeroed().assume_init() };
    let mut out_len = mem::size_of::<SockType>() as libc::socklen_t;
    Errno::from_success_code(unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            &mut buffer as *mut SockType as *mut _,
            &mut out_len,
        )
    })?;
    assert_eq!(
        out_len as usize,
        mem::size_of::<SockType>(),
        "invalid SockType value"
    );
    Ok(buffer)
}

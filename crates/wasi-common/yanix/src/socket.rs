use crate::{Errno, Result};
use std::os::unix::prelude::*;

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum SockType {
    Stream = libc::SOCK_STREAM,
    Datagram = libc::SOCK_DGRAM,
    SeqPacket = libc::SOCK_SEQPACKET,
    Raw = libc::SOCK_RAW,
    Rdm = libc::SOCK_RDM,
}

pub unsafe fn get_socket_type(fd: RawFd) -> Result<SockType> {
    use std::mem::{self, MaybeUninit};
    let mut buffer = MaybeUninit::<SockType>::zeroed().assume_init();
    let mut out_len = mem::size_of::<SockType>() as libc::socklen_t;
    Errno::from_success_code(libc::getsockopt(
        fd,
        libc::SOL_SOCKET,
        libc::SO_TYPE,
        &mut buffer as *mut SockType as *mut _,
        &mut out_len,
    ))?;
    assert_eq!(
        out_len as usize,
        mem::size_of::<SockType>(),
        "invalid SockType value"
    );
    Ok(buffer)
}

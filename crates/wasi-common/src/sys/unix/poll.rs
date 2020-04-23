use super::super::oshandle::OsHandle;
use crate::poll::{ClockEventData, FdEventData};
use crate::sys::oshandle::AsFile;
use crate::wasi::{types, Errno, Result};
use std::io;
use std::{convert::TryInto, os::unix::prelude::AsRawFd};
use yanix::file::fionread;
use yanix::poll::{poll, PollFd, PollFlags};

pub(crate) fn oneoff(
    timeout: Option<ClockEventData>,
    fd_events: Vec<FdEventData>,
    events: &mut Vec<types::Event>,
) -> Result<()> {
    if fd_events.is_empty() && timeout.is_none() {
        return Ok(());
    }

    let mut poll_fds: Vec<_> = fd_events
        .iter()
        .map(|event| {
            let mut flags = PollFlags::empty();
            match event.r#type {
                types::Eventtype::FdRead => flags.insert(PollFlags::POLLIN),
                types::Eventtype::FdWrite => flags.insert(PollFlags::POLLOUT),
                // An event on a file descriptor can currently only be of type FD_READ or FD_WRITE
                // Nothing else has been defined in the specification, and these are also the only two
                // events we filtered before. If we get something else here, the code has a serious bug.
                _ => unreachable!(),
            };
            let handle = event
                .handle
                .as_any()
                .downcast_ref::<OsHandle>()
                .expect("can poll FdEvent for OS resources only");
            unsafe { PollFd::new(handle.as_raw_fd(), flags) }
        })
        .collect();

    let poll_timeout = timeout.map_or(-1, |timeout| {
        let delay = timeout.delay / 1_000_000; // poll syscall requires delay to expressed in milliseconds
        delay.try_into().unwrap_or(libc::c_int::max_value())
    });
    log::debug!("poll_oneoff poll_timeout = {:?}", poll_timeout);

    let ready = loop {
        match poll(&mut poll_fds, poll_timeout) {
            Err(_) => {
                let last_err = io::Error::last_os_error();
                if last_err.raw_os_error().unwrap() == libc::EINTR {
                    continue;
                }
                return Err(last_err.into());
            }
            Ok(ready) => break ready,
        }
    };

    Ok(if ready == 0 {
        handle_timeout_event(timeout.expect("timeout should not be None"), events)
    } else {
        let ready_events = fd_events.into_iter().zip(poll_fds.into_iter()).take(ready);
        handle_fd_event(ready_events, events)?
    })
}

fn handle_timeout_event(timeout: ClockEventData, events: &mut Vec<types::Event>) {
    events.push(types::Event {
        userdata: timeout.userdata,
        error: Errno::Success,
        type_: types::Eventtype::Clock,
        fd_readwrite: types::EventFdReadwrite {
            flags: types::Eventrwflags::empty(),
            nbytes: 0,
        },
    });
}

fn handle_fd_event(
    ready_events: impl Iterator<Item = (FdEventData, yanix::poll::PollFd)>,
    events: &mut Vec<types::Event>,
) -> Result<()> {
    fn query_nbytes(handle: &OsHandle) -> Result<u64> {
        // fionread may overflow for large files, so use another way for regular files.
        if let OsHandle::OsFile(file) = handle {
            let meta = file.as_file().metadata()?;
            if meta.file_type().is_file() {
                use yanix::file::tell;
                let len = meta.len();
                let host_offset = unsafe { tell(file.as_raw_fd())? };
                return Ok(len - host_offset);
            }
        }
        unsafe { Ok(fionread(handle.as_raw_fd())?.into()) }
    }

    for (fd_event, poll_fd) in ready_events {
        // log::debug!("poll_oneoff_handle_fd_event fd_event = {:?}", fd_event);
        log::debug!("poll_oneoff_handle_fd_event poll_fd = {:?}", poll_fd);

        let revents = match poll_fd.revents() {
            Some(revents) => revents,
            None => continue,
        };

        log::debug!("poll_oneoff_handle_fd_event revents = {:?}", revents);

        let nbytes = if fd_event.r#type == types::Eventtype::FdRead {
            let handle = fd_event
                .handle
                .as_any()
                .downcast_ref::<OsHandle>()
                .expect("can poll FdEvent for OS resources only");
            query_nbytes(handle)?
        } else {
            0
        };

        let output_event = if revents.contains(PollFlags::POLLNVAL) {
            types::Event {
                userdata: fd_event.userdata,
                error: Errno::Badf,
                type_: fd_event.r#type,
                fd_readwrite: types::EventFdReadwrite {
                    nbytes: 0,
                    flags: types::Eventrwflags::FD_READWRITE_HANGUP,
                },
            }
        } else if revents.contains(PollFlags::POLLERR) {
            types::Event {
                userdata: fd_event.userdata,
                error: Errno::Io,
                type_: fd_event.r#type,
                fd_readwrite: types::EventFdReadwrite {
                    nbytes: 0,
                    flags: types::Eventrwflags::FD_READWRITE_HANGUP,
                },
            }
        } else if revents.contains(PollFlags::POLLHUP) {
            types::Event {
                userdata: fd_event.userdata,
                error: Errno::Success,
                type_: fd_event.r#type,
                fd_readwrite: types::EventFdReadwrite {
                    nbytes: 0,
                    flags: types::Eventrwflags::FD_READWRITE_HANGUP,
                },
            }
        } else if revents.contains(PollFlags::POLLIN) | revents.contains(PollFlags::POLLOUT) {
            types::Event {
                userdata: fd_event.userdata,
                error: Errno::Success,
                type_: fd_event.r#type,
                fd_readwrite: types::EventFdReadwrite {
                    nbytes: nbytes.try_into()?,
                    flags: types::Eventrwflags::empty(),
                },
            }
        } else {
            continue;
        };

        events.push(output_event);
    }

    Ok(())
}

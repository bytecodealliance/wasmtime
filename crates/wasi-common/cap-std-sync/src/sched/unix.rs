use cap_std::time::Duration;
use io_lifetimes::{AsFd, BorrowedFd};
use rsix::io::{PollFd, PollFlags};
use std::convert::TryInto;
use wasi_common::{
    file::WasiFile,
    sched::{
        subscription::{RwEventFlags, Subscription},
        Poll,
    },
    Error, ErrorExt,
};

pub async fn poll_oneoff<'a>(poll: &mut Poll<'a>) -> Result<(), Error> {
    if poll.is_empty() {
        return Ok(());
    }
    let mut pollfds = Vec::new();
    for s in poll.rw_subscriptions() {
        match s {
            Subscription::Read(f) => {
                let fd = wasi_file_fd(f.file).ok_or(
                    Error::invalid_argument().context("read subscription fd downcast failed"),
                )?;
                pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::IN));
            }

            Subscription::Write(f) => {
                let fd = wasi_file_fd(f.file).ok_or(
                    Error::invalid_argument().context("write subscription fd downcast failed"),
                )?;
                pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::OUT));
            }
            Subscription::MonotonicClock { .. } => unreachable!(),
        }
    }

    let ready = loop {
        let poll_timeout = if let Some(t) = poll.earliest_clock_deadline() {
            let duration = t.duration_until().unwrap_or(Duration::from_secs(0));
            (duration.as_millis() + 1) // XXX try always rounding up?
                .try_into()
                .map_err(|_| Error::overflow().context("poll timeout"))?
        } else {
            std::os::raw::c_int::max_value()
        };
        tracing::debug!(
            poll_timeout = tracing::field::debug(poll_timeout),
            poll_fds = tracing::field::debug(&pollfds),
            "poll"
        );
        match rsix::io::poll(&mut pollfds, poll_timeout) {
            Ok(ready) => break ready,
            Err(rsix::io::Error::INTR) => continue,
            Err(err) => return Err(err.into()),
        }
    };
    if ready > 0 {
        for (rwsub, pollfd) in poll.rw_subscriptions().zip(pollfds.into_iter()) {
            let revents = pollfd.revents();
            let (nbytes, rwsub) = match rwsub {
                Subscription::Read(sub) => {
                    let ready = sub.file.num_ready_bytes().await?;
                    (std::cmp::max(ready, 1), sub)
                }
                Subscription::Write(sub) => (0, sub),
                _ => unreachable!(),
            };
            if revents.contains(PollFlags::NVAL) {
                rwsub.error(Error::badf());
            } else if revents.contains(PollFlags::ERR) {
                rwsub.error(Error::io());
            } else if revents.contains(PollFlags::HUP) {
                rwsub.complete(nbytes, RwEventFlags::HANGUP);
            } else {
                rwsub.complete(nbytes, RwEventFlags::empty());
            };
        }
    } else {
        poll.earliest_clock_deadline()
            .expect("timed out")
            .result()
            .expect("timer deadline is past")
            .unwrap()
    }
    Ok(())
}

fn wasi_file_fd(f: &dyn WasiFile) -> Option<BorrowedFd<'_>> {
    let a = f.as_any();
    if a.is::<crate::file::File>() {
        Some(a.downcast_ref::<crate::file::File>().unwrap().as_fd())
    } else if a.is::<crate::stdio::Stdin>() {
        Some(a.downcast_ref::<crate::stdio::Stdin>().unwrap().as_fd())
    } else if a.is::<crate::stdio::Stdout>() {
        Some(a.downcast_ref::<crate::stdio::Stdout>().unwrap().as_fd())
    } else if a.is::<crate::stdio::Stderr>() {
        Some(a.downcast_ref::<crate::stdio::Stderr>().unwrap().as_fd())
    } else {
        None
    }
}

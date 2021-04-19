use anyhow::Context;
use std::ops::Deref;
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use wasi_common::{
    file::WasiFile,
    sched::{
        subscription::{RwEventFlags, Subscription},
        Poll,
    },
    Error, ErrorExt,
};
pub async fn poll_oneoff<'a>(poll: &'_ Poll<'a>) -> Result<(), Error> {
    if poll.is_empty() {
        return Ok(());
    }

    let mut ready = false;
    let timeout = poll.earliest_clock_deadline();

    let mut stdin_read_subs = Vec::new();
    let mut immediate_subs = Vec::new();
    for s in poll.rw_subscriptions() {
        match s {
            Subscription::Read(r) if r.file.as_any().is::<crate::stdio::Stdin>() => {
                stdin_read_subs.push(r);
            }
            Subscription::Read(rw) | Subscription::Write(rw) => {
                if wasi_file_raw_handle(rw.file.deref()).is_some() {
                    immediate_subs.push(s);
                } else {
                    return Err(Error::invalid_argument()
                        .context("read/write subscription fd downcast failed"));
                }
            }
            Subscription::MonotonicClock { .. } => unreachable!(),
        }
    }

    if !stdin_read_subs.is_empty() {
        let waitmode = if let Some(t) = timeout {
            if let Some(duration) = t.duration_until() {
                WaitMode::Timeout(duration)
            } else {
                WaitMode::Immediate
            }
        } else {
            if ready {
                WaitMode::Immediate
            } else {
                WaitMode::Infinite
            }
        };
        let state = STDIN_POLL
            .lock()
            .map_err(|_| Error::trap("failed to take lock of STDIN_POLL"))?
            .poll(waitmode)?;
        for readsub in stdin_read_subs.into_iter() {
            match state {
                PollState::Ready => {
                    readsub.complete(1, RwEventFlags::empty());
                    ready = true;
                }
                PollState::NotReady | PollState::TimedOut => {}
                PollState::Error(ref e) => {
                    // Unfortunately, we need to deliver the Error to each of the
                    // subscriptions, but there is no Clone on std::io::Error. So, we convert it to the
                    // kind, and then back to std::io::Error, and finally to anyhow::Error.
                    // When its time to turn this into an errno elsewhere, the error kind will
                    // be inspected.
                    let ekind = e.kind();
                    let ioerror = std::io::Error::from(ekind);
                    readsub.error(ioerror.into());
                    ready = true;
                }
            }
        }
    }
    for sub in immediate_subs {
        match sub {
            Subscription::Read(r) => {
                // XXX This doesnt strictly preserve the behavior in the earlier
                // implementation, which would always do complete(0) for reads from
                // stdout/err.
                match r.file.num_ready_bytes().await {
                    Ok(ready_bytes) => {
                        r.complete(ready_bytes, RwEventFlags::empty());
                        ready = true;
                    }
                    Err(e) => {
                        r.error(e);
                        ready = true;
                    }
                }
            }
            Subscription::Write(w) => {
                // Everything is always ready for writing, apparently?
                w.complete(0, RwEventFlags::empty());
                ready = true;
            }
            Subscription::MonotonicClock { .. } => unreachable!(),
        }
    }

    if !ready {
        if let Some(t) = timeout {
            if let Some(duration) = t.duration_until() {
                thread::sleep(duration);
            }
        }
    }

    Ok(())
}

fn wasi_file_raw_handle(f: &dyn WasiFile) -> Option<RawHandle> {
    let a = f.as_any();
    if a.is::<crate::file::File>() {
        Some(
            a.downcast_ref::<crate::file::File>()
                .unwrap()
                .as_raw_handle(),
        )
    } else if a.is::<crate::stdio::Stdin>() {
        Some(
            a.downcast_ref::<crate::stdio::Stdin>()
                .unwrap()
                .as_raw_handle(),
        )
    } else if a.is::<crate::stdio::Stdout>() {
        Some(
            a.downcast_ref::<crate::stdio::Stdout>()
                .unwrap()
                .as_raw_handle(),
        )
    } else if a.is::<crate::stdio::Stderr>() {
        Some(
            a.downcast_ref::<crate::stdio::Stderr>()
                .unwrap()
                .as_raw_handle(),
        )
    } else {
        None
    }
}

enum PollState {
    Ready,
    NotReady, // Not ready, but did not wait
    TimedOut, // Not ready, waited until timeout
    Error(std::io::Error),
}

enum WaitMode {
    Timeout(Duration),
    Infinite,
    Immediate,
}

struct StdinPoll {
    request_tx: Sender<()>,
    notify_rx: Receiver<PollState>,
}

lazy_static::lazy_static! {
    static ref STDIN_POLL: Mutex<StdinPoll> = StdinPoll::new();
}

impl StdinPoll {
    pub fn new() -> Mutex<Self> {
        let (request_tx, request_rx) = mpsc::channel();
        let (notify_tx, notify_rx) = mpsc::channel();
        thread::spawn(move || Self::event_loop(request_rx, notify_tx));
        Mutex::new(StdinPoll {
            request_tx,
            notify_rx,
        })
    }

    // This function should not be used directly.
    // Correctness of this function crucially depends on the fact that
    // mpsc::Receiver is !Sync.
    fn poll(&self, wait_mode: WaitMode) -> Result<PollState, Error> {
        match self.notify_rx.try_recv() {
            // Clean up possibly unread result from previous poll.
            Ok(_) | Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                return Err(Error::trap("StdinPoll notify_rx channel closed"))
            }
        }

        // Notify the worker thread to poll stdin
        self.request_tx
            .send(())
            .context("request_tx channel closed")?;

        // Wait for the worker thread to send a readiness notification
        match wait_mode {
            WaitMode::Timeout(timeout) => match self.notify_rx.recv_timeout(timeout) {
                Ok(r) => Ok(r),
                Err(RecvTimeoutError::Timeout) => Ok(PollState::TimedOut),
                Err(RecvTimeoutError::Disconnected) => {
                    Err(Error::trap("StdinPoll notify_rx channel closed"))
                }
            },
            WaitMode::Infinite => self
                .notify_rx
                .recv()
                .context("StdinPoll notify_rx channel closed"),
            WaitMode::Immediate => match self.notify_rx.try_recv() {
                Ok(r) => Ok(r),
                Err(TryRecvError::Empty) => Ok(PollState::NotReady),
                Err(TryRecvError::Disconnected) => {
                    Err(Error::trap("StdinPoll notify_rx channel closed"))
                }
            },
        }
    }

    fn event_loop(request_rx: Receiver<()>, notify_tx: Sender<PollState>) -> ! {
        use std::io::BufRead;
        loop {
            // Wait on a request:
            request_rx.recv().expect("request_rx channel");
            // Wait for data to appear in stdin. If fill_buf returns any slice, it means
            // that either:
            // (a) there is some data in stdin, if non-empty,
            // (b) EOF was recieved, if its empty
            // Linux returns `POLLIN` in both cases, so we imitate this behavior.
            let resp = match std::io::stdin().lock().fill_buf() {
                Ok(_) => PollState::Ready,
                Err(e) => PollState::Error(e),
            };
            // Notify about data in stdin. If the read on this channel has timed out, the
            // next poller will have to clean the channel.
            notify_tx.send(resp).expect("notify_tx channel");
        }
    }
}

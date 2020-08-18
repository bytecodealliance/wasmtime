use crate::handle::Handle;
use crate::poll::{ClockEventData, FdEventData};
use crate::sys::osdir::OsDir;
use crate::sys::osfile::OsFile;
use crate::sys::osother::OsOther;
use crate::sys::stdio::{Stderr, Stdin, Stdout};
use crate::sys::AsFile;
use crate::wasi::{types, Errno, Result};
use lazy_static::lazy_static;
use std::convert::TryInto;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, trace, warn};

struct StdinPoll {
    request_tx: Sender<()>,
    notify_rx: Receiver<PollState>,
}

enum PollState {
    Ready,
    NotReady, // it's not ready, but we didn't wait
    TimedOut, // it's not ready and a timeout has occurred
    Error(Errno),
}

enum WaitMode {
    Timeout(Duration),
    Infinite,
    Immediate,
}

impl StdinPoll {
    // This function should not be used directly
    // Correctness of this function crucially depends on the fact that
    // mpsc::Receiver is !Sync.
    fn poll(&self, wait_mode: WaitMode) -> PollState {
        // Clean up possible unread result from the previous poll
        match self.notify_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("notify_rx channel closed"),
        }

        // Notify the worker thread that we want to poll stdin
        self.request_tx.send(()).expect("request_tx channel closed");

        // Wait for the worker thread to send a readiness notification
        let pollret = match wait_mode {
            WaitMode::Timeout(timeout) => {
                self.notify_rx
                    .recv_timeout(timeout)
                    .unwrap_or_else(|e| match e {
                        RecvTimeoutError::Disconnected => panic!("notify_rx channel closed"),
                        RecvTimeoutError::Timeout => PollState::TimedOut,
                    })
            }
            WaitMode::Infinite => self.notify_rx.recv().expect("notify_rx channel closed"),
            WaitMode::Immediate => self.notify_rx.try_recv().unwrap_or_else(|e| match e {
                TryRecvError::Disconnected => panic!("notify_rx channel closed"),
                TryRecvError::Empty => PollState::NotReady,
            }),
        };

        pollret
    }

    fn event_loop(request_rx: Receiver<()>, notify_tx: Sender<PollState>) -> ! {
        use std::io::BufRead;
        loop {
            // Wait for the request to poll stdin
            request_rx.recv().expect("request_rx channel closed");

            // Wait for data to appear in stdin.
            // If `fill_buf` returns any slice, then it means that either
            // (a) there some data in stdin, if it's non-empty
            // (b) EOF was received, if it's empty
            // Linux returns `POLLIN` in both cases, and we imitate this behavior.
            let resp = match std::io::stdin().lock().fill_buf() {
                Ok(_) => PollState::Ready,
                Err(e) => PollState::Error(Errno::from(e)),
            };

            // Notify the requestor about data in stdin. They may have already timed out,
            // then the next requestor will have to clean the channel.
            notify_tx.send(resp).expect("notify_tx channel closed");
        }
    }
}

lazy_static! {
    static ref STDIN_POLL: Mutex<StdinPoll> = {
        let (request_tx, request_rx) = mpsc::channel();
        let (notify_tx, notify_rx) = mpsc::channel();
        thread::spawn(move || StdinPoll::event_loop(request_rx, notify_tx));
        Mutex::new(StdinPoll {
            request_tx,
            notify_rx,
        })
    };
}

fn make_rw_event(event: &FdEventData, nbytes: Result<u64>) -> types::Event {
    let (nbytes, error) = match nbytes {
        Ok(nbytes) => (nbytes, Errno::Success),
        Err(e) => (u64::default(), e),
    };
    types::Event {
        userdata: event.userdata,
        type_: event.r#type,
        error,
        fd_readwrite: types::EventFdReadwrite {
            nbytes,
            flags: types::Eventrwflags::empty(),
        },
    }
}

fn make_timeout_event(timeout: &ClockEventData) -> types::Event {
    types::Event {
        userdata: timeout.userdata,
        type_: types::Eventtype::Clock,
        error: Errno::Success,
        fd_readwrite: types::EventFdReadwrite {
            nbytes: 0,
            flags: types::Eventrwflags::empty(),
        },
    }
}

fn handle_timeout(
    timeout_event: ClockEventData,
    timeout: Duration,
    events: &mut Vec<types::Event>,
) {
    thread::sleep(timeout);
    handle_timeout_event(timeout_event, events);
}

fn handle_timeout_event(timeout_event: ClockEventData, events: &mut Vec<types::Event>) {
    let new_event = make_timeout_event(&timeout_event);
    events.push(new_event);
}

fn handle_rw_event(event: FdEventData, out_events: &mut Vec<types::Event>) {
    let handle = &event.handle;
    let size = if let Some(_) = handle.as_any().downcast_ref::<Stdin>() {
        // We return the only universally correct lower bound, see the comment later in the function.
        Ok(1)
    } else if let Some(_) = handle.as_any().downcast_ref::<Stdout>() {
        // On Unix, ioctl(FIONREAD) will return 0 for stdout. Emulate the same behavior on Windows.
        Ok(0)
    } else if let Some(_) = handle.as_any().downcast_ref::<Stderr>() {
        // On Unix, ioctl(FIONREAD) will return 0 for stdout/stderr. Emulate the same behavior on Windows.
        Ok(0)
    } else {
        if event.r#type == types::Eventtype::FdRead {
            handle
                .as_file()
                .and_then(|f| f.metadata())
                .map(|m| m.len())
                .map_err(Into::into)
        } else {
            // The spec is unclear what nbytes should actually be for __WASI_EVENTTYPE_FD_WRITE and
            // the implementation on Unix just returns 0 here, so it's probably fine
            // to do the same on Windows for now.
            // cf. https://github.com/WebAssembly/WASI/issues/148
            Ok(0)
        }
    };
    let new_event = make_rw_event(&event, size);
    out_events.push(new_event);
}

fn handle_error_event(event: FdEventData, error: Errno, out_events: &mut Vec<types::Event>) {
    let new_event = make_rw_event(&event, Err(error));
    out_events.push(new_event);
}

pub(crate) fn oneoff(
    timeout: Option<ClockEventData>,
    fd_events: Vec<FdEventData>,
    events: &mut Vec<types::Event>,
) -> Result<()> {
    let timeout = timeout
        .map(|event| {
            event
                .delay
                .try_into()
                .map(Duration::from_nanos)
                .map(|dur| (event, dur))
        })
        .transpose()?;

    // With no events to listen, poll_oneoff just becomes a sleep.
    if fd_events.is_empty() {
        match timeout {
            Some((event, dur)) => return Ok(handle_timeout(event, dur, events)),
            // The implementation has to return Ok(()) in this case,
            // cf. the comment in src/hostcalls_impl/misc.rs
            None => return Ok(()),
        }
    }

    let mut stdin_events = vec![];
    let mut immediate_events = vec![];
    let mut pipe_events = vec![];

    for event in fd_events {
        let handle = &event.handle;
        if let Some(_) = handle.as_any().downcast_ref::<OsFile>() {
            immediate_events.push(event);
        } else if let Some(_) = handle.as_any().downcast_ref::<OsDir>() {
            immediate_events.push(event);
        } else if let Some(_) = handle.as_any().downcast_ref::<Stdin>() {
            stdin_events.push(event);
        } else if let Some(_) = handle.as_any().downcast_ref::<Stdout>() {
            // stdout are always considered ready to write because there seems to
            // be no way of checking if a write to stdout would block.
            //
            // If stdin is polled for anything else then reading, then it is also
            // considered immediately ready, following the behavior on Linux.
            immediate_events.push(event);
        } else if let Some(_) = handle.as_any().downcast_ref::<Stderr>() {
            // stderr are always considered ready to write because there seems to
            // be no way of checking if a write to stdout would block.
            //
            // If stdin is polled for anything else then reading, then it is also
            // considered immediately ready, following the behavior on Linux.
            immediate_events.push(event);
        } else if let Some(other) = handle.as_any().downcast_ref::<OsOther>() {
            if other.get_file_type() == types::Filetype::SocketStream {
                // We map pipe to SocketStream
                pipe_events.push(event);
            } else {
                debug!(
                    "poll_oneoff: unsupported file type: {}",
                    other.get_file_type()
                );
                handle_error_event(event, Errno::Notsup, events);
            }
        } else {
            tracing::error!("can poll FdEvent for OS resources only");
            return Err(Errno::Badf);
        }
    }

    let immediate = !immediate_events.is_empty();
    // Process all the events that do not require waiting.
    if immediate {
        trace!("    | have immediate events, will return immediately");
        for event in immediate_events {
            handle_rw_event(event, events);
        }
    }
    if !stdin_events.is_empty() {
        // waiting for data to arrive on stdin. This thread will not terminate.
        //
        // We'd like to do the following:
        // (1) wait in a non-blocking way for data to be available in stdin, with timeout
        // (2) find out, how many bytes are there available to be read.
        //
        // One issue is that we are currently relying on the Rust libstd for interaction
        // with stdin. More precisely, `io::stdin` is used via the `BufRead` trait,
        // in the `fd_read` function, which always does buffering on the libstd side. [1]
        // This means that even if there's still some unread data in stdin,
        // the lower-level Windows system calls may return false negatives,
        // claiming that stdin is empty.
        //
        // Theoretically, one could use `WaitForSingleObject` on the stdin handle
        // to achieve (1). Unfortunately, this function doesn't seem to honor the
        // requested timeout and to misbehaves after the stdin is closed.
        //
        // There appears to be no way of achieving (2) on Windows.
        // [1]: https://github.com/rust-lang/rust/pull/12422
        let waitmode = if immediate {
            trace!("     | tentatively checking stdin");
            WaitMode::Immediate
        } else {
            trace!("     | passively waiting on stdin");
            match timeout {
                Some((_event, dur)) => WaitMode::Timeout(dur),
                None => WaitMode::Infinite,
            }
        };
        let state = STDIN_POLL.lock().unwrap().poll(waitmode);
        for event in stdin_events {
            match state {
                PollState::Ready => handle_rw_event(event, events),
                PollState::NotReady => {} // not immediately available, so just ignore
                PollState::TimedOut => handle_timeout_event(timeout.unwrap().0, events),
                PollState::Error(e) => handle_error_event(event, e, events),
            }
        }
    }

    if !immediate && !pipe_events.is_empty() {
        trace!("     | actively polling pipes");
        match timeout {
            Some((event, dur)) => {
                // In the tests stdin is replaced with a dummy pipe, so for now
                // we just time out. Support for pipes will be decided later on.
                warn!("Polling pipes not supported on Windows, will just time out.");
                handle_timeout(event, dur, events);
            }
            None => {
                error!("Polling only pipes with no timeout not supported on Windows.");
                return Err(Errno::Notsup);
            }
        }
    }

    Ok(())
}

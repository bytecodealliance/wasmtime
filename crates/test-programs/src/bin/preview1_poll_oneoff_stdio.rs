#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::collections::HashMap;
use std::mem::MaybeUninit;
use test_programs::preview1::{STDERR_FD, STDIN_FD, STDOUT_FD, assert_errno};

const TIMEOUT: u64 = 200_000_000u64; // 200 milliseconds, required to satisfy slow execution in CI
const CLOCK_ID: wasip1::Userdata = 0x0123_45678;
const STDIN_ID: wasip1::Userdata = 0x8765_43210;

unsafe fn poll_oneoff_impl(
    r#in: &[wasip1::Subscription],
) -> Result<Vec<wasip1::Event>, wasip1::Errno> {
    let mut out: Vec<wasip1::Event> = Vec::new();
    out.resize_with(r#in.len(), || {
        MaybeUninit::<wasip1::Event>::zeroed().assume_init()
    });
    let size = wasip1::poll_oneoff(r#in.as_ptr(), out.as_mut_ptr(), r#in.len())?;
    out.truncate(size);
    Ok(out)
}

unsafe fn test_stdin_read() {
    let clock = wasip1::SubscriptionClock {
        id: wasip1::CLOCKID_MONOTONIC,
        timeout: TIMEOUT,
        precision: 0,
        flags: 0,
    };
    let fd_readwrite = wasip1::SubscriptionFdReadwrite {
        file_descriptor: STDIN_FD,
    };
    // Either stdin can be ready for reading, or this poll can timeout.
    let r#in = [
        wasip1::Subscription {
            userdata: CLOCK_ID,
            u: wasip1::SubscriptionU {
                tag: wasip1::EVENTTYPE_CLOCK.raw(),
                u: wasip1::SubscriptionUU { clock },
            },
        },
        wasip1::Subscription {
            userdata: STDIN_ID,
            u: wasip1::SubscriptionU {
                tag: wasip1::EVENTTYPE_FD_READ.raw(),
                u: wasip1::SubscriptionUU {
                    fd_read: fd_readwrite,
                },
            },
        },
    ];
    let out = poll_oneoff_impl(&r#in).unwrap();
    // The result should be either a timeout, or that stdin is ready for reading.
    // Both are valid behaviors that depend on the test environment.
    assert!(out.len() >= 1, "stdin read should return at least 1 event");
    for event in out {
        if event.type_ == wasip1::EVENTTYPE_CLOCK {
            assert_errno!(event.error, wasip1::ERRNO_SUCCESS);
            assert_eq!(
                event.userdata, CLOCK_ID,
                "the event.userdata should contain CLOCK_ID",
            );
        } else if event.type_ == wasip1::EVENTTYPE_FD_READ {
            assert_errno!(event.error, wasip1::ERRNO_SUCCESS);
            assert_eq!(
                event.userdata, STDIN_ID,
                "the event.userdata should contain STDIN_ID",
            );
        } else {
            panic!("unexpected event type {}", event.type_.raw());
        }
    }
}

fn writable_subs(h: &HashMap<u64, wasip1::Fd>) -> Vec<wasip1::Subscription> {
    h.iter()
        .map(|(ud, fd)| wasip1::Subscription {
            userdata: *ud,
            u: wasip1::SubscriptionU {
                tag: wasip1::EVENTTYPE_FD_WRITE.raw(),
                u: wasip1::SubscriptionUU {
                    fd_write: wasip1::SubscriptionFdReadwrite {
                        file_descriptor: *fd,
                    },
                },
            },
        })
        .collect()
}

unsafe fn test_stdout_stderr_write() {
    let mut writable: HashMap<u64, wasip1::Fd> =
        [(1, STDOUT_FD), (2, STDERR_FD)].into_iter().collect();

    let clock = wasip1::Subscription {
        userdata: CLOCK_ID,
        u: wasip1::SubscriptionU {
            tag: wasip1::EVENTTYPE_CLOCK.raw(),
            u: wasip1::SubscriptionUU {
                clock: wasip1::SubscriptionClock {
                    id: wasip1::CLOCKID_MONOTONIC,
                    timeout: TIMEOUT,
                    precision: 0,
                    flags: 0,
                },
            },
        },
    };
    let mut timed_out = false;
    while !writable.is_empty() {
        if timed_out {
            panic!("timed out with the following pending subs: {writable:?}")
        }
        let mut subs = writable_subs(&writable);
        subs.push(clock);
        let out = poll_oneoff_impl(&subs).unwrap();
        for event in out {
            match event.userdata {
                CLOCK_ID => timed_out = true,
                ud => {
                    if let Some(_) = writable.remove(&ud) {
                        assert_eq!(event.type_, wasip1::EVENTTYPE_FD_WRITE);
                        assert_errno!(event.error, wasip1::ERRNO_SUCCESS);
                    } else {
                        panic!("Unknown userdata {ud}, pending sub: {writable:?}")
                    }
                }
            }
        }
    }
}

unsafe fn test_poll_oneoff() {
    // NB we assume that stdin/stdout/stderr are valid and open
    // for the duration of the test case
    test_stdin_read();
    test_stdout_stderr_write();
}
fn main() {
    // Run the tests.
    unsafe { test_poll_oneoff() }
}

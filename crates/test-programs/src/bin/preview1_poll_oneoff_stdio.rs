use std::collections::HashMap;
use std::mem::MaybeUninit;
use test_programs::preview1::{assert_errno, STDERR_FD, STDIN_FD, STDOUT_FD};

const TIMEOUT: u64 = 200_000_000u64; // 200 milliseconds, required to satisfy slow execution in CI
const CLOCK_ID: wasi::Userdata = 0x0123_45678;
const STDIN_ID: wasi::Userdata = 0x8765_43210;

unsafe fn poll_oneoff_impl(r#in: &[wasi::Subscription]) -> Result<Vec<wasi::Event>, wasi::Errno> {
    let mut out: Vec<wasi::Event> = Vec::new();
    out.resize_with(r#in.len(), || {
        MaybeUninit::<wasi::Event>::zeroed().assume_init()
    });
    let size = wasi::poll_oneoff(r#in.as_ptr(), out.as_mut_ptr(), r#in.len())?;
    out.truncate(size);
    Ok(out)
}

unsafe fn test_stdin_read() {
    let clock = wasi::SubscriptionClock {
        id: wasi::CLOCKID_MONOTONIC,
        timeout: TIMEOUT,
        precision: 0,
        flags: 0,
    };
    let fd_readwrite = wasi::SubscriptionFdReadwrite {
        file_descriptor: STDIN_FD,
    };
    // Either stdin can be ready for reading, or this poll can timeout.
    let r#in = [
        wasi::Subscription {
            userdata: CLOCK_ID,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_CLOCK.raw(),
                u: wasi::SubscriptionUU { clock },
            },
        },
        wasi::Subscription {
            userdata: STDIN_ID,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_READ.raw(),
                u: wasi::SubscriptionUU {
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
        if event.type_ == wasi::EVENTTYPE_CLOCK {
            assert_errno!(event.error, wasi::ERRNO_SUCCESS);
            assert_eq!(
                event.userdata, CLOCK_ID,
                "the event.userdata should contain CLOCK_ID",
            );
        } else if event.type_ == wasi::EVENTTYPE_FD_READ {
            assert_errno!(event.error, wasi::ERRNO_SUCCESS);
            assert_eq!(
                event.userdata, STDIN_ID,
                "the event.userdata should contain STDIN_ID",
            );
        } else {
            panic!("unexpected event type {}", event.type_.raw());
        }
    }
}

fn writable_subs(h: &HashMap<u64, wasi::Fd>) -> Vec<wasi::Subscription> {
    h.iter()
        .map(|(ud, fd)| wasi::Subscription {
            userdata: *ud,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_WRITE.raw(),
                u: wasi::SubscriptionUU {
                    fd_write: wasi::SubscriptionFdReadwrite {
                        file_descriptor: *fd,
                    },
                },
            },
        })
        .collect()
}

unsafe fn test_stdout_stderr_write() {
    let mut writable: HashMap<u64, wasi::Fd> =
        [(1, STDOUT_FD), (2, STDERR_FD)].into_iter().collect();

    let clock = wasi::Subscription {
        userdata: CLOCK_ID,
        u: wasi::SubscriptionU {
            tag: wasi::EVENTTYPE_CLOCK.raw(),
            u: wasi::SubscriptionUU {
                clock: wasi::SubscriptionClock {
                    id: wasi::CLOCKID_MONOTONIC,
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
                        assert_eq!(event.type_, wasi::EVENTTYPE_FD_WRITE);
                        assert_errno!(event.error, wasi::ERRNO_SUCCESS);
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

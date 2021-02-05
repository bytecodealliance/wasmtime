use std::mem::MaybeUninit;
use wasi_tests::{assert_errno, STDERR_FD, STDIN_FD, STDOUT_FD};

const CLOCK_ID: wasi::Userdata = 0x0123_45678;
const STDIN_ID: wasi::Userdata = 0x8765_43210;

unsafe fn poll_oneoff_impl(r#in: &[wasi::Subscription]) -> Result<Vec<wasi::Event>, wasi::Error> {
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
        timeout: 5_000_000u64, // 5 milliseconds
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
                tag: wasi::EVENTTYPE_CLOCK,
                u: wasi::SubscriptionUU { clock },
            },
        },
        wasi::Subscription {
            userdata: STDIN_ID,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_READ,
                u: wasi::SubscriptionUU {
                    fd_read: fd_readwrite,
                },
            },
        },
    ];
    let out = poll_oneoff_impl(&r#in).unwrap();
    // The result should be either a timeout, or that stdin is ready for reading.
    // Both are valid behaviors that depend on the test environment.
    assert_eq!(out.len(), 1, "should return 1 event");
    let event = &out[0];
    if event.r#type == wasi::EVENTTYPE_CLOCK {
        assert_errno!(event.error, wasi::ERRNO_SUCCESS);
        assert_eq!(
            event.userdata, CLOCK_ID,
            "the event.userdata should contain CLOCK_ID",
        );
    } else if event.r#type == wasi::EVENTTYPE_FD_READ {
        assert_errno!(event.error, wasi::ERRNO_SUCCESS);
        assert_eq!(
            event.userdata, STDIN_ID,
            "the event.userdata should contain STDIN_ID",
        );
    } else {
        panic!("unexpected event type {}", event.r#type);
    }
}

unsafe fn test_stdout_stderr_write() {
    let stdout_readwrite = wasi::SubscriptionFdReadwrite {
        file_descriptor: STDOUT_FD,
    };
    let stderr_readwrite = wasi::SubscriptionFdReadwrite {
        file_descriptor: STDERR_FD,
    };
    let r#in = [
        wasi::Subscription {
            userdata: 1,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_WRITE,
                u: wasi::SubscriptionUU {
                    fd_write: stdout_readwrite,
                },
            },
        },
        wasi::Subscription {
            userdata: 2,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_WRITE,
                u: wasi::SubscriptionUU {
                    fd_write: stderr_readwrite,
                },
            },
        },
    ];
    let out = poll_oneoff_impl(&r#in).unwrap();
    assert_eq!(out.len(), 2, "should return 2 events");
    assert_eq!(
        out[0].userdata, 1,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_errno!(out[0].error, wasi::ERRNO_SUCCESS);
    assert_eq!(
        out[0].r#type,
        wasi::EVENTTYPE_FD_WRITE,
        "the event.type should equal FD_WRITE"
    );
    assert_eq!(
        out[1].userdata, 2,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_errno!(out[1].error, wasi::ERRNO_SUCCESS);
    assert_eq!(
        out[1].r#type,
        wasi::EVENTTYPE_FD_WRITE,
        "the event.type should equal FD_WRITE"
    );
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

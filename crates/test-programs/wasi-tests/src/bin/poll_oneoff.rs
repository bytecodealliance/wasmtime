use more_asserts::assert_gt;
use std::{env, mem::MaybeUninit, process};
use wasi_tests::{assert_errno, open_scratch_directory};

const CLOCK_ID: wasi::Userdata = 0x0123_45678;

unsafe fn poll_oneoff_impl(r#in: &[wasi::Subscription]) -> Result<Vec<wasi::Event>, wasi::Error> {
    let mut out: Vec<wasi::Event> = Vec::new();
    out.resize_with(r#in.len(), || {
        MaybeUninit::<wasi::Event>::zeroed().assume_init()
    });
    let size = wasi::poll_oneoff(r#in.as_ptr(), out.as_mut_ptr(), r#in.len())?;
    out.truncate(size);
    Ok(out)
}

unsafe fn test_empty_poll() {
    let r#in = [];
    let mut out: Vec<wasi::Event> = Vec::new();
    assert_errno!(
        wasi::poll_oneoff(r#in.as_ptr(), out.as_mut_ptr(), r#in.len())
            .expect_err("empty poll_oneoff should fail")
            .raw_error(),
        wasi::ERRNO_INVAL
    );
}

unsafe fn test_timeout() {
    let timeout = 5_000_000u64; // 5 milliseconds
    let clock = wasi::SubscriptionClock {
        id: wasi::CLOCKID_MONOTONIC,
        timeout,
        precision: 0,
        flags: 0,
    };
    let r#in = [wasi::Subscription {
        userdata: CLOCK_ID,
        u: wasi::SubscriptionU {
            tag: wasi::EVENTTYPE_CLOCK,
            u: wasi::SubscriptionUU { clock },
        },
    }];
    let before = wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 0).unwrap();
    let out = poll_oneoff_impl(&r#in).unwrap();
    let after = wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 0).unwrap();
    assert_eq!(out.len(), 1, "should return 1 event");
    let event = &out[0];
    assert_errno!(event.error, wasi::ERRNO_SUCCESS);
    assert_eq!(
        event.r#type,
        wasi::EVENTTYPE_CLOCK,
        "the event.type should equal clock"
    );
    assert_eq!(
        event.userdata, CLOCK_ID,
        "the event.userdata should contain clock_id specified by the user"
    );
    assert!(after - before >= timeout, "poll_oneoff should sleep for the specified interval");
}

// Like test_timeout, but uses `CLOCKID_REALTIME`, as WASI libc's sleep
// functions do.
unsafe fn test_sleep() {
    let timeout = 5_000_000u64; // 5 milliseconds
    let clock = wasi::SubscriptionClock {
        id: wasi::CLOCKID_REALTIME,
        timeout,
        precision: 0,
        flags: 0,
    };
    let r#in = [wasi::Subscription {
        userdata: CLOCK_ID,
        u: wasi::SubscriptionU {
            tag: wasi::EVENTTYPE_CLOCK,
            u: wasi::SubscriptionUU { clock },
        },
    }];
    let before = wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 0).unwrap();
    let out = poll_oneoff_impl(&r#in).unwrap();
    let after = wasi::clock_time_get(wasi::CLOCKID_MONOTONIC, 0).unwrap();
    assert_eq!(out.len(), 1, "should return 1 event");
    let event = &out[0];
    assert_errno!(event.error, wasi::ERRNO_SUCCESS);
    assert_eq!(
        event.r#type,
        wasi::EVENTTYPE_CLOCK,
        "the event.type should equal clock"
    );
    assert_eq!(
        event.userdata, CLOCK_ID,
        "the event.userdata should contain clock_id specified by the user"
    );
    assert!(after - before >= timeout, "poll_oneoff should sleep for the specified interval");
}

unsafe fn test_fd_readwrite(fd: wasi::Fd, error_code: wasi::Errno) {
    let fd_readwrite = wasi::SubscriptionFdReadwrite {
        file_descriptor: fd,
    };
    let r#in = [
        wasi::Subscription {
            userdata: 1,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_READ,
                u: wasi::SubscriptionUU {
                    fd_read: fd_readwrite,
                },
            },
        },
        wasi::Subscription {
            userdata: 2,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_WRITE,
                u: wasi::SubscriptionUU {
                    fd_write: fd_readwrite,
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
    assert_errno!(out[0].error, error_code);
    assert_eq!(
        out[0].r#type,
        wasi::EVENTTYPE_FD_READ,
        "the event.type_ should equal FD_READ"
    );
    assert_eq!(
        out[1].userdata, 2,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_errno!(out[1].error, error_code);
    assert_eq!(
        out[1].r#type,
        wasi::EVENTTYPE_FD_WRITE,
        "the event.type_ should equal FD_WRITE"
    );
}

unsafe fn test_fd_readwrite_valid_fd(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE | wasi::RIGHTS_POLL_FD_READWRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    test_fd_readwrite(file_fd, wasi::ERRNO_SUCCESS);

    wasi::fd_close(file_fd).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "file").expect("removing a file");
}

unsafe fn test_fd_readwrite_invalid_fd() {
    let fd_readwrite = wasi::SubscriptionFdReadwrite {
        file_descriptor: wasi::Fd::max_value(),
    };
    let r#in = [
        wasi::Subscription {
            userdata: 1,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_READ,
                u: wasi::SubscriptionUU {
                    fd_read: fd_readwrite,
                },
            },
        },
        wasi::Subscription {
            userdata: 2,
            u: wasi::SubscriptionU {
                tag: wasi::EVENTTYPE_FD_WRITE,
                u: wasi::SubscriptionUU {
                    fd_write: fd_readwrite,
                },
            },
        },
    ];
    let err = poll_oneoff_impl(&r#in).unwrap_err();
    assert_eq!(err.raw_error(), wasi::ERRNO_BADF)
}

unsafe fn test_poll_oneoff(dir_fd: wasi::Fd) {
    test_timeout();
    test_sleep();
    test_empty_poll();
    test_fd_readwrite_valid_fd(dir_fd);
    test_fd_readwrite_invalid_fd();
}
fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {} <scratch directory>", prog);
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_poll_oneoff(dir_fd) }
}

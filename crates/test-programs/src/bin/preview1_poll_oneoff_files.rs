use std::{env, mem::MaybeUninit, process};
use test_programs::preview1::{assert_errno, open_scratch_directory};

const CLOCK_ID: wasip1::Userdata = 0x0123_45678;

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

/// Repeatedly call `poll_oneoff` until all the subscriptions in `in` have
/// seen their events occur.
unsafe fn poll_oneoff_with_retry(
    r#in: &[wasip1::Subscription],
) -> Result<Vec<wasip1::Event>, wasip1::Errno> {
    let mut subscriptions = r#in.to_vec();
    let mut events = Vec::new();
    while !subscriptions.is_empty() {
        let mut out: Vec<wasip1::Event> = Vec::new();
        out.resize_with(subscriptions.len(), || {
            MaybeUninit::<wasip1::Event>::zeroed().assume_init()
        });
        let size = wasip1::poll_oneoff(
            subscriptions.as_ptr(),
            out.as_mut_ptr(),
            subscriptions.len(),
        )?;
        out.truncate(size);

        // Append the events from this `poll` to the result.
        events.extend_from_slice(&out);

        // Assuming userdata fields are unique, filter out any subscriptions
        // whose event has occurred.
        subscriptions.retain(|sub| !out.iter().any(|event| event.userdata == sub.userdata));
    }
    Ok(events)
}

unsafe fn test_empty_poll() {
    let r#in = [];
    let mut out: Vec<wasip1::Event> = Vec::new();
    assert_errno!(
        wasip1::poll_oneoff(r#in.as_ptr(), out.as_mut_ptr(), r#in.len())
            .expect_err("empty poll_oneoff should fail"),
        wasip1::ERRNO_INVAL
    );
}

unsafe fn test_timeout() {
    let timeout = 5_000_000u64; // 5 milliseconds
    let clock = wasip1::SubscriptionClock {
        id: wasip1::CLOCKID_MONOTONIC,
        timeout,
        precision: 0,
        flags: 0,
    };
    let r#in = [wasip1::Subscription {
        userdata: CLOCK_ID,
        u: wasip1::SubscriptionU {
            tag: wasip1::EVENTTYPE_CLOCK.raw(),
            u: wasip1::SubscriptionUU { clock },
        },
    }];
    let before = wasip1::clock_time_get(wasip1::CLOCKID_MONOTONIC, 0).unwrap();
    let out = poll_oneoff_impl(&r#in).unwrap();
    let after = wasip1::clock_time_get(wasip1::CLOCKID_MONOTONIC, 0).unwrap();
    assert_eq!(out.len(), 1, "should return 1 event");
    let event = &out[0];
    assert_errno!(event.error, wasip1::ERRNO_SUCCESS);
    assert_eq!(
        event.type_,
        wasip1::EVENTTYPE_CLOCK,
        "the event.type should equal clock"
    );
    assert_eq!(
        event.userdata, CLOCK_ID,
        "the event.userdata should contain clock_id specified by the user"
    );
    assert!(
        after - before >= timeout,
        "poll_oneoff should sleep for the specified interval of {timeout}. before: {before}, after: {after}"
    );
}

// Like test_timeout, but uses `CLOCKID_REALTIME`, as WASI libc's sleep
// functions do.
unsafe fn test_sleep() {
    let timeout = 5_000_000u64; // 5 milliseconds
    let clock = wasip1::SubscriptionClock {
        id: wasip1::CLOCKID_REALTIME,
        timeout,
        precision: 0,
        flags: 0,
    };
    let r#in = [wasip1::Subscription {
        userdata: CLOCK_ID,
        u: wasip1::SubscriptionU {
            tag: wasip1::EVENTTYPE_CLOCK.raw(),
            u: wasip1::SubscriptionUU { clock },
        },
    }];
    let before = wasip1::clock_time_get(wasip1::CLOCKID_MONOTONIC, 0).unwrap();
    let out = poll_oneoff_impl(&r#in).unwrap();
    let after = wasip1::clock_time_get(wasip1::CLOCKID_MONOTONIC, 0).unwrap();
    assert_eq!(out.len(), 1, "should return 1 event");
    let event = &out[0];
    assert_errno!(event.error, wasip1::ERRNO_SUCCESS);
    assert_eq!(
        event.type_,
        wasip1::EVENTTYPE_CLOCK,
        "the event.type should equal clock"
    );
    assert_eq!(
        event.userdata, CLOCK_ID,
        "the event.userdata should contain clock_id specified by the user"
    );
    assert!(
        after - before >= timeout,
        "poll_oneoff should sleep for the specified interval of {timeout}. before: {before}, after: {after}"
    );
}

unsafe fn test_fd_readwrite(
    readable_fd: wasip1::Fd,
    writable_fd: wasip1::Fd,
    error_code: wasip1::Errno,
) {
    let r#in = [
        wasip1::Subscription {
            userdata: 1,
            u: wasip1::SubscriptionU {
                tag: wasip1::EVENTTYPE_FD_READ.raw(),
                u: wasip1::SubscriptionUU {
                    fd_read: wasip1::SubscriptionFdReadwrite {
                        file_descriptor: readable_fd,
                    },
                },
            },
        },
        wasip1::Subscription {
            userdata: 2,
            u: wasip1::SubscriptionU {
                tag: wasip1::EVENTTYPE_FD_WRITE.raw(),
                u: wasip1::SubscriptionUU {
                    fd_write: wasip1::SubscriptionFdReadwrite {
                        file_descriptor: writable_fd,
                    },
                },
            },
        },
    ];
    let out = poll_oneoff_with_retry(&r#in).unwrap();
    assert_eq!(out.len(), 2, "should return 2 events, got: {out:?}");

    let (read, write) = if out[0].userdata == 1 {
        (&out[0], &out[1])
    } else {
        (&out[1], &out[0])
    };
    assert_eq!(
        read.userdata, 1,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_errno!(read.error, error_code);
    assert_eq!(
        read.type_,
        wasip1::EVENTTYPE_FD_READ,
        "the event.type_ should equal FD_READ"
    );
    assert_eq!(
        write.userdata, 2,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_errno!(write.error, error_code);
    assert_eq!(
        write.type_,
        wasip1::EVENTTYPE_FD_WRITE,
        "the event.type_ should equal FD_WRITE"
    );
}

unsafe fn test_fd_readwrite_valid_fd(dir_fd: wasip1::Fd) {
    // Create a file in the scratch directory.
    let nonempty_file = wasip1::path_open(
        dir_fd,
        0,
        "readable_file",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("create writable file");
    // Write to file
    let contents = &[1u8];
    let ciovec = wasip1::Ciovec {
        buf: contents.as_ptr() as *const _,
        buf_len: contents.len(),
    };
    wasip1::fd_write(nonempty_file, &[ciovec]).expect("write");
    wasip1::fd_close(nonempty_file).expect("close");

    // Now open the file for reading
    let readable_fd =
        wasip1::path_open(dir_fd, 0, "readable_file", 0, wasip1::RIGHTS_FD_READ, 0, 0)
            .expect("opening a readable file");

    assert!(
        readable_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
    // Create a file in the scratch directory.
    let writable_fd = wasip1::path_open(
        dir_fd,
        0,
        "writable_file",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a writable file");
    assert!(
        writable_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    test_fd_readwrite(readable_fd, writable_fd, wasip1::ERRNO_SUCCESS);

    wasip1::fd_close(readable_fd).expect("closing readable_file");
    wasip1::fd_close(writable_fd).expect("closing writable_file");
    wasip1::path_unlink_file(dir_fd, "readable_file").expect("removing readable_file");
    wasip1::path_unlink_file(dir_fd, "writable_file").expect("removing writable_file");
}

unsafe fn test_fd_readwrite_invalid_fd() {
    let fd_readwrite = wasip1::SubscriptionFdReadwrite {
        file_descriptor: wasip1::Fd::max_value(),
    };
    let r#in = [
        wasip1::Subscription {
            userdata: 1,
            u: wasip1::SubscriptionU {
                tag: wasip1::EVENTTYPE_FD_READ.raw(),
                u: wasip1::SubscriptionUU {
                    fd_read: fd_readwrite,
                },
            },
        },
        wasip1::Subscription {
            userdata: 2,
            u: wasip1::SubscriptionU {
                tag: wasip1::EVENTTYPE_FD_WRITE.raw(),
                u: wasip1::SubscriptionUU {
                    fd_write: fd_readwrite,
                },
            },
        },
    ];
    let err = poll_oneoff_impl(&r#in).unwrap_err();
    assert_eq!(err, wasip1::ERRNO_BADF)
}

unsafe fn test_poll_oneoff(dir_fd: wasip1::Fd) {
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
        eprintln!("usage: {prog} <scratch directory>");
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_poll_oneoff(dir_fd) }
}

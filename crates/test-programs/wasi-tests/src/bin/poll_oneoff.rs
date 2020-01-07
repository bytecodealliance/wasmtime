use more_asserts::assert_gt;
use std::{env, mem::MaybeUninit, process};
use wasi_tests::{open_scratch_directory, STDERR_FD, STDIN_FD, STDOUT_FD};

const CLOCK_ID: wasi::Userdata = 0x0123_45678;

unsafe fn poll_oneoff_impl(r#in: &[wasi::Subscription], nexpected: usize) -> Vec<wasi::Event> {
    let mut out: Vec<wasi::Event> = Vec::new();
    out.resize_with(r#in.len(), || {
        MaybeUninit::<wasi::Event>::zeroed().assume_init()
    });
    let size = wasi::poll_oneoff(r#in.as_ptr(), out.as_mut_ptr(), r#in.len())
        .expect("poll_oneoff should succeed");
    assert_eq!(
        size, nexpected,
        "poll_oneoff should return {} events",
        nexpected
    );
    out
}

unsafe fn test_timeout() {
    let clock = wasi::SubscriptionClock {
        id: wasi::CLOCKID_MONOTONIC,
        timeout: 5_000_000u64, // 5 milliseconds
        precision: 0,
        flags: 0,
    };
    let r#in = [wasi::Subscription {
        userdata: CLOCK_ID,
        r#type: wasi::EVENTTYPE_CLOCK,
        u: wasi::SubscriptionU { clock },
    }];
    let out = poll_oneoff_impl(&r#in, 1);
    let event = &out[0];
    assert_eq!(
        event.userdata, CLOCK_ID,
        "the event.userdata should contain clock_id specified by the user"
    );
    assert_eq!(
        event.error,
        wasi::ERRNO_SUCCESS,
        "the event.error should be set to ESUCCESS"
    );
    assert_eq!(
        event.r#type,
        wasi::EVENTTYPE_CLOCK,
        "the event.type should equal clock"
    );
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
    let r#in = [
        wasi::Subscription {
            userdata: CLOCK_ID,
            r#type: wasi::EVENTTYPE_CLOCK,
            u: wasi::SubscriptionU { clock },
        },
        wasi::Subscription {
            userdata: 1,
            r#type: wasi::EVENTTYPE_FD_READ,
            u: wasi::SubscriptionU { fd_readwrite },
        },
    ];
    let out = poll_oneoff_impl(&r#in, 1);
    let event = &out[0];
    assert_eq!(
        event.userdata, CLOCK_ID,
        "the event.userdata should contain clock_id specified by the user"
    );
    assert_eq!(
        event.error,
        wasi::ERRNO_SUCCESS,
        "the event.error should be set to ESUCCESS"
    );
    assert_eq!(
        event.r#type,
        wasi::EVENTTYPE_CLOCK,
        "the event.type should equal clock"
    );
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
            r#type: wasi::EVENTTYPE_FD_WRITE,
            u: wasi::SubscriptionU {
                fd_readwrite: stdout_readwrite,
            },
        },
        wasi::Subscription {
            userdata: 2,
            r#type: wasi::EVENTTYPE_FD_WRITE,
            u: wasi::SubscriptionU {
                fd_readwrite: stderr_readwrite,
            },
        },
    ];
    let out = poll_oneoff_impl(&r#in, 2);
    assert_eq!(
        out[0].userdata, 1,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_eq!(
        out[0].error,
        wasi::ERRNO_SUCCESS,
        "the event.error should be set to ERRNO_SUCCESS",
    );
    assert_eq!(
        out[0].r#type,
        wasi::EVENTTYPE_FD_WRITE,
        "the event.type should equal FD_WRITE"
    );
    assert_eq!(
        out[1].userdata, 2,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_eq!(
        out[1].error,
        wasi::ERRNO_SUCCESS,
        "the event.error should be set to ERRNO_SUCCESS",
    );
    assert_eq!(
        out[1].r#type,
        wasi::EVENTTYPE_FD_WRITE,
        "the event.type should equal FD_WRITE"
    );
}

unsafe fn test_fd_readwrite(fd: wasi::Fd, error_code: wasi::Errno) {
    let fd_readwrite = wasi::SubscriptionFdReadwrite {
        file_descriptor: fd,
    };
    let r#in = [
        wasi::Subscription {
            userdata: 1,
            r#type: wasi::EVENTTYPE_FD_READ,
            u: wasi::SubscriptionU { fd_readwrite },
        },
        wasi::Subscription {
            userdata: 2,
            r#type: wasi::EVENTTYPE_FD_WRITE,
            u: wasi::SubscriptionU { fd_readwrite },
        },
    ];
    let out = poll_oneoff_impl(&r#in, 2);
    assert_eq!(
        out[0].userdata, 1,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_eq!(
        out[0].error, error_code,
        "the event.error should be set to {}",
        error_code
    );
    assert_eq!(
        out[0].r#type,
        wasi::EVENTTYPE_FD_READ,
        "the event.type_ should equal FD_READ"
    );
    assert_eq!(
        out[1].userdata, 2,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_eq!(
        out[1].error, error_code,
        "the event.error should be set to {}",
        error_code
    );
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
    test_fd_readwrite(wasi::Fd::max_value(), wasi::ERRNO_BADF)
}

unsafe fn test_poll_oneoff(dir_fd: wasi::Fd) {
    test_timeout();
    // NB we assume that stdin/stdout/stderr are valid and open
    // for the duration of the test case
    test_stdin_read();
    test_stdout_stderr_write();
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

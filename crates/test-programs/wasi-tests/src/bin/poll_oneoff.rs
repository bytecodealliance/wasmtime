use more_asserts::assert_gt;
use std::{env, mem::MaybeUninit, process};
use wasi_old::wasi_unstable;
use wasi_tests::{
    open_scratch_directory,
    utils::{cleanup_file, close_fd},
    wasi_wrappers::wasi_path_open,
};

const CLOCK_ID: wasi_unstable::Userdata = 0x0123_45678;

unsafe fn poll_oneoff_impl(
    in_: &[wasi_unstable::Subscription],
    nexpected: usize,
) -> Vec<wasi_unstable::Event> {
    let mut out: Vec<wasi_unstable::Event> = Vec::new();
    out.resize_with(in_.len(), || {
        MaybeUninit::<wasi_unstable::Event>::zeroed().assume_init()
    });
    let res = wasi_unstable::poll_oneoff(&in_, out.as_mut_slice());
    let res = res.expect("poll_oneoff should succeed");
    assert_eq!(
        res, nexpected,
        "poll_oneoff should return {} events",
        nexpected
    );
    out
}

unsafe fn test_timeout() {
    let clock = wasi_unstable::raw::__wasi_subscription_u_clock_t {
        identifier: CLOCK_ID,
        clock_id: wasi_unstable::CLOCK_MONOTONIC,
        timeout: 5_000_000u64, // 5 milliseconds
        precision: 0,
        flags: 0,
    };
    let in_ = [wasi_unstable::Subscription {
        userdata: CLOCK_ID,
        type_: wasi_unstable::EVENTTYPE_CLOCK,
        u: wasi_unstable::raw::__wasi_subscription_u { clock },
    }];
    let out = poll_oneoff_impl(&in_, 1);
    let event = &out[0];
    assert_eq!(
        event.userdata, CLOCK_ID,
        "the event.userdata should contain clock_id specified by the user"
    );
    assert_eq!(
        event.error,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "the event.error should be set to ESUCCESS"
    );
    assert_eq!(
        event.type_,
        wasi_unstable::EVENTTYPE_CLOCK,
        "the event.type_ should equal clock"
    );
}

unsafe fn test_stdin_read() {
    let clock = wasi_unstable::raw::__wasi_subscription_u_clock_t {
        identifier: CLOCK_ID,
        clock_id: wasi_unstable::CLOCK_MONOTONIC,
        timeout: 5_000_000u64, // 5 milliseconds
        precision: 0,
        flags: 0,
    };
    let fd_readwrite = wasi_unstable::raw::__wasi_subscription_u_fd_readwrite_t {
        fd: wasi_unstable::STDIN_FD,
    };
    let in_ = [
        wasi_unstable::Subscription {
            userdata: CLOCK_ID,
            type_: wasi_unstable::EVENTTYPE_CLOCK,
            u: wasi_unstable::raw::__wasi_subscription_u { clock },
        },
        wasi_unstable::Subscription {
            userdata: 1,
            type_: wasi_unstable::EVENTTYPE_FD_READ,
            u: wasi_unstable::raw::__wasi_subscription_u { fd_readwrite },
        },
    ];
    let out = poll_oneoff_impl(&in_, 1);
    let event = &out[0];
    assert_eq!(
        event.userdata, CLOCK_ID,
        "the event.userdata should contain clock_id specified by the user"
    );
    assert_eq!(
        event.error,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "the event.error should be set to ESUCCESS"
    );
    assert_eq!(
        event.type_,
        wasi_unstable::EVENTTYPE_CLOCK,
        "the event.type_ should equal clock"
    );
}

unsafe fn test_stdout_stderr_write() {
    let stdout_readwrite = wasi_unstable::raw::__wasi_subscription_u_fd_readwrite_t {
        fd: wasi_unstable::STDOUT_FD,
    };
    let stderr_readwrite = wasi_unstable::raw::__wasi_subscription_u_fd_readwrite_t {
        fd: wasi_unstable::STDERR_FD,
    };
    let in_ = [
        wasi_unstable::Subscription {
            userdata: 1,
            type_: wasi_unstable::EVENTTYPE_FD_WRITE,
            u: wasi_unstable::raw::__wasi_subscription_u {
                fd_readwrite: stdout_readwrite,
            },
        },
        wasi_unstable::Subscription {
            userdata: 2,
            type_: wasi_unstable::EVENTTYPE_FD_WRITE,
            u: wasi_unstable::raw::__wasi_subscription_u {
                fd_readwrite: stderr_readwrite,
            },
        },
    ];
    let out = poll_oneoff_impl(&in_, 2);
    assert_eq!(
        out[0].userdata, 1,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_eq!(
        out[0].error,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "the event.error should be set to {}",
        wasi_unstable::raw::__WASI_ESUCCESS
    );
    assert_eq!(
        out[0].type_,
        wasi_unstable::EVENTTYPE_FD_WRITE,
        "the event.type_ should equal FD_WRITE"
    );
    assert_eq!(
        out[1].userdata, 2,
        "the event.userdata should contain fd userdata specified by the user"
    );
    assert_eq!(
        out[1].error,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "the event.error should be set to {}",
        wasi_unstable::raw::__WASI_ESUCCESS
    );
    assert_eq!(
        out[1].type_,
        wasi_unstable::EVENTTYPE_FD_WRITE,
        "the event.type_ should equal FD_WRITE"
    );
}

unsafe fn test_fd_readwrite(fd: wasi_unstable::Fd, error_code: wasi_unstable::raw::__wasi_errno_t) {
    let fd_readwrite = wasi_unstable::raw::__wasi_subscription_u_fd_readwrite_t { fd };
    let in_ = [
        wasi_unstable::Subscription {
            userdata: 1,
            type_: wasi_unstable::EVENTTYPE_FD_READ,
            u: wasi_unstable::raw::__wasi_subscription_u { fd_readwrite },
        },
        wasi_unstable::Subscription {
            userdata: 2,
            type_: wasi_unstable::EVENTTYPE_FD_WRITE,
            u: wasi_unstable::raw::__wasi_subscription_u { fd_readwrite },
        },
    ];
    let out = poll_oneoff_impl(&in_, 2);
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
        out[0].type_,
        wasi_unstable::EVENTTYPE_FD_READ,
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
        out[1].type_,
        wasi_unstable::EVENTTYPE_FD_WRITE,
        "the event.type_ should equal FD_WRITE"
    );
}

unsafe fn test_fd_readwrite_valid_fd(dir_fd: wasi_unstable::Fd) {
    // Create a file in the scratch directory.
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let status = wasi_path_open(
        dir_fd,
        0,
        "file",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    test_fd_readwrite(file_fd, wasi_unstable::raw::__WASI_ESUCCESS);

    close_fd(file_fd);
    cleanup_file(dir_fd, "file");
}

unsafe fn test_fd_readwrite_invalid_fd() {
    test_fd_readwrite(
        wasi_unstable::Fd::max_value(),
        wasi_unstable::raw::__WASI_EBADF,
    )
}

unsafe fn test_poll_oneoff(dir_fd: wasi_unstable::Fd) {
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

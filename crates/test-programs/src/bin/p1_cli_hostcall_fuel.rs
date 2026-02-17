use std::ptr;

fn main() {
    big_poll();
    big_string();
    big_iovecs();
}

fn big_string() {
    let mut s = String::new();
    for _ in 0..10_000 {
        s.push_str("hello world");
    }
    let dir_fd = test_programs::preview1::open_scratch_directory(".").unwrap();
    assert_eq!(
        unsafe { wasip1::path_create_directory(dir_fd, &s) },
        Err(wasip1::ERRNO_NOMEM)
    );
}

fn big_iovecs() {
    let mut iovs = Vec::new();
    let mut ciovs = Vec::new();
    for _ in 0..10_000 {
        iovs.push(wasip1::Iovec {
            buf: ptr::null_mut(),
            buf_len: 0,
        });
        ciovs.push(wasip1::Ciovec {
            buf: ptr::null(),
            buf_len: 0,
        });
    }
    let dir_fd = test_programs::preview1::open_scratch_directory(".").unwrap();
    let fd = unsafe {
        wasip1::path_open(
            dir_fd,
            0,
            "hi",
            wasip1::OFLAGS_CREAT,
            wasip1::RIGHTS_FD_WRITE | wasip1::RIGHTS_FD_READ,
            0,
            0,
        )
        .unwrap()
    };

    unsafe {
        assert_eq!(wasip1::fd_write(fd, &ciovs), Err(wasip1::ERRNO_NOMEM));
        assert_eq!(wasip1::fd_read(fd, &iovs), Err(wasip1::ERRNO_NOMEM));
        assert_eq!(wasip1::fd_pwrite(fd, &ciovs, 0), Err(wasip1::ERRNO_NOMEM));
        assert_eq!(wasip1::fd_pread(fd, &iovs, 0), Err(wasip1::ERRNO_NOMEM));
    }

    ciovs.truncate(1);
    iovs.truncate(1);
    iovs.push(wasip1::Iovec {
        buf: ptr::null_mut(),
        buf_len: 10_000,
    });
    ciovs.push(wasip1::Ciovec {
        buf: ptr::null(),
        buf_len: 10_000,
    });
    unsafe {
        assert_eq!(wasip1::fd_write(fd, &ciovs), Err(wasip1::ERRNO_NOMEM));
        assert_eq!(wasip1::fd_read(fd, &iovs), Err(wasip1::ERRNO_NOMEM));
        assert_eq!(wasip1::fd_pwrite(fd, &ciovs, 0), Err(wasip1::ERRNO_NOMEM));
        assert_eq!(wasip1::fd_pread(fd, &iovs, 0), Err(wasip1::ERRNO_NOMEM));
    }
}

fn big_poll() {
    let mut huge_poll = Vec::new();
    let mut huge_events = Vec::new();
    for _ in 0..10_000 {
        huge_poll.push(subscribe_timeout(0));
        huge_events.push(empty_event());
    }
    let err = unsafe {
        wasip1::poll_oneoff(
            huge_poll.as_ptr(),
            huge_events.as_mut_ptr(),
            huge_poll.len(),
        )
        .unwrap_err()
    };
    assert_eq!(err, wasip1::ERRNO_NOMEM);

    fn subscribe_timeout(timeout: u64) -> wasip1::Subscription {
        wasip1::Subscription {
            userdata: 0,
            u: wasip1::SubscriptionU {
                tag: wasip1::EVENTTYPE_CLOCK.raw(),
                u: wasip1::SubscriptionUU {
                    clock: wasip1::SubscriptionClock {
                        id: wasip1::CLOCKID_MONOTONIC,
                        timeout,
                        precision: 0,
                        flags: 0,
                    },
                },
            },
        }
    }

    fn empty_event() -> wasip1::Event {
        wasip1::Event {
            error: wasip1::ERRNO_SUCCESS,
            fd_readwrite: wasip1::EventFdReadwrite {
                nbytes: 0,
                flags: 0,
            },
            type_: wasip1::EVENTTYPE_CLOCK,
            userdata: 0,
        }
    }
}

use std::process;
use test_programs::preview1::open_scratch_directory;

fn main() {
    let arg = std::env::args().nth(1).unwrap();
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1)
        }
    };

    // Wait for just one timeout (maybe hitting a fast path)
    let subs = [subscribe_timeout(0)];
    for _ in 0..1000 {
        let mut events = [empty_event()];
        unsafe {
            wasip1::poll_oneoff(subs.as_ptr(), events.as_mut_ptr(), 1).unwrap();
        }
    }

    // Wait for two timeouts
    let subs = [subscribe_timeout(0), subscribe_timeout(0)];
    for _ in 0..1000 {
        let mut events = [empty_event(), empty_event()];
        unsafe {
            wasip1::poll_oneoff(subs.as_ptr(), events.as_mut_ptr(), 2).unwrap();
        }
    }

    let file_fd = unsafe {
        wasip1::path_open(
            dir_fd,
            0,
            "hello.txt",
            wasip1::OFLAGS_CREAT,
            wasip1::RIGHTS_FD_WRITE | wasip1::RIGHTS_FD_READ,
            0,
            0,
        )
        .expect("creating a file for writing")
    };

    // Wait for a timeout fd operations
    let subs = [
        subscribe_timeout(0),
        subscribe_fd(wasip1::EVENTTYPE_FD_READ, file_fd),
        subscribe_fd(wasip1::EVENTTYPE_FD_WRITE, file_fd),
    ];
    for _ in 0..1000 {
        let mut events = [empty_event(), empty_event(), empty_event()];
        unsafe {
            wasip1::poll_oneoff(subs.as_ptr(), events.as_mut_ptr(), 3).unwrap();
        }
    }
}

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

fn subscribe_fd(ty: wasip1::Eventtype, file_descriptor: wasip1::Fd) -> wasip1::Subscription {
    wasip1::Subscription {
        userdata: 0,
        u: wasip1::SubscriptionU {
            tag: ty.raw(),
            u: wasip1::SubscriptionUU {
                fd_read: wasip1::SubscriptionFdReadwrite { file_descriptor },
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

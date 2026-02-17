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
            wasi::poll_oneoff(subs.as_ptr(), events.as_mut_ptr(), 1).unwrap();
        }
    }

    // Wait for two timeouts
    let subs = [subscribe_timeout(0), subscribe_timeout(0)];
    for _ in 0..1000 {
        let mut events = [empty_event(), empty_event()];
        unsafe {
            wasi::poll_oneoff(subs.as_ptr(), events.as_mut_ptr(), 2).unwrap();
        }
    }

    let file_fd = unsafe {
        wasi::path_open(
            dir_fd,
            0,
            "hello.txt",
            wasi::OFLAGS_CREAT,
            wasi::RIGHTS_FD_WRITE | wasi::RIGHTS_FD_READ,
            0,
            0,
        )
        .expect("creating a file for writing")
    };

    // Wait for a timeout fd operations
    let subs = [
        subscribe_timeout(0),
        subscribe_fd(wasi::EVENTTYPE_FD_READ, file_fd),
        subscribe_fd(wasi::EVENTTYPE_FD_WRITE, file_fd),
    ];
    for _ in 0..1000 {
        let mut events = [empty_event(), empty_event(), empty_event()];
        unsafe {
            wasi::poll_oneoff(subs.as_ptr(), events.as_mut_ptr(), 3).unwrap();
        }
    }
}

fn subscribe_timeout(timeout: u64) -> wasi::Subscription {
    wasi::Subscription {
        userdata: 0,
        u: wasi::SubscriptionU {
            tag: wasi::EVENTTYPE_CLOCK.raw(),
            u: wasi::SubscriptionUU {
                clock: wasi::SubscriptionClock {
                    id: wasi::CLOCKID_MONOTONIC,
                    timeout,
                    precision: 0,
                    flags: 0,
                },
            },
        },
    }
}

fn subscribe_fd(ty: wasi::Eventtype, file_descriptor: wasi::Fd) -> wasi::Subscription {
    wasi::Subscription {
        userdata: 0,
        u: wasi::SubscriptionU {
            tag: ty.raw(),
            u: wasi::SubscriptionUU {
                fd_read: wasi::SubscriptionFdReadwrite { file_descriptor },
            },
        },
    }
}

fn empty_event() -> wasi::Event {
    wasi::Event {
        error: wasi::ERRNO_SUCCESS,
        fd_readwrite: wasi::EventFdReadwrite {
            nbytes: 0,
            flags: 0,
        },
        type_: wasi::EVENTTYPE_CLOCK,
        userdata: 0,
    }
}

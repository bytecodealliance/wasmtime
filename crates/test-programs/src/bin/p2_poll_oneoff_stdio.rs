use std::collections::HashMap;
use wasip2::clocks::monotonic_clock;
use wasip2::io::poll::{Pollable, poll};

const TIMEOUT: u64 = 200_000_000u64; // 200 milliseconds, required to satisfy slow execution in CI

fn test_stdin_read() {
    let stdin = wasip2::cli::stdin::get_stdin();
    let clock = monotonic_clock::subscribe_duration(TIMEOUT);
    let stdin_ready = stdin.subscribe();

    // Either stdin can be ready for reading, or this poll can timeout.
    let out = poll(&[&clock, &stdin_ready]);
    // The result should be either a timeout, or that stdin is ready for reading.
    // Both are valid behaviors that depend on the test environment.
    assert!(out.len() >= 1, "stdin read should return at least 1 event");
    for index in out {
        match index {
            0 | 1 => {}
            other => panic!("unexpected ready index {other}"),
        }
    }
}

fn test_stdout_stderr_write() {
    let stdout = wasip2::cli::stdout::get_stdout();
    let stderr = wasip2::cli::stderr::get_stderr();
    let mut writable: HashMap<u64, Pollable> = [(1, stdout.subscribe()), (2, stderr.subscribe())]
        .into_iter()
        .collect();

    let clock = monotonic_clock::subscribe_duration(TIMEOUT);
    let mut timed_out = false;
    while !writable.is_empty() {
        if timed_out {
            panic!(
                "timed out with the following pending subs: {:?}",
                writable.keys().collect::<Vec<_>>()
            )
        }
        let mut ids = Vec::new();
        let mut pollables = Vec::new();
        for (id, pollable) in writable.iter() {
            ids.push(*id);
            pollables.push(pollable);
        }
        pollables.push(&clock);
        let out = poll(&pollables);
        for index in out {
            let index = index as usize;
            if index == ids.len() {
                timed_out = true;
            } else {
                let id = ids[index];
                if writable.remove(&id).is_none() {
                    panic!("Unknown id {id}, pending subs: {ids:?}")
                }
            }
        }
    }
}

fn test_poll_oneoff() {
    // NB we assume that stdin/stdout/stderr are valid and open
    // for the duration of the test case
    test_stdin_read();
    test_stdout_stderr_write();
}

fn main() {
    // Run the tests.
    test_poll_oneoff()
}

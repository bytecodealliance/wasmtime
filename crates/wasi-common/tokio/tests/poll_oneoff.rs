use anyhow::{Context, Error};
use cap_std::time::Duration;
use std::collections::HashMap;
use wasi_common::{
    file::{FdFlags, OFlags},
    sched::{Poll, RwEventFlags, SubscriptionResult, Userdata},
    WasiDir, WasiFile,
};
use wasi_tokio::{clocks_ctx, sched::poll_oneoff, Dir};

#[tokio::test(flavor = "multi_thread")]
async fn empty_file_readable() -> Result<(), Error> {
    let clocks = clocks_ctx();

    let workspace = unsafe { cap_tempfile::tempdir().expect("create tempdir") };
    workspace.create_dir("d").context("create dir")?;
    let d = workspace.open_dir("d").context("open dir")?;
    let d = Dir::from_cap_std(d);

    let f = d
        .open_file(false, "f", OFlags::CREATE, false, true, FdFlags::empty())
        .await
        .context("create writable file f")?;
    let to_write: Vec<u8> = vec![0];
    f.write_vectored(&vec![std::io::IoSlice::new(&to_write)])
        .await
        .context("write to f")?;
    drop(f);

    let mut f = d
        .open_file(false, "f", OFlags::empty(), true, false, FdFlags::empty())
        .await
        .context("open f as readable")?;

    let mut poll = Poll::new();
    poll.subscribe_read(&mut *f, Userdata::from(123));
    // Timeout bounds time in poll_oneoff
    poll.subscribe_monotonic_clock(
        &*clocks.monotonic,
        clocks
            .monotonic
            .now(clocks.monotonic.resolution())
            .checked_add(Duration::from_millis(5))
            .unwrap(),
        clocks.monotonic.resolution(),
        Userdata::from(0),
    );
    poll_oneoff(&mut poll).await?;

    let events = poll.results();

    assert_eq!(events.len(), 1, "expected 1 event, got: {:?}", events);
    match events[0] {
        (SubscriptionResult::Read(Ok((1, flags))), ud) => {
            assert_eq!(flags, RwEventFlags::empty());
            assert_eq!(ud, Userdata::from(123));
        }
        _ => panic!("expected (Read(Ok(1, empty), 123), got: {:?}", events[0]),
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_file_writable() -> Result<(), Error> {
    let clocks = clocks_ctx();

    let workspace = unsafe { cap_tempfile::tempdir().expect("create tempdir") };
    workspace.create_dir("d").context("create dir")?;
    let d = workspace.open_dir("d").context("open dir")?;
    let d = Dir::from_cap_std(d);

    let mut writable_f = d
        .open_file(false, "f", OFlags::CREATE, true, true, FdFlags::empty())
        .await
        .context("create writable file")?;

    let mut poll = Poll::new();
    poll.subscribe_write(&mut *writable_f, Userdata::from(123));
    // Timeout bounds time in poll_oneoff
    poll.subscribe_monotonic_clock(
        &*clocks.monotonic,
        clocks
            .monotonic
            .now(clocks.monotonic.resolution())
            .checked_add(Duration::from_millis(5))
            .unwrap(),
        clocks.monotonic.resolution(),
        Userdata::from(0),
    );
    poll_oneoff(&mut poll).await?;

    let events = poll.results();

    assert_eq!(events.len(), 1);
    match events[0] {
        (SubscriptionResult::Write(Ok((0, flags))), ud) => {
            assert_eq!(flags, RwEventFlags::empty());
            assert_eq!(ud, Userdata::from(123));
        }
        _ => panic!(""),
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn stdio_readable() -> Result<(), Error> {
    let clocks = clocks_ctx();

    let deadline = clocks
        .monotonic
        .now(clocks.monotonic.resolution())
        .checked_add(Duration::from_millis(5))
        .unwrap();

    let mut waiting_on: HashMap<u64, Box<dyn WasiFile>> = vec![
        (
            1,
            Box::new(wasi_tokio::stdio::stdout()) as Box<dyn WasiFile>,
        ),
        (2, Box::new(wasi_tokio::stdio::stderr())),
    ]
    .into_iter()
    .collect();

    while !waiting_on.is_empty() {
        let mut poll = Poll::new();

        for (ix, file) in waiting_on.iter_mut() {
            poll.subscribe_write(&mut **file, Userdata::from(*ix));
        }
        // Timeout bounds time in poll_oneoff
        poll.subscribe_monotonic_clock(
            &*clocks.monotonic,
            deadline,
            clocks.monotonic.resolution(),
            Userdata::from(999),
        );
        poll_oneoff(&mut poll).await?;
        let events = poll.results();

        for e in events {
            match e {
                (SubscriptionResult::Write(Ok(_)), ud) => {
                    let _ = waiting_on.remove(&u64::from(ud));
                }
                (SubscriptionResult::Write(Err(_)), ud) => {
                    panic!("error on ix {}", u64::from(ud))
                }
                (SubscriptionResult::Read { .. }, _) => unreachable!(),
                (SubscriptionResult::MonotonicClock { .. }, _) => {
                    panic!("timed out before stdin and stdout ready for reading")
                }
            }
        }
    }

    Ok(())
}

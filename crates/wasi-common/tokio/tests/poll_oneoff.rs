use anyhow::{Context, Error};
use wasi_common::{
    file::{FdFlags, OFlags},
    sched::{Poll, RwEventFlags, SubscriptionResult, Userdata},
    WasiDir,
};
use wasi_tokio::{sched::poll_oneoff, Dir};

#[tokio::test(flavor = "multi_thread")]
async fn empty_file_readable() -> Result<(), Error> {
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

use anyhow::{Context, Error};
use std::ops::Deref;
use wasi_common::{
    file::{FdFlags, OFlags},
    sched::{Poll, Userdata},
    WasiDir, WasiFile,
};
use wasi_tokio::{sched::poll_oneoff, Dir, File};

#[tokio::test(flavor = "multi_thread")]
async fn main() -> Result<(), Error> {
    let workspace = unsafe { cap_tempfile::tempdir().expect("create tempdir") };
    workspace.create_dir("d").context("create dir")?;
    let d = workspace.open_dir("d").context("open dir")?;
    let d = Dir::from_cap_std(d);

    let mut readable_f = d
        .open_file(false, "f", OFlags::CREATE, true, false, FdFlags::empty())
        .await
        .context("create readable file")?;

    let mut poll = Poll::new();
    poll.subscribe_read(&mut *readable_f, Userdata::from(123));
    let poll_events = poll_oneoff(&mut poll).await?;

    Ok(())
}

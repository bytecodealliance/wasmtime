use cap_std::time::Duration;
use std::convert::TryInto;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::task::Context;
use wasi_common::{
    file::WasiFile,
    sched::{
        subscription::{RwEventFlags, Subscription},
        Poll,
    },
    Error, ErrorExt,
};

pub async fn poll_oneoff<'a>(poll: &'_ Poll<'a>) -> Result<(), Error> {
    if poll.is_empty() {
        return Ok(());
    }
    let mut futures: Vec<Pin<Box<dyn Future<Output = Result<(), Error>>>>> = Vec::new();
    let timeout = poll.earliest_clock_deadline();
    for s in poll.rw_subscriptions() {
        match s {
            Subscription::Read(f) => {
                futures.push(Box::pin(async move {
                    f.file()?.readable().await?;
                    f.complete(f.file()?.num_ready_bytes().await?, RwEventFlags::empty());
                    Ok(())
                }));
            }

            Subscription::Write(f) => {
                futures.push(Box::pin(async move {
                    f.file()?.writable().await?;
                    f.complete(0, RwEventFlags::empty());
                    Ok(())
                }));
            }
            Subscription::MonotonicClock { .. } => unreachable!(),
        }
    }

    // Incorrect, but lets get the type errors fixed before we write the right multiplexer here:
    for f in futures {
        f.await?;
    }
    Ok(())
}

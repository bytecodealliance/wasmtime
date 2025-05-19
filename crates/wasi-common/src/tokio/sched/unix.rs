use crate::{
    Error,
    sched::{
        Poll,
        subscription::{RwEventFlags, Subscription},
    },
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll as FPoll};

struct FirstReady<'a, T>(Vec<Pin<Box<dyn Future<Output = T> + Send + 'a>>>);

impl<'a, T> FirstReady<'a, T> {
    fn new() -> Self {
        FirstReady(Vec::new())
    }
    fn push(&mut self, f: impl Future<Output = T> + Send + 'a) {
        self.0.push(Box::pin(f));
    }
}

impl<'a, T> Future for FirstReady<'a, T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> FPoll<T> {
        let mut result = FPoll::Pending;
        for f in self.as_mut().0.iter_mut() {
            match f.as_mut().poll(cx) {
                FPoll::Ready(r) => match result {
                    // First ready gets to set the result. But, continue the loop so all futures
                    // which are ready simultaneously (often on first poll) get to report their
                    // readiness.
                    FPoll::Pending => {
                        result = FPoll::Ready(r);
                    }
                    _ => {}
                },
                _ => continue,
            }
        }
        return result;
    }
}

pub async fn poll_oneoff<'a>(poll: &mut Poll<'a>) -> Result<(), Error> {
    if poll.is_empty() {
        return Ok(());
    }

    let duration = poll
        .earliest_clock_deadline()
        .map(|sub| sub.duration_until());

    let mut futures = FirstReady::new();
    for s in poll.rw_subscriptions() {
        match s {
            Subscription::Read(f) => {
                futures.push(async move {
                    f.file
                        .readable()
                        .await
                        .map_err(|e| e.context("readable future"))?;
                    f.complete(
                        f.file
                            .num_ready_bytes()
                            .map_err(|e| e.context("read num_ready_bytes"))?,
                        RwEventFlags::empty(),
                    );
                    Ok::<(), Error>(())
                });
            }

            Subscription::Write(f) => {
                futures.push(async move {
                    f.file
                        .writable()
                        .await
                        .map_err(|e| e.context("writable future"))?;
                    f.complete(0, RwEventFlags::empty());
                    Ok(())
                });
            }
            Subscription::MonotonicClock { .. } => unreachable!(),
        }
    }
    if let Some(Some(remaining_duration)) = duration {
        match tokio::time::timeout(remaining_duration, futures).await {
            Ok(r) => r?,
            Err(_deadline_elapsed) => {}
        }
    } else {
        futures.await?;
    }

    Ok(())
}

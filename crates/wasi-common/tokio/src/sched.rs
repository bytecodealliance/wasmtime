#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::poll_oneoff;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::poll_oneoff;

use wasi_common::{
    sched::{Duration, Poll, WasiSched},
    Error,
};

pub fn sched_ctx() -> Box<dyn wasi_common::WasiSched> {
    struct AsyncSched;

    #[wiggle::async_trait]
    impl WasiSched for AsyncSched {
        async fn poll_oneoff<'a>(&self, poll: &'_ Poll<'a>) -> Result<(), Error> {
            poll_oneoff(poll).await
        }
        async fn sched_yield(&self) -> Result<(), Error> {
            tokio::task::yield_now().await;
            Ok(())
        }
        async fn sleep(&self, duration: Duration) -> Result<(), Error> {
            tokio::time::sleep(duration).await;
            Ok(())
        }
    }

    Box::new(AsyncSched)
}

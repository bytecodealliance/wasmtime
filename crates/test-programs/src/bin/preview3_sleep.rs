use core::future::Future as _;
use core::pin::pin;
use core::ptr;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use test_programs::wasi::clocks0_3_0::monotonic_clock;

// Adapted from https://github.com/rust-lang/rust/blob/cd805f09ffbfa3896c8f50a619de9b67e1d9f3c3/library/core/src/task/wake.rs#L63-L77
// TODO: Replace by `Waker::noop` once MSRV is raised to 1.85
const NOOP_RAW_WAKER: RawWaker = {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        // Cloning just returns a new no-op raw waker
        |_| NOOP_RAW_WAKER,
        // `wake` does nothing
        |_| {},
        // `wake_by_ref` does nothing
        |_| {},
        // Dropping does nothing as we don't allocate anything
        |_| {},
    );
    RawWaker::new(ptr::null(), &VTABLE)
};

const NOOP_WAKER: &'static Waker = &unsafe { Waker::from_raw(NOOP_RAW_WAKER) };

#[tokio::main(flavor = "current_thread")]
async fn main() {
    sleep_10ms().await;
    sleep_0ms();
    sleep_backwards_in_time();
}

async fn sleep_10ms() {
    let dur = 10_000_000;
    monotonic_clock::wait_until(monotonic_clock::now() + dur).await;
    monotonic_clock::wait_for(dur).await;
}

fn sleep_0ms() {
    let mut cx = Context::from_waker(NOOP_WAKER);

    assert_eq!(
        pin!(monotonic_clock::wait_until(monotonic_clock::now())).poll(&mut cx),
        Poll::Ready(()),
        "waiting until now() is ready immediately",
    );
    assert_eq!(
        pin!(monotonic_clock::wait_for(0)).poll(&mut cx),
        Poll::Ready(()),
        "waiting for 0 is ready immediately",
    );
}

fn sleep_backwards_in_time() {
    let mut cx = Context::from_waker(NOOP_WAKER);

    assert_eq!(
        pin!(monotonic_clock::wait_until(monotonic_clock::now() - 1)).poll(&mut cx),
        Poll::Ready(()),
        "waiting until instant which has passed is ready immediately",
    );
}

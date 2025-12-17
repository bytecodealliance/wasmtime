//! Fuzzing infrastructure for Wasmtime.

#![deny(missing_docs)]

use std::pin::Pin;
use std::task::{Context, Poll, Waker};

pub use wasm_mutate;
pub use wasm_smith;
pub mod generators;
pub mod mutators;
pub mod oom;
pub mod oracles;
pub mod single_module_fuzzer;

/// One time start up initialization for fuzzing:
///
/// * Enables `env_logger`.
///
/// * Restricts `rayon` to a single thread in its thread pool, for more
///   deterministic executions.
///
/// If a fuzz target is taking raw input bytes from the fuzzer, it is fine to
/// call this function in the fuzz target's oracle or in the fuzz target
/// itself. However, if the fuzz target takes an `Arbitrary` type, and the
/// `Arbitrary` implementation is not derived and does interesting things, then
/// the `Arbitrary` implementation should call this function, since it runs
/// before the fuzz target itself.
pub fn init_fuzzing() {
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| {
        let _ = env_logger::try_init();
    });
}

/// One time start up initialization for fuzzing:
///
/// * Enables `env_logger`.
///
/// * Restricts `rayon` to a single thread in its thread pool, for more
///   deterministic executions.
///
/// If a fuzz target is taking raw input bytes from the fuzzer, it is fine to
/// call this function in the fuzz target's oracle or in the fuzz target
/// itself. However, if the fuzz target takes an `Arbitrary` type, and the
/// `Arbitrary` implementation is not derived and does interesting things, then
/// the `Arbitrary` implementation should call this function, since it runs
/// before the fuzz target itself.
pub fn misc_init() {
    init_fuzzing();
    oracles::component_async::init();
}

fn block_on<F: Future>(future: F) -> F::Output {
    const MAX_POLLS: u32 = 100_000;

    let mut f = Box::pin(future);
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..MAX_POLLS {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => {}
        }
    }

    panic!("future didn't become ready")
}

/// Helper future to yield N times before resolving.
struct YieldN(u32);

impl Future for YieldN {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 == 0 {
            Poll::Ready(())
        } else {
            self.0 -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod test {
    use arbitrary::{Arbitrary, Unstructured};
    use rand::prelude::*;

    pub fn gen_until_pass<T: for<'a> Arbitrary<'a>>(
        mut f: impl FnMut(T, &mut Unstructured<'_>) -> anyhow::Result<bool>,
    ) -> bool {
        let mut rng = SmallRng::seed_from_u64(0);
        let mut buf = vec![0; 2048];
        let n = 3000;
        for _ in 0..n {
            rng.fill_bytes(&mut buf);
            let mut u = Unstructured::new(&buf);

            if let Ok(config) = u.arbitrary() {
                if f(config, &mut u).unwrap() {
                    return true;
                }
            }
        }
        false
    }

    /// Runs `f` with random data until it returns `Ok(())` `iters` times.
    pub fn test_n_times<T: for<'a> Arbitrary<'a>>(
        iters: u32,
        mut f: impl FnMut(T, &mut Unstructured<'_>) -> arbitrary::Result<()>,
    ) {
        let mut to_test = 0..iters;
        let ok = gen_until_pass(|a, b| {
            if f(a, b).is_ok() {
                Ok(to_test.next().is_none())
            } else {
                Ok(false)
            }
        });
        assert!(ok);
    }
}

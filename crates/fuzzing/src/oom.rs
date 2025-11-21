//! Utilities for testing and fuzzing out-of-memory handling.
//!
//! Inspired by SpiderMonkey's `oomTest()` helper:
//! https://firefox-source-docs.mozilla.org/js/hacking_tips.html#how-to-debug-oomtest-failures

use anyhow::bail;
use backtrace::Backtrace;
use std::{alloc::GlobalAlloc, cell::Cell, ptr, time};
use wasmtime::{Error, Result};

/// An allocator for use with `OomTest`.
#[non_exhaustive]
pub struct OomTestAllocator;

impl OomTestAllocator {
    /// Create a new OOM test allocator.
    pub const fn new() -> Self {
        OomTestAllocator
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
enum OomState {
    /// We are in code that is not part of an OOM test.
    #[default]
    OutsideOomTest,

    /// We are inside an OOM test and should inject an OOM when the counter
    /// reaches zero.
    OomOnAlloc(u32),

    /// We are inside an OOM test and we already injected an OOM.
    DidOom,
}

thread_local! {
    static OOM_STATE: Cell<OomState> = const { Cell::new(OomState::OutsideOomTest) };
}

/// Set the new OOM state, returning the old state.
fn set_oom_state(state: OomState) -> OomState {
    OOM_STATE.with(|s| s.replace(state))
}

unsafe impl GlobalAlloc for OomTestAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let old_state = set_oom_state(OomState::OutsideOomTest);

        let new_state;
        let ptr;
        {
            // NB: It's okay to log/backtrace/etc... in this block because the
            // current state is `OutsideOomTest`, so any re-entrant allocations
            // will be passed through to the system allocator.

            match old_state {
                OomState::OutsideOomTest => {
                    new_state = OomState::OutsideOomTest;
                    ptr = unsafe { std::alloc::System.alloc(layout) };
                }
                OomState::OomOnAlloc(0) => {
                    log::trace!(
                        "injecting OOM for allocation: {layout:?}\nAllocation backtrace:\n{:?}",
                        Backtrace::new(),
                    );
                    new_state = OomState::DidOom;
                    ptr = ptr::null_mut();
                }
                OomState::OomOnAlloc(c) => {
                    new_state = OomState::OomOnAlloc(c - 1);
                    ptr = unsafe { std::alloc::System.alloc(layout) };
                }
                OomState::DidOom => {
                    panic!("OOM test attempted to allocate after OOM: {layout:?}")
                }
            }
        }

        set_oom_state(new_state);
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe {
            std::alloc::System.dealloc(ptr, layout);
        }
    }
}

/// A test helper that checks that some code handles OOM correctly.
///
/// `OomTest` will only work correctly when `OomTestAllocator` is configured as
/// the global allocator.
///
/// `OomTest` does not support reentrancy, so you cannot run an `OomTest` within
/// an `OomTest`.
///
/// # Example
///
/// ```no_run
/// use std::time::Duration;
/// use wasmtime::Result;
/// use wasmtime_fuzzing::oom::{OomTest, OomTestAllocator};
///
/// #[global_allocator]
/// static GLOBAL_ALOCATOR: OomTestAllocator = OomTestAllocator::new();
///
/// #[test]
/// fn my_oom_test() -> Result<()> {
///     OomTest::new()
///         .max_iters(1_000_000)
///         .max_duration(Duration::from_secs(5))
///         .test(|| {
///             todo!("insert code here that should handle OOM here...")
///         })
/// }
/// ```
pub struct OomTest {
    max_iters: Option<u32>,
    max_duration: Option<time::Duration>,
}

impl OomTest {
    /// Create a new OOM test.
    ///
    /// By default there is no iteration or time limit, tests will be executed
    /// until the pass (or fail).
    pub fn new() -> Self {
        let _ = env_logger::try_init();
        OomTest {
            max_iters: None,
            max_duration: None,
        }
    }

    /// Configure the maximum number of times to run an OOM test.
    pub fn max_iters(&mut self, max_iters: u32) -> &mut Self {
        self.max_iters = Some(max_iters);
        self
    }

    /// Configure the maximum duration of time to run an OOM text.
    pub fn max_duration(&mut self, max_duration: time::Duration) -> &mut Self {
        self.max_duration = Some(max_duration);
        self
    }

    /// Repeatedly run the given test function, injecting OOMs at different
    /// times and checking that it correctly handles them.
    ///
    /// The test function should return an `Err(_)` if and only if it encounters
    /// an OOM.
    ///
    /// Returns early once the test function returns `Ok(())` before an OOM has
    /// been injected.
    pub fn test(&self, test_func: impl Fn() -> Result<()>) -> Result<()> {
        let start = time::Instant::now();

        for i in 0.. {
            if self.max_iters.is_some_and(|n| i >= n)
                || self.max_duration.is_some_and(|d| start.elapsed() >= d)
            {
                break;
            }

            log::trace!("=== Injecting OOM after {i} allocations ===");
            let old_state = set_oom_state(OomState::OomOnAlloc(i));
            assert_eq!(old_state, OomState::OutsideOomTest);

            let result = test_func();
            let old_state = set_oom_state(OomState::OutsideOomTest);

            match (result, old_state) {
                (_, OomState::OutsideOomTest) => unreachable!(),

                // The test function completed successfully before we ran out of
                // allocation fuel, so we're done.
                (Ok(()), OomState::OomOnAlloc(_)) => break,

                // We injected an OOM and the test function handled it
                // correctly; continue to the next iteration.
                (Err(e), OomState::DidOom) if self.is_oom_error(&e) => {}

                // Missed OOMs.
                (Ok(()), OomState::DidOom) => {
                    bail!("OOM test function missed an OOM: returned Ok(())");
                }
                (Err(e), OomState::DidOom) => {
                    return Err(
                        e.context("OOM test function missed an OOM: returned non-OOM error")
                    );
                }

                // Unexpected error.
                (Err(e), OomState::OomOnAlloc(_)) => {
                    return Err(
                        e.context("OOM test function returned an error when there was no OOM")
                    );
                }
            }
        }

        Ok(())
    }

    fn is_oom_error(&self, _: &Error) -> bool {
        // TODO: We don't have an OOM error yet. Will likely need to make it so
        // that `wasmtime::Error != anyhow::Error` as a first step here.
        false
    }
}

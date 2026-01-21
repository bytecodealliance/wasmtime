//! Utilities for testing and fuzzing out-of-memory handling.
//!
//! Inspired by SpiderMonkey's `oomTest()` helper:
//! https://firefox-source-docs.mozilla.org/js/hacking_tips.html#how-to-debug-oomtest-failures

use backtrace::Backtrace;
use std::{alloc::GlobalAlloc, cell::Cell, mem, ptr, time};
use wasmtime_error::{Error, OutOfMemory, Result, bail};

/// An allocator for use with `OomTest`.
#[non_exhaustive]
pub struct OomTestAllocator;

impl OomTestAllocator {
    /// Create a new OOM test allocator.
    pub const fn new() -> Self {
        OomTestAllocator
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

/// RAII helper to set the OOM state within a block of code and reset it upon
/// exiting that block (even if exiting via panic unwinding).
struct ScopedOomState {
    prev_state: OomState,
}

impl ScopedOomState {
    fn new(state: OomState) -> Self {
        ScopedOomState {
            prev_state: set_oom_state(state),
        }
    }

    /// Finish this OOM state scope early, resetting the OOM state to what it
    /// was before this scope was created, and returning the previous state that
    /// was just overwritten by the reset.
    fn finish(&self) -> OomState {
        set_oom_state(self.prev_state.clone())
    }
}

impl Drop for ScopedOomState {
    fn drop(&mut self) {
        set_oom_state(mem::take(&mut self.prev_state));
    }
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
                    log::trace!(
                        "Attempt to allocate {layout:?} after OOM:\n{:?}",
                        Backtrace::new(),
                    );
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

        // NB: `std::backtrace::Backtrace` doesn't have ways to handle
        // OOM. Ideally we would just disable the `"backtrace"` cargo feature,
        // but workspace feature resolution doesn't play nice with that.
        wasmtime_error::disable_backtrace();

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
    /// The test function should not use threads, or else allocations may not be
    /// tracked correctly and OOM injection may be incorrect.
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
            let (result, old_state) = {
                let guard = ScopedOomState::new(OomState::OomOnAlloc(i));
                assert_eq!(guard.prev_state, OomState::OutsideOomTest);

                let result = test_func();

                (result, guard.finish())
            };

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

    fn is_oom_error(&self, e: &Error) -> bool {
        e.is::<OutOfMemory>()
    }
}

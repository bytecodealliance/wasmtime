//! Measuring benchmarks with Callgrind as well as the wall clock.
//!
//! Wasmtime's benchmarks time their routines with [`Timer`], or [`iter`] in
//! place of Criterion's `Bencher::iter`. Run natively, that is the usual
//! wall-clock timing and nothing else. Run under Valgrind, each timed region is
//! also a Callgrind measurement: starting a timer turns on instrumentation and
//! zeroes Callgrind's counts, and stopping it dumps them, labeled
//! [`DUMP_LABEL`].
//!
//! To measure a bench this way, build it with `cargo bench --no-run`, which
//! prints the path of its executable, and run that in Criterion's test mode,
//! which runs each routine once, under Callgrind with instrumentation off at
//! start:
//!
//! ```text
//! valgrind --tool=callgrind --instr-atstart=no <bench executable> --bench --test
//! ```
//!
//! Each benchmark then produces one dump, in the order Criterion prints its
//! `Testing <id>` lines. Whether to measure is decided by asking Valgrind
//! whether it is running the process, so there is nothing to configure.

use std::ffi::CStr;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use valgrind_requests::{callgrind, valgrind};

/// The label of each dump a [`Timer`] makes, which Callgrind records as
/// `Client Request: wasmtime-bench` in the dump's `desc: Trigger:` line.
pub const DUMP_LABEL: &CStr = c"wasmtime-bench";

/// Whether this process is running under Valgrind, with any tool.
///
/// Benchmarks also use this to fit in the address space Valgrind gives a
/// process, which is far smaller than the pooling allocator's defaults reserve.
pub fn on_valgrind() -> bool {
    static ON_VALGRIND: OnceLock<bool> = OnceLock::new();
    *ON_VALGRIND.get_or_init(|| valgrind::running_on_valgrind() > 0)
}

/// Times a region of a benchmark, and under Valgrind measures it with
/// Callgrind.
///
/// Use it where a benchmark would use `Instant::now()` and `elapsed()`.
///
/// # One timer per benchmark
///
/// Under Callgrind, each benchmark must start and stop exactly one `Timer`,
/// once, in the single run of its routine that Criterion's test mode makes.
/// Each [`Timer::start`] zeroes Callgrind's counts and each [`Timer::stop`]
/// makes a dump, and a reader of the dumps matches them to benchmarks by
/// order, one each. So:
///
/// - A benchmark that times more than one region, such as one per iteration
///   or one per phase, makes extra dumps, and every benchmark after it is
///   matched to the wrong one.
/// - A benchmark that times nothing makes no dump, with the same effect.
/// - Overlapping timers measure the wrong thing: starting the inner one zeroes
///   the outer one's counts.
///
/// To time one of several regions, pick it with a parameter and skip the
/// others. [`iter`] starts one timer around its whole loop, so it follows
/// this rule.
///
/// Natively none of this applies: timers are just clock readings, and a
/// routine may start and stop as many as it likes.
#[must_use = "a timer measures until it is stopped"]
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Starts timing, and under Valgrind starts Callgrind's instrumentation
    /// and zeroes its counts.
    pub fn start() -> Timer {
        let start = Instant::now();
        if on_valgrind() {
            callgrind::start_instrumentation();
            callgrind::zero_stats();
        }
        Timer { start }
    }

    /// Stops timing and returns the time since [`Timer::start`], and under
    /// Valgrind dumps Callgrind's counts.
    pub fn stop(self) -> Duration {
        if on_valgrind() {
            callgrind::dump_stats_at(DUMP_LABEL);
        }
        self.start.elapsed()
    }
}

/// Like Criterion's `Bencher::iter`, but timed with a [`Timer`].
///
/// Natively this times exactly as `Bencher::iter` does: one clock reading
/// either side of a loop of `iters` calls.
pub fn iter<O>(b: &mut criterion::Bencher<'_>, mut routine: impl FnMut() -> O) {
    b.iter_custom(|iters| {
        let timer = Timer::start();
        for _ in 0..iters {
            std::hint::black_box(routine());
        }
        timer.stop()
    })
}

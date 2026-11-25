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
//! Callgrind writes its dumps to files, which say nothing of which benchmark
//! made them, so each dump is also announced on stdout with a [`MARKER`] line,
//! numbered from 1 in the order of the dumps. Criterion's test mode prints
//! `Testing <id>` on stdout before each benchmark, so the markers between one
//! `Testing` line and the next are that benchmark's dumps.
//!
//! Whether to measure is decided by asking Valgrind whether it is running the
//! process, so there is nothing to configure.

use std::ffi::CStr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use valgrind_requests::{callgrind, valgrind};

/// The label of each dump a [`Timer`] makes, which Callgrind records as
/// `Client Request: wasmtime-bench` in the dump's `desc: Trigger:` line.
pub const DUMP_LABEL: &CStr = c"wasmtime-bench";

/// The start of the line [`Timer::stop`] prints on stdout after each dump,
/// followed by the dump's number: `wasmtime-bench dump 1` for the process's
/// first [`DUMP_LABEL`] dump, and so on.
pub const MARKER: &str = "wasmtime-bench dump ";

/// Whether this process is running under Valgrind. May produce false negatives
/// if valgrind headers weren't available at build time.
pub fn on_valgrind() -> bool {
    static ON_VALGRIND: OnceLock<bool> = OnceLock::new();
    *ON_VALGRIND.get_or_init(|| {
        // This constant was set at build time based on available valgrind
        // headers. It needs to be checked first because if it's none then
        // `running_on_valgrind` will panic.
        if valgrind_requests::VALGRIND_VERSION.is_some() {
            valgrind::running_on_valgrind() > 0
        } else {
            false
        }
    })
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
/// makes a dump and prints a [`MARKER`], and a reader of the dumps takes a
/// benchmark's measurement from the one dump its marker announces.
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
    /// Valgrind dumps Callgrind's counts and prints a [`MARKER`] line.
    pub fn stop(self) -> Duration {
        if on_valgrind() {
            static DUMPS: AtomicU32 = AtomicU32::new(0);
            callgrind::dump_stats_at(DUMP_LABEL);
            let n = DUMPS.fetch_add(1, Ordering::Relaxed) + 1;
            println!("{MARKER}{n}");
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

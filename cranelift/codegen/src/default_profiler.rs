//! The default profiler used by the pass timing infrastructure.

use core::fmt;
use std::any::Any;
use std::boxed::Box;
use std::cell::{Cell, RefCell};
use std::mem;
use std::time::{Duration, Instant};

use crate::timing::{Pass, Profiler, NUM_PASSES};

/// A timing token is responsible for timing the currently running pass. Timing starts when it
/// is created and ends when it is dropped.
///
/// Multiple passes can be active at the same time, but they must be started and stopped in a
/// LIFO fashion.
struct DefaultTimingToken {
    /// Start time for this pass.
    start: Instant,

    // Pass being timed by this token.
    pass: Pass,

    // The previously active pass which will be restored when this token is dropped.
    prev: Pass,
}

/// Accumulated timing information for a single pass.
#[derive(Default, Copy, Clone)]
struct PassTime {
    /// Total time spent running this pass including children.
    total: Duration,

    /// Time spent running in child passes.
    child: Duration,
}

/// Accumulated timing for all passes.
pub struct PassTimes {
    pass: [PassTime; NUM_PASSES],
}

impl PassTimes {
    /// Add `other` to the timings of this `PassTimes`.
    pub fn add(&mut self, other: &Self) {
        for (a, b) in self.pass.iter_mut().zip(&other.pass[..]) {
            a.total += b.total;
            a.child += b.child;
        }
    }

    /// Returns the total amount of time taken by all the passes measured.
    pub fn total(&self) -> Duration {
        self.pass.iter().map(|p| p.total - p.child).sum()
    }
}

impl Default for PassTimes {
    fn default() -> Self {
        Self {
            pass: [Default::default(); NUM_PASSES],
        }
    }
}

impl fmt::Display for PassTimes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "======== ========  ==================================")?;
        writeln!(f, "   Total     Self  Pass")?;
        writeln!(f, "-------- --------  ----------------------------------")?;
        for (pass, time) in self.pass.iter().enumerate() {
            // Omit passes that haven't run.
            if time.total == Duration::default() {
                continue;
            }

            // Write a duration as secs.millis, trailing space.
            fn fmtdur(mut dur: Duration, f: &mut fmt::Formatter) -> fmt::Result {
                // Round to nearest ms by adding 500us.
                dur += Duration::new(0, 500_000);
                let ms = dur.subsec_millis();
                write!(f, "{:4}.{:03} ", dur.as_secs(), ms)
            }

            fmtdur(time.total, f)?;
            if let Some(s) = time.total.checked_sub(time.child) {
                fmtdur(s, f)?;
            }
            writeln!(f, " {}", Pass::from_idx(pass).description())?;
        }
        writeln!(f, "======== ========  ==================================")
    }
}

// Information about passes in a single thread.
thread_local! {
    static CURRENT_PASS: Cell<Pass> = const { Cell::new(Pass::None) };
    static PASS_TIME: RefCell<PassTimes> = RefCell::new(Default::default());
}

/// The default profiler. You can get the results using [`take_current`].
pub struct DefaultProfiler;

impl Profiler for DefaultProfiler {
    fn start_pass(&self, pass: Pass) -> Box<dyn Any> {
        let prev = CURRENT_PASS.with(|p| p.replace(pass));
        log::debug!("timing: Starting {}, (during {})", pass, prev);
        Box::new(DefaultTimingToken {
            start: Instant::now(),
            pass,
            prev,
        })
    }
}

/// Dropping a timing token indicated the end of the pass.
impl Drop for DefaultTimingToken {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        log::debug!("timing: Ending {}", self.pass);
        let old_cur = CURRENT_PASS.with(|p| p.replace(self.prev));
        debug_assert_eq!(self.pass, old_cur, "Timing tokens dropped out of order");
        PASS_TIME.with(|rc| {
            let mut table = rc.borrow_mut();
            table.pass[self.pass.idx()].total += duration;
            if let Some(parent) = table.pass.get_mut(self.prev.idx()) {
                parent.child += duration;
            }
        })
    }
}

/// Take the current accumulated pass timings and reset the timings for the current thread.
///
/// Only applies when [`DefaultProfiler`] is used.
pub fn take_current() -> PassTimes {
    PASS_TIME.with(|rc| mem::take(&mut *rc.borrow_mut()))
}

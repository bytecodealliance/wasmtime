use anyhow::Result;
use fxprof_processed_profile::{
    CategoryHandle, CpuDelta, Frame, FrameFlags, FrameInfo, Profile, Timestamp,
};
use std::time::{Duration, Instant};

use crate::{AsContext, WasmBacktrace};

// TODO: collect more data
// - Provide additional hooks for recording host-guest transitions, to be
//   invoked from a Store::call_hook
// - On non-Windows, measure thread-local CPU usage between events with
//   rustix::time::clock_gettime(ClockId::ThreadCPUTime)
// - Report which wasm module, and maybe instance, each frame came from

// TODO: batch symbolication using Frame::RelativeAddressFromReturnAddress

/// Collects profiling data for a single WebAssembly guest.
///
/// To use this, you'll need to arrange to call [`GuestProfiler::sample`] at
/// regular intervals while the guest is on the stack. The most straightforward
/// way to do that is to call it from a callback registered with
/// [`Store::epoch_deadline_callback()`](crate::Store::epoch_deadline_callback).
pub struct GuestProfiler {
    profile: Profile,
    process: fxprof_processed_profile::ProcessHandle,
    thread: fxprof_processed_profile::ThreadHandle,
    start: Instant,
}

impl GuestProfiler {
    /// Begin profiling a new guest. When this function is called, the current
    /// wall-clock time is recorded as the start time for the guest.
    ///
    /// The `module_name` parameter is recorded in the profile to help identify
    /// where the profile came from.
    ///
    /// The `interval` parameter should match the rate at which you intend
    /// to call `sample`. However, this is used as a hint and not required to
    /// exactly match the real sample rate.
    pub fn new(module_name: &str, interval: Duration) -> Self {
        let mut profile = Profile::new(
            module_name,
            std::time::SystemTime::now().into(),
            interval.into(),
        );
        let process = profile.add_process(module_name, 0, Timestamp::from_nanos_since_reference(0));
        let thread = profile.add_thread(process, 0, Timestamp::from_nanos_since_reference(0), true);
        let start = Instant::now();
        Self {
            profile,
            process,
            thread,
            start,
        }
    }

    /// Add a sample to the profile. This function collects a backtrace from
    /// any stack frames associated with the given `store` on the current
    /// stack. It should typically be called from a callback registered using
    /// [`Store::epoch_deadline_callback()`](crate::Store::epoch_deadline_callback).
    pub fn sample(&mut self, store: impl AsContext) {
        let now = Timestamp::from_nanos_since_reference(
            self.start.elapsed().as_nanos().try_into().unwrap(),
        );

        let trace = WasmBacktrace::force_capture(store);
        // Samply needs to see the oldest frame first, but we list the newest
        // first, so iterate in reverse.
        let frames = Vec::from_iter(trace.frames().iter().rev().map(|frame| {
            let frame = if let Some(name) = frame.func_name() {
                let idx = self.profile.intern_string(name);
                Frame::Label(idx)
            } else {
                // `func_offset` should always be set because we force
                // `generate_address_map` on if profiling is enabled.
                let func_offset = frame.func_offset().unwrap();
                Frame::InstructionPointer(func_offset.try_into().unwrap())
            };
            FrameInfo {
                frame,
                category_pair: CategoryHandle::OTHER.into(),
                flags: FrameFlags::empty(),
            }
        }));

        self.profile
            .add_sample(self.thread, now, frames.into_iter(), CpuDelta::ZERO, 1);
    }

    /// When the guest finishes running, call this function to write the
    /// profile to the given `output`.
    pub fn finish(&mut self, output: impl std::io::Write) -> Result<()> {
        let now = Timestamp::from_nanos_since_reference(
            self.start.elapsed().as_nanos().try_into().unwrap(),
        );
        self.profile.set_thread_end_time(self.thread, now);
        self.profile.set_process_end_time(self.process, now);

        serde_json::to_writer(output, &self.profile)?;
        Ok(())
    }
}

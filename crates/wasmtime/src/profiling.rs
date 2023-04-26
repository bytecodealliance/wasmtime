#![allow(missing_docs)]
use anyhow::Result;
use fxprof_processed_profile::{
    CategoryHandle, CpuDelta, Frame, FrameFlags, FrameInfo, Profile, Timestamp,
};
use std::io::Write;
use std::time::{Duration, Instant};

use crate::{AsContext, WasmBacktrace};

pub struct GuestProfiler {
    profile: Profile,
    process: fxprof_processed_profile::ProcessHandle,
    thread: fxprof_processed_profile::ThreadHandle,
    start: Instant,
}

impl GuestProfiler {
    pub fn new(interval: Duration) -> Self {
        let mut profile = Profile::new(
            "Wasmtime",
            std::time::SystemTime::now().into(),
            interval.into(),
        );
        let process = profile.add_process("main", 0, Timestamp::from_nanos_since_reference(0));
        let thread = profile.add_thread(process, 0, Timestamp::from_nanos_since_reference(0), true);
        let start = Instant::now();
        Self {
            profile,
            process,
            thread,
            start,
        }
    }

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

    pub fn finish(&mut self, output: impl Write) -> Result<()> {
        let now = Timestamp::from_nanos_since_reference(
            self.start.elapsed().as_nanos().try_into().unwrap(),
        );
        self.profile.set_thread_end_time(self.thread, now);
        self.profile.set_process_end_time(self.process, now);

        serde_json::to_writer(output, &self.profile)?;
        Ok(())
    }
}

mod api;
mod async_;
mod sync;

// Advances on each host call so clock-pause tests do not depend on real time.
#[derive(Default)]
struct TickingClock(std::cell::Cell<u64>);

impl wasmtime_wasi::clocks::HostMonotonicClock for TickingClock {
    fn resolution(&self) -> u64 {
        1
    }

    fn now(&self) -> u64 {
        let next = self.0.get() + 1;
        self.0.set(next);
        next
    }
}

impl wasmtime_wasi::clocks::HostWallClock for TickingClock {
    fn resolution(&self) -> std::time::Duration {
        std::time::Duration::from_nanos(1)
    }

    fn now(&self) -> std::time::Duration {
        std::time::Duration::from_nanos(wasmtime_wasi::clocks::HostMonotonicClock::now(self))
    }
}

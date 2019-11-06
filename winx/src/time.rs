use cvt::cvt;
use winapi::um::{profileapi::QueryPerformanceFrequency, winnt::LARGE_INTEGER};

pub fn perf_counter_frequency() -> std::io::Result<u64> {
    unsafe {
        let mut frequency: LARGE_INTEGER = std::mem::zeroed();
        cvt(QueryPerformanceFrequency(&mut frequency))?;
        Ok(*frequency.QuadPart() as u64)
    }
}

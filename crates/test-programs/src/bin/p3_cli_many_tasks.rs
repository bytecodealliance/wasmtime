use test_programs::p3::wasi as wasip3;

#[link(wasm_import_module = "wasi:clocks/monotonic-clock@0.3.0-rc-2026-03-15")]
unsafe extern "C" {
    #[link_name = "[async-lower]wait-for"]
    fn wait_for(dur: u64) -> u32;
}

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        // Execute it once to ensure we get type information pulled in.
        wasip3::clocks::monotonic_clock::wait_for(0).await;

        // Execute the raw function without Rust bindings to stress invoking it
        // many times.
        for _ in 0..1000 {
            unsafe {
                wait_for(1 << 60);
            }
        }

        panic!("should have trapped before now");
    }
}

fn main() {
    unreachable!();
}

use crate::wasi::{clocks::monotonic_clock, poll::poll};

wit_bindgen::generate!({
    path: "../../wasi/wit",
    world: "wasmtime:wasi/command-extended",
});

fn main() {
    // Sleep ten milliseconds.  Note that we call the relevant host functions directly rather than go through
    // libstd, since we want to ensure we're calling `monotonic_clock::subscribe` with an `absolute` parameter of
    // `true`, which libstd won't necessarily do (but which e.g. CPython _will_ do).
    poll::poll_oneoff(&[monotonic_clock::subscribe(
        monotonic_clock::now() + 10_000_000,
        true,
    )]);
}

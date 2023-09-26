use crate::wasi::{clocks::monotonic_clock, io::poll};

wit_bindgen::generate!({
    path: "../../wasi/wit",
    world: "wasmtime:wasi/command-extended",
});

fn main() {
    // Sleep ten milliseconds.  Note that we call the relevant host functions directly rather than go through
    // libstd, since we want to ensure we're calling `monotonic_clock::subscribe` with an `absolute` parameter of
    // `true`, which libstd won't necessarily do (but which e.g. CPython _will_ do).
    eprintln!("calling subscribe");
    let p = monotonic_clock::subscribe(monotonic_clock::now() + 10_000_000, true);
    dbg!(&p as *const _);
    let list = &[&p];
    dbg!(list.as_ptr());
    eprintln!("calling poll");
    poll::poll_list(list);
    eprintln!("done");
}

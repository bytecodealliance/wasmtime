# Debugging WebAssembly with Core Dumps

Wasmtime can be configured to generate [the standard Wasm core dump
format][spec] whenever guest Wasm programs trap. These core dumps can then be
consumed by external tooling (such as [`wasmgdb`][wasmgdb]) for post-mortem analysis.

This page focuses on generating and inspecting core dumps via the Wasmtime
command-line interface. For details on how to generate core dumps via the
`wasmtime` embedding API, see [Core Dumps in a Rust
Embedding](./examples-rust-core-dumps.md).

First, we need to compile some code to Wasm that can trap. Consider the
following Rust code:

```rust,no_run
// trap.rs

fn main() {
    foo(42);
}

fn foo(x: u32) {
    bar(x);
}

fn bar(x: u32) {
    baz(x);
}

fn baz(x: u32) {
    assert!(x != 42);
}
```

We can compile it to Wasm with the following command:

```shell-session
$ rustc --target wasm32-wasip1 -o ./trap.wasm ./trap.rs
```

Next, we can run it in Wasmtime and capture a core dump when it traps:

```shell-session
$ wasmtime -D coredump=./trap.coredump ./trap.wasm
thread 'main' panicked at /home/nick/scratch/trap.rs:14:5:
assertion failed: x != 42
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
Error: failed to run main module `/home/nick/scratch/trap.wasm`

Caused by:
    0: core dumped at /home/nick/scratch/trap.coredump
    1: failed to invoke command default
    2: wasm coredump generated while executing store_name:
       modules:
         <module>
       instances:
         Instance(store=1, index=1)
       memories:
         Memory(store=1, index=1)
       globals:
         Global(store=1, index=0)
       backtrace:
       error while executing at wasm backtrace:
           0: 0x5961 - <unknown>!__rust_start_panic
           1: 0x562a - <unknown>!rust_panic
           2: 0x555d - <unknown>!std::panicking::rust_panic_with_hook::h58e7d0b3d70e95b6
           3: 0x485d - <unknown>!std::panicking::begin_panic_handler::{{closure}}::h1853004619879cfd
           4: 0x47bd - <unknown>!std::sys_common::backtrace::__rust_end_short_backtrace::hed32bc5557405634
           5: 0x4f02 - <unknown>!rust_begin_unwind
           6: 0xac01 - <unknown>!core::panicking::panic_fmt::h53ca5bf48b428895
           7: 0xb1c5 - <unknown>!core::panicking::panic::h62c2c2bb054da7e1
           8:  0x661 - <unknown>!trap::baz::h859f39b65389c077
           9:  0x616 - <unknown>!trap::bar::h7ad12f9c5b730d17
          10:  0x60a - <unknown>!trap::foo::ha69c95723611c1a0
          11:  0x5fe - <unknown>!trap::main::hdfcd9f2d150fc3dc
          12:  0x434 - <unknown>!core::ops::function::FnOnce::call_once::h24336e950fb97d1e
          13:  0x40b - <unknown>!std::sys_common::backtrace::__rust_begin_short_backtrace::h2b37384d2b1a57ff
          14:  0x4ec - <unknown>!std::rt::lang_start::{{closure}}::he86eb1b6ac6d7501
          15: 0x24f7 - <unknown>!std::rt::lang_start_internal::h21f6a1d8f3633b54
          16:  0x497 - <unknown>!std::rt::lang_start::h7d256f21902ff32b
          17:  0x687 - <unknown>!__main_void
          18:  0x3e6 - <unknown>!_start
       note: using the `WASMTIME_BACKTRACE_DETAILS=1` environment variable may show more debugging information
```

You now have a core dump at `./trap.coredump` that can be consumed by external
tooling to do post-mortem analysis of the failure.

[spec]: https://github.com/WebAssembly/tool-conventions/blob/main/Coredump.md
[wasmgdb]: https://github.com/xtuc/wasm-coredump/blob/main/bin/wasmgdb/README.md

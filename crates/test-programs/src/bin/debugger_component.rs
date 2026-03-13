//! Debug-main guest to run debugger tests.
//!
//! Invoked by tests/all/debug_component.rs with particular debuggees
//! loaded for each test (selected by argv) below. We print "OK" to
//! stderr to communicate success.

use std::time::Duration;
use wstd::runtime::AsyncPollable;

mod api {
    wit_bindgen::generate!({
        world: "bytecodealliance:wasmtime/debug-main",
        path: "../../crates/debugger/wit",
        with: {
            "wasi:io/poll@0.2.6": wasip2::io::poll,
            "wasi:io/error@0.2.6": wasip2::io::error,
            "wasi:io/streams@0.2.6": wasip2::io::streams,
            "wasi:clocks/monotonic-clock@0.2.6": wasip2::clocks::monotonic_clock,
            "wasi:clocks/wall-clock@0.2.6": wasip2::clocks::wall_clock,
            "wasi:filesystem/types@0.2.6": wasip2::filesystem::types,
            "wasi:filesystem/preopens@0.2.6": wasip2::filesystem::preopens,
            "wasi:sockets/network@0.2.6": wasip2::sockets::network,
            "wasi:sockets/instance-network@0.2.6": wasip2::sockets::instance_network,
            "wasi:sockets/udp@0.2.6": wasip2::sockets::udp,
            "wasi:sockets/tcp@0.2.6": wasip2::sockets::tcp,
            "wasi:sockets/udp-create-socket@0.2.6": wasip2::sockets::udp_create_socket,
            "wasi:sockets/tcp-create-socket@0.2.6": wasip2::sockets::tcp_create_socket,
            "wasi:sockets/ip-name-lookup@0.2.6": wasip2::sockets::ip_name_lookup,
            "wasi:random/random@0.2.6": wasip2::random::random,
            "wasi:random/insecure@0.2.6": wasip2::random::insecure,
            "wasi:random/insecure-seed@0.2.6": wasip2::random::insecure_seed,
            "wasi:cli/stdin@0.2.6": wasip2::cli::stdin,
            "wasi:cli/stdout@0.2.6": wasip2::cli::stdout,
            "wasi:cli/stderr@0.2.6": wasip2::cli::stderr,
            "wasi:cli/terminal-input@0.2.6": wasip2::cli::terminal_input,
            "wasi:cli/terminal-output@0.2.6": wasip2::cli::terminal_output,
            "wasi:cli/terminal-stdin@0.2.6": wasip2::cli::terminal_stdin,
            "wasi:cli/terminal-stdout@0.2.6": wasip2::cli::terminal_stdout,
            "wasi:cli/terminal-stderr@0.2.6": wasip2::cli::terminal_stderr,
            "wasi:cli/environment@0.2.6": wasip2::cli::environment,
            "wasi:cli/exit@0.2.6": wasip2::cli::exit,
        }
    });
}
use api::bytecodealliance::wasmtime::debuggee::*;

struct Component;
api::export!(Component with_types_in api);

impl api::exports::bytecodealliance::wasmtime::debugger::Guest for Component {
    fn debug(d: &Debuggee, args: Vec<String>) {
        wstd::runtime::block_on(async {
            match args.get(1).map(|s| s.as_str()) {
                Some("simple") => test_simple(d).await,
                Some("loop") => test_loop(d).await,
                other => panic!("unknown test mode: {other:?}"),
            }
        });
    }
}

struct Resumption {
    future: EventFuture,
    pollable: Option<AsyncPollable>,
}

impl Resumption {
    fn single_step(d: &Debuggee) -> Self {
        let future = d.single_step(ResumptionValue::Normal);
        let pollable = Some(AsyncPollable::new(future.subscribe()));
        Self { future, pollable }
    }

    fn continue_(d: &Debuggee) -> Self {
        let future = d.continue_(ResumptionValue::Normal);
        let pollable = Some(AsyncPollable::new(future.subscribe()));
        Self { future, pollable }
    }

    async fn wait(&mut self) {
        if let Some(p) = self.pollable.as_mut() {
            p.wait_for().await;
        }
    }

    fn result(mut self, d: &Debuggee) -> Result<Event, Error> {
        let _ = self.pollable.take();
        EventFuture::finish(self.future, d)
    }
}

/// Tests single-stepping.
///
/// Tests against `debugger_debuggee_simple.wat`.
async fn test_simple(d: &Debuggee) {
    // Step once to reach the first instruction.
    let mut r = Resumption::single_step(d);
    r.wait().await;
    let _event = r.result(d).unwrap();

    let mut pcs = vec![];

    for _ in 0..5 {
        let frames = d.exit_frames();
        let pc = frames[0].get_pc(d).unwrap();
        pcs.push(pc);

        let mut r = Resumption::single_step(d);
        r.wait().await;
        match r.result(d).unwrap() {
            Event::Breakpoint => {}
            other => panic!("unexpected event: {other:?}"),
        }
    }

    // There should be five PCs and they should each be distinct from the previous.
    assert_eq!(pcs.len(), 5);
    assert!(pcs.windows(2).all(|p| p[0] != p[1]));

    eprintln!("OK");
}

/// Interrupt test: continue an infinite-loop debuggee, interrupt it,
/// verify the interrupt, then set the exit flag in memory and continue
/// to completion.
///
/// Tests against `debugger_debuggee_loop.wat`.
async fn test_loop(d: &Debuggee) {
    // Continue execution (the debuggee should loop).
    let mut r = Resumption::continue_(d);

    // Yield to the event loop and let it run for a bit.
    std::thread::sleep(Duration::from_millis(100));

    // Request interrupt.
    d.interrupt();

    // Wait for the interrupt event.
    r.wait().await;
    let event = r.result(d).unwrap();
    assert!(
        matches!(event, Event::Interrupted),
        "expected Interrupted, got {event:?}"
    );

    // Set the exit-flag to kill the infinite loop in the guest (the
    // debugger environment will not otherwise end until the guest
    // ends; we have no way of forcing an early exit yet).
    for inst in &d.all_instances() {
        if let Ok(mem) = inst.get_memory(d, 0) {
            mem.set_bytes(d, 0, &[1]).unwrap();
        }
    }

    // Continue; the debuggee should exit normally now.
    let mut r = Resumption::continue_(d);
    r.wait().await;
    let event = r.result(d).unwrap();
    assert!(
        matches!(event, Event::Complete),
        "expected Complete, got {event:?}"
    );

    eprintln!("OK");
}

fn main() {}

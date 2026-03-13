//! Debug-main guest to run debugger tests.
//!
//! Invoked by tests/all/debug_component.rs with particular debuggees
//! loaded for each test (selected by argv) below. We print "OK" to
//! stderr to communicate success.

use std::time::Duration;

mod api {
    wit_bindgen::generate!({
        world: "bytecodealliance:wasmtime/debug-main",
        path: "../../crates/debugger/wit",
        with: {
            "wasi:io/poll@0.2.6": wasip2::io::poll,
        }
    });
}
use api::bytecodealliance::wasmtime::debuggee::*;

struct Component;
api::export!(Component with_types_in api);

impl api::exports::bytecodealliance::wasmtime::debugger::Guest for Component {
    fn debug(d: &Debuggee, args: Vec<String>) {
        match args.get(1).map(|s| s.as_str()) {
            Some("simple") => {
                test_simple(d);
            }
            Some("loop") => {
                test_loop(d);
            }
            other => panic!("unknown test mode: {other:?}"),
        }
    }
}

struct Resumption {
    future: EventFuture,
}

impl Resumption {
    fn single_step(d: &Debuggee) -> Self {
        let future = d.single_step(ResumptionValue::Normal);
        Self { future }
    }

    fn continue_(d: &Debuggee) -> Self {
        let future = d.continue_(ResumptionValue::Normal);
        Self { future }
    }

    fn result(self, d: &Debuggee) -> Result<Event, Error> {
        EventFuture::finish(self.future, d)
    }
}

/// Tests single-stepping.
///
/// Tests against `debugger_debuggee_simple.wat`.
fn test_simple(d: &Debuggee) {
    // Step once to reach the first instruction.
    let r = Resumption::single_step(d);
    let _event = r.result(d).unwrap();

    let mut pcs = vec![];

    for _ in 0..5 {
        let frames = d.exit_frames();
        let pc = frames[0].get_pc(d).unwrap();
        pcs.push(pc);

        let r = Resumption::single_step(d);
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
fn test_loop(d: &Debuggee) {
    // Continue execution (the debuggee should loop).
    let r = Resumption::continue_(d);

    // Yield to the event loop and let it run for a bit.
    std::thread::sleep(Duration::from_millis(100));

    // Request interrupt.
    d.interrupt();

    // Wait for the interrupt event.
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
    let r = Resumption::continue_(d);
    let event = r.result(d).unwrap();
    assert!(
        matches!(event, Event::Complete),
        "expected Complete, got {event:?}"
    );

    eprintln!("OK");
}

fn main() {}

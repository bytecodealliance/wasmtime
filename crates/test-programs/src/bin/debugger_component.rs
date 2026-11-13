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
            "wasi:io/poll@0.2.12": wasip2::io::poll,
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
            Some("watch") => {
                test_watch(d);
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
    assert!(pcs.array_windows().all(|[a, b]| a != b));

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

/// Watchpoint test: watch bytes [8, 12) of memory 0 and check that
/// writes overlapping them (and only those) pause execution before
/// the write happens.
///
/// Tests against `debugger_debuggee_watch.wat`.
fn test_watch(d: &Debuggee) {
    // Step once so that the instance exists.
    let r = Resumption::single_step(d);
    assert!(matches!(r.result(d).unwrap(), Event::Breakpoint));

    let mem = d
        .all_instances()
        .iter()
        .find_map(|inst| inst.get_memory(d, 0).ok())
        .expect("debuggee has a memory");

    // Ranges must be within the memory.
    let size = mem.size_bytes(d);
    assert!(matches!(
        mem.add_watchpoint(d, size - 1, 2),
        Err(Error::OutOfBounds)
    ));
    assert!(matches!(
        mem.add_watchpoint(d, u64::MAX, 2),
        Err(Error::OutOfBounds)
    ));

    mem.add_watchpoint(d, 8, 4).unwrap();

    // The `i32.store` to [6, 10) is the first write to hit the watch.
    let r = Resumption::continue_(d);
    let Event::Watchpoint(hit) = r.result(d).unwrap() else {
        panic!("expected a watchpoint event");
    };
    assert_eq!(hit.memory.unique_id(), mem.unique_id());
    assert_eq!((hit.address, hit.len), (6, 4));
    assert_eq!(hit.value, Some(0x11223344_u32.to_le_bytes().to_vec()));
    // The write has not happened yet; the store to byte 7 has.
    assert_eq!(mem.get_bytes(d, 6, 4).unwrap(), [0, 1, 0, 0]);

    // Next, the `memory.fill`, which has no single value.
    let r = Resumption::continue_(d);
    let Event::Watchpoint(hit) = r.result(d).unwrap() else {
        panic!("expected a watchpoint event");
    };
    assert_eq!((hit.address, hit.len, hit.value), (0, 16, None));
    assert_eq!(mem.get_u32(d, 6).unwrap(), 0x11223344);
    assert_eq!(mem.get_u8(d, 12).unwrap(), 2);

    // Once unwatched, the fill proceeds and the debuggee runs to
    // completion.
    mem.remove_watchpoint(d, 0, 16).unwrap();
    let r = Resumption::continue_(d);
    let event = r.result(d).unwrap();
    assert!(
        matches!(event, Event::Complete),
        "expected Complete, got {event:?}"
    );

    eprintln!("OK");
}

fn main() {}

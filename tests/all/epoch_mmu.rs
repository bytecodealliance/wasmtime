#![cfg(not(miri))]

use object::{LittleEndian, Object, ObjectSection, U32Bytes};
use std::future::Future;
use std::pin::Pin;
use std::ptr::null;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use wasmtime::{Config, Engine, Instance, Module, Store};
use wasmtime_environ::obj::ELF_WASMTIME_EPOCH_CHECKS;

/// Returns an `Engine` with MMU-based epochs enabled.
fn config_with_mmu_epochs() -> Config {
    let mut config = Config::new();
    config.epoch_interruption_via_mmu(true);
    config.async_support(true);
    config
}

/// Returns a new `Store` set up to trap at the first encountered epoch check.
fn store_with_ended_epoch(engine: &Engine) -> Store<()> {
    // Trap as soon as the first epoch check is encountered, in the function
    // prologue. Recall that MMU-based epochs don't operate based on a numeric
    // deadline but on an external entity protecting the memory page, typically
    // on a timer.
    let mut store = Store::new(&engine, ());
    store.epoch_deadline_trap(); // Allegedly the default.
    // Protect the memory page:
    store.mmu_interrupter().unwrap().interrupt();
    store
}

/// Asserts that each epoch-check offset encoded into the binary points to the
/// byte after its corresponding dead load.
#[test]
fn epoch_check_offsets() {
    let mut config = config_with_mmu_epochs();
    config.target("x86_64").unwrap();
    let engine = Engine::new(&config).unwrap();

    // A function with an infinite loop contains two epoch checks: one in the
    // function prologue and another at the loop backedge.
    let elf_bytes = engine
        .precompile_module(
            // If you change this wat, change it in
            // epoch-interruption-mmu-compile-loop.wat, too.
            r#"(module
             (memory 0)
             (func (loop (br 0)))
           )"#
            .as_bytes(),
        )
        .unwrap();

    let elf = object::read::elf::ElfFile64::<object::Endianness>::parse(&*elf_bytes)
        .expect("ELF should be parseable");
    let section = elf
        .section_by_name(ELF_WASMTIME_EPOCH_CHECKS)
        .expect(&format!(
            "{ELF_WASMTIME_EPOCH_CHECKS} section should be present"
        ));
    let data = section.data().unwrap();

    let (count_raw, rest) = object::from_bytes::<U32Bytes<LittleEndian>>(data).expect(
        ".wasmtime.epochchecks section should be long enough to contain a count of epoch checks",
    );
    let count = count_raw.get(LittleEndian) as usize;
    let (starts_raw, rest) = object::slice_from_bytes::<U32Bytes<LittleEndian>>(rest, count)
        .expect(".wasmtime.epochchecks section should be long enough to contain a location for each epoch check");
    let starts: Vec<u32> = starts_raw.iter().map(|b| b.get(LittleEndian)).collect();
    let (length_bits, _rest) = object::slice_from_bytes::<u8>(rest, count.div_ceil(8))
        .expect(".wasmtime.epochchecks section should be long enough to contain a length bit for each epoch check");

    // The emitted machine code is nailed down by the
    // epoch-interruption-mmu-compile-loop.wat disas test. As long as that keeps
    // passing, these values remain valid.
    assert_eq!(
        starts,
        vec![12, 15],
        "There should be 2 epoch checks (function prologue & loop backedge). The offset of the prologue's dead load should be 12, and that of the loop's backedge should be 15."
    );
    assert_eq!(
        length_bits,
        vec![0],
        "Neither check's load instruction uses R12 of RSP as its source, so all length bits should be 0."
    );
}

/// Runs a wasm function with MMU-based epoch interruption enabled and the epoch
/// ended. Make sure the function returns happily after the interruption.
#[tokio::test]
async fn epoch_mmu_trap_via_signal_handler() {
    let config = config_with_mmu_epochs();
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(
        &engine,
        r#"(module
             (memory 0)
             (func (export "answer") (result i32)
                i32.const 42
             )
           )"#,
    )
    .unwrap();

    let mut store = store_with_ended_epoch(&engine);
    let instance = Instance::new_async(&mut store, &module, &[]).await.unwrap();
    let func = instance
        .get_typed_func::<(), i32>(&mut store, "answer")
        .unwrap();

    let result = func.call_async(&mut store, ()).await.unwrap();
    assert_eq!(result, 42);
}

/// Runs a Wasm function to an epoch check point, lets it yield, then drops the
/// future driving it. This exercises the cancellation path of
/// `yield_current_fiber()`, which should unwind the stack cleanly.
#[test]
// TODO: run only on x86.
fn epoch_mmu_cancellation_during_yield() {
    // Returns a no-op waker that lets nothing re-poll our future after it
    // yields the first time. This keeps the fiber parked inside the yield until
    // we explicitly drop its future.
    fn null_waker() -> Waker {
        const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW, |_| {}, |_| {}, |_| {});
        const RAW: RawWaker = RawWaker::new(null(), &VTABLE);
        unsafe { Waker::from_raw(RAW) }
    }

    /// Polls a future continually until it is complete, returning its result.
    fn busy_poll_until_complete<F: Future>(mut future: F) -> F::Output {
        let waker = null_waker();
        let mut ctx = Context::from_waker(&waker);
        // SAFETY: `future` lives until function returns, and we never move it.
        let mut future = unsafe { Pin::new_unchecked(&mut future) };
        loop {
            if let Poll::Ready(r) = future.as_mut().poll(&mut ctx) {
                return r;
            }
        }
    }

    let config = config_with_mmu_epochs();
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(
        &engine,
        r#"(module
             (memory 0)
             (func (export "loop") (loop (br 0)))
           )"#,
    )
    .unwrap();

    let mut store = store_with_ended_epoch(&engine);
    let instance = busy_poll_until_complete(Instance::new_async(&mut store, &module, &[])).unwrap();
    let func = instance
        .get_typed_func::<(), ()>(&mut store, "loop")
        .unwrap();

    let waker = null_waker();
    let mut ctx = Context::from_waker(&waker);

    // Pin future so we're allowed to poll it.
    let mut future = Box::pin(func.call_async(&mut store, ()));

    // Poll once to run into the epoch check.
    match future.as_mut().poll(&mut ctx) {
        // When `yield_current_fiber()` switches fibers, the old fiber's
        // `Pending` should percolate up via `block_on()`.
        Poll::Pending => {}
        Poll::Ready(r) => panic!(
            "the fiber should have suspended itself, returning Pending, but it returned Ready({r:?}) instead"
        ),
    }

    // Drop the suspended future. This triggers `FiberFuture::Drop` →
    // `StoreFiber::dispose()`, which gets cranky that we're dropping a fiber
    // that isn't done and resumes the fiber with an `Err`. This triggers the
    // `yield_current_fiber` path we're interested in: stack unwinding.
    drop(future);

    // If the unwinding went wrong, the above drop would have spun forever (in a
    // release build) or hit the `debug_assert!(result.is_ok())` (in debug) in
    // `StoreFiber::dispose()`. Thus, getting here means success.
}

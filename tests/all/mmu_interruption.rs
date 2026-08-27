#![cfg(not(miri))]

use object::{LittleEndian, Object, ObjectSection, U32};
use std::future::Future;
use std::pin::Pin;
use std::ptr::null;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use wasmtime::{Config, Engine, Module, Result};
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
use wasmtime::{Instance, Store};
use wasmtime_environ::obj::ELF_WASMTIME_MMU_INTERRUPT_CHECKS;
use wasmtime_test_macros::wasmtime_test;

// Parses the MMU-interrupt-check section out of a precompiled module,
// returning the check offsets and the packed bits giving each check's load
// length.
fn mmu_interrupt_checks(elf_bytes: &[u8]) -> (Vec<u32>, Vec<u8>) {
    let elf = object::read::elf::ElfFile64::<object::Endianness>::parse(elf_bytes)
        .expect("ELF should be parseable");
    let section = elf
        .section_by_name(ELF_WASMTIME_MMU_INTERRUPT_CHECKS)
        .expect(&format!(
            "{ELF_WASMTIME_MMU_INTERRUPT_CHECKS} section should be present"
        ));
    let data = section.data().unwrap();

    let (count_raw, rest) = object::from_bytes::<U32<LittleEndian>>(data).expect(
        ".wasmtime.mmu_interrupt_checks section should be long enough to contain a count of MMU-interrupt checks",
    );
    let count = count_raw.get(LittleEndian) as usize;
    let (starts_raw, rest) = object::slice_from_bytes::<U32<LittleEndian>>(rest, count)
        .expect(".wasmtime.mmu_interrupt_checks section should be long enough to contain a location for each MMU-interrupt check");
    let starts: Vec<u32> = starts_raw.iter().map(|b| b.get(LittleEndian)).collect();
    let (length_bits, _rest) = object::slice_from_bytes::<u8>(rest, count.div_ceil(8))
        .expect(".wasmtime.mmu_interrupt_checks section should be long enough to contain a length bit for each MMU-interrupt check");

    (starts, length_bits.to_vec())
}

// A function with an infinite loop contains two MMU-interrupt checks, one in
// the function prologue and another at the loop backedge. If you change this
// wat, change it in mmu-interruption-compile-loop{,-aarch64}.wat, too.
const LOOPING_MODULE: &str = r#"(module
             (memory 0)
             (func (loop (br 0)))
           )"#;

// Asserts that each MMU-interrupt-check offset encoded into the binary points
// to the byte after its corresponding dead load.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn mmu_interrupt_check_offsets(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.target("x86_64").unwrap();
    let engine = Engine::new(config).unwrap();

    let elf_bytes = engine.precompile_module(LOOPING_MODULE.as_bytes()).unwrap();
    let (starts, length_bits) = mmu_interrupt_checks(&elf_bytes);

    // The emitted machine code is nailed down by the
    // mmu-interruption-compile-loop.wat disas test. As long as that keeps
    // passing, these values remain valid.
    assert_eq!(
        starts,
        vec![12, 15],
        "There should be 2 MMU-interrupt checks (function prologue & loop backedge). The offset of the prologue's dead load should be 12, and that of the loop's backedge should be 15."
    );
    assert_eq!(
        length_bits,
        vec![0],
        "Neither check's load instruction uses R12 or RSP as its source, so all length bits should be 0."
    );
    Ok(())
}

// The aarch64 counterpart of `mmu_interrupt_check_offsets`.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn mmu_interrupt_check_offsets_aarch64(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.target("aarch64").unwrap();
    let engine = Engine::new(config).unwrap();

    let elf_bytes = engine.precompile_module(LOOPING_MODULE.as_bytes()).unwrap();
    let (starts, length_bits) = mmu_interrupt_checks(&elf_bytes);

    // The emitted machine code is nailed down by the
    // mmu-interruption-compile-loop-aarch64.wat disas test. As long as that
    // keeps passing, these values remain valid.
    assert_eq!(
        starts,
        vec![20, 24],
        "There should be 2 MMU-interrupt checks (function prologue & loop backedge). The offset of the prologue's dead load should be 20, and that of the loop's backedge should be 24."
    );
    assert_eq!(
        length_bits,
        vec![0b11],
        "Every aarch64 instruction is 4 bytes wide, so both checks' length bits should be 1."
    );
    Ok(())
}

// Runs two Wasm functions, interleaved, with MMU interruption enabled and
// interruptions triggered. Shows that the functions return happily after
// interruption. Loops several times to test multiple interrupts switching
// between Wasm modules in a single `Store`.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
async fn mmu_interruption_signal_handler_trapping_and_switching(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    let engine = Engine::new(config).unwrap();

    let module_one = Module::new(
        &engine,
        r#"(module
             (memory 0)
             (func (export "one") (result i32)
                i32.const 1
             )
           )"#,
    )
    .unwrap();
    let module_two = Module::new(
        &engine,
        r#"(module
             (memory 0)
             (func (export "two") (result i32)
                i32.const 2
             )
           )"#,
    )
    .unwrap();

    let mut store = Store::new(&engine, ());
    store.epoch_deadline_trap();
    let interrupter = store.mmu_interrupter().unwrap();

    let instance_one = Instance::new_async(&mut store, &module_one, &[])
        .await
        .unwrap();
    let instance_two = Instance::new_async(&mut store, &module_two, &[])
        .await
        .unwrap();
    let func_one = instance_one
        .get_typed_func::<(), i32>(&mut store, "one")
        .unwrap();
    let func_two = instance_two
        .get_typed_func::<(), i32>(&mut store, "two")
        .unwrap();

    for _ in 0..5 {
        // Trap as soon as the first MMU-interrupt check is encountered, in the
        // function prologue. Recall that MMU interruption doesn't operate based
        // on a numeric deadline but on an external entity protecting the memory
        // page, typically on a timer.
        interrupter.interrupt();
        assert_eq!(func_one.call_async(&mut store, ()).await.unwrap(), 1);
        interrupter.interrupt();
        assert_eq!(func_two.call_async(&mut store, ()).await.unwrap(), 2);
    }
    Ok(())
}

// Runs a Wasm function to an MMU-interrupt check point, lets it yield, then
// drops the future driving it. This exercises the cancellation path of
// `yield_current_fiber()`, which should unwind the stack cleanly.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn mmu_interruption_cancellation_during_yield(config: &mut Config) -> Result<()> {
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

    config.mmu_interruption(true);
    let engine = Engine::new(config).unwrap();
    let module = Module::new(
        &engine,
        r#"(module
             (memory 0)
             (func (export "loop") (loop (br 0)))
           )"#,
    )
    .unwrap();

    let mut store = Store::new(&engine, ());
    store.epoch_deadline_trap();
    store.mmu_interrupter().unwrap().interrupt();

    let instance = busy_poll_until_complete(Instance::new_async(&mut store, &module, &[])).unwrap();
    let func = instance
        .get_typed_func::<(), ()>(&mut store, "loop")
        .unwrap();

    let waker = null_waker();
    let mut ctx = Context::from_waker(&waker);

    // Pin future so we're allowed to poll it.
    let mut future = Box::pin(func.call_async(&mut store, ()));

    // Poll once to run into the MMU-interrupt check.
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
    Ok(())
}

// For aot compilation, signals based traps are required.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn requires_signals_based_traps(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.signals_based_traps(false);
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption requires signals-based traps",
    );
    Ok(())
}

// With Cranelift only the x64 and aarch64 backends are supported.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn requires_supported_target(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.target("riscv64").unwrap();
    config.signals_based_traps(true);
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "MMU interruption is supported only on x86_64 and aarch64, not for `riscv64-unknown-unknown-elf`",
    );
    Ok(())
}

// The Winch backend does not support this feature.
#[wasmtime_test(strategies(only(Winch)))]
fn rejected_by_winch(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "Winch does not currently support MMU interruption",
    );
    Ok(())
}

// Pulley does not support this feature, since it does not support signals
// based traps.
#[wasmtime_test(strategies(only(CraneliftPulley)))]
fn rejected_by_pulley(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert!(
        err.to_string().contains("MMU interruption"),
        "unexpected error: {err}"
    );
    Ok(())
}

// AOT compilation succeeds with the right flags set.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn precompile_succeeds_for_valid_config_on_any_host(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.target("x86_64-unknown-linux-gnu").unwrap();
    config.signals_based_traps(true);
    let engine = Engine::new(config).unwrap();
    engine
        .precompile_module(r#"(module (memory 0) (func))"#.as_bytes())
        .expect("precompilation should succeed regardless of host");
    Ok(())
}

#[cfg(not(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
)))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn compile_and_run_fails_on_unsupported_host(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.signals_based_traps(true);
    let err = match Engine::new(config) {
        Err(err) => err,
        Ok(engine) => Module::new(&engine, "(module)")
            .expect_err("compile-and-run should fail on an unsupported host"),
    };
    let err = format!("{err:?}");
    assert!(
        err.contains("supported only on x86_64 and aarch64"),
        "unexpected error: {err}"
    );
    Ok(())
}

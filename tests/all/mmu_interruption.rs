#![cfg(not(miri))]

use object::{Object, ObjectSection};
use std::future::Future;
use std::pin::Pin;
use std::ptr::null;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use wasmtime::{Config, Engine, Module, Result};
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
use wasmtime::{Instance, Store};
use wasmtime::{MmuInterrupter, PageHandle, TimingWheelInterrupter};
use wasmtime_environ::obj::ELF_WASMTIME_TRAPS;
use wasmtime_environ::{CompiledTrap, iterate_traps};
use wasmtime_test_macros::wasmtime_test;

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
mod armable {
    use rustix::mm::{MapFlags, MprotectFlags, ProtFlags, mmap_anonymous, mprotect, munmap};
    use std::ffi::c_void;
    use std::ptr::{NonNull, null_mut};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
    use wasmtime::{MmuInterrupter, PageHandle};

    /// An `MmuInterrupter` with no timer: once armed, it hands out its next
    /// page already protected, so the store is interrupted at its first
    /// checkpoint.
    #[derive(Default)]
    pub struct ArmableInterrupter {
        armed: AtomicBool,
        acquisitions: AtomicUsize,
        pages: Mutex<Vec<usize>>,
    }

    impl ArmableInterrupter {
        pub fn arm(&self) {
            self.armed.store(true, SeqCst);
        }

        pub fn acquisitions(&self) -> usize {
            self.acquisitions.load(SeqCst)
        }

        /// Protects the most recently acquired page so its store is interrupted
        /// at the next checkpoint.
        pub fn protect_latest(&self) {
            let page = *self.pages.lock().unwrap().last().unwrap();
            unsafe { mprotect(page as *mut c_void, 1, MprotectFlags::empty()).unwrap() };
        }
    }

    struct TestPage(usize);

    impl PageHandle for TestPage {
        fn page_ptr(&self) -> NonNull<c_void> {
            NonNull::new(self.0 as *mut c_void).unwrap()
        }
    }

    impl MmuInterrupter for ArmableInterrupter {
        /// Returns a protect page if I've been armed, an unprotected one
        /// otherwise.
        fn acquire_page(&self) -> Box<dyn PageHandle> {
            self.acquisitions.fetch_add(1, SeqCst);
            // A 1-byte length maps (and protects) the whole page.
            unsafe {
                let page =
                    mmap_anonymous(null_mut(), 1, ProtFlags::READ, MapFlags::PRIVATE).unwrap();
                if self.armed.swap(false, SeqCst) {
                    mprotect(page, 1, MprotectFlags::empty()).unwrap();
                }
                self.pages.lock().unwrap().push(page as usize);
                Box::new(TestPage(page as usize))
            }
        }
    }

    impl Drop for ArmableInterrupter {
        // Dropping the last reference means no Engine can run Wasm against
        // these pages anymore.
        fn drop(&mut self) {
            for &page in self.pages.lock().unwrap().iter() {
                unsafe { munmap(page as *mut c_void, 1).unwrap() };
            }
        }
    }
}

// Returns the offset of every MMU-interrupt check recorded in a compiled
// module's trap table.
fn mmu_interrupt_checks(elf_bytes: &[u8]) -> Vec<u32> {
    let elf = object::read::elf::ElfFile64::<object::Endianness>::parse(elf_bytes)
        .expect("ELF should be parseable");
    let section = elf
        .section_by_name(ELF_WASMTIME_TRAPS)
        .expect(&format!("{ELF_WASMTIME_TRAPS} section should be present"));

    iterate_traps(section.data().unwrap())
        .expect(&format!("{ELF_WASMTIME_TRAPS} section should be parseable"))
        .filter_map(|(offset, trap)| match trap {
            CompiledTrap::MmuInterrupt => Some(offset),
            _ => None,
        })
        .collect()
}

// A function with an infinite loop contains two MMU-interrupt checks, one in
// the function prologue and another at the loop backedge. If you change this
// wat, change it in mmu-interruption-compile-loop{,-aarch64}.wat, too.
const LOOPING_MODULE: &str = r#"(module
             (memory 0)
             (func (loop (br 0)))
           )"#;

// Asserts that each MMU-interrupt check is recorded in the trap table at the
// offset of its dead load.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn mmu_interrupt_check_offsets(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.target("x86_64").unwrap();
    let engine = Engine::new(config).unwrap();

    let elf_bytes = engine.precompile_module(LOOPING_MODULE.as_bytes()).unwrap();
    let starts = mmu_interrupt_checks(&elf_bytes);

    // The emitted machine code is nailed down by the
    // mmu-interruption-compile-loop.wat disas test. As long as that keeps
    // passing, these values remain valid.
    assert_eq!(
        starts,
        vec![12, 15],
        "There should be 2 MMU-interrupt checks (function prologue & loop backedge). The offset of the prologue's dead load should be 12, and that of the loop's backedge should be 15."
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
    let starts = mmu_interrupt_checks(&elf_bytes);

    // The emitted machine code is nailed down by the
    // mmu-interruption-compile-loop-aarch64.wat disas test. As long as that
    // keeps passing, these values remain valid.
    assert_eq!(
        starts,
        vec![20, 24],
        "There should be 2 MMU-interrupt checks (function prologue & loop backedge). The offset of the prologue's dead load should be 20, and that of the loop's backedge should be 24."
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
    let interrupter = Arc::new(armable::ArmableInterrupter::default());
    config
        .mmu_interruption(true)
        .with_mmu_interrupter(interrupter.clone());
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
        // function prologue.
        let before = interrupter.acquisitions();
        interrupter.arm();
        assert_eq!(func_one.call_async(&mut store, ()).await.unwrap(), 1);
        // There are 2 acquisitions: the one when the function initially starts
        // running (acquiring a protected page), then another after the
        // interruption at the first checkpoint, upon resumption.
        assert_eq!(interrupter.acquisitions() - before, 2);

        let before = interrupter.acquisitions();
        interrupter.arm();
        assert_eq!(func_two.call_async(&mut store, ()).await.unwrap(), 2);
        assert_eq!(interrupter.acquisitions() - before, 2);
    }
    Ok(())
}

// Interrupts a callee, then shows its caller, whose cached interrupt-page ptr
// has become stale, refreshes that cache at its next checkpoint without yielding.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
async fn mmu_interruption_refreshes_stale_caller_cache(config: &mut Config) -> Result<()> {
    let interrupter = Arc::new(armable::ArmableInterrupter::default());
    config
        .mmu_interruption(true)
        .with_mmu_interrupter(interrupter.clone());
    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"(module
             (import "" "protect" (func $protect))
             (func $callee)
             (func (export "caller") (result i32)
                (local $i i32)
                ;; After the prologue, so the caller has cached the page
                call $protect
                call $callee
                ;; Loop 3 times: the first loop-backedge checkpoint faults on the
                ;; stale ptr and refreshes it; later ones must pass cleanly.
                (loop $l
                  (br_if $l (i32.lt_u
                    (local.tee $i (i32.add (local.get $i) (i32.const 1)))
                    (i32.const 3))))
                i32.const 1
             )
           )"#,
    )?;

    let mut store = Store::new(&engine, ());
    let protect = wasmtime::Func::wrap(&mut store, {
        let interrupter = interrupter.clone();
        move || interrupter.protect_latest()
    });
    let instance = Instance::new_async(&mut store, &module, &[protect.into()]).await?;
    let caller = instance.get_typed_func::<(), i32>(&mut store, "caller")?;

    let before = interrupter.acquisitions();
    assert_eq!(caller.call_async(&mut store, ()).await?, 1);
    // One at entry and one when the callee resumes. A yield on the caller's
    // stale checkpoint would add a third.
    assert_eq!(interrupter.acquisitions() - before, 2);
    Ok(())
}

// Runs a Wasm function to an MMU-interrupt check point, lets it yield, then
// drops the future driving it. This exercises the cancellation path of
// `maybe_yield_fiber()`, which should unwind the stack cleanly.
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

    let interrupter = Arc::new(armable::ArmableInterrupter::default());
    config
        .mmu_interruption(true)
        .with_mmu_interrupter(interrupter.clone());
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
    interrupter.arm();

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
        // When `maybe_yield_fiber()` switches fibers, the old fiber's
        // `Pending` should percolate up via `block_on()`.
        Poll::Pending => {}
        Poll::Ready(r) => panic!(
            "the fiber should have suspended itself, returning Pending, but it returned Ready({r:?}) instead"
        ),
    }

    // Drop the suspended future. This triggers `FiberFuture::Drop` →
    // `StoreFiber::dispose()`, which gets cranky that we're dropping a fiber
    // that isn't done and resumes the fiber with an `Err`. This triggers the
    // `maybe_yield_fiber` path we're interested in: stack unwinding.
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

// Shows constructing an engine with an interrupter but no compiled-in
// checkpoints raises an error. The interrupter can't do anything without the
// checks!
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn interrupter_requires_mmu_interruption(config: &mut Config) -> Result<()> {
    config.mmu_interruption(false);
    config.with_mmu_interrupter(Arc::new(armable::ArmableInterrupter::default()));
    let err = Engine::new(config).expect_err("engine creation should fail");
    assert_eq!(
        err.to_string(),
        "an MMU interrupter was configured, but MMU interruption is not enabled",
    );
    Ok(())
}

// Shows Wasm can't be loaded to run without an interrupter to service its
// checks.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn mmu_interruption_requires_interrupter(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    let engine = Engine::new(config)?;
    let err = Module::new(&engine, "(module)").expect_err("module loading should fail");
    let err = format!("{err:?}");
    assert!(
        err.contains(
            "MMU interruption requires an MMU interrupter; see `Config::with_mmu_interrupter()`"
        ),
        "unexpected error: {err}"
    );
    Ok(())
}

// Host functions need no interrupter. This shows they can run without one
// configured.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
async fn host_func_runs_without_interrupter(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());
    let add = wasmtime::Func::wrap(&mut store, |a: i32, b: i32| a + b);
    let add = add.typed::<(i32, i32), i32>(&store)?;
    assert_eq!(add.call_async(&mut store, (1, 2)).await?, 3);
    Ok(())
}

// Shows that, like an unincremented epoch, an unticked `TimingWheelInterrupter`
// never interrupts, letting Wasm run to completion.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
async fn unticked_timing_wheel_runs_wasm(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.with_mmu_interrupter(Arc::new(wasmtime::TimingWheelInterrupter::new(0)));
    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"(module
             (func (export "one") (result i32)
                (loop (br_if 0 (i32.const 0)))
                i32.const 1
             )
           )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new_async(&mut store, &module, &[]).await?;
    let one = instance.get_typed_func::<(), i32>(&mut store, "one")?;
    assert_eq!(one.call_async(&mut store, ()).await?, 1);
    Ok(())
}

// Shows that a ticked `TimingWheelInterrupter` interrupts a busy loop but only
// after it has run for at least the timeslice.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_os = "linux"
))]
#[wasmtime_test(strategies(only(CraneliftNative)))]
async fn timing_wheel_interrupts_after_timeslice(config: &mut Config) -> Result<()> {
    const TIMESLICE: u32 = 3;
    const TOTAL_TICKS: u32 = 20;

    /// Records how many ticks each page was held for
    #[derive(Default)]
    struct Ticks {
        count: AtomicU32,
        acquired_at: AtomicU32,
        held: Mutex<Vec<u32>>,
    }

    struct Counted {
        wheel: TimingWheelInterrupter,
        ticks: Arc<Ticks>,
    }

    struct CountedPage {
        inner: Box<dyn PageHandle>,
        ticks: Arc<Ticks>,
    }

    impl PageHandle for CountedPage {
        fn page_ptr(&self) -> std::ptr::NonNull<std::ffi::c_void> {
            self.inner.page_ptr()
        }
    }

    impl Drop for CountedPage {
        fn drop(&mut self) {
            let held = self.ticks.count.load(SeqCst) - self.ticks.acquired_at.load(SeqCst);
            self.ticks.held.lock().unwrap().push(held);
        }
    }

    impl MmuInterrupter for Counted {
        fn acquire_page(&self) -> Box<dyn PageHandle> {
            self.ticks
                .acquired_at
                .store(self.ticks.count.load(SeqCst), SeqCst);
            Box::new(CountedPage {
                inner: self.wheel.acquire_page(),
                ticks: self.ticks.clone(),
            })
        }
    }

    let ticks = Arc::new(Ticks::default());
    let counted = Arc::new(Counted {
        wheel: TimingWheelInterrupter::new(TIMESLICE),
        ticks: ticks.clone(),
    });
    config.mmu_interruption(true);
    config.with_mmu_interrupter(counted.clone());
    let engine = Engine::new(config)?;
    let module = Module::new(
        &engine,
        r#"(module
             (import "" "tick" (func $tick (result i32)))
             (func (export "spin")
                (loop (br_if 0 (i32.eqz (call $tick))))
             )
           )"#,
    )?;
    let mut store = Store::new(&engine, ());
    let tick = wasmtime::Func::wrap(&mut store, {
        let counted = counted.clone();
        move || {
            counted.wheel.tick();
            i32::from(counted.ticks.count.fetch_add(1, SeqCst) + 1 >= TOTAL_TICKS)
        }
    });
    let instance = Instance::new_async(&mut store, &module, &[tick.into()]).await?;
    let spin = instance.get_typed_func::<(), ()>(&mut store, "spin")?;
    ticks.held.lock().unwrap().clear();

    spin.call_async(&mut store, ()).await?;

    let held = ticks.held.lock().unwrap();
    assert!(held.len() > 1, "the loop should have been interrupted");
    // The last release is the call finishing, not an interruption.
    for interrupted in &held[..held.len() - 1] {
        assert_eq!(*interrupted, TIMESLICE + 1);
    }
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

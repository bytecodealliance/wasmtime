//! A standalone test to assert that Wasmtime can operate in "no signal handlers
//! mode"
//!
//! This is a test for `Config::signals_based_traps(false)` which resides in its
//! own binary to assert properties about signal handlers that Wasmtime uses.
//! Due to the global nature of signals no other tests can be in this binary.
//! This will ensure that various trapping scenarios all work and additionally
//! signal handlers are not registered.

use wasmtime::Result;
use wasmtime::{Config, Engine, Instance, Module, Store, Trap};

#[test]
#[cfg_attr(miri, ignore)]
fn no_host_trap_handlers() -> Result<()> {
    let mut config = Config::new();
    config.signals_based_traps(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
                (memory 1)

                (func (export "load") (param i32) (result i32)
                    (i32.load (local.get 0)))

                (func (export "div") (param i32 i32) (result i32)
                    (i32.div_s (local.get 0) (local.get 1)))

                (func (export "unreachable") unreachable)
                (func $oflow (export "overflow") call $oflow)
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let load = instance.get_typed_func::<i32, i32>(&mut store, "load")?;
    let div = instance.get_typed_func::<(i32, i32), i32>(&mut store, "div")?;
    let unreachable = instance.get_typed_func::<(), ()>(&mut store, "unreachable")?;
    let overflow = instance.get_typed_func::<(), ()>(&mut store, "overflow")?;

    let trap = load
        .call(&mut store, 1 << 20)
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::MemoryOutOfBounds);

    let trap = div
        .call(&mut store, (1, 0))
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::IntegerDivisionByZero);

    let trap = unreachable
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::UnreachableCodeReached);

    let trap = overflow
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert_eq!(trap, Trap::StackOverflow);

    assert_host_signal_handlers_are_unset();

    Ok(())
}

fn assert_host_signal_handlers_are_unset() {
    #[cfg(unix)]
    unsafe {
        let mut prev = std::mem::zeroed::<libc::sigaction>();
        let rc = libc::sigaction(libc::SIGILL, std::ptr::null(), &mut prev);
        assert_eq!(rc, 0);
        assert_eq!(
            prev.sa_sigaction,
            libc::SIG_DFL,
            "fault handler was installed when it shouldn't have been"
        );
    }
    #[cfg(windows)]
    {
        // Note that this can't be checked on Windows because vectored exception
        // handlers work a bit differently and aren't as "global" as a signal
        // handler. For now rely on the check above on unix to also guarantee
        // that on Windows we don't register any vectored exception handlers.
    }
}

/// Generates a module with a function `$big` whose frame is large: it loads
/// `num_live` values out of linear memory, calls `$callee`, and then sums all
/// of the loaded values, which keeps them all live across the call and so
/// forces the register allocator to spill them all to the stack. Loads are
/// used (rather than, e.g., constants) so that they can't be rematerialized
/// after the call.
///
/// `$big` takes a recursion depth, recursing until it reaches zero, and is
/// exported as `big`.
fn big_frame_module(num_live: usize) -> String {
    let mut wat = String::new();
    wat.push_str("(module\n");
    wat.push_str("  (memory 1)\n");
    wat.push_str("  (func $big (export \"big\") (param $depth i32) (result i64)\n");
    for i in 0..num_live {
        wat.push_str(&format!("    (local $l{i} i64)\n"));
    }
    for i in 0..num_live {
        wat.push_str(&format!(
            "    (local.set $l{i} (i64.load offset={} (i32.const 0)))\n",
            (i * 8) % 65536
        ));
    }
    // Recurse, or not, with all of the locals above live across the call.
    wat.push_str(
        "    (if (result i64) (local.get $depth)\n\
         \x20     (then (call $big (i32.sub (local.get $depth) (i32.const 1))))\n\
         \x20     (else (i64.const 0)))\n",
    );
    for i in 0..num_live {
        wat.push_str(&format!("    (i64.add (local.get $l{i}))\n"));
    }
    wat.push_str("  )\n");
    // Initialize memory so the sum is nontrivial: each word holds 1.
    wat.push_str("  (func (export \"init\")\n");
    wat.push_str("    (local $i i32)\n");
    wat.push_str(
        "    (loop $l\n\
         \x20     (i64.store (local.get $i) (i64.const 1))\n\
         \x20     (local.set $i (i32.add (local.get $i) (i32.const 8)))\n\
         \x20     (br_if $l (i32.lt_u (local.get $i) (i32.const 65536)))))\n",
    );
    wat.push_str(")\n");
    wat
}

/// Number of values live across the call in `big_frame_module`. At 8 bytes
/// each, the frame is at least 32 KiB.
const NUM_LIVE: usize = 4096;

/// Tests that a stack overflow is detected and reported as a trap, with
/// signals-based traps disabled, when the overflowing function has a frame
/// much bigger than the whole wasm stack: the check must happen before any of
/// that frame is allocated or written to.
#[test]
#[cfg_attr(miri, ignore)]
fn stack_overflow_big_frame_tiny_stack() -> Result<()> {
    let mut config = Config::new();
    config.signals_based_traps(false);
    config.max_wasm_stack(4096);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, big_frame_module(NUM_LIVE))?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let big = instance.get_typed_func::<i32, i64>(&mut store, "big")?;

    // Even a non-recursive call overflows the tiny stack: this is the first
    // wasm frame on the stack, directly called from the host.
    let err = big.call(&mut store, 0).unwrap_err();
    assert_eq!(*err.downcast_ref::<Trap>().unwrap(), Trap::StackOverflow);

    // The store is still usable afterwards.
    let init = instance.get_typed_func::<(), ()>(&mut store, "init")?;
    init.call(&mut store, ())?;

    assert_host_signal_handlers_are_unset();
    Ok(())
}

/// Like `stack_overflow_big_frame_tiny_stack`, but the stack is big enough for
/// a few frames of a big-frame function before overflowing, so that the
/// overflow is detected in a function called from other wasm.
#[test]
#[cfg_attr(miri, ignore)]
fn stack_overflow_big_frame_recursion() -> Result<()> {
    let mut config = Config::new();
    config.signals_based_traps(false);
    config.max_wasm_stack(512 << 10);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, big_frame_module(NUM_LIVE))?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let init = instance.get_typed_func::<(), ()>(&mut store, "init")?;
    let big = instance.get_typed_func::<i32, i64>(&mut store, "big")?;
    init.call(&mut store, ())?;

    // A few levels of recursion fit fine, and the stack-limit check in the
    // prologue doesn't disturb the function's arguments.
    let depth = 3;
    assert_eq!(
        big.call(&mut store, depth)?,
        (NUM_LIVE as i64) * (i64::from(depth) + 1)
    );

    // Unbounded recursion overflows the stack, and is reported as a trap.
    let err = big.call(&mut store, 1_000_000).unwrap_err();
    assert_eq!(*err.downcast_ref::<Trap>().unwrap(), Trap::StackOverflow);

    // The backtrace walks through all of the `big` frames that fit on the
    // stack. The overflowing call itself has no frame yet, so the most recent
    // frame is that of its caller.
    let bt = err.downcast_ref::<wasmtime::WasmBacktrace>().unwrap();
    let max_frames = (512 << 10) / (NUM_LIVE * 8);
    assert!(
        bt.frames().len() > 1 && bt.frames().len() <= max_frames,
        "unexpected backtrace length {}: {bt:?}",
        bt.frames().len()
    );
    for frame in bt.frames() {
        assert_eq!(frame.func_name(), Some("big"));
    }

    // The store is still usable afterwards.
    assert_eq!(big.call(&mut store, 0)?, NUM_LIVE as i64);

    assert_host_signal_handlers_are_unset();
    Ok(())
}

/// Tests stack-overflow detection, with signals-based traps disabled, in a
/// function with many results: enough that Cranelift passes an implicit
/// return-area pointer, which must not disturb where the stack-overflow
/// handler finds its `vmctx` argument.
#[test]
#[cfg_attr(miri, ignore)]
fn stack_overflow_many_results() -> Result<()> {
    const N: usize = 16;
    let results = vec!["i64"; N].join(" ");
    let gets = "(local.get $x) ".repeat(N);
    let wat = format!(
        r#"
            (module
                (func $f (export "f") (param $depth i32) (param $x i64) (result {results})
                    (if (result {results}) (local.get $depth)
                        (then (call $f (i32.sub (local.get $depth) (i32.const 1))
                                       (i64.add (local.get $x) (i64.const 1))))
                        (else {gets}))))
        "#
    );
    let mut config = Config::new();
    config.signals_based_traps(false);
    config.cranelift_debug_verifier(true);
    config.max_wasm_stack(64 << 10);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_func(&mut store, "f").unwrap();

    let mut out = vec![wasmtime::Val::I64(0); N];
    f.call(&mut store, &[3i32.into(), 10i64.into()], &mut out)?;
    for v in &out {
        assert_eq!(v.unwrap_i64(), 13);
    }

    let err = f
        .call(&mut store, &[i32::MAX.into(), 0i64.into()], &mut out)
        .unwrap_err();
    assert_eq!(*err.downcast_ref::<Trap>().unwrap(), Trap::StackOverflow);

    // The store is still usable afterwards.
    f.call(&mut store, &[1i32.into(), 5i64.into()], &mut out)?;
    assert_eq!(out[N - 1].unwrap_i64(), 6);

    assert_host_signal_handlers_are_unset();
    Ok(())
}

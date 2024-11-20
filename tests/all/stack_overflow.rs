#![cfg(not(miri))]

use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::*;
use wasmtime_test_macros::wasmtime_test;

#[test]
fn host_always_has_some_stack() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);
    // assume hosts always have at least 128k of stack
    const HOST_STACK: usize = 128 * 1024;

    let mut store = if cfg!(target_arch = "x86_64") {
        let mut config = Config::new();
        // Force cranelift-based libcalls to show up by ensuring that platform
        // support is turned off.
        unsafe {
            config.cranelift_flag_set("has_avx", "false");
            config.cranelift_flag_set("has_sse42", "false");
            config.cranelift_flag_set("has_sse41", "false");
            config.cranelift_flag_set("has_ssse3", "false");
            config.cranelift_flag_set("has_sse3", "false");
        }
        Store::new(&Engine::new(&config)?, ())
    } else {
        Store::<()>::default()
    };

    // Create a module that's infinitely recursive, but calls the host on each
    // level of wasm stack to always test how much host stack we have left.
    //
    // Each of the function exports of this module calls out to the host in a
    // different way, and each one is tested below to make sure that the way of
    // exiting out to the host is tested thoroughly.
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "" "" (func $host1))
                (import "" "" (func $host2))

                ;; exit via wasm-to-native trampoline
                (func $recursive1 (export "f1")
                    call $host1
                    call $recursive1)

                ;; exit via wasm-to-array trampoline
                (func $recursive2 (export "f2")
                    call $host2
                    call $recursive2)

                ;; exit via a wasmtime-based libcall
                (memory 1)
                (func $recursive3 (export "f3")
                    (drop (memory.grow (i32.const 0)))
                    call $recursive3)

                ;; exit via a cranelift-based libcall
                (func $recursive4 (export "f4")
                    (drop (call $f32_ceil (f32.const 0)))
                    call $recursive4)
                (func $f32_ceil (param f32) (result f32)
                    (f32.ceil (local.get 0)))
            )
        "#,
    )?;
    let host1 = Func::wrap(&mut store, test_host_stack);
    let ty = FuncType::new(store.engine(), [], []);
    let host2 = Func::new(&mut store, ty, |_, _, _| {
        test_host_stack();
        Ok(())
    });
    let instance = Instance::new(&mut store, &module, &[host1.into(), host2.into()])?;
    let f1 = instance.get_typed_func::<(), ()>(&mut store, "f1")?;
    let f2 = instance.get_typed_func::<(), ()>(&mut store, "f2")?;
    let f3 = instance.get_typed_func::<(), ()>(&mut store, "f3")?;
    let f4 = instance.get_typed_func::<(), ()>(&mut store, "f4")?;

    // Make sure that our function traps and the trap says that the call stack
    // has been exhausted.
    let hits1 = HITS.load(SeqCst);
    let trap = f1.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::StackOverflow);
    let hits2 = HITS.load(SeqCst);
    let trap = f2.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::StackOverflow);
    let hits3 = HITS.load(SeqCst);
    let trap = f3.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::StackOverflow);
    let hits4 = HITS.load(SeqCst);
    let trap = f4.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::StackOverflow);
    let hits5 = HITS.load(SeqCst);

    // Additionally, however, and this is the crucial test, make sure that the
    // host function actually completed. If HITS is 1 then we entered but didn't
    // exit meaning we segfaulted while executing the host, yet still tried to
    // recover from it with longjmp.
    assert_eq!(hits1, 0);
    assert_eq!(hits2, 0);
    assert_eq!(hits3, 0);
    assert_eq!(hits4, 0);
    assert_eq!(hits5, 0);

    return Ok(());

    fn test_host_stack() {
        HITS.fetch_add(1, SeqCst);
        assert!(consume_some_stack(0, HOST_STACK) > 0);
        HITS.fetch_sub(1, SeqCst);
    }

    #[inline(never)]
    fn consume_some_stack(ptr: usize, stack: usize) -> usize {
        if stack == 0 {
            return ptr;
        }
        let mut space = [0u8; 1024];
        consume_some_stack(space.as_mut_ptr() as usize, stack.saturating_sub(1024))
    }
}

// Don't test Cranelift here because it takes too long to compiler in debug
// mode.
#[wasmtime_test]
fn big_stack_works_ok(config: &mut Config) -> Result<()> {
    const N: usize = 10000;

    // Build a module with a function that uses a very large amount of stack space,
    // modeled here by calling an i64-returning-function many times followed by
    // adding them all into one i64.
    //
    // This should exercise the ability to consume multi-page stacks and
    // only touch a few internals of it at a time.
    let mut s = String::new();
    s.push_str("(module\n");
    s.push_str("(func (export \"\") (result i64)\n");
    s.push_str("i64.const 0\n");
    for _ in 0..N {
        s.push_str("call $get\n");
    }
    for _ in 0..N {
        s.push_str("i64.add\n");
    }
    s.push_str(")\n");
    s.push_str("(func $get (result i64) i64.const 0)\n");
    s.push_str(")\n");

    config.cranelift_opt_level(OptLevel::None);
    config.cranelift_regalloc_algorithm(RegallocAlgorithm::SinglePass);
    let engine = Engine::new(config)?;

    let mut store = Store::new(&engine, ());
    let module = Module::new(store.engine(), &s)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_typed_func::<(), i64>(&mut store, "")?;
    assert_eq!(func.call(&mut store, ())?, 0);
    Ok(())
}

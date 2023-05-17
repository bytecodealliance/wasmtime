#![cfg(not(miri))]

use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::*;

#[test]
fn host_always_has_some_stack() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);
    // assume hosts always have at least 128k of stack
    const HOST_STACK: usize = 128 * 1024;

    let mut store = Store::<()>::default();

    // Create a module that's infinitely recursive, but calls the host on each
    // level of wasm stack to always test how much host stack we have left.
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "" "" (func $host))
                (func $recursive (export "foo")
                    call $host
                    call $recursive)
            )
        "#,
    )?;
    let func = Func::wrap(&mut store, test_host_stack);
    let instance = Instance::new(&mut store, &module, &[func.into()])?;
    let foo = instance.get_typed_func::<(), ()>(&mut store, "foo")?;

    // Make sure that our function traps and the trap says that the call stack
    // has been exhausted.
    let trap = foo.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(trap, Trap::StackOverflow);

    // Additionally, however, and this is the crucial test, make sure that the
    // host function actually completed. If HITS is 1 then we entered but didn't
    // exit meaning we segfaulted while executing the host, yet still tried to
    // recover from it with longjmp.
    assert_eq!(HITS.load(SeqCst), 0);

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

#[test]
fn big_stack_works_ok() -> Result<()> {
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

    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &s)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_typed_func::<(), i64>(&mut store, "")?;
    assert_eq!(func.call(&mut store, ())?, 0);
    Ok(())
}

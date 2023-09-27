#![cfg(not(miri))]

use super::TypedFuncExt;
use anyhow::bail;
use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{CallHook, Store, StoreContextMut, Trap};

#[test]
// Stolen from func.rs
fn thunks() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (func (export "thunk"))
                (func (export "thunk-trap") unreachable)
            )
            (core instance $i (instantiate $m))
            (func (export "thunk")
                (canon lift (core func $i "thunk"))
            )
            (func (export "thunk-trap")
                (canon lift (core func $i "thunk-trap"))
            )
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, State::default());
    store.call_hook(State::call_hook);

    let instance = Linker::new(&engine).instantiate(&mut store, &component)?;
    instance
        .get_typed_func::<(), ()>(&mut store, "thunk")?
        .call_and_post_return(&mut store, ())?;
    let err = instance
        .get_typed_func::<(), ()>(&mut store, "thunk-trap")?
        .call(&mut store, ())
        .unwrap_err();
    assert_eq!(err.downcast::<Trap>()?, Trap::UnreachableCodeReached);

    assert_eq!(store.data().calls_into_wasm, 2);
    assert_eq!(store.data().returns_from_wasm, 2);
    assert_eq!(store.data().calls_into_host, 0);
    assert_eq!(store.data().returns_from_host, 0);

    Ok(())
}

#[test]
// Stolen from import.rs
fn simple() -> Result<()> {
    let component = r#"
        (component
            (import "a" (func $log (param "a" string)))

            (core module $libc
                (memory (export "memory") 1)

                (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                    unreachable)
            )
            (core instance $libc (instantiate $libc))
            (core func $log_lower
                (canon lower (func $log) (memory $libc "memory") (realloc (func $libc "realloc")))
            )
            (core module $m
                (import "libc" "memory" (memory 1))
                (import "host" "log" (func $log (param i32 i32)))

                (func (export "call")
                    i32.const 5
                    i32.const 11
                    call $log)

                (data (i32.const 5) "hello world")
            )
            (core instance $i (instantiate $m
                (with "libc" (instance $libc))
                (with "host" (instance (export "log" (func $log_lower))))
            ))
            (func (export "call")
                (canon lift (core func $i "call"))
            )
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, (State::default(), None));
    store.call_hook(|t, h| State::call_hook(&mut t.0, h));
    assert!(store.data().1.is_none());

    // First, test the static API

    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "a",
        |mut store: StoreContextMut<'_, (State, Option<String>)>,
         (arg,): (WasmStr,)|
         -> Result<_> {
            let s = arg.to_str(&store)?.to_string();
            assert!(store.data().1.is_none());
            *(&mut store.data_mut().1) = Some(s);
            Ok(())
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    instance
        .get_typed_func::<(), ()>(&mut store, "call")?
        .call(&mut store, ())?;
    assert_eq!(store.data().1.as_ref().unwrap(), "hello world");

    assert_eq!(store.data().0.calls_into_wasm, 1);
    assert_eq!(store.data().0.returns_from_wasm, 1);
    assert_eq!(store.data().0.calls_into_host, 1);
    assert_eq!(store.data().0.returns_from_host, 1);

    // Next, test the dynamic API

    *(&mut store.data_mut().1) = None;
    let mut linker = Linker::new(&engine);
    linker.root().func_new(
        &component,
        "a",
        |mut store: StoreContextMut<'_, (State, Option<String>)>, args, _results| {
            if let Val::String(s) = &args[0] {
                assert!(store.data().1.is_none());
                *(&mut store.data_mut().1) = Some(s.to_string());
                Ok(())
            } else {
                panic!()
            }
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    instance
        .get_func(&mut store, "call")
        .unwrap()
        .call(&mut store, &[], &mut [])?;
    assert_eq!(store.data().1.as_ref().unwrap(), "hello world");

    assert_eq!(store.data().0.calls_into_wasm, 2);
    assert_eq!(store.data().0.returns_from_wasm, 2);
    assert_eq!(store.data().0.calls_into_host, 2);
    assert_eq!(store.data().0.returns_from_host, 2);
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum Context {
    Host,
    Wasm,
}

#[derive(Debug)]
struct State {
    context: Vec<Context>,

    calls_into_host: usize,
    returns_from_host: usize,
    calls_into_wasm: usize,
    returns_from_wasm: usize,

    trap_next_call_host: bool,
    trap_next_return_host: bool,
    trap_next_call_wasm: bool,
    trap_next_return_wasm: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            context: Vec::new(),
            calls_into_host: 0,
            returns_from_host: 0,
            calls_into_wasm: 0,
            returns_from_wasm: 0,
            trap_next_call_host: false,
            trap_next_return_host: false,
            trap_next_call_wasm: false,
            trap_next_return_wasm: false,
        }
    }
}

impl State {
    // This implementation asserts that hooks are always called in a stack-like manner.
    fn call_hook(&mut self, s: CallHook) -> Result<()> {
        match s {
            CallHook::CallingHost => {
                self.calls_into_host += 1;
                if self.trap_next_call_host {
                    bail!("call_hook: trapping on CallingHost");
                } else {
                    self.context.push(Context::Host);
                }
            }
            CallHook::ReturningFromHost => match self.context.pop() {
                Some(Context::Host) => {
                    self.returns_from_host += 1;
                    if self.trap_next_return_host {
                        bail!("call_hook: trapping on ReturningFromHost");
                    }
                }
                c => panic!(
                    "illegal context: expected Some(Host), got {:?}. remaining: {:?}",
                    c, self.context
                ),
            },
            CallHook::CallingWasm => {
                self.calls_into_wasm += 1;
                if self.trap_next_call_wasm {
                    bail!("call_hook: trapping on CallingWasm");
                } else {
                    self.context.push(Context::Wasm);
                }
            }
            CallHook::ReturningFromWasm => match self.context.pop() {
                Some(Context::Wasm) => {
                    self.returns_from_wasm += 1;
                    if self.trap_next_return_wasm {
                        bail!("call_hook: trapping on ReturningFromWasm");
                    }
                }
                c => panic!(
                    "illegal context: expected Some(Wasm), got {:?}. remaining: {:?}",
                    c, self.context
                ),
            },
        }
        Ok(())
    }
}

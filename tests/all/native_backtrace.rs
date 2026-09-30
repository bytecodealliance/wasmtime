#![cfg(not(miri))]

use wasmtime::{Caller, Config, Engine, Func, Inlining, Instance, Module, Result, Store};

fn check_stack(wat: &str, expected: &[&str], value: i32) -> Result<()> {
    let mut config = Config::new();
    config
        .native_unwind_info(true)
        .compiler_inlining(Inlining::No);
    let engine = Engine::new(&config)?;
    // Interpreted Wasm calls do not create native frames for the unwinder to find.
    if engine.is_pulley() {
        return Ok(());
    }
    let module = Module::new(&engine, wat)?;
    let start = module.text().as_ptr() as usize;
    let functions = module
        .functions()
        .map(|f| (f.name, start + f.offset, f.len))
        .collect::<Vec<_>>();
    let mut store = Store::new(&engine, Vec::<usize>::with_capacity(128));
    let capture = Func::wrap(&mut store, |mut caller: Caller<'_, Vec<usize>>| {
        backtrace::trace(|frame| {
            let frames = caller.data_mut();
            // Stop before growing the buffer to avoid allocating in the callback.
            if frames.len() == frames.capacity() {
                return false;
            }
            frames.push(frame.ip() as usize);
            true
        });
        10_i32
    });
    let instance = Instance::new(&mut store, &module, &[capture.into()])?;
    let actual = instance
        .get_typed_func::<(), i32>(&mut store, "run")?
        .call(&mut store, ())?;
    assert_eq!(actual, value);
    let names = store
        .data()
        .iter()
        .filter_map(|pc| {
            // A recovered return address may point one byte past a function.
            functions
                .iter()
                .find(|(_, lo, len)| *pc > *lo && *pc <= *lo + *len)
                .and_then(|(name, _, _)| name.as_deref())
        })
        .collect::<Vec<_>>();
    assert_eq!(names, expected);
    Ok(())
}

#[test]
fn native_unwind_through_host_call() -> Result<()> {
    check_stack(
        r#"(module
            (import "" "capture" (func $capture (result i32)))
            (func $inner (result i32) call $capture i32.const 1 i32.add)
            (func $middle (result i32) call $inner i32.const 2 i32.add)
            (func $run (export "run") (result i32) call $middle i32.const 4 i32.add))"#,
        &["inner", "middle", "run"],
        17,
    )
}

#[test]
fn native_unwind_after_tail_call_changes_stack_arguments() -> Result<()> {
    // More arguments than fit in registers forces the tail caller to resize
    // the stack. Its frame must disappear, while its caller remains walkable.
    let params = "i32 ".repeat(24);
    let args = "i32.const 1 ".repeat(24);
    check_stack(
        &format!(
            r#"(module
                (import "" "capture" (func $capture (result i32)))
                (func $inner (param {params}) (result i32)
                    call $capture local.get 23 i32.add)
                (func $tail (result i32) {args} return_call $inner)
                (func $run (export "run") (result i32)
                    call $tail i32.const 4 i32.add))"#,
        ),
        &["inner", "run"],
        15,
    )
}

#[test]
fn native_unwind_after_tail_call_to_host() -> Result<()> {
    check_stack(
        r#"(module
            (import "" "capture" (func $capture (result i32)))
            (func $tail (result i32) return_call $capture)
            (func $run (export "run") (result i32) call $tail i32.const 4 i32.add))"#,
        &["run"],
        14,
    )
}

#[test]
fn native_unwind_after_tail_call_removes_stack_arguments() -> Result<()> {
    let params = "i32 ".repeat(24);
    let args = "i32.const 1 ".repeat(24);
    check_stack(
        &format!(
            r#"(module
                (import "" "capture" (func $capture (result i32)))
                (func $inner (result i32) call $capture i32.const 1 i32.add)
                (func $tail (param {params}) (result i32) return_call $inner)
                (func $run (export "run") (result i32)
                    {args} call $tail i32.const 4 i32.add))"#,
        ),
        &["inner", "run"],
        15,
    )
}

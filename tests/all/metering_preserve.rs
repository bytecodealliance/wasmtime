#![cfg(all(not(miri), feature = "cranelift", feature = "gc"))]

use std::{
    borrow::Cow,
    future::Future,
    task::{Context, Poll, Waker},
};
use wasm_encoder::{
    BlockType, Catch, CodeSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, Instruction as I, Module as EncodedModule, TagKind, TagSection,
    TagType, TypeSection, ValType,
};
use wasmtime::{
    Config, Engine, Error, Inlining, Instance, Linker, Module, Store, Trap, UpdateDeadline, V128,
};

fn engine() -> Engine {
    let mut config = Config::new();
    #[cfg(feature = "gc-copying")]
    config.collector(wasmtime::Collector::Copying);
    #[cfg(all(feature = "gc-null", not(feature = "gc-copying")))]
    config.collector(wasmtime::Collector::Null);
    config
        .consume_fuel(true)
        .epoch_interruption(true)
        .compiler_inlining(Inlining::No)
        .wasm_exceptions(true);
    Engine::new(&config).unwrap()
}

fn finish<T>(future: impl Future<Output = T>) -> (T, usize) {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for polls in 1..10_000 {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return (value, polls);
        }
    }
    panic!("metered invocation did not finish");
}

fn register_module() -> Vec<u8> {
    let mut m = EncodedModule::new();
    let mut types = TypeSection::new();
    types.ty().function(
        [
            ValType::I32,
            ValType::F64,
            ValType::F64,
            ValType::I64,
            ValType::I64,
            ValType::V128,
        ],
        [ValType::I64, ValType::F64, ValType::V128],
    );
    m.section(&types);
    let mut functions = FunctionSection::new();
    functions.function(0);
    m.section(&functions);
    let mut exports = ExportSection::new();
    exports.export("mixed", ExportKind::Func, 0);
    m.section(&exports);
    let mut f = Function::new([]);
    for i in [
        I::Block(BlockType::Empty),
        I::Loop(BlockType::Empty),
        I::LocalGet(0),
        I::I32Eqz,
        I::BrIf(1),
        I::LocalGet(1),
        I::LocalGet(2),
        I::F64Add,
        I::LocalSet(1),
        I::LocalGet(3),
        I::LocalGet(4),
        I::I64Xor,
        I::LocalSet(3),
        I::LocalGet(5),
        I::I32Const(7),
        I::I8x16Splat,
        I::I8x16Add,
        I::LocalSet(5),
        I::LocalGet(0),
        I::I32Const(1),
        I::I32Sub,
        I::LocalSet(0),
        I::Br(0),
        I::End,
        I::End,
        I::LocalGet(3),
        I::LocalGet(1),
        I::LocalGet(5),
        I::End,
    ] {
        f.instruction(&i);
    }
    let mut code = CodeSection::new();
    code.function(&f);
    m.section(&code);
    m.finish()
}

fn exception_module() -> Vec<u8> {
    let mut m = EncodedModule::new();
    let mut t = TypeSection::new();
    t.ty().function([ValType::I64], []);
    t.ty().function([ValType::I64], [ValType::I64]);
    t.ty().function([], []);
    m.section(&t);
    let mut imports = ImportSection::new();
    imports.import("host", "tick", EntityType::Function(2));
    m.section(&imports);
    let mut f = FunctionSection::new();
    f.function(1).function(1);
    m.section(&f);
    let mut tags = TagSection::new();
    tags.tag(TagType {
        kind: TagKind::Exception,
        func_type_idx: 0,
    });
    m.section(&tags);
    let mut e = ExportSection::new();
    e.export("caught", ExportKind::Func, 1);
    m.section(&e);
    let mut c = CodeSection::new();
    let mut run = Function::new([]);
    for i in [
        I::Block(BlockType::Result(ValType::I64)),
        I::TryTable(
            BlockType::Result(ValType::I64),
            Cow::Owned(vec![Catch::One { tag: 0, label: 0 }]),
        ),
        I::Call(0),
        I::LocalGet(0),
        I::Call(2),
        I::End,
        I::End,
        I::End,
    ] {
        run.instruction(&i);
    }
    c.function(&run);
    let mut throwing = Function::new([]);
    for i in [I::LocalGet(0), I::Throw(0), I::End] {
        throwing.instruction(&i);
    }
    c.function(&throwing);
    m.section(&c);
    m.finish()
}

#[test]
fn live_values_across_fuel_refills_and_epoch_updates() {
    let engine = engine();
    let module = Module::new(&engine, register_module()).unwrap();
    let vector = 0x0102030405060708090a0b0c0d0e0f10_u128;
    let expected_vector =
        u128::from_le_bytes(vector.to_le_bytes().map(|lane| lane.wrapping_add(13 * 7)));
    for interval in [None, Some(1), Some(7), Some(31)] {
        for mode in [0, 1, 2, 3] {
            for fuel in [0, 1, 19, 127, 4096] {
                let mut store = Store::new(&engine, 0_u32);
                let instance = Instance::new(&mut store, &module, &[]).unwrap();
                store.set_fuel(fuel).unwrap();
                if let Some(interval) = interval {
                    store.fuel_async_yield_interval(Some(interval)).unwrap();
                }
                store.set_epoch_deadline(if mode == 0 { 1 } else { 0 });
                store.epoch_deadline_callback(move |mut context| {
                    *context.data_mut() += 1;
                    Ok(match mode {
                        1 => UpdateDeadline::Continue(0),
                        2 => UpdateDeadline::Yield(0),
                        _ => UpdateDeadline::Continue(1),
                    })
                });
                let run = instance
                    .get_typed_func::<(i32, f64, f64, i64, i64, V128), (i64, f64, V128)>(
                        &mut store, "mixed",
                    )
                    .unwrap();
                let (result, polls) = finish(run.call_async(
                    &mut store,
                    (
                        13,
                        1.25,
                        0.5,
                        0x1234567890,
                        0x1020304050,
                        V128::from(vector),
                    ),
                ));
                if fuel < 280 {
                    assert_eq!(
                        result.unwrap_err().downcast_ref::<Trap>(),
                        Some(&Trap::OutOfFuel)
                    );
                    assert_eq!(store.get_fuel().unwrap(), 0);
                    continue;
                }
                let (integer, float, vector) = result.unwrap();
                assert_eq!(integer, 0x1234567890 ^ 0x1020304050);
                assert_eq!(float, 1.25 + 13.0 * 0.5);
                assert_eq!(vector.as_u128(), expected_vector);
                assert_eq!(store.get_fuel().unwrap(), fuel - 280);
                assert_eq!(polls > 1, interval.is_some() || mode == 2);
                match mode {
                    0 => assert_eq!(*store.data(), 0),
                    3 => assert_eq!(*store.data(), 1),
                    _ => assert!(*store.data() > 1),
                }
            }
        }
    }
}

#[test]
fn metering_traps_do_not_become_wasm_exceptions() {
    let engine = engine();
    let module = Module::new(&engine, exception_module()).unwrap();
    let mut linker = Linker::new(&engine);
    let tick = engine.clone();
    linker
        .func_wrap("host", "tick", move || {
            tick.increment_epoch();
        })
        .unwrap();
    for mode in 0..3 {
        for fuel in [0, 1, 7, 32, 1000] {
            let mut store = Store::new(&engine, 0_u32);
            let instance = linker.instantiate(&mut store, &module).unwrap();
            store.set_fuel(fuel).unwrap();
            store.set_epoch_deadline(1);
            store.epoch_deadline_callback(move |mut context| {
                *context.data_mut() += 1;
                match mode {
                    0 => Ok(UpdateDeadline::Continue(1)),
                    1 => Ok(UpdateDeadline::Interrupt),
                    _ => Err(Error::msg("epoch callback stop")),
                }
            });
            let run = instance
                .get_typed_func::<i64, i64>(&mut store, "caught")
                .unwrap();
            match run.call(&mut store, 0x1234_5678) {
                Ok(value) => {
                    assert_eq!(mode, 0);
                    assert_eq!(value, 0x1234_5678);
                }
                Err(error) => match error.downcast_ref::<Trap>() {
                    Some(Trap::OutOfFuel) => assert!(fuel < 1000),
                    Some(Trap::Interrupt) => assert_eq!(mode, 1),
                    None => {
                        assert_eq!(mode, 2);
                        assert_eq!(error.root_cause().to_string(), "epoch callback stop");
                    }
                    trap => panic!("unexpected trap: {trap:?}"),
                },
            }
            assert!(!store.has_pending_exception());
            if fuel == 1000 {
                assert_eq!(*store.data(), 1);
            }
        }
    }
}

#[test]
fn cancelled_fuel_yield_leaves_store_usable() {
    let engine = engine();
    let module = Module::new(&engine, register_module()).unwrap();
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    store.set_fuel(1_000_000).unwrap();
    store.set_epoch_deadline(1);
    store.fuel_async_yield_interval(Some(1)).unwrap();
    let run = instance
        .get_typed_func::<(i32, f64, f64, i64, i64, V128), (i64, f64, V128)>(&mut store, "mixed")
        .unwrap();
    {
        let mut future =
            std::pin::pin!(run.call_async(&mut store, (1_000_000, 1.0, 2.0, 3, 4, V128::from(0))));
        let mut context = Context::from_waker(Waker::noop());
        for _ in 0..32 {
            assert!(future.as_mut().poll(&mut context).is_pending());
        }
    }
    assert!(store.get_fuel().unwrap() < 1_000_000);
    store.fuel_async_yield_interval(None).unwrap();
    store.set_fuel(1000).unwrap();
    // Once fuel yielding has been enabled, this Store requires async entrypoints
    // even after its interval has been disabled.
    let (result, polls) = finish(run.call_async(&mut store, (1, 1.0, 2.0, 3, 4, V128::from(0))));
    let (integer, float, vector) = result.unwrap();
    assert_eq!(integer, 7);
    assert_eq!(float, 3.0);
    assert_eq!(vector.as_u128(), u128::from_le_bytes([7; 16]));
    assert_eq!(polls, 1);
}

#[test]
fn references_survive_metering_yields_and_moving_gc() {
    let engine = engine();
    let module = Module::new(
        &engine,
        r#"(module
        (import "host" "value" (func $value (result externref)))
        (import "host" "collect" (func $collect))
        (func (export "run") (result externref)
            (local $value externref) (local $n i32)
            call $value local.set $value
            i32.const 8 local.set $n
            (loop $again
                call $collect
                local.get $n i32.const 1 i32.sub local.tee $n
                br_if $again)
            local.get $value))"#,
    )
    .unwrap();
    let mut linker = Linker::new(&engine);
    linker.func_wrap("host", "value", |mut caller: wasmtime::Caller<'_, u32>|
        -> Result<Option<wasmtime::Rooted<wasmtime::ExternRef>>, Error> {
        Ok(Some(wasmtime::ExternRef::new(&mut caller, 42_i32)?))
    }).unwrap();
    linker
        .func_wrap(
            "host",
            "collect",
            |mut caller: wasmtime::Caller<'_, u32>| caller.gc(None),
        )
        .unwrap();
    let mut store = Store::new(&engine, 0_u32);
    store.set_fuel(100_000).unwrap();
    store.set_epoch_deadline(0);
    store.epoch_deadline_callback(|mut context| {
        *context.data_mut() += 1;
        Ok(UpdateDeadline::Yield(0))
    });
    let instance = linker.instantiate(&mut store, &module).unwrap();
    store.fuel_async_yield_interval(Some(1)).unwrap();
    let run = instance
        .get_typed_func::<(), Option<wasmtime::Rooted<wasmtime::ExternRef>>>(&mut store, "run")
        .unwrap();
    let (result, polls) = finish(run.call_async(&mut store, ()));
    let value = result.unwrap().unwrap();
    assert!(polls > 8);
    assert!(*store.data() > 1);
    assert_eq!(
        value.data(&store).unwrap().unwrap().downcast_ref::<i32>(),
        Some(&42)
    );
}

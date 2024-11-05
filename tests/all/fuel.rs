use wasmtime::*;
use wasmtime_test_macros::wasmtime_test;
use wast::parser::{self, Parse, ParseBuffer, Parser};
use wast::token::Span;

mod kw {
    wast::custom_keyword!(assert_fuel);
}

struct FuelWast<'a> {
    assertions: Vec<(Span, u64, wast::core::Module<'a>)>,
}

impl<'a> Parse<'a> for FuelWast<'a> {
    fn parse(parser: Parser<'a>) -> parser::Result<Self> {
        let mut assertions = Vec::new();
        while !parser.is_empty() {
            assertions.push(parser.parens(|p| {
                let span = p.parse::<kw::assert_fuel>()?.0;
                Ok((span, p.parse()?, p.parens(|p| p.parse())?))
            })?);
        }
        Ok(FuelWast { assertions })
    }
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn run(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let test = std::fs::read_to_string("tests/all/fuel.wast")?;
    let buf = ParseBuffer::new(&test)?;
    let mut wast = parser::parse::<FuelWast<'_>>(&buf)?;
    for (span, fuel, module) in wast.assertions.iter_mut() {
        let consumed = fuel_consumed(&config, &module.encode()?);
        if consumed == *fuel {
            continue;
        }
        let (line, col) = span.linecol_in(&test);
        panic!(
            "tests/all/fuel.wast:{}:{} - expected {} fuel, found {}",
            line + 1,
            col + 1,
            fuel,
            consumed
        );
    }
    Ok(())
}

fn fuel_consumed(config: &Config, wasm: &[u8]) -> u64 {
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, wasm).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(u64::MAX).unwrap();
    drop(Instance::new(&mut store, &module, &[]));
    u64::MAX - store.get_fuel().unwrap()
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn iloop(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func loop br 0 end)
            )
        "#,
    );
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func loop i32.const 1 br_if 0 end)
            )
        "#,
    );
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func loop i32.const 0 br_table 0 end)
            )
        "#,
    );
    iloop_aborts(
        &config,
        r#"
            (module
                (start 0)
                (func $f0 call $f1 call $f1)
                (func $f1 call $f2 call $f2)
                (func $f2 call $f3 call $f3)
                (func $f3 call $f4 call $f4)
                (func $f4 call $f5 call $f5)
                (func $f5 call $f6 call $f6)
                (func $f6 call $f7 call $f7)
                (func $f7 call $f8 call $f8)
                (func $f8 call $f9 call $f9)
                (func $f9 call $f10 call $f10)
                (func $f10 call $f11 call $f11)
                (func $f11 call $f12 call $f12)
                (func $f12 call $f13 call $f13)
                (func $f13 call $f14 call $f14)
                (func $f14 call $f15 call $f15)
                (func $f15 call $f16 call $f16)
                (func $f16)
            )
        "#,
    );

    fn iloop_aborts(config: &Config, wat: &str) {
        let engine = Engine::new(&config).unwrap();
        let module = Module::new(&engine, wat).unwrap();
        let mut store = Store::new(&engine, ());
        store.set_fuel(10_000).unwrap();
        let error = Instance::new(&mut store, &module, &[]).err().unwrap();
        assert_eq!(error.downcast::<Trap>().unwrap(), Trap::OutOfFuel);
    }

    Ok(())
}

#[wasmtime_test]
fn manual_fuel(config: &mut Config) {
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000).unwrap();
    assert_eq!(store.get_fuel().ok(), Some(10_000));
    assert_eq!(store.set_fuel(1).ok(), Some(()));
    assert_eq!(store.get_fuel().ok(), Some(1));
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn host_function_consumes_all(config: &mut Config) {
    const FUEL: u64 = 10_000;
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (func))
                (func (export "")
                    call 0
                    call $other)
                (func $other))
        "#,
    )
    .unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(FUEL).unwrap();
    let func = Func::wrap(&mut store, |mut caller: Caller<'_, ()>| {
        let remaining = caller.get_fuel().unwrap();
        assert_eq!(remaining, FUEL - 2);
        assert!(caller.set_fuel(1).is_ok());
    });

    let instance = Instance::new(&mut store, &module, &[func.into()]).unwrap();
    let export = instance.get_typed_func::<(), ()>(&mut store, "").unwrap();
    let trap = export.call(&mut store, ()).unwrap_err();
    assert_eq!(trap.downcast::<Trap>().unwrap(), Trap::OutOfFuel);
}

#[wasmtime_test]
fn manual_edge_cases(config: &mut Config) {
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(u64::MAX).unwrap();
    assert_eq!(store.get_fuel().unwrap(), u64::MAX);
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn unconditionally_trapping_memory_accesses_save_fuel_before_trapping(config: &mut Config) {
    config.consume_fuel(true);
    config.static_memory_maximum_size(0x1_0000);

    let engine = Engine::new(&config).unwrap();

    let module = Module::new(
        &engine,
        r#"
            (module
              (memory 1 1)
              (func (export "f") (param i32) (result i32)
                local.get 0
                local.get 0
                i32.add
                ;; This offset is larger than our memory max size and therefore
                ;; will unconditionally trap.
                i32.load8_s offset=0xffffffff))
        "#,
    )
    .unwrap();

    let mut store = Store::new(&engine, ());
    let init_fuel = 1_000;
    store.set_fuel(init_fuel).unwrap();
    assert_eq!(init_fuel, store.get_fuel().unwrap());

    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let f = instance
        .get_typed_func::<i32, i32>(&mut store, "f")
        .unwrap();

    let trap = f.call(&mut store, 0).unwrap_err();
    assert_eq!(trap.downcast::<Trap>().unwrap(), Trap::MemoryOutOfBounds);

    // The `i32.add` consumed some fuel before the unconditionally trapping
    // memory access.
    let consumed_fuel = init_fuel - store.get_fuel().unwrap();
    assert!(consumed_fuel > 0);
}

#[wasmtime_test]
#[cfg_attr(miri, ignore)]
fn get_fuel_clamps_at_zero(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(
        &engine,
        r#"
(module
  (func $add2 (export "add2") (param $n i32) (result i32)
    (i32.add (local.get $n) (i32.const 2))
  )
)
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;

    let add2 = instance.get_typed_func::<i32, i32>(&mut store, "add2")?;

    // Start with 6 fuel and one invocation of this function should cost 4 fuel
    store.set_fuel(6)?;
    assert_eq!(store.get_fuel()?, 6);
    add2.call(&mut store, 10)?;
    assert_eq!(store.get_fuel()?, 2);

    // One more invocation of the function would technically take us to -2 fuel,
    // but that's not representable, so the store should report 0 fuel after
    // this completes.
    add2.call(&mut store, 10)?;
    assert_eq!(store.get_fuel()?, 0);

    // Any further attempts should fail.
    assert!(add2.call(&mut store, 10).is_err());

    Ok(())
}

#[wasmtime_test(strategies(not(Cranelift)))]
#[cfg_attr(miri, ignore)]
fn ensure_stack_alignment(config: &mut Config) -> Result<()> {
    config.consume_fuel(true);
    let engine = Engine::new(config)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(100000000)?;

    let bytes = include_bytes!("../misc_testsuite/winch/fuel_stack_alignment.wat");
    let module = Module::new(&engine, bytes)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_typed_func::<f32, ()>(&mut store, "")?;
    let trap = func.call(&mut store, 50397184.0).unwrap_err();
    assert_eq!(
        trap.downcast::<Trap>().unwrap(),
        Trap::UnreachableCodeReached
    );
    Ok(())
}

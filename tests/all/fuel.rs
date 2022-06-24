use anyhow::Result;
use wasmtime::*;
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

#[test]
fn run() -> Result<()> {
    let test = std::fs::read_to_string("tests/all/fuel.wast")?;
    let buf = ParseBuffer::new(&test)?;
    let mut wast = parser::parse::<FuelWast<'_>>(&buf)?;
    for (span, fuel, module) in wast.assertions.iter_mut() {
        let consumed = fuel_consumed(&module.encode()?);
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

fn fuel_consumed(wasm: &[u8]) -> u64 {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, wasm).unwrap();
    let mut store = Store::new(&engine, ());
    store.add_fuel(u64::max_value()).unwrap();
    drop(Instance::new(&mut store, &module, &[]));
    store.fuel_consumed().unwrap()
}

#[test]
fn iloop() {
    iloop_aborts(
        r#"
            (module
                (start 0)
                (func loop br 0 end)
            )
        "#,
    );
    iloop_aborts(
        r#"
            (module
                (start 0)
                (func loop i32.const 1 br_if 0 end)
            )
        "#,
    );
    iloop_aborts(
        r#"
            (module
                (start 0)
                (func loop i32.const 0 br_table 0 end)
            )
        "#,
    );
    iloop_aborts(
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

    fn iloop_aborts(wat: &str) {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).unwrap();
        let module = Module::new(&engine, wat).unwrap();
        let mut store = Store::new(&engine, ());
        store.add_fuel(10_000).unwrap();
        let error = Instance::new(&mut store, &module, &[]).err().unwrap();
        assert!(
            error.to_string().contains("all fuel consumed"),
            "bad error: {}",
            error
        );
    }
}

#[test]
fn manual_fuel() {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, ());
    store.add_fuel(10_000).unwrap();
    assert_eq!(store.fuel_consumed(), Some(0));
    assert_eq!(store.consume_fuel(1).unwrap(), 9_999);
    assert_eq!(store.fuel_consumed(), Some(1));
    assert!(store.consume_fuel(10_000).is_err());
    assert_eq!(store.consume_fuel(999).unwrap(), 9_000);
    assert!(store.consume_fuel(10_000).is_err());
    assert_eq!(store.consume_fuel(8998).unwrap(), 2);
    assert!(store.consume_fuel(2).is_err());
    assert_eq!(store.consume_fuel(1).unwrap(), 1);
    assert!(store.consume_fuel(1).is_err());
    assert_eq!(store.consume_fuel(0).unwrap(), 1);
}

#[test]
fn host_function_consumes_all() {
    const FUEL: u64 = 10_000;
    let mut config = Config::new();
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
    store.add_fuel(FUEL).unwrap();
    let func = Func::wrap(&mut store, |mut caller: Caller<'_, ()>| {
        let consumed = caller.fuel_consumed().unwrap();
        assert_eq!(caller.consume_fuel((FUEL - consumed) - 1).unwrap(), 1);
    });

    let instance = Instance::new(&mut store, &module, &[func.into()]).unwrap();
    let export = instance
        .get_typed_func::<(), (), _>(&mut store, "")
        .unwrap();
    let trap = export.call(&mut store, ()).err().unwrap().to_string();
    assert!(trap.contains("all fuel consumed"), "bad error: {}", trap);
}

#[test]
fn manual_edge_cases() {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::new(&engine, ());
    store.add_fuel(u64::MAX).unwrap();
    assert_eq!(store.fuel_consumed(), Some(0));
    assert!(store.consume_fuel(u64::MAX).is_err());
    assert!(store.consume_fuel(i64::MAX as u64).is_err());
    assert!(store.consume_fuel(i64::MAX as u64 + 1).is_err());
    assert_eq!(store.consume_fuel(i64::MAX as u64 - 1).unwrap(), 1);
}

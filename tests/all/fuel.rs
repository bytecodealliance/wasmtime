use anyhow::Result;
use wasmtime::*;
use wast::parser::{self, Parse, ParseBuffer, Parser};

mod kw {
    wast::custom_keyword!(assert_fuel);
}

struct FuelWast<'a> {
    assertions: Vec<(u64, wast::Module<'a>)>,
}

impl<'a> Parse<'a> for FuelWast<'a> {
    fn parse(parser: Parser<'a>) -> parser::Result<Self> {
        let mut assertions = Vec::new();
        while !parser.is_empty() {
            assertions.push(parser.parens(|p| {
                p.parse::<kw::assert_fuel>()?;
                Ok((p.parse()?, p.parens(|p| p.parse())?))
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
    for (fuel, module) in wast.assertions.iter_mut() {
        assert_fuel(*fuel, &module.encode()?);
    }
    Ok(())
}

fn assert_fuel(fuel: u64, wasm: &[u8]) {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, wasm).unwrap();
    let store = Store::new(&engine);
    drop(Instance::new(&store, &module, &[]));
    assert_eq!(store.fuel_consumed(), Some(fuel));
}

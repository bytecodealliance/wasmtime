//! Small helper utility to disassemble `*.cwasm` files produced by Wasmtime.
//!
//! Run with:
//!
//!     cargo run --example objdump -F disas -p pulley-interpreter foo.cwasm

use anyhow::{Result, bail};
use object::{File, Object as _, ObjectSection, ObjectSymbol, SymbolKind};
use pulley_interpreter::decode::Decoder;
use pulley_interpreter::disas::Disassembler;

fn main() -> Result<()> {
    let cwasm = std::fs::read(std::env::args().nth(1).unwrap())?;

    let image = File::parse(&cwasm[..])?;

    let text = match image.sections().find(|s| s.name().ok() == Some(".text")) {
        Some(section) => section.data()?,
        None => bail!("no text section"),
    };

    for sym in image.symbols() {
        if !sym.is_definition() {
            continue;
        }
        if sym.kind() != SymbolKind::Text {
            continue;
        }
        let address = sym.address();
        let size = sym.size();
        if size == 0 {
            continue;
        }

        let name = sym.name()?;
        let code = &text[address as usize..][..size as usize];

        println!("{address:#08x}: <{name}>:");
        let mut disas = Disassembler::new(code);
        disas.start_offset(address as usize);
        let result = Decoder::decode_all(&mut disas);
        println!("{}", disas.disas());
        if let Err(e) = result {
            println!("        : error disassembling: {e:?}");
        }
    }
    Ok(())
}

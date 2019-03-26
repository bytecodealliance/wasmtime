use crate::error::Error;
use capstone::prelude::*;
use std::fmt::Write;

pub fn disassemble(mem: &[u8]) -> Result<(), Error> {
    let mut cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .build()?;

    println!("{} bytes:", mem.len());
    let insns = cs.disasm_all(&mem, 0x0).unwrap();
    for i in insns.iter() {
        let mut line = String::new();

        write!(&mut line, "{:4x}:\t", i.address()).unwrap();

        let mut bytes_str = String::new();
        for b in i.bytes() {
            write!(&mut bytes_str, "{:02x} ", b).unwrap();
        }
        write!(&mut line, "{:24}\t", bytes_str).unwrap();

        if let Some(s) = i.mnemonic() {
            write!(&mut line, "{}\t", s).unwrap();
        }

        if let Some(s) = i.op_str() {
            write!(&mut line, "{}", s).unwrap();
        }

        println!("{}", line);
    }

    Ok(())
}

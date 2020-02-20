use capstone::prelude::*;
use dynasmrt::AssemblyOffset;
use std::error::Error;
use std::fmt::{Display, Write};

pub fn disassemble<D: Display>(
    mem: &[u8],
    mut ops: &[(AssemblyOffset, D)],
) -> Result<(), Box<dyn Error>> {
    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .build()?;

    println!("{} bytes:", mem.len());
    let insns = cs.disasm_all(&mem, 0x0)?;
    for i in insns.iter() {
        let mut line = String::new();

        let address = i.address();

        while let Some((offset, op)) = ops.first() {
            if offset.0 as u64 <= address {
                ops = &ops[1..];
                println!("{}", op);
            } else {
                break;
            }
        }

        write!(&mut line, "{:4x}:\t", i.address())?;

        let mut bytes_str = String::new();
        for b in i.bytes() {
            write!(&mut bytes_str, "{:02x} ", b)?;
        }
        write!(&mut line, "{:24}\t", bytes_str)?;

        if let Some(s) = i.mnemonic() {
            write!(&mut line, "{}\t", s)?;
        }

        if let Some(s) = i.op_str() {
            write!(&mut line, "{}", s)?;
        }

        println!("{}", line);
    }

    Ok(())
}

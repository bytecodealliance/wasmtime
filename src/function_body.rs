use backend::*;
use disassemble::disassemble;
use error::Error;
use wasmparser::{FunctionBody, Operator};

pub fn translate(body: &FunctionBody) -> Result<(), Error> {
    let locals = body.get_locals_reader()?;
    for local in locals {
        local?; // TODO
    }

    let mut ops = dynasmrt::x64::Assembler::new().unwrap();
    let operators = body.get_operators_reader()?;
    let mut regs = Registers::new();
    for op in operators {
        match op? {
            Operator::I32Add => {
                add_i32(&mut ops, &mut regs);
            }
            _ => {
                unsupported_opcode(&mut ops);
            }
        }
    }

    let output = ops
        .finalize()
        .map_err(|_asm| Error::Assembler("assembler error".to_owned()))?;

    // TODO: Do something with the output.
    disassemble(&output)?;

    Ok(())
}

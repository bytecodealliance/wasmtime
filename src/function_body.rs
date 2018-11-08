use backend::*;
use error::Error;
use wasmparser::{FunctionBody, Operator};

pub fn translate(session: &mut CodeGenSession, body: &FunctionBody) -> Result<(), Error> {
    let locals = body.get_locals_reader()?;

    // Assume signature is (i32, i32) -> i32 for now.
    // TODO: Use a real signature
    const ARG_COUNT: u32 = 2;

    let mut framesize = ARG_COUNT;
    for local in locals {
        let (count, _ty) = local?;
        framesize += count;
    }

    let mut ctx = session.new_context();
    let operators = body.get_operators_reader()?;

    prologue(&mut ctx, framesize);

    for arg_pos in 0..ARG_COUNT {
        copy_incoming_arg(&mut ctx, arg_pos);
    }

    for op in operators {
        match op? {
            Operator::I32Add => {
                add_i32(&mut ctx);
            }
            Operator::GetLocal { local_index } => {
                get_local_i32(&mut ctx, local_index);
            }
            Operator::End => {
                // TODO: This is super naive and makes a lot of unfounded assumptions 
                // but will do for the start.
                prepare_return_value(&mut ctx);
            }
            _ => {
                unsupported_opcode(&mut ctx);
            }
        }
    }
    epilogue(&mut ctx);

    Ok(())
}

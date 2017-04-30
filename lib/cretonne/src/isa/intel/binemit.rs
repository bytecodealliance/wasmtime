//! Emitting binary Intel machine code.

use binemit::{CodeSink, bad_encoding};
use ir::{Function, Inst, InstructionData};
use isa::RegUnit;

include!(concat!(env!("OUT_DIR"), "/binemit-intel.rs"));

pub static RELOC_NAMES: [&'static str; 1] = ["Call"];

fn put_op1<CS: CodeSink + ?Sized>(bits: u16, sink: &mut CS) {
    debug_assert!(bits & 0x0f00 == 0, "Invalid encoding bits for Op1*");
    sink.put1(bits as u8);
}

fn modrm_rr<CS: CodeSink + ?Sized>(rm: RegUnit, reg: RegUnit, sink: &mut CS) {
    let reg = reg as u8 & 7;
    let rm = rm as u8 & 7;
    let mut b = 0b11000000;
    b |= reg << 3;
    b |= rm;
    sink.put1(b);
}

fn recipe_op1rr<CS: CodeSink + ?Sized>(func: &Function, inst: Inst, sink: &mut CS) {
    if let InstructionData::Binary { args, .. } = func.dfg[inst] {
        put_op1(func.encodings[inst].bits(), sink);
        modrm_rr(func.locations[args[0]].unwrap_reg(),
                 func.locations[args[1]].unwrap_reg(),
                 sink);
    } else {
        panic!("Expected Binary format: {:?}", func.dfg[inst]);
    }
}

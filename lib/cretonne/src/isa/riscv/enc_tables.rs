//! Encoding tables for RISC-V.

use ir::{Opcode, InstructionData};
use ir::instructions::InstructionFormat;
use ir::types;
use predicates;
use isa::enc_tables::{Level1Entry, Level2Entry};

// Include the generated encoding tables:
// - `LEVEL1_RV32`
// - `LEVEL1_RV64`
// - `LEVEL2`
// - `ENCLIST`
include!(concat!(env!("OUT_DIR"), "/encoding-riscv.rs"));

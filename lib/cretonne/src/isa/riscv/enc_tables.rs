//! Encoding tables for RISC-V.

use ir::condcodes::IntCC;
use ir::types;
use ir::{Opcode, InstructionData};
use isa::EncInfo;
use isa::constraints::*;
use isa::enc_tables::{Level1Entry, Level2Entry};
use predicates;
use super::registers::*;

// Include the generated encoding tables:
// - `LEVEL1_RV32`
// - `LEVEL1_RV64`
// - `LEVEL2`
// - `ENCLIST`
// - `INFO`
include!(concat!(env!("OUT_DIR"), "/encoding-riscv.rs"));

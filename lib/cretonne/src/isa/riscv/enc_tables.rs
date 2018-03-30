//! Encoding tables for RISC-V.

use super::registers::*;
use ir;
use isa;
use isa::constraints::*;
use isa::enc_tables::*;
use isa::encoding::RecipeSizing;
use predicates;

// Include the generated encoding tables:
// - `LEVEL1_RV32`
// - `LEVEL1_RV64`
// - `LEVEL2`
// - `ENCLIST`
// - `INFO`
include!(concat!(env!("OUT_DIR"), "/encoding-riscv.rs"));
include!(concat!(env!("OUT_DIR"), "/legalize-riscv.rs"));

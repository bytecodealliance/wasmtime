//! Encoding tables for ARM32 ISA.

use ir::InstructionData;
use ir::types;
use isa::EncInfo;
use isa::constraints::*;
use isa::enc_tables::*;
use isa::encoding::RecipeSizing;

include!(concat!(env!("OUT_DIR"), "/encoding-arm32.rs"));

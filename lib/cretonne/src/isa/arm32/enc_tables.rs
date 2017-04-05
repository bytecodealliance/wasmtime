//! Encoding tables for ARM32 ISA.

use ir::InstructionData;
use ir::types;
use isa::EncInfo;
use isa::constraints::*;
use isa::enc_tables::{Level1Entry, Level2Entry};

include!(concat!(env!("OUT_DIR"), "/encoding-arm32.rs"));

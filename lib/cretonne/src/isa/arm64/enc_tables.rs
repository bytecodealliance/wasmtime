//! Encoding tables for ARM64 ISA.

use ir::InstructionData;
use ir::types;
use isa::EncInfo;
use isa::constraints::*;
use isa::enc_tables::{Level1Entry, Level2Entry};
use isa::encoding::RecipeSizing;

include!(concat!(env!("OUT_DIR"), "/encoding-arm64.rs"));

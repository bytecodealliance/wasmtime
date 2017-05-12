//! Encoding tables for Intel ISAs.

use ir::types;
use ir::{Opcode, InstructionData};
use isa::EncInfo;
use isa::constraints::*;
use isa::enc_tables::{Level1Entry, Level2Entry};
use isa::encoding::RecipeSizing;
use predicates;
use super::registers::*;

include!(concat!(env!("OUT_DIR"), "/encoding-intel.rs"));

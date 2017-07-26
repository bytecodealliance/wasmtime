//! Encoding tables for Intel ISAs.

use ir::{self, types, Opcode};
use isa::EncInfo;
use isa::constraints::*;
use isa::enc_tables::*;
use isa::encoding::RecipeSizing;
use predicates;
use super::registers::*;

include!(concat!(env!("OUT_DIR"), "/encoding-intel.rs"));

//! Encoding tables for Intel ISAs.

use bitset::BitSet;
use ir;
use isa::constraints::*;
use isa::enc_tables::*;
use isa::encoding::RecipeSizing;
use isa;
use predicates;
use super::registers::*;

include!(concat!(env!("OUT_DIR"), "/encoding-intel.rs"));
include!(concat!(env!("OUT_DIR"), "/legalize-intel.rs"));

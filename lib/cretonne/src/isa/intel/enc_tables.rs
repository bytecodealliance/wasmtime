//! Encoding tables for Intel ISAs.

use ir;
use isa;
use isa::constraints::*;
use isa::enc_tables::*;
use isa::encoding::RecipeSizing;
use predicates;
use super::registers::*;

include!(concat!(env!("OUT_DIR"), "/encoding-intel.rs"));
include!(concat!(env!("OUT_DIR"), "/legalize-intel.rs"));

//! Encoding tables for ARM32 ISA.

use crate::ir;
use crate::isa;
use crate::isa::constraints::*;
use crate::isa::enc_tables::*;
use crate::isa::encoding::RecipeSizing;

include!(concat!(env!("OUT_DIR"), "/encoding-arm32.rs"));
include!(concat!(env!("OUT_DIR"), "/legalize-arm32.rs"));

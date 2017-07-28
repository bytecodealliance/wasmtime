//! Encoding tables for ARM32 ISA.

use ir;
use isa;
use isa::constraints::*;
use isa::enc_tables::*;
use isa::encoding::RecipeSizing;

include!(concat!(env!("OUT_DIR"), "/encoding-arm32.rs"));
include!(concat!(env!("OUT_DIR"), "/legalize-arm32.rs"));

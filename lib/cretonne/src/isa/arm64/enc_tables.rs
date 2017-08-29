//! Encoding tables for ARM64 ISA.

use ir;
use isa;
use isa::constraints::*;
use isa::enc_tables::*;
use isa::encoding::RecipeSizing;

include!(concat!(env!("OUT_DIR"), "/encoding-arm64.rs"));
include!(concat!(env!("OUT_DIR"), "/legalize-arm64.rs"));

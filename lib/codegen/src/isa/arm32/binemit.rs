//! Emitting binary ARM32 machine code.

use binemit::{bad_encoding, CodeSink};
use ir::{Function, Inst};
use regalloc::RegDiversions;

include!(concat!(env!("OUT_DIR"), "/binemit-arm32.rs"));

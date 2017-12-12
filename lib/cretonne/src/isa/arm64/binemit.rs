//! Emitting binary ARM64 machine code.

use binemit::{CodeSink, bad_encoding};
use ir::{Function, Inst};
use regalloc::RegDiversions;

include!(concat!(env!("OUT_DIR"), "/binemit-arm64.rs"));

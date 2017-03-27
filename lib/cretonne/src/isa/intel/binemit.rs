//! Emitting binary Intel machine code.

use binemit::{CodeSink, bad_encoding};
use ir::{Function, Inst};

include!(concat!(env!("OUT_DIR"), "/binemit-intel.rs"));

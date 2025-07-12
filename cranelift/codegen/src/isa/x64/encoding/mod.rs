//! Contains the encoding machinery for the various x64 instruction formats.
use crate::{isa::x64, machinst::MachBuffer};
use std::vec::Vec;

pub mod evex;
pub mod rex;

/// The encoding formats in this module all require a way of placing bytes into
/// a buffer.
pub trait ByteSink {
    /// Add 1 byte to the code section.
    fn put1(&mut self, _: u8);

    /// Add 4 bytes to the code section.
    fn put4(&mut self, _: u32);
}

impl ByteSink for MachBuffer<x64::inst::Inst> {
    fn put1(&mut self, value: u8) {
        self.put1(value)
    }

    fn put4(&mut self, value: u32) {
        self.put4(value)
    }
}

/// Provide a convenient implementation for testing.
impl ByteSink for Vec<u8> {
    fn put1(&mut self, v: u8) {
        self.extend_from_slice(&[v])
    }

    fn put4(&mut self, v: u32) {
        self.extend_from_slice(&v.to_le_bytes())
    }
}

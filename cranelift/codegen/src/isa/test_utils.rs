// This is unused when no platforms with the new backend are enabled.
#![allow(dead_code)]

use crate::binemit::{Addend, CodeOffset, CodeSink, Reloc};
use crate::ir::{ExternalName, Opcode, SourceLoc, TrapCode};

use alloc::vec::Vec;
use std::string::String;

pub struct TestCodeSink {
    bytes: Vec<u8>,
}

impl TestCodeSink {
    /// Create a new TestCodeSink.
    pub fn new() -> TestCodeSink {
        TestCodeSink { bytes: vec![] }
    }

    /// Return the code emitted to this sink as a hex string.
    pub fn stringify(&self) -> String {
        // This is pretty lame, but whatever ..
        use std::fmt::Write;
        let mut s = String::with_capacity(self.bytes.len() * 2);
        for b in &self.bytes {
            write!(&mut s, "{:02X}", b).unwrap();
        }
        s
    }
}

impl CodeSink for TestCodeSink {
    fn offset(&self) -> CodeOffset {
        self.bytes.len() as CodeOffset
    }

    fn put1(&mut self, x: u8) {
        self.bytes.push(x);
    }

    fn put2(&mut self, x: u16) {
        self.bytes.push((x >> 0) as u8);
        self.bytes.push((x >> 8) as u8);
    }

    fn put4(&mut self, mut x: u32) {
        for _ in 0..4 {
            self.bytes.push(x as u8);
            x >>= 8;
        }
    }

    fn put8(&mut self, mut x: u64) {
        for _ in 0..8 {
            self.bytes.push(x as u8);
            x >>= 8;
        }
    }

    fn reloc_external(
        &mut self,
        _srcloc: SourceLoc,
        _rel: Reloc,
        _name: &ExternalName,
        _addend: Addend,
    ) {
    }

    fn trap(&mut self, _code: TrapCode, _srcloc: SourceLoc) {}

    fn end_codegen(&mut self) {}

    fn add_call_site(&mut self, _opcode: Opcode, _srcloc: SourceLoc) {}
}

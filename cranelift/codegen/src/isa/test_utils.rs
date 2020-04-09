use crate::binemit::{Addend, CodeOffset, CodeSink, Reloc};
use crate::ir::Value;
use crate::ir::{ConstantOffset, ExternalName, Function, JumpTable, Opcode, SourceLoc, TrapCode};
use crate::isa::TargetIsa;

use alloc::vec::Vec;
use std::string::{String, ToString};

pub struct TestCodeSink {
    bytes: Vec<u8>,
}

impl TestCodeSink {
    /// Create a new TestCodeSink.
    pub fn new() -> TestCodeSink {
        TestCodeSink { bytes: vec![] }
    }

    /// This is pretty lame, but whatever ..
    pub fn stringify(&self) -> String {
        let mut s = "".to_string();
        for b in &self.bytes {
            s = s + &format!("{:02X}", b).to_string();
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

    fn reloc_block(&mut self, _rel: Reloc, _block_offset: CodeOffset) {}

    fn reloc_external(
        &mut self,
        _srcloc: SourceLoc,
        _rel: Reloc,
        _name: &ExternalName,
        _addend: Addend,
    ) {
    }

    fn reloc_constant(&mut self, _rel: Reloc, _constant_offset: ConstantOffset) {}

    fn reloc_jt(&mut self, _rel: Reloc, _jt: JumpTable) {}

    fn trap(&mut self, _code: TrapCode, _srcloc: SourceLoc) {}

    fn begin_jumptables(&mut self) {}

    fn begin_rodata(&mut self) {}

    fn end_codegen(&mut self) {}

    fn add_stackmap(&mut self, _val_list: &[Value], _func: &Function, _isa: &dyn TargetIsa) {}

    fn add_call_site(&mut self, _opcode: Opcode, _srcloc: SourceLoc) {}
}

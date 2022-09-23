use super::regs::{rbp, reg_name, rsp};
use crate::isa::reg::Reg;
use crate::masm::MacroAssembler as Masm;

#[derive(Default)]
pub(crate) struct MacroAssembler {
    asm: Assembler,
}

impl Masm for MacroAssembler {
    fn prologue(&mut self) {
        let frame_pointer = rbp();
        let stack_pointer = rsp();

        self.asm.pushr(frame_pointer);
        self.asm.movrr(frame_pointer, stack_pointer);
    }

    fn epilogue(&mut self) {}

    fn finalize(self) -> Vec<String> {
        self.asm.finalize()
    }
}

/// Low level assembler implementation for X64
// NB
// This is an interim, debug approach; the long term idea
// is to make each ISA assembler available through
// `cranelift_asm`

#[derive(Default)]
struct Assembler {
    buffer: Vec<String>,
}

impl Assembler {
    /// Push register
    pub fn pushr(&mut self, reg: Reg) {
        self.buffer.push(format!("push {}", reg_name(reg)));
    }

    /// Register to register move
    pub fn movrr(&mut self, dst: Reg, src: Reg) {
        let dst = reg_name(dst);
        let src = reg_name(src);

        self.buffer.push(format!("mov {} {}", dst, src));
    }

    /// Return the emitted code
    pub fn finalize(self) -> Vec<String> {
        self.buffer.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::MacroAssembler;
    use crate::masm::MacroAssembler as Masm;
    #[test]
    fn print() {
        let mut masm = MacroAssembler::default();
        masm.prologue();
        let result = masm.finalize();

        for i in &result {
            println!("{}", i);
        }
    }
}

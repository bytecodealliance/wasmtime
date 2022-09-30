use super::regs::{rbp, reg_name, rsp};
use crate::abi::addressing_mode::Address;
use crate::abi::local::LocalSlot;
use crate::isa::reg::Reg;
use crate::masm::{MacroAssembler as Masm, OperandSize};

#[derive(Default)]
pub(crate) struct MacroAssembler {
    sp_offset: u32,
    asm: Assembler,
}

impl Masm for MacroAssembler {
    fn prologue(&mut self) {
        let frame_pointer = rbp();
        let stack_pointer = rsp();

        self.asm.push_r(frame_pointer);
        self.asm.mov_rr(stack_pointer, frame_pointer);
    }

    fn reserve_stack(&mut self, bytes: u32) {
        self.asm.sub_ir(bytes, rsp());
        self.increment_sp(bytes);
    }

    fn local_address(&mut self, local: &LocalSlot) -> Address {
        let (reg, offset) = local
            .addressed_from_sp()
            .then(|| {
                let offset = self.sp_offset.checked_sub(local.offset).expect(&format!(
                    "Invalid local offset = {}; sp offset = {}",
                    local.offset, self.sp_offset
                ));
                (rsp(), offset)
            })
            .unwrap_or((rbp(), local.offset));

        Address::base(reg, offset)
    }

    fn store(&mut self, src: Reg, dst: Address, size: OperandSize) {
        let src: Operand = src.into();
        let dst: Operand = dst.into();

        match size {
            OperandSize::S32 => {
                self.asm.movl(src, dst);
            }
            OperandSize::S64 => {
                self.asm.mov(src, dst);
            }
        }
    }

    fn sp_offset(&mut self) -> u32 {
        self.sp_offset
    }

    fn epilogue(&mut self) {}

    fn finalize(&mut self) -> &[String] {
        self.asm.finalize()
    }
}

impl MacroAssembler {
    fn increment_sp(&mut self, bytes: u32) {
        self.sp_offset += bytes;
    }

    fn decrement_sp(&mut self, bytes: u32) {
        assert!(
            self.sp_offset >= bytes,
            "sp offset = {}; bytes = {}",
            self.sp_offset,
            bytes
        );
        self.sp_offset -= bytes;
    }
}

/// A x64 instruction operand
#[derive(Debug, Copy, Clone)]
enum Operand {
    Reg(Reg),
    Mem(Address),
    Imm(u32),
}

/// Low level assembler implementation for x64
/// All instructions denote a 64 bit size, unless
/// otherwise specified by the corresponding function
/// name suffix

// NOTE
// This is an interim, debug approach; the long term idea
// is to make each ISA assembler available through
// `cranelift_asm`
#[derive(Default)]
struct Assembler {
    buffer: Vec<String>,
}

impl Assembler {
    pub fn push_r(&mut self, reg: Reg) {
        self.buffer.push(format!("push {}", reg_name(reg, 8)));
    }

    pub fn mov(&mut self, src: Operand, dst: Operand) {
        // r, r
        // r, m (displacement)
        // r, m (displace,ent, index)
        // i, r
        // i, m (displacement)
        // i, m (displacement, index)
        // load combinations
        match &(src, dst) {
            (Operand::Reg(lhs), Operand::Reg(rhs)) => self.mov_rr(*lhs, *rhs),
            (Operand::Reg(r), Operand::Mem(addr)) => match addr {
                Address::Base { base, imm } => self.mov_rm(*r, *base, *imm),
            },
            _ => panic!(
                "Invalid operand combination for movl; src = {:?}; dst = {:?}",
                src, dst
            ),
        }
    }

    pub fn mov_rr(&mut self, src: Reg, dst: Reg) {
        let dst = reg_name(dst, 8);
        let src = reg_name(src, 8);

        self.buffer.push(format!("mov {}, {}", dst, src));
    }

    pub fn mov_rm(&mut self, src: Reg, base: Reg, imm: u32) {
        let src = reg_name(src, 8);
        let dst = reg_name(base, 8);

        let addr = if imm == 0 {
            format!("[{}]", dst)
        } else {
            format!("[{} + {}]", dst, imm)
        };

        self.buffer.push(format!("mov {}, {}", addr, src));
    }

    pub fn movl(&mut self, src: Operand, dst: Operand) {
        // r, r
        // r, m (displacement)
        // r, m (displace,ent, index)
        // i, r
        // i, m (displacement)
        // i, m (displacement, index)
        // load combinations
        match &(src, dst) {
            (Operand::Reg(lhs), Operand::Reg(rhs)) => self.movl_rr(*lhs, *rhs),
            (Operand::Reg(r), Operand::Mem(addr)) => match addr {
                Address::Base { base, imm } => self.movl_rm(*r, *base, *imm),
            },
            _ => panic!(
                "Invalid operand combination for movl; src = {:?}; dst = {:?}",
                src, dst
            ),
        }
    }

    pub fn movl_rr(&mut self, src: Reg, dst: Reg) {
        let dst = reg_name(dst, 4);
        let src = reg_name(src, 4);

        self.buffer.push(format!("mov {}, {}", dst, src));
    }

    pub fn movl_rm(&mut self, src: Reg, base: Reg, imm: u32) {
        let src = reg_name(src, 4);
        let dst = reg_name(base, 8);

        let addr = if imm == 0 {
            format!("[{}]", dst)
        } else {
            format!("[{} + {}]", dst, imm)
        };

        self.buffer.push(format!("movl {}, {}", addr, src));
    }

    pub fn sub_ir(&mut self, imm: u32, dst: Reg) {
        let dst = reg_name(dst, 8);
        self.buffer.push(format!("sub {}, {}", dst, imm));
    }

    /// Return the emitted code
    pub fn finalize(&mut self) -> &[String] {
        &self.buffer
    }
}

impl From<Reg> for Operand {
    fn from(reg: Reg) -> Self {
        Operand::Reg(reg)
    }
}

impl From<Address> for Operand {
    fn from(addr: Address) -> Self {
        Operand::Mem(addr)
    }
}

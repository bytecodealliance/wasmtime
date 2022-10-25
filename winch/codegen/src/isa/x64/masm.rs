use super::regs::{rbp, reg_name, rsp};
use crate::abi::addressing_mode::Address;
use crate::abi::local::LocalSlot;
use crate::isa::reg::Reg;
use crate::masm::{MacroAssembler as Masm, OperandSize, RegImm};

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

    fn push(&mut self, reg: Reg) -> u32 {
        self.asm.push_r(reg);
        // In x64 the push instruction takes either
        // 2 or 8 bytes; in our case we're always
        // assuming 8 bytes per push.
        self.increment_sp(8);

        self.sp_offset
    }

    fn reserve_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }

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

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) {
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

    fn load(&mut self, src: Address, dst: Reg, size: OperandSize) {
        let src = src.into();
        let dst = dst.into();

        match size {
            OperandSize::S32 => self.asm.movl(src, dst),
            OperandSize::S64 => self.asm.mov(src, dst),
        }
    }

    fn sp_offset(&mut self) -> u32 {
        self.sp_offset
    }

    fn zero(&mut self, reg: Reg) {
        self.asm.xorl_rr(reg, reg);
    }

    fn mov(&mut self, src: RegImm, dst: RegImm, size: OperandSize) {
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

    fn add(&mut self, dst: RegImm, lhs: RegImm, rhs: RegImm, size: OperandSize) {
        let (src, dst): (Operand, Operand) = if dst == lhs {
            (rhs.into(), dst.into())
        } else {
            panic!(
                "the destination and first source argument must be the same, dst={:?}, lhs={:?}",
                dst, lhs
            );
        };

        match size {
            OperandSize::S32 => {
                self.asm.addl(src, dst);
            }
            OperandSize::S64 => {
                self.asm.add(src, dst);
            }
        }
    }

    fn epilogue(&mut self, locals_size: u32) {
        let rsp = rsp();
        if locals_size > 0 {
            self.asm.add_ir(locals_size as i32, rsp);
        }
        self.asm.pop_r(rbp());
        self.asm.ret();
    }

    fn finalize(&mut self) -> &[String] {
        self.asm.finalize()
    }
}

impl MacroAssembler {
    /// Crate a x64 MacroAssembler
    pub fn new() -> Self {
        Self {
            sp_offset: 0,
            asm: Default::default(),
        }
    }

    fn increment_sp(&mut self, bytes: u32) {
        self.sp_offset += bytes;
    }

    #[allow(dead_code)]
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

/// A x64 instruction operand.
#[derive(Debug, Copy, Clone)]
enum Operand {
    Reg(Reg),
    Mem(Address),
    Imm(i32),
}

/// Low level assembler implementation for x64
/// All instructions denote a 64 bit size, unless
/// otherwise specified by the corresponding function
/// name suffix.

// NOTE
// This is an interim, debug approach; the long term idea
// is to make each ISA assembler available through
// `cranelift_asm`. The literal representation of the
// instructions use intel syntax for easier manual verification.
// This shouldn't be an issue, once we plug in Cranelift's backend
// we are going to be able to properly disassemble.
#[derive(Default)]
struct Assembler {
    buffer: Vec<String>,
}

impl Assembler {
    pub fn push_r(&mut self, reg: Reg) {
        self.buffer.push(format!("push {}", reg_name(reg, 8)));
    }

    pub fn pop_r(&mut self, reg: Reg) {
        self.buffer.push(format!("pop {}", reg_name(reg, 8)));
    }

    pub fn ret(&mut self) {
        self.buffer.push("ret".into());
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
            (Operand::Imm(op), Operand::Mem(addr)) => match addr {
                Address::Base { base, imm } => self.mov_im(*op, *base, *imm),
            },
            (Operand::Imm(imm), Operand::Reg(reg)) => self.mov_ir(*imm, *reg),
            (Operand::Mem(addr), Operand::Reg(reg)) => match addr {
                Address::Base { base, imm } => self.mov_mr(*base, *imm, *reg),
            },
            _ => panic!(
                "Invalid operand combination for mov; src = {:?}; dst = {:?}",
                src, dst
            ),
        }
    }

    pub fn mov_rr(&mut self, src: Reg, dst: Reg) {
        let dst = reg_name(dst, 8);
        let src = reg_name(src, 8);

        self.buffer.push(format!("mov {}, {}", dst, src));
    }

    pub fn mov_rm(&mut self, src: Reg, base: Reg, disp: u32) {
        let src = reg_name(src, 8);
        let dst = reg_name(base, 8);

        let addr = if disp == 0 {
            format!("[{}]", dst)
        } else {
            format!("[{} + {}]", dst, disp)
        };

        self.buffer.push(format!("mov {}, {}", addr, src));
    }

    pub fn mov_im(&mut self, imm: i32, base: Reg, disp: u32) {
        let reg = reg_name(base, 8);

        let addr = if disp == 0 {
            format!("[{}]", reg)
        } else {
            format!("[{} + {}]", reg, disp)
        };

        self.buffer.push(format!("mov qword {}, {}", addr, imm));
    }

    pub fn mov_ir(&mut self, imm: i32, dst: Reg) {
        let reg = reg_name(dst, 8);

        self.buffer.push(format!("mov {}, {}", reg, imm));
    }

    pub fn mov_mr(&mut self, base: Reg, disp: u32, dst: Reg) {
        let base = reg_name(base, 8);
        let dst = reg_name(dst, 8);

        let addr = if disp == 0 {
            format!("[{}]", base)
        } else {
            format!("[{} + {}]", base, disp)
        };

        self.buffer.push(format!("mov {}, {}", dst, addr));
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
            (Operand::Imm(op), Operand::Mem(addr)) => match addr {
                Address::Base { base, imm } => self.movl_im(*op, *base, *imm),
            },
            (Operand::Imm(imm), Operand::Reg(reg)) => self.movl_ir(*imm, *reg),
            (Operand::Mem(addr), Operand::Reg(reg)) => match addr {
                Address::Base { base, imm } => self.movl_mr(*base, *imm, *reg),
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

    pub fn movl_rm(&mut self, src: Reg, base: Reg, disp: u32) {
        let src = reg_name(src, 4);
        let dst = reg_name(base, 8);

        let addr = if disp == 0 {
            format!("[{}]", dst)
        } else {
            format!("[{} + {}]", dst, disp)
        };

        self.buffer.push(format!("mov {}, {}", addr, src));
    }

    pub fn movl_im(&mut self, imm: i32, base: Reg, disp: u32) {
        let reg = reg_name(base, 8);

        let addr = if disp == 0 {
            format!("[{}]", reg)
        } else {
            format!("[{} + {}]", reg, disp)
        };

        self.buffer.push(format!("mov dword {}, {}", addr, imm));
    }

    pub fn movl_ir(&mut self, imm: i32, dst: Reg) {
        let reg = reg_name(dst, 4);

        self.buffer.push(format!("mov {}, {}", reg, imm));
    }

    pub fn movl_mr(&mut self, base: Reg, disp: u32, dst: Reg) {
        let base = reg_name(base, 8);
        let dst = reg_name(dst, 4);

        let addr = if disp == 0 {
            format!("[{}]", base)
        } else {
            format!("[{} + {}]", base, disp)
        };

        self.buffer.push(format!("mov {}, {}", dst, addr));
    }

    pub fn sub_ir(&mut self, imm: u32, dst: Reg) {
        let dst = reg_name(dst, 8);
        self.buffer.push(format!("sub {}, {}", dst, imm));
    }

    pub fn add(&mut self, src: Operand, dst: Operand) {
        match &(src, dst) {
            (Operand::Imm(imm), Operand::Reg(dst)) => self.add_ir(*imm, *dst),
            (Operand::Reg(src), Operand::Reg(dst)) => self.add_rr(*src, *dst),
            _ => panic!(
                "Invalid operand combination for add; src = {:?} dst = {:?}",
                src, dst
            ),
        }
    }

    pub fn add_ir(&mut self, imm: i32, dst: Reg) {
        let dst = reg_name(dst, 8);

        self.buffer.push(format!("add {}, {}", dst, imm));
    }

    pub fn add_rr(&mut self, src: Reg, dst: Reg) {
        let src = reg_name(src, 8);
        let dst = reg_name(dst, 8);

        self.buffer.push(format!("add {}, {}", dst, src));
    }

    pub fn addl(&mut self, src: Operand, dst: Operand) {
        match &(src, dst) {
            (Operand::Imm(imm), Operand::Reg(dst)) => self.addl_ir(*imm, *dst),
            (Operand::Reg(src), Operand::Reg(dst)) => self.addl_rr(*src, *dst),
            _ => panic!(
                "Invalid operand combination for add; src = {:?} dst = {:?}",
                src, dst
            ),
        }
    }

    pub fn addl_ir(&mut self, imm: i32, dst: Reg) {
        let dst = reg_name(dst, 4);

        self.buffer.push(format!("add {}, {}", dst, imm));
    }

    pub fn addl_rr(&mut self, src: Reg, dst: Reg) {
        let src = reg_name(src, 4);
        let dst = reg_name(dst, 4);

        self.buffer.push(format!("add {}, {}", dst, src));
    }

    pub fn xorl_rr(&mut self, src: Reg, dst: Reg) {
        let src = reg_name(src, 4);
        let dst = reg_name(dst, 4);

        self.buffer.push(format!("xor {}, {}", dst, src));
    }

    /// Return the emitted code
    pub fn finalize(&mut self) -> &[String] {
        &self.buffer
    }
}

impl From<RegImm> for Operand {
    fn from(rimm: RegImm) -> Self {
        match rimm {
            RegImm::Reg(r) => r.into(),
            RegImm::Imm(imm) => Operand::Imm(imm),
        }
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

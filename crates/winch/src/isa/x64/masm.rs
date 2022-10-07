use super::regs::{rbp, reg_name, rsp};
use crate::abi::local::LocalSlot;
use crate::abi::{addressing_mode::Address, align_to, ABI};
use crate::frame::DefinedLocalsRange;
use crate::isa::reg::Reg;
use crate::masm::{MacroAssembler as Masm, OperandSize, RegImm};
use crate::regset::RegSet;
use crate::stack::Stack;

pub(crate) struct MacroAssembler {
    sp_offset: u32,
    regset: RegSet,
    stack: Stack,
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

    fn sp_offset(&mut self) -> u32 {
        self.sp_offset
    }

    fn zero_local_slots<A: ABI>(&mut self, range: &DefinedLocalsRange, abi: &A) {
        if range.0.is_empty() {
            return;
        }

        // Divide the locals range into word-size slots; first ensure that the range limits
        // are word size aligned; there's no guarantee about their alignment. The aligned "upper"
        // limit should always be less than or equal to the size of the local area, which gets
        // validated when getting the address of a local

        let word_size = <A as ABI>::word_bytes();
        // If the locals range start is not aligned to the word size, zero the last four bytes
        let range_start = range
            .0
            .start()
            .checked_rem(word_size)
            .map_or(*range.0.start(), |_| {
                // TODO use `align_to` instead?
                let start = range.0.start() + 4;
                let addr = self.local_address(&LocalSlot::i32(start));
                // rsp, rbp
                self.store(RegImm::imm(0), addr, OperandSize::S64);
                start
            });

        // Ensure that the range end is also word-size aligned
        let range_end = align_to(*range.0.end(), word_size);
        // Divide the range into word-size slots
        let slots = (range_end - range_start) / word_size;

        match slots {
            1 => {
                let slot = LocalSlot::i64(range_start + word_size);
                let addr = self.local_address(&slot);
                self.store(RegImm::imm(0), addr, OperandSize::S64);
            }
            // TODO
            // Add an upper bound to this generation;
            // given a considerably large amount of slots
            // this will be inefficient
            n => {
                // Request a gpr and zero it
                let zero = self.any_gpr();
                self.asm.xorl_rr(zero, zero);
                // store zero in each of the slots in the range
                for step in (range_start..range_end)
                    .into_iter()
                    .step_by(word_size as usize)
                {
                    let slot = LocalSlot::i64(step + word_size);
                    let addr = self.local_address(&slot);
                    self.store(RegImm::reg(zero), addr, OperandSize::S64);
                }
                self.regset.free_gpr(zero);
            }
        }
    }

    fn epilogue(&mut self) {}

    fn finalize(&mut self) -> &[String] {
        self.asm.finalize()
    }
}

impl MacroAssembler {
    /// Crate a x64 MacroAssembler
    pub fn new(regset: RegSet, stack: Stack) -> Self {
        Self {
            sp_offset: 0,
            asm: Default::default(),
            regset,
            stack,
        }
    }

    /// Allocate the next available general purpose register,
    /// spilling if none available
    fn any_gpr(&mut self) -> Reg {
        match self.regset.any_gpr() {
            None => {
                self.spill();
                self.regset
                    .any_gpr()
                    .expect("any allocatable general purpose register to be available")
            }
            Some(r) => r,
        }
    }

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
    Imm(i32),
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
            (Operand::Imm(op), Operand::Mem(addr)) => match addr {
                Address::Base { base, imm } => self.mov_im(*op, *base, *imm),
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

        self.buffer.push(format!("mov {} {}", addr, imm));
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

        self.buffer.push(format!("movl {}, {}", addr, src));
    }

    pub fn movl_im(&mut self, imm: i32, base: Reg, disp: u32) {
        let reg = reg_name(base, 4);

        let addr = if disp == 0 {
            format!("[{}]", reg)
        } else {
            format!("[{} + {}]", reg, disp)
        };

        self.buffer.push(format!("mov {}, {}", addr, imm));
    }

    pub fn sub_ir(&mut self, imm: u32, dst: Reg) {
        let dst = reg_name(dst, 8);
        self.buffer.push(format!("sub {}, {}", dst, imm));
    }

    pub fn xorl_rr(&mut self, src: Reg, dst: Reg) {
        let src = reg_name(src, 4);
        let dst = reg_name(dst, 4);

        self.buffer.push(format!("xorl {} {}", dst, src));
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

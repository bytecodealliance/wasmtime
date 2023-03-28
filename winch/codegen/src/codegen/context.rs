use crate::{
    frame::Frame,
    masm::{MacroAssembler, OperandSize, RegImm},
    reg::Reg,
    regalloc::RegAlloc,
    stack::{Stack, Val},
};
use std::ops::RangeBounds;

/// The code generation context.
/// The code generation context is made up of three
/// essential data structures:
///
/// * The register allocator, in charge of keeping the inventory of register
///   availability.
/// * The value stack, which keeps track of the state of the values
///   after each operation.
/// * The current function's frame.
///
/// These data structures normally require cooperating with each other
/// to perform most of the operations needed during the code
/// generation process. The code generation context should
/// be generally used as the single entry point to access
/// the compound functionality provided by its elements.
pub(crate) struct CodeGenContext<'a> {
    /// The register allocator.
    pub regalloc: RegAlloc,
    /// The value stack.
    pub stack: Stack,
    /// The current function's frame.
    pub frame: &'a Frame,
}

impl<'a> CodeGenContext<'a> {
    /// Create a new code generation context.
    pub fn new(regalloc: RegAlloc, stack: Stack, frame: &'a Frame) -> Self {
        Self {
            regalloc,
            stack,
            frame,
        }
    }

    /// Request a specific general purpose register to the register allocator,
    /// spilling if not available.
    pub fn gpr<M: MacroAssembler>(&mut self, named: Reg, masm: &mut M) -> Reg {
        self.regalloc.gpr(named, &mut |regalloc| {
            Self::spill(&mut self.stack, regalloc, &self.frame, masm)
        })
    }

    /// Request the next avaiable general purpose register to the register allocator,
    /// spilling if no registers are available.
    pub fn any_gpr<M: MacroAssembler>(&mut self, masm: &mut M) -> Reg {
        self.regalloc
            .any_gpr(&mut |regalloc| Self::spill(&mut self.stack, regalloc, &self.frame, masm))
    }

    /// Free the given general purpose register.
    pub fn free_gpr(&mut self, reg: Reg) {
        self.regalloc.free_gpr(reg);
    }

    /// Loads the stack top value into the next available register, if
    /// it isn't already one; spilling if there are no registers
    /// available.  Optionally the caller may specify a specific
    /// destination register.
    pub fn pop_to_reg<M: MacroAssembler>(
        &mut self,
        masm: &mut M,
        named: Option<Reg>,
        size: OperandSize,
    ) -> Reg {
        let (in_stack, dst) = if let Some(dst) = named {
            self.stack
                .pop_named_reg(dst)
                .map(|reg| (true, reg))
                .unwrap_or_else(|| (false, self.gpr(dst, masm)))
        } else {
            self.stack
                .pop_reg()
                .map(|reg| (true, reg))
                .unwrap_or_else(|| (false, self.any_gpr(masm)))
        };

        if in_stack {
            return dst;
        }

        let val = self.stack.pop().expect("a value at stack top");
        if val.is_mem() {
            masm.pop(dst);
        } else {
            self.move_val_to_reg(&val, dst, masm, size);
        }

        dst
    }

    /// Move a stack value to the given register.
    pub fn move_val_to_reg<M: MacroAssembler>(
        &self,
        src: &Val,
        dst: Reg,
        masm: &mut M,
        size: OperandSize,
    ) {
        match src {
            Val::Reg(src) => masm.mov(RegImm::reg(*src), RegImm::reg(dst), size),
            Val::I32(imm) => masm.mov(RegImm::imm((*imm).into()), RegImm::reg(dst), size),
            Val::I64(imm) => masm.mov(RegImm::imm(*imm), RegImm::reg(dst), size),
            Val::Local(index) => {
                let slot = self
                    .frame
                    .get_local(*index)
                    .unwrap_or_else(|| panic!("valid local at index = {}", index));
                let addr = masm.local_address(&slot);
                masm.load(addr, dst, slot.ty.into());
            }
            Val::Memory(offset) => {
                let addr = masm.address_from_sp(*offset);
                masm.load(addr, dst, size);
            }
        };
    }

    /// Prepares arguments for emitting an i32 binary operation.
    pub fn i32_binop<F, M>(&mut self, masm: &mut M, emit: &mut F)
    where
        F: FnMut(&mut M, RegImm, RegImm, OperandSize),
        M: MacroAssembler,
    {
        let top = self.stack.peek().expect("value at stack top");

        if top.is_i32_const() {
            let val = self
                .stack
                .pop_i32_const()
                .expect("i32 const value at stack top");
            let reg = self.pop_to_reg(masm, None, OperandSize::S32);
            emit(
                masm,
                RegImm::reg(reg),
                RegImm::imm(val as i64),
                OperandSize::S32,
            );
            self.stack.push(Val::reg(reg));
        } else {
            let src = self.pop_to_reg(masm, None, OperandSize::S32);
            let dst = self.pop_to_reg(masm, None, OperandSize::S32);
            emit(masm, dst.into(), src.into(), OperandSize::S32);
            self.regalloc.free_gpr(src);
            self.stack.push(Val::reg(dst));
        }
    }

    /// Prepares arguments for emitting an i64 binary operation.
    pub fn i64_binop<F, M>(&mut self, masm: &mut M, emit: &mut F)
    where
        F: FnMut(&mut M, RegImm, RegImm, OperandSize),
        M: MacroAssembler,
    {
        let top = self.stack.peek().expect("value at stack top");
        if top.is_i64_const() {
            let val = self
                .stack
                .pop_i64_const()
                .expect("i64 const value at stack top");
            let reg = self.pop_to_reg(masm, None, OperandSize::S64);
            emit(masm, RegImm::reg(reg), RegImm::imm(val), OperandSize::S64);
            self.stack.push(Val::reg(reg));
        } else {
            let src = self.pop_to_reg(masm, None, OperandSize::S64);
            let dst = self.pop_to_reg(masm, None, OperandSize::S64);
            emit(masm, dst.into(), src.into(), OperandSize::S64);
            self.regalloc.free_gpr(src);
            self.stack.push(Val::reg(dst));
        }
    }

    /// Saves any live registers in the value stack in a particular
    /// range defined by the caller.  This is a specialization of the
    /// spill function; made available for cases in which spilling
    /// locals is not required, like for example for function calls in
    /// which locals are not reachable by the callee.  It also tracks
    /// down the number of memory values in the given range.
    ///
    /// Returns the number of spilled registers and the number of
    /// memory values in the given range of the value stack.
    pub fn spill_regs_and_count_memory_in<M, R>(&mut self, masm: &mut M, range: R) -> (u32, u32)
    where
        R: RangeBounds<usize>,
        M: MacroAssembler,
    {
        let mut spilled: u32 = 0;
        let mut memory_values = 0;
        for i in self.stack.inner_mut().range_mut(range) {
            if i.is_reg() {
                let reg = i.get_reg();
                let offset = masm.push(reg);
                self.regalloc.free_gpr(reg);
                *i = Val::Memory(offset);
                spilled += 1;
            } else if i.is_mem() {
                memory_values += 1;
            }
        }

        (spilled, memory_values)
    }

    /// Drops the last `n` elements of the stack, freeing any
    /// registers located in that region.
    pub fn drop_last(&mut self, last: usize) {
        let len = self.stack.len();
        assert!(last <= len);
        let truncate = self.stack.len() - last;

        self.stack.inner_mut().range(truncate..).for_each(|v| {
            if v.is_reg() {
                self.regalloc.free_gpr(v.get_reg());
            }
        });
        self.stack.inner_mut().truncate(truncate);
    }

    /// Spill locals and registers to memory.
    // TODO optimize the spill range;
    //
    // At any point in the program, the stack
    // might already contain Memory entries;
    // we could effectively ignore that range;
    // only focusing on the range that contains
    // spillable values.
    fn spill<M: MacroAssembler>(
        stack: &mut Stack,
        regalloc: &mut RegAlloc,
        frame: &Frame,
        masm: &mut M,
    ) {
        stack.inner_mut().iter_mut().for_each(|v| match v {
            Val::Reg(r) => {
                let offset = masm.push(*r);
                regalloc.free_gpr(*r);
                *v = Val::Memory(offset);
            }
            Val::Local(index) => {
                let slot = frame.get_local(*index).expect("valid local at slot");
                let addr = masm.local_address(&slot);
                masm.load(addr, regalloc.scratch, slot.ty.into());
                let offset = masm.push(regalloc.scratch);
                *v = Val::Memory(offset);
            }
            v => {
                println!("trying to spill something unknown {:?}", v);
            }
        });
    }
}

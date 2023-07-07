use super::ControlStackFrame;
use crate::{
    abi::{ABIResult, ABI},
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
    /// Reachability state.
    pub reachable: bool,
}

impl<'a> CodeGenContext<'a> {
    /// Create a new code generation context.
    pub fn new(regalloc: RegAlloc, stack: Stack, frame: &'a Frame) -> Self {
        Self {
            regalloc,
            stack,
            frame,
            reachable: true,
        }
    }

    /// Request a specific general purpose register to the register allocator,
    /// spilling if not available.
    pub fn gpr<M: MacroAssembler>(&mut self, named: Reg, masm: &mut M) -> Reg {
        self.regalloc.gpr(named, &mut |regalloc| {
            Self::spill_impl(&mut self.stack, regalloc, &self.frame, masm)
        })
    }

    /// Request the next avaiable general purpose register to the register allocator,
    /// spilling if no registers are available.
    pub fn any_gpr<M: MacroAssembler>(&mut self, masm: &mut M) -> Reg {
        self.regalloc
            .any_gpr(&mut |regalloc| Self::spill_impl(&mut self.stack, regalloc, &self.frame, masm))
    }

    /// Free the given general purpose register.
    pub fn free_gpr(&mut self, reg: Reg) {
        self.regalloc.free_gpr(reg);
    }

    /// Loads the stack top value into the next available register, if
    /// it isn't already one; spilling if there are no registers
    /// available.  Optionally the caller may specify a specific
    /// destination register.
    /// When a named register is requested and it's not at the top of the
    /// stack a move from register to register might happen, in which case
    /// the source register will be freed.
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
            // Free the source value if it is a register.
            if val.is_reg() {
                self.regalloc.free_gpr(val.get_reg());
            }
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
        }
    }

    /// Prepares arguments for emitting a unary operation.
    pub fn unop<F, M>(&mut self, masm: &mut M, size: OperandSize, emit: &mut F)
    where
        F: FnMut(&mut M, Reg, OperandSize),
        M: MacroAssembler,
    {
        let reg = self.pop_to_reg(masm, None, size);
        emit(masm, reg, size);
        self.stack.push(Val::reg(reg));
    }

    /// Prepares arguments for emitting an i32 binary operation.
    pub fn i32_binop<F, M>(&mut self, masm: &mut M, mut emit: F)
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
    pub fn i64_binop<F, M>(&mut self, masm: &mut M, mut emit: F)
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

    /// Pops the stack pointer to ensure that it is correctly placed according to the expectations
    /// of the destination branch.
    ///
    /// This function must be used when performing unconditional jumps, as the machine stack might
    /// be left unbalanced at the jump site, due to register spills. In this context unbalanced
    /// refers to possible extra space created at the jump site, which might cause invaid memory
    /// accesses. Note that in some cases the stack pointer offset might be already less than or
    /// equal to the original stack pointer offset registered when entering the destination control
    /// stack frame, which effectively means that when reaching the jump site no extra space was
    /// allocated similar to what would happen in a fall through in which we assume that the
    /// program has allocated and deallocated the right amount of stack space.
    ///
    /// More generally speaking the current stack pointer will be less than the original stack
    /// pointer offset in cases in which the top value in the value stack is a memory entry which
    /// needs to be popped into the return location according to the ABI (a register for single
    /// value returns and a memory slot for 1+ returns). In short, this could happen given that we
    /// handle return values preemptively when emitting unconditional branches, and push them back
    /// to the value stack at control flow joins.
    pub fn pop_sp_for_branch<M: MacroAssembler>(
        &mut self,
        destination: &ControlStackFrame,
        masm: &mut M,
    ) {
        let (_, original_sp_offset) = destination.original_stack_len_and_sp_offset();
        let current_sp_offset = masm.sp_offset();

        assert!(
            current_sp_offset >= original_sp_offset
                || (current_sp_offset + <M::ABI as ABI>::word_bytes()) == original_sp_offset
        );

        if current_sp_offset > original_sp_offset {
            masm.free_stack(current_sp_offset - original_sp_offset);
        }
    }

    /// Convenience wrapper around [`Self::spill_callback`].
    ///
    /// This function exists for cases in which triggering an unconditional
    /// spill is needed, like before entering control flow.
    pub fn spill<M: MacroAssembler>(&mut self, masm: &mut M) {
        Self::spill_impl(&mut self.stack, &mut self.regalloc, &mut self.frame, masm);
    }

    /// Handles the emission of the ABI result. This function is used at the end
    /// of a block or function to pop the results from the value stack into the
    /// corresponding ABI result representation.
    pub fn pop_abi_results<M: MacroAssembler>(&mut self, result: &ABIResult, masm: &mut M) {
        if result.is_void() {
            return;
        }

        let reg = self.pop_to_reg(masm, Some(result.result_reg()), OperandSize::S64);
        self.regalloc.free_gpr(reg);
    }

    /// Push ABI results in to the value stack. This function is used at the end
    /// of a block or after a function call to push the corresponding ABI
    /// results into the value stack.
    pub fn push_abi_results<M: MacroAssembler>(&mut self, result: &ABIResult, masm: &mut M) {
        if result.is_void() {
            return;
        }

        match result {
            ABIResult::Reg { reg, .. } => {
                assert!(self.regalloc.gpr_available(*reg));
                let result_reg = Val::reg(self.gpr(*reg, masm));
                self.stack.push(result_reg);
            }
        }
    }

    /// Pops the value at the stack top and assigns it to the local at the given
    /// index, returning the register holding the source value.
    pub fn set_local<M: MacroAssembler>(&mut self, masm: &mut M, index: u32) -> Reg {
        let slot = self
            .frame
            .get_local(index)
            .unwrap_or_else(|| panic!("valid local at slot = {}", index));
        let size: OperandSize = slot.ty.into();
        let src = self.pop_to_reg(masm, None, size);
        let addr = masm.local_address(&slot);
        masm.store(RegImm::reg(src), addr, size);

        src
    }

    /// Spill locals and registers to memory.
    // TODO optimize the spill range;
    //
    // At any point in the program, the stack
    // might already contain Memory entries;
    // we could effectively ignore that range;
    // only focusing on the range that contains
    // spillable values.
    fn spill_impl<M: MacroAssembler>(
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
            _ => {}
        });
    }
}

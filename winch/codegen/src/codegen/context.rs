use crate::{
    frame::Frame,
    masm::{MacroAssembler, OperandSize, RegImm},
    reg::Reg,
    regalloc::RegAlloc,
    stack::{Stack, Val},
};

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

    /// Loads the stack top value into a register, if it isn't already one;
    /// spilling if there are no registers available.
    pub fn pop_to_reg<M: MacroAssembler>(&mut self, masm: &mut M, size: OperandSize) -> Reg {
        if let Some(reg) = self.stack.pop_reg() {
            return reg;
        }

        let dst = self.any_gpr(masm);
        let val = self.stack.pop().expect("a value at stack top");
        Self::move_val_to_reg(val, dst, masm, self.frame, size);
        dst
    }

    /// Checks if the stack top contains the given register. The register
    /// gets allocated otherwise, potentially causing a spill.
    /// Once the requested register is allocated, the value at the top of the stack
    /// gets loaded into the register.
    pub fn pop_to_named_reg<M: MacroAssembler>(
        &mut self,
        masm: &mut M,
        named: Reg,
        size: OperandSize,
    ) -> Reg {
        if let Some(reg) = self.stack.pop_named_reg(named) {
            return reg;
        }

        let dst = self.gpr(named, masm);
        let val = self.stack.pop().expect("a value at stack top");
        Self::move_val_to_reg(val, dst, masm, self.frame, size);
        dst
    }

    fn move_val_to_reg<M: MacroAssembler>(
        src: Val,
        dst: Reg,
        masm: &mut M,
        frame: &Frame,
        size: OperandSize,
    ) {
        match src {
            Val::Reg(src) => masm.mov(RegImm::reg(src), RegImm::reg(dst), size),
            Val::I32(imm) => masm.mov(RegImm::imm(imm.into()), RegImm::reg(dst), size),
            Val::I64(imm) => masm.mov(RegImm::imm(imm), RegImm::reg(dst), size),
            Val::Local(index) => {
                let slot = frame
                    .get_local(index)
                    .expect(&format!("valid locat at index = {}", index));
                let addr = masm.local_address(&slot);
                masm.load(addr, dst, slot.ty.into());
            }
            v => panic!("Unsupported value {:?}", v),
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
            let reg = self.pop_to_reg(masm, OperandSize::S32);
            emit(
                masm,
                RegImm::reg(reg),
                RegImm::imm(val as i64),
                OperandSize::S32,
            );
            self.stack.push(Val::reg(reg));
        } else {
            let src = self.pop_to_reg(masm, OperandSize::S32);
            let dst = self.pop_to_reg(masm, OperandSize::S32);
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
            let reg = self.pop_to_reg(masm, OperandSize::S64);
            emit(masm, RegImm::reg(reg), RegImm::imm(val), OperandSize::S64);
            self.stack.push(Val::reg(reg));
        } else {
            let src = self.pop_to_reg(masm, OperandSize::S64);
            let dst = self.pop_to_reg(masm, OperandSize::S64);
            emit(masm, dst.into(), src.into(), OperandSize::S64);
            self.regalloc.free_gpr(src);
            self.stack.push(Val::reg(dst));
        }
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

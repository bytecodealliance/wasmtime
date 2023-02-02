use crate::{
    codegen::CodeGenContext,
    frame::Frame,
    isa::reg::Reg,
    masm::{MacroAssembler, OperandSize, RegImm},
    regset::RegSet,
    stack::Val,
};

/// The register allocator.
///
/// The register allocator uses a single-pass algorithm;
/// its implementation uses a bitset as a freelist
/// to track per-class register availability.
///
/// If a particular register is not available upon request
/// the register allocation will perform a "spill", essentially
/// moving Local and Register values in the stack to memory.
/// This processs ensures that whenever a register is requested,
/// it is going to be available.
pub(crate) struct RegAlloc {
    pub scratch: Reg,
    regset: RegSet,
}

impl RegAlloc {
    /// Create a new register allocator
    /// from a register set.
    pub fn new(regset: RegSet, scratch: Reg) -> Self {
        Self { regset, scratch }
    }

    /// Loads the stack top value into a register, if it isn't already one;
    /// spilling if there are no registers available.
    pub fn pop_to_reg<M: MacroAssembler>(
        &mut self,
        context: &mut CodeGenContext<M>,
        size: OperandSize,
    ) -> Reg {
        if let Some(reg) = context.stack.pop_reg() {
            return reg;
        }

        let dst = self.any_gpr(context);
        let val = context.stack.pop().expect("a value at stack top");
        Self::move_val_to_reg(val, dst, context.masm, context.frame, size);
        dst
    }

    /// Checks if the stack top contains the given register. The register
    /// gets allocated otherwise, potentially causing a spill.
    /// Once the requested register is allocated, the value at the top of the stack
    /// gets loaded into the register.
    pub fn pop_to_named_reg<M: MacroAssembler>(
        &mut self,
        context: &mut CodeGenContext<M>,
        named: Reg,
        size: OperandSize,
    ) -> Reg {
        if let Some(reg) = context.stack.pop_named_reg(named) {
            return reg;
        }

        let dst = self.gpr(context, named);
        let val = context.stack.pop().expect("a value at stack top");
        Self::move_val_to_reg(val, dst, context.masm, context.frame, size);
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
            Val::I32(imm) => masm.mov(RegImm::imm(imm), RegImm::reg(dst), size),
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

    /// Allocate the next available general purpose register,
    /// spilling if none available.
    pub fn any_gpr<M: MacroAssembler>(&mut self, context: &mut CodeGenContext<M>) -> Reg {
        self.regset.any_gpr().unwrap_or_else(|| {
            self.spill(context);
            self.regset.any_gpr().expect("any gpr to be available")
        })
    }

    /// Request a specific general purpose register,
    /// spilling if not available.
    pub fn gpr<M: MacroAssembler>(&mut self, context: &mut CodeGenContext<M>, named: Reg) -> Reg {
        self.regset.gpr(named).unwrap_or_else(|| {
            self.spill(context);
            self.regset
                .gpr(named)
                .expect(&format!("gpr {:?} to be available", named))
        })
    }

    /// Mark a particular general purpose register as available.
    pub fn free_gpr(&mut self, reg: Reg) {
        self.regset.free_gpr(reg);
    }

    /// Spill locals and registers to memory.
    // TODO optimize the spill range;
    //
    // At any point in the program, the stack
    // might already contain Memory entries;
    // we could effectively ignore that range;
    // only focusing on the range that contains
    // spillable values.
    fn spill<M: MacroAssembler>(&mut self, context: &mut CodeGenContext<M>) {
        context.stack.inner_mut().iter_mut().for_each(|v| match v {
            Val::Reg(r) => {
                let offset = context.masm.push(*r);
                self.free_gpr(*r);
                *v = Val::Memory(offset);
            }
            Val::Local(index) => {
                let slot = context
                    .frame
                    .get_local(*index)
                    .expect("valid local at slot");
                let addr = context.masm.local_address(&slot);
                context
                    .masm
                    .store(RegImm::reg(self.scratch), addr, slot.ty.into());
                let offset = context.masm.push(self.scratch);
                *v = Val::Memory(offset);
            }
            _ => {}
        });
    }
}

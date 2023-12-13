use wasmtime_environ::{VMOffsets, WasmHeapType, WasmType};

use super::ControlStackFrame;
use crate::{
    abi::{ABIOperand, ABIResultsData, RetArea, ABI},
    codegen::BuiltinFunctions,
    frame::Frame,
    isa::reg::RegClass,
    masm::{MacroAssembler, OperandSize, RegImm, SPOffset, StackSlot},
    reg::Reg,
    regalloc::RegAlloc,
    stack::{Stack, TypedReg, Val},
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
pub(crate) struct CodeGenContext<'a, 'builtins: 'a> {
    /// The register allocator.
    pub regalloc: RegAlloc,
    /// The value stack.
    pub stack: Stack,
    /// The current function's frame.
    pub frame: Frame,
    /// Reachability state.
    pub reachable: bool,
    /// The built-in functions available to the JIT code.
    pub builtins: &'builtins mut BuiltinFunctions,
    /// A reference to the VMOffsets.
    pub vmoffsets: &'a VMOffsets<u8>,
}

impl<'a, 'builtins> CodeGenContext<'a, 'builtins> {
    /// Create a new code generation context.
    pub fn new(
        regalloc: RegAlloc,
        stack: Stack,
        frame: Frame,
        builtins: &'builtins mut BuiltinFunctions,
        vmoffsets: &'a VMOffsets<u8>,
    ) -> Self {
        Self {
            regalloc,
            stack,
            frame,
            reachable: true,
            builtins,
            vmoffsets,
        }
    }

    /// Request a specific register to the register allocator,
    /// spilling if not available.
    pub fn reg<M: MacroAssembler>(&mut self, named: Reg, masm: &mut M) -> Reg {
        self.regalloc.reg(named, |regalloc| {
            Self::spill_impl(&mut self.stack, regalloc, &self.frame, masm)
        })
    }

    /// Allocate a register for the given WebAssembly type.
    pub fn reg_for_type<M: MacroAssembler>(&mut self, ty: WasmType, masm: &mut M) -> Reg {
        use WasmType::*;
        match ty {
            I32 | I64 => self.reg_for_class(RegClass::Int, masm),
            F32 | F64 => self.reg_for_class(RegClass::Float, masm),
            Ref(rt) => match rt.heap_type {
                WasmHeapType::Func => self.reg_for_class(RegClass::Int, masm),
                ht => unimplemented!("Support for WasmHeapType: {ht}"),
            },
            t => unimplemented!("Support for WasmType: {t}"),
        }
    }

    /// Request the register allocator to provide the next available
    /// register of the specified class.
    pub fn reg_for_class<M: MacroAssembler>(&mut self, class: RegClass, masm: &mut M) -> Reg {
        self.regalloc.reg_for_class(class, &mut |regalloc| {
            Self::spill_impl(&mut self.stack, regalloc, &self.frame, masm)
        })
    }

    /// Convenience wrapper around `CodeGenContext::reg_for_class`, to
    /// request the next available general purpose register.
    pub fn any_gpr<M: MacroAssembler>(&mut self, masm: &mut M) -> Reg {
        self.reg_for_class(RegClass::Int, masm)
    }

    /// Executes the provided function, guaranteeing that the specified set of
    /// registers, if any, remain unallocatable throughout the function's
    /// execution.
    pub fn without<'r, T, M, F>(
        &mut self,
        regs: impl IntoIterator<Item = &'r Reg> + Copy,
        masm: &mut M,
        mut f: F,
    ) -> T
    where
        M: MacroAssembler,
        F: FnMut(&mut Self, &mut M) -> T,
    {
        for r in regs {
            self.reg(*r, masm);
        }

        let result = f(self, masm);

        for r in regs {
            self.free_reg(*r);
        }

        result
    }

    /// Free the given register.
    pub fn free_reg(&mut self, reg: impl Into<Reg>) {
        let reg: Reg = reg.into();
        self.regalloc.free(reg);
    }

    /// Loads the stack top value into the next available register, if
    /// it isn't already one; spilling if there are no registers
    /// available.  Optionally the caller may specify a specific
    /// destination register.
    /// When a named register is requested and it's not at the top of the
    /// stack a move from register to register might happen, in which case
    /// the source register will be freed.
    pub fn pop_to_reg<M: MacroAssembler>(&mut self, masm: &mut M, named: Option<Reg>) -> TypedReg {
        let typed_reg = if let Some(dst) = named {
            self.stack.pop_named_reg(dst)
        } else {
            self.stack.pop_reg()
        };

        if let Some(dst) = typed_reg {
            return dst;
        }

        let val = self.stack.pop().expect("a value at stack top");
        let reg = if let Some(r) = named {
            self.reg(r, masm)
        } else {
            self.reg_for_type(val.ty(), masm)
        };

        if val.is_mem() {
            let mem = val.unwrap_mem();
            debug_assert!(mem.slot.offset.as_u32() == masm.sp_offset().as_u32());
            masm.pop(reg, val.ty().into());
        } else {
            self.move_val_to_reg(&val, reg, masm);
            // Free the source value if it is a register.
            if val.is_reg() {
                self.free_reg(val.unwrap_reg());
            }
        }

        TypedReg::new(val.ty(), reg)
    }

    /// Pops the value stack top and stores it at the specified address.
    fn pop_to_addr<M: MacroAssembler>(&mut self, masm: &mut M, addr: M::Address) {
        let val = self.stack.pop().expect("a value at stack top");
        let size: OperandSize = val.ty().into();
        match val {
            Val::Reg(tr) => {
                masm.store(tr.reg.into(), addr, size);
                self.free_reg(tr.reg);
            }
            Val::I32(v) => masm.store(RegImm::i32(v), addr, size),
            Val::I64(v) => masm.store(RegImm::i64(v), addr, size),
            Val::F32(v) => masm.store(RegImm::f32(v.bits()), addr, size),
            Val::F64(v) => masm.store(RegImm::f64(v.bits()), addr, size),
            Val::Local(local) => {
                let slot = self
                    .frame
                    .get_local(local.index)
                    .unwrap_or_else(|| panic!("invalid local at index = {}", local.index));
                let scratch = <M::ABI as ABI>::scratch_reg();
                let local_addr = masm.local_address(&slot);
                masm.load(local_addr, scratch, size);
                masm.store(scratch.into(), addr, size);
            }
            Val::Memory(_) => {
                let scratch = <M::ABI as ABI>::scratch_reg();
                masm.pop(scratch, size);
                masm.store(scratch.into(), addr, size);
            }
        }
    }

    /// Move a stack value to the given register.
    pub fn move_val_to_reg<M: MacroAssembler>(&self, src: &Val, dst: Reg, masm: &mut M) {
        let size: OperandSize = src.ty().into();
        match src {
            Val::Reg(tr) => masm.mov(RegImm::reg(tr.reg), dst, size),
            Val::I32(imm) => masm.mov(RegImm::i32(*imm), dst, size),
            Val::I64(imm) => masm.mov(RegImm::i64(*imm), dst, size),
            Val::F32(imm) => masm.mov(RegImm::f32(imm.bits()), dst, size),
            Val::F64(imm) => masm.mov(RegImm::f64(imm.bits()), dst, size),
            Val::Local(local) => {
                let slot = self
                    .frame
                    .get_local(local.index)
                    .unwrap_or_else(|| panic!("invalid local at index = {}", local.index));
                let addr = masm.local_address(&slot);
                masm.load(addr, dst, slot.ty.into());
            }
            Val::Memory(mem) => {
                let addr = masm.address_from_sp(mem.slot.offset);
                masm.load(addr, dst, size);
            }
        }
    }

    /// Prepares arguments for emitting a unary operation.
    ///
    /// The `emit` function returns the `TypedReg` to put on the value stack.
    pub fn unop<F, M>(&mut self, masm: &mut M, size: OperandSize, emit: &mut F)
    where
        F: FnMut(&mut M, Reg, OperandSize) -> TypedReg,
        M: MacroAssembler,
    {
        let typed_reg = self.pop_to_reg(masm, None);
        let dst = emit(masm, typed_reg.reg, size);
        self.stack.push(dst.into());
    }

    /// Prepares arguments for emitting a binary operation.
    ///
    /// The `emit` function returns the `TypedReg` to put on the value stack.
    pub fn binop<F, M>(&mut self, masm: &mut M, size: OperandSize, mut emit: F)
    where
        F: FnMut(&mut M, Reg, Reg, OperandSize) -> TypedReg,
        M: MacroAssembler,
    {
        let src = self.pop_to_reg(masm, None);
        let dst = self.pop_to_reg(masm, None);
        let dst = emit(masm, dst.reg, src.reg.into(), size);
        self.free_reg(src);
        self.stack.push(dst.into());
    }

    /// Prepares arguments for emitting an f32 or f64 comparison operation.
    pub fn float_cmp_op<F, M>(&mut self, masm: &mut M, size: OperandSize, mut emit: F)
    where
        F: FnMut(&mut M, Reg, Reg, Reg, OperandSize),
        M: MacroAssembler,
    {
        let src1 = self.pop_to_reg(masm, None);
        let src2 = self.pop_to_reg(masm, None);
        let dst = self.any_gpr(masm);
        emit(masm, dst, src1.reg, src2.reg, size);
        self.free_reg(src1);
        self.free_reg(src2);

        let dst = match size {
            OperandSize::S32 => TypedReg::i32(dst),
            OperandSize::S64 => TypedReg::i64(dst),
            OperandSize::S128 => unreachable!(),
        };
        self.stack.push(dst.into());
    }

    /// Prepares arguments for emitting an i32 binary operation.
    ///
    /// The `emit` function returns the `TypedReg` to put on the value stack.
    pub fn i32_binop<F, M>(&mut self, masm: &mut M, mut emit: F)
    where
        F: FnMut(&mut M, Reg, RegImm, OperandSize) -> TypedReg,
        M: MacroAssembler,
    {
        let top = self.stack.peek().expect("value at stack top");

        if top.is_i32_const() {
            let val = self
                .stack
                .pop_i32_const()
                .expect("i32 const value at stack top");
            let typed_reg = self.pop_to_reg(masm, None);
            let dst = emit(masm, typed_reg.reg, RegImm::i32(val), OperandSize::S32);
            self.stack.push(dst.into());
        } else {
            self.binop(masm, OperandSize::S32, |masm, dst, src, size| {
                emit(masm, dst, src.into(), size)
            });
        }
    }

    /// Prepares arguments for emitting an i64 binary operation.
    ///
    /// The `emit` function returns the `TypedReg` to put on the value stack.
    pub fn i64_binop<F, M>(&mut self, masm: &mut M, mut emit: F)
    where
        F: FnMut(&mut M, Reg, RegImm, OperandSize) -> TypedReg,
        M: MacroAssembler,
    {
        let top = self.stack.peek().expect("value at stack top");
        if top.is_i64_const() {
            let val = self
                .stack
                .pop_i64_const()
                .expect("i64 const value at stack top");
            let typed_reg = self.pop_to_reg(masm, None);
            let dst = emit(masm, typed_reg.reg, RegImm::i64(val), OperandSize::S64);
            self.stack.push(dst.into());
        } else {
            self.binop(masm, OperandSize::S64, |masm, dst, src, size| {
                emit(masm, dst, src.into(), size)
            });
        };
    }

    /// Drops the last `n` elements of the stack, calling the provided
    /// function for each `n` stack value.
    /// The values are dropped in top-to-bottom order.
    pub fn drop_last<F>(&mut self, last: usize, mut f: F)
    where
        F: FnMut(&mut RegAlloc, &Val),
    {
        if last > 0 {
            let len = self.stack.len();
            assert!(last <= len);
            let truncate = self.stack.len() - last;
            let stack_mut = &mut self.stack.inner_mut();

            // Invoke the callback in top-to-bottom order.
            for v in stack_mut.range(truncate..).rev() {
                f(&mut self.regalloc, v)
            }
            stack_mut.truncate(truncate);
        }
    }

    /// Convenience wrapper around [`Self::spill_callback`].
    ///
    /// This function exists for cases in which triggering an unconditional
    /// spill is needed, like before entering control flow.
    pub fn spill<M: MacroAssembler>(&mut self, masm: &mut M) {
        Self::spill_impl(&mut self.stack, &mut self.regalloc, &mut self.frame, masm);
    }

    /// Prepares the compiler to emit an uncoditional jump to the given
    /// destination branch.  This process involves:
    /// * Balancing the machine
    ///   stack pointer and value stack by popping it to match the destination
    ///   branch.
    /// * Updating the reachability state.
    /// * Marking the destination frame as a destination target.
    pub fn unconditional_jump<M, F>(&mut self, dest: &mut ControlStackFrame, masm: &mut M, mut f: F)
    where
        M: MacroAssembler,
        F: FnMut(&mut M, &mut Self, &mut ControlStackFrame),
    {
        let (_, target_sp) = dest.base_stack_len_and_sp();
        // Invariant: The SP, must be greater or equal to the target
        // SP, given that we haven't popped any results by this point
        // yet. But it may happen in the callback.
        assert!(masm.sp_offset().as_u32() >= target_sp.as_u32());
        f(masm, self, dest);

        // The following snippet, pops the stack pointer and to ensure that it
        // is correctly placed according to the expectations of the destination
        // branch.
        //
        // This is done in the context of unconditional jumps, as the machine
        // stack might be left unbalanced at the jump site, due to register
        // spills. Note that in some cases the stack pointer offset might be
        // already less than or equal to the original stack pointer offset
        // registered when entering the destination control stack frame, which
        // effectively means that when reaching the jump site no extra space was
        // allocated similar to what would happen in a fall through in which we
        // assume that the program has allocated and deallocated the right
        // amount of stack space.
        //
        // More generally speaking the current stack pointer will be less than
        // the original stack pointer offset in cases in which the top value in
        // the value stack is a memory entry which needs to be popped into the
        // return location according to the ABI (a register for single value
        // returns and a memory slot for 1+ returns). This could happen in the
        // callback invocation above if the callback invokes
        // `CodeGenContext::pop_abi_results` (e.g. `br` instruction).
        //
        // After an unconditional jump, the compiler will enter in an
        // unreachable state; instead of immediately truncating the value stack
        // to the expected length of the destination branch, we let the
        // reachability analysis code decide what should happen with the length
        // of the value stack once reachability is actually restored. At that
        // point, the right stack pointer offset will also be restored, which
        // should match the contents of the value stack.
        masm.ensure_sp_for_jump(target_sp);
        dest.set_as_target();
        masm.jmp(*dest.label());
        self.reachable = false;
    }

    /// A combination of [Self::pop_abi_results] and [Self::push_abi_results]
    /// to be used on conditional branches: br_if and br_table.
    pub fn top_abi_results<M: MacroAssembler>(&mut self, result: &ABIResultsData, masm: &mut M) {
        self.pop_abi_results(result, masm);
        self.push_abi_results(result, masm);
    }

    /// Handles the emission of the ABI result. This function is used at the end
    /// of a block or function to pop the results from the value stack into the
    /// corresponding ABI result location.
    pub fn pop_abi_results<M: MacroAssembler>(&mut self, data: &ABIResultsData, masm: &mut M) {
        let retptr = data
            .results
            .has_stack_results()
            .then(|| match data.unwrap_ret_area() {
                RetArea::Slot(slot) => {
                    let base = self
                        .without::<_, M, _>(data.results.regs(), masm, |cx, masm| cx.any_gpr(masm));
                    let local_addr = masm.local_address(slot);
                    masm.load_ptr(local_addr, base);
                    Some(base)
                }
                _ => None,
            })
            .flatten();

        // Results are popped in reverse order, starting from registers, continuing
        // to memory values in order to maintain the value stack ordering invariant.
        // See comments in [ABIResults] for more details.
        for operand in data.results.operands().iter().rev() {
            match operand {
                ABIOperand::Reg { reg, .. } => {
                    let TypedReg { reg, .. } = self.pop_to_reg(masm, Some(*reg));
                    self.free_reg(reg);
                }
                ABIOperand::Stack { offset, .. } => {
                    let addr = match data.unwrap_ret_area() {
                        RetArea::SP(base) => {
                            let slot_offset = base.as_u32() - *offset;
                            masm.address_from_sp(SPOffset::from_u32(slot_offset))
                        }
                        RetArea::Slot(_) => masm.address_at_reg(retptr.unwrap(), *offset),
                    };

                    self.pop_to_addr(masm, addr);
                }
            }
        }

        if let Some(reg) = retptr {
            self.free_reg(reg);
        }
    }

    /// Push ABI results into the value stack. This function is used at the end
    /// of a block or after a function call to push the corresponding ABI
    /// results into the value stack.
    pub fn push_abi_results<M: MacroAssembler>(&mut self, data: &ABIResultsData, masm: &mut M) {
        for operand in data.results.operands().iter() {
            match operand {
                ABIOperand::Reg { reg, ty, .. } => {
                    assert!(self.regalloc.reg_available(*reg));
                    let typed_reg = TypedReg::new(*ty, self.reg(*reg, masm));
                    self.stack.push(typed_reg.into());
                }
                ABIOperand::Stack { ty, offset, size } => match data.unwrap_ret_area() {
                    RetArea::SP(sp_offset) => {
                        let slot =
                            StackSlot::new(SPOffset::from_u32(sp_offset.as_u32() - offset), *size);
                        self.stack.push(Val::mem(*ty, slot));
                    }
                    // This function is only expected to be called when dealing
                    // with control flow and when calling functions; as a
                    // callee, only [Self::pop_abi_results] is needed when
                    // finalizing the function compilation.
                    _ => unreachable!(),
                },
            }
        }
    }

    /// Truncates the value stack to the specified target.
    /// This function is intended to only be used when restoring the code
    /// generation's reachability state, when handling an unreachable end or
    /// else.
    fn truncate_stack_to(&mut self, target: usize) {
        if self.stack.len() > target {
            self.drop_last(self.stack.len() - target, |regalloc, val| match val {
                Val::Reg(tr) => regalloc.free(tr.reg),
                _ => {}
            });
        }
    }

    /// This function ensures that the state of the -- machine and value --
    /// stack  is the right one when reaching a control frame branch in which
    /// reachability is restored or when reaching the end of a function in an
    /// unreachable state. This function is intended to be called when handling
    /// an unreachable else or end.
    ///
    /// This function will truncate the value stack to the length expected by
    /// the control frame and will also set the stack pointer offset to
    /// reflect the new length of the value stack.
    pub fn ensure_stack_state<M: MacroAssembler>(
        &mut self,
        masm: &mut M,
        frame: &ControlStackFrame,
    ) {
        let (base_len, base_sp) = frame.base_stack_len_and_sp();
        masm.reset_stack_pointer(base_sp);
        self.truncate_stack_to(base_len);

        // The size of the stack sometimes can be less given that locals are
        // removed last, and not accounted as part of the [SPOffset].
        debug_assert!(self.stack.sizeof(self.stack.len()) <= base_sp.as_u32());
    }

    /// Spill locals and registers to memory.
    // TODO: optimize the spill range;
    // At any point in the program, the stack might already contain memory
    // entries; we could effectively ignore that range; only focusing on the
    // range that contains spillable values.
    fn spill_impl<M: MacroAssembler>(
        stack: &mut Stack,
        regalloc: &mut RegAlloc,
        frame: &Frame,
        masm: &mut M,
    ) {
        stack.inner_mut().iter_mut().for_each(|v| match v {
            Val::Reg(r) => {
                let slot = masm.push(r.reg, r.ty.into());
                regalloc.free(r.reg);
                *v = Val::mem(r.ty, slot);
            }
            Val::Local(local) => {
                let slot = frame.get_local(local.index).expect("valid local at slot");
                let addr = masm.local_address(&slot);
                let scratch = <M::ABI as ABI>::scratch_for(&slot.ty);
                masm.load(addr, scratch, slot.ty.into());
                let stack_slot = masm.push(scratch, slot.ty.into());
                *v = Val::mem(slot.ty, stack_slot);
            }
            _ => {}
        });
    }
}

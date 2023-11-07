use wasmtime_environ::{VMOffsets, WasmHeapType, WasmType};

use super::ControlStackFrame;
use crate::{
    abi::{ABIResult, ABI},
    codegen::BuiltinFunctions,
    frame::Frame,
    isa::reg::RegClass,
    masm::{MacroAssembler, OperandSize, RegImm},
    reg::Reg,
    regalloc::RegAlloc,
    stack::{Stack, TypedReg, Val},
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
    /// execution. Only the registers in the `free` iterator will be freed. The
    /// caller must guarantee that in case the iterators are different, the free
    /// iterator must be a subset of the alloc iterator.
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

    /// Similar to [`Self::without`] but takes an optional, single register
    /// as a paramter.
    pub fn maybe_without1<T, M, F>(&mut self, reg: Option<Reg>, masm: &mut M, mut f: F) -> T
    where
        M: MacroAssembler,
        F: FnMut(&mut Self, &mut M) -> T,
    {
        match reg {
            Some(r) => self.without(&[r], masm, f),
            None => f(self, masm),
        }
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
            masm.pop(reg, val.ty().into());
        } else {
            self.move_val_to_reg(&val, reg, masm);
            // Free the source value if it is a register.
            if val.is_reg() {
                self.free_reg(val.get_reg());
            }
        }

        TypedReg::new(val.ty(), reg)
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
                    .unwrap_or_else(|| panic!("valid local at index = {}", local.index));
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
    pub fn unop<F, M>(&mut self, masm: &mut M, size: OperandSize, emit: &mut F)
    where
        F: FnMut(&mut M, Reg, OperandSize),
        M: MacroAssembler,
    {
        let typed_reg = self.pop_to_reg(masm, None);
        emit(masm, typed_reg.reg, size);
        self.stack.push(typed_reg.into());
    }

    /// Prepares arguments for emitting a binary operation.
    pub fn binop<F, M>(&mut self, masm: &mut M, size: OperandSize, mut emit: F)
    where
        F: FnMut(&mut M, Reg, Reg, OperandSize),
        M: MacroAssembler,
    {
        let src = self.pop_to_reg(masm, None);
        let dst = self.pop_to_reg(masm, None);
        emit(masm, dst.reg, src.reg.into(), size);
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
    pub fn i32_binop<F, M>(&mut self, masm: &mut M, mut emit: F)
    where
        F: FnMut(&mut M, Reg, RegImm, OperandSize),
        M: MacroAssembler,
    {
        let top = self.stack.peek().expect("value at stack top");

        if top.is_i32_const() {
            let val = self
                .stack
                .pop_i32_const()
                .expect("i32 const value at stack top");
            let typed_reg = self.pop_to_reg(masm, None);
            emit(masm, typed_reg.reg, RegImm::i32(val), OperandSize::S32);
            self.stack.push(typed_reg.into());
        } else {
            self.binop(masm, OperandSize::S32, |masm, dst, src, size| {
                emit(masm, dst, src.into(), size)
            });
        }
    }

    /// Prepares arguments for emitting an i64 binary operation.
    pub fn i64_binop<F, M>(&mut self, masm: &mut M, mut emit: F)
    where
        F: FnMut(&mut M, Reg, RegImm, OperandSize),
        M: MacroAssembler,
    {
        let top = self.stack.peek().expect("value at stack top");
        if top.is_i64_const() {
            let val = self
                .stack
                .pop_i64_const()
                .expect("i64 const value at stack top");
            let typed_reg = self.pop_to_reg(masm, None);
            emit(masm, typed_reg.reg, RegImm::i64(val), OperandSize::S64);
            self.stack.push(typed_reg.into());
        } else {
            self.binop(masm, OperandSize::S64, |masm, dst, src, size| {
                emit(masm, dst, src.into(), size)
            });
        }
    }

    /// Saves any live registers in the value stack in a particular
    /// range defined by the caller.  This is a specialization of the
    /// spill function; made available for cases in which spilling
    /// locals is not required, like for example for function calls in
    /// which locals are not reachable by the callee.  
    ///
    /// Returns the size in bytes of the specified range.
    pub fn save_live_registers_and_calculate_sizeof<M, R>(&mut self, masm: &mut M, range: R) -> u32
    where
        R: RangeBounds<usize>,
        M: MacroAssembler,
    {
        let mut size = 0u32;
        for v in self.stack.inner_mut().range_mut(range) {
            match v {
                Val::Reg(TypedReg { reg, ty }) => {
                    let slot = masm.push(*reg, (*ty).into());
                    self.regalloc.free(*reg);
                    *v = Val::mem(*ty, slot);
                    size += slot.size
                }
                Val::Memory(mem) => size += mem.slot.size,
                _ => {}
            }
        }

        size
    }

    /// Drops the last `n` elements of the stack, calling the provided
    /// function for each `n` stack value.
    pub fn drop_last<F>(&mut self, last: usize, mut f: F)
    where
        F: FnMut(&mut RegAlloc, &Val),
    {
        let len = self.stack.len();
        assert!(last <= len);
        let truncate = self.stack.len() - last;
        let stack_mut = &mut self.stack.inner_mut();

        for v in stack_mut.range(truncate..) {
            f(&mut self.regalloc, v)
        }
        stack_mut.truncate(truncate);
    }

    /// Convenience wrapper around [`Self::spill_callback`].
    ///
    /// This function exists for cases in which triggering an unconditional
    /// spill is needed, like before entering control flow.
    pub fn spill<M: MacroAssembler>(&mut self, masm: &mut M) {
        Self::spill_impl(&mut self.stack, &mut self.regalloc, &mut self.frame, masm);
    }

    /// Prepares the compiler to emit an uncoditional jump to the
    /// given destination branch.  This process involves:
    /// * Balancing the machine stack pointer by popping it to
    ///   match the destination branch.
    /// * Updating the reachability state.
    /// * Marking the destination frame as a destination target.
    pub fn unconditional_jump<M, F>(&mut self, dest: &mut ControlStackFrame, masm: &mut M, mut f: F)
    where
        M: MacroAssembler,
        F: FnMut(&mut M, &mut Self, &mut ControlStackFrame),
    {
        let (_, target_sp) = dest.original_stack_len_and_sp_offset();
        // Invariant: The SP, must be greater or equal to the target
        // SP, given that we haven't popped any results by this point
        // yet. But it may happen in the callback.
        assert!(masm.sp_offset() >= target_sp);
        f(masm, self, dest);

        // The following snippet, pops the stack pointer to ensure
        // that it is correctly placed according to the expectations
        // of the destination branch.
        //
        // This is done in the context of unconditional jumps, as the
        // machine stack might be left unbalanced at the jump site,
        // due to register spills. In this context unbalanced refers
        // to possible extra space created at the jump site, which
        // might cause invalid memory accesses. Note that in some cases
        // the stack pointer offset might be already less than or
        // equal to the original stack pointer offset registered when
        // entering the destination control stack frame, which
        // effectively means that when reaching the jump site no extra
        // space was allocated similar to what would happen in a fall
        // through in which we assume that the program has allocated
        // and deallocated the right amount of stack space.
        //
        // More generally speaking the current stack pointer will be
        // less than the original stack pointer offset in cases in
        // which the top value in the value stack is a memory entry
        // which needs to be popped into the return location according
        // to the ABI (a register for single value returns and a
        // memory slot for 1+ returns). This could happen in the
        // callback invocation above if the callback invokes
        // `CodeGenContext::pop_abi_results` (e.g. `br` instruction).
        let current_sp = masm.sp_offset();
        if current_sp > target_sp {
            masm.free_stack(current_sp - target_sp);
        }

        dest.set_as_target();
        masm.jmp(*dest.label());
        self.reachable = false;
    }

    /// Handles the emission of the ABI result. This function is used at the end
    /// of a block or function to pop the results from the value stack into the
    /// corresponding ABI result representation.
    pub fn pop_abi_results<M: MacroAssembler>(&mut self, result: &ABIResult, masm: &mut M) {
        match result {
            ABIResult::Void => {}
            ABIResult::Reg { reg, .. } => {
                let TypedReg { reg, ty: _ } = self.pop_to_reg(masm, Some(*reg));
                self.free_reg(reg);
            }
        }
    }

    /// Push ABI results in to the value stack. This function is used at the end
    /// of a block or after a function call to push the corresponding ABI
    /// results into the value stack.
    pub fn push_abi_results<M: MacroAssembler>(&mut self, result: &ABIResult, masm: &mut M) {
        match result {
            ABIResult::Void => {}
            ABIResult::Reg { ty, reg } => {
                assert!(self.regalloc.reg_available(*reg));
                let typed_reg = TypedReg::new(*ty, self.reg(*reg, masm));
                self.stack.push(typed_reg.into());
            }
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
                let scratch = <M::ABI as ABI>::scratch_reg();
                masm.load(addr, scratch, slot.ty.into());
                let stack_slot = masm.push(scratch, slot.ty.into());
                *v = Val::mem(slot.ty, stack_slot);
            }
            _ => {}
        });
    }
}

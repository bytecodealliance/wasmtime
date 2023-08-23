use regalloc2::RegClass;
use wasmtime_environ::WasmType;

use super::ControlStackFrame;
use crate::{
    abi::{ABIResult, ABI},
    frame::Frame,
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
            t => panic!("unsupported type {:?}", t),
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

    /// Executes the provided function, guaranteeing that the
    /// specified register, if any, remains unallocatable throughout
    /// the function's execution.
    pub fn without<T, M, F>(&mut self, reg: &Option<Reg>, masm: &mut M, mut f: F) -> T
    where
        M: MacroAssembler,
        F: FnMut(&mut Self, &mut M) -> T,
    {
        if let Some(reg) = reg {
            self.reg(*reg, masm);
        }

        let result = f(self, masm);

        if let Some(reg) = reg {
            self.free_reg(*reg);
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
    pub fn pop_to_reg<M: MacroAssembler>(
        &mut self,
        masm: &mut M,
        named: Option<Reg>,
        size: OperandSize,
    ) -> TypedReg {
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
            self.move_val_to_reg(&val, reg, masm, size);
            // Free the source value if it is a register.
            if val.is_reg() {
                self.free_reg(val.get_reg());
            }
        }

        TypedReg::new(val.ty(), reg)
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
            Val::Reg(tr) => masm.mov(RegImm::reg(tr.reg), RegImm::reg(dst), size),
            Val::I32(imm) => masm.mov(RegImm::i32(*imm), RegImm::reg(dst), size),
            Val::I64(imm) => masm.mov(RegImm::i64(*imm), RegImm::reg(dst), size),
            Val::F32(imm) => masm.mov(RegImm::f32(imm.bits()), RegImm::reg(dst), size),
            Val::F64(imm) => masm.mov(RegImm::f64(imm.bits()), RegImm::reg(dst), size),
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
        let typed_reg = self.pop_to_reg(masm, None, size);
        emit(masm, typed_reg.reg, size);
        self.stack.push(typed_reg.into());
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
            let typed_reg = self.pop_to_reg(masm, None, OperandSize::S32);
            emit(
                masm,
                RegImm::reg(typed_reg.reg),
                RegImm::i32(val),
                OperandSize::S32,
            );
            self.stack.push(typed_reg.into());
        } else {
            let src = self.pop_to_reg(masm, None, OperandSize::S32);
            let dst = self.pop_to_reg(masm, None, OperandSize::S32);
            emit(masm, dst.reg.into(), src.reg.into(), OperandSize::S32);
            self.free_reg(src);
            self.stack.push(dst.into());
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
            let typed_reg = self.pop_to_reg(masm, None, OperandSize::S64);
            emit(
                masm,
                RegImm::reg(typed_reg.reg),
                RegImm::i64(val),
                OperandSize::S64,
            );
            self.stack.push(typed_reg.into());
        } else {
            let src = self.pop_to_reg(masm, None, OperandSize::S64);
            let dst = self.pop_to_reg(masm, None, OperandSize::S64);
            emit(masm, dst.reg.into(), src.reg.into(), OperandSize::S64);
            self.free_reg(src);
            self.stack.push(dst.into());
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

    /// Drops the last `n` elements of the stack, freeing any
    /// registers located in that region.
    pub fn drop_last(&mut self, last: usize) {
        let len = self.stack.len();
        assert!(last <= len);
        let truncate = self.stack.len() - last;

        self.stack.inner_mut().range(truncate..).for_each(|v| {
            if v.is_reg() {
                self.regalloc.free(v.get_reg().into());
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

        let TypedReg { reg, ty: _ } =
            self.pop_to_reg(masm, Some(result.result_reg().unwrap()), OperandSize::S64);
        self.free_reg(reg);
    }

    /// Push ABI results in to the value stack. This function is used at the end
    /// of a block or after a function call to push the corresponding ABI
    /// results into the value stack.
    pub fn push_abi_results<M: MacroAssembler>(&mut self, result: &ABIResult, masm: &mut M) {
        if result.is_void() {
            return;
        }

        match result {
            ABIResult::Reg { ty, reg } => {
                let reg = reg.unwrap();
                let ty = ty.unwrap();
                assert!(self.regalloc.reg_available(reg));
                let typed_reg = TypedReg::new(ty, self.reg(reg, masm));
                self.stack.push(typed_reg.into());
            }
        }
    }

    /// Pops the value at the stack top and assigns it to the local at
    /// the given index, returning the typed register holding the
    /// source value.
    pub fn set_local<M: MacroAssembler>(&mut self, masm: &mut M, index: u32) -> TypedReg {
        let slot = self
            .frame
            .get_local(index)
            .unwrap_or_else(|| panic!("invalid local slot = {}", index));
        let size: OperandSize = slot.ty.into();
        let src = self.pop_to_reg(masm, None, size);
        let addr = masm.local_address(&slot);
        masm.store(RegImm::reg(src.reg), addr, size);

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
                let slot = masm.push(r.reg, r.ty.into());
                regalloc.free(r.reg);
                *v = Val::mem(r.ty, slot);
            }
            Val::Local(local) => {
                let slot = frame.get_local(local.index).expect("valid local at slot");
                let addr = masm.local_address(&slot);
                masm.load(addr, regalloc.scratch, slot.ty.into());
                let stack_slot = masm.push(regalloc.scratch, slot.ty.into());
                *v = Val::mem(slot.ty, stack_slot);
            }
            _ => {}
        });
    }
}

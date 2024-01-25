use super::{
    abi::X64ABI,
    address::Address,
    asm::Assembler,
    regs::{self, rbp, rsp},
};

use crate::masm::{
    DivKind, ExtendKind, FloatCmpKind, Imm as I, IntCmpKind, MacroAssembler as Masm, OperandSize,
    RegImm, RemKind, RoundingMode, ShiftKind, TrapCode,
};
use crate::{
    abi::ABI,
    masm::{SPOffset, StackSlot},
    stack::TypedReg,
};
use crate::{
    abi::{self, align_to, calculate_frame_adjustment, LocalSlot},
    codegen::{ptr_type_from_ptr_size, Callee, CodeGenContext, FnCall, TableData},
    stack::Val,
};
use crate::{
    isa::reg::{Reg, RegClass},
    masm::CalleeKind,
};
use cranelift_codegen::{
    isa::unwind::UnwindInst, isa::x64::settings as x64_settings, settings, Final,
    MachBufferFinalized, MachLabel,
};

use wasmtime_environ::{PtrSize, WasmType, WASM_PAGE_SIZE};

/// x64 MacroAssembler.
pub(crate) struct MacroAssembler {
    /// Stack pointer offset.
    sp_offset: u32,
    /// Low level assembler.
    asm: Assembler,
    /// ISA flags.
    flags: x64_settings::Flags,
    /// Shared flags.
    shared_flags: settings::Flags,
    /// The target pointer size.
    ptr_size: OperandSize,
}

impl Masm for MacroAssembler {
    type Address = Address;
    type Ptr = u8;
    type ABI = X64ABI;

    fn prologue(&mut self) {
        let frame_pointer = rbp();
        let stack_pointer = rsp();

        self.asm.push_r(frame_pointer);

        if self.shared_flags.unwind_info() {
            self.asm.emit_unwind_inst(UnwindInst::PushFrameRegs {
                offset_upward_to_caller_sp: Self::ABI::arg_base_offset().try_into().unwrap(),
            })
        }

        self.asm
            .mov_rr(stack_pointer, frame_pointer, OperandSize::S64);

        if self.shared_flags.unwind_info() {
            self.asm.emit_unwind_inst(UnwindInst::DefineNewFrame {
                offset_upward_to_caller_sp: Self::ABI::arg_base_offset().try_into().unwrap(),

                // Clobbers appear directly after the RET and FP if they're present. As we just
                // pushed the frame pointer, the offset to the clobbers will be `0`.
                offset_downward_to_clobbers: 0,
            })
        }
    }

    fn check_stack(&mut self) {
        let ptr_size: u8 = self.ptr_size.bytes().try_into().unwrap();
        let scratch = regs::scratch();

        self.load_ptr(
            self.address_at_vmctx(ptr_size.vmcontext_runtime_limits().into()),
            scratch,
        );

        self.load_ptr(
            Address::offset(scratch, ptr_size.vmruntime_limits_stack_limit().into()),
            scratch,
        );

        self.asm.cmp_rr(regs::rsp(), scratch, self.ptr_size);
        self.asm.trapif(IntCmpKind::GtU, TrapCode::StackOverflow);
    }

    fn push(&mut self, reg: Reg, size: OperandSize) -> StackSlot {
        let bytes = match (reg.class(), size) {
            (RegClass::Int, OperandSize::S64) => {
                let word_bytes = <Self::ABI as ABI>::word_bytes();
                self.asm.push_r(reg);
                self.increment_sp(word_bytes);
                word_bytes
            }
            (RegClass::Int, OperandSize::S32) => {
                let bytes = size.bytes();
                self.reserve_stack(bytes);
                let sp_offset = SPOffset::from_u32(self.sp_offset);
                self.asm.mov_rm(reg, &self.address_from_sp(sp_offset), size);
                bytes
            }
            (RegClass::Float, _) => {
                let bytes = size.bytes();
                self.reserve_stack(bytes);
                let sp_offset = SPOffset::from_u32(self.sp_offset);
                self.asm
                    .xmm_mov_rm(reg, &self.address_from_sp(sp_offset), size);
                bytes
            }
            _ => unreachable!(),
        };

        StackSlot {
            offset: SPOffset::from_u32(self.sp_offset),
            size: bytes,
        }
    }

    fn save(&mut self, clobber_offset: u32, reg: Reg, size: OperandSize) -> StackSlot {
        let slot = self.push(reg, size);

        if self.shared_flags.unwind_info() {
            self.asm.emit_unwind_inst(UnwindInst::SaveReg {
                clobber_offset,
                reg: reg.into(),
            });
        }

        slot
    }

    fn reserve_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }

        self.asm.sub_ir(bytes as i32, rsp(), OperandSize::S64);
        self.increment_sp(bytes);
    }

    fn free_stack(&mut self, bytes: u32) {
        if bytes == 0 {
            return;
        }
        self.asm.add_ir(bytes as i32, rsp(), OperandSize::S64);
        self.decrement_sp(bytes);
    }

    fn reset_stack_pointer(&mut self, offset: SPOffset) {
        self.sp_offset = offset.as_u32();
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

        Address::offset(reg, offset)
    }

    fn table_elem_address(
        &mut self,
        index: Reg,
        ptr_base: Reg,
        table_data: &TableData,
        context: &mut CodeGenContext,
    ) -> Self::Address {
        let vmctx = <Self::ABI as ABI>::vmctx_reg();
        let scratch = regs::scratch();
        let bound = context.any_gpr(self);
        let tmp = context.any_gpr(self);

        if let Some(offset) = table_data.import_from {
            // If the table data declares a particular offset base,
            // load the address into a register to further use it as
            // the table address.
            self.asm
                .mov_mr(&self.address_at_vmctx(offset), ptr_base, self.ptr_size);
        } else {
            // Else, simply move the vmctx register into the addr register as
            // the base to calculate the table address.
            self.asm.mov_rr(vmctx, ptr_base, self.ptr_size);
        };

        // OOB check.
        let bound_addr = self.address_at_reg(ptr_base, table_data.current_elems_offset);
        let bound_size = table_data.current_elements_size;
        self.asm.mov_mr(&bound_addr, bound, bound_size);
        self.asm.cmp_rr(bound, index, bound_size);
        self.asm.trapif(IntCmpKind::GeU, TrapCode::TableOutOfBounds);

        // Move the index into the scratch register to calcualte the table
        // element address.
        // Moving the value of the index register to the scratch register
        // also avoids overwriting the context of the index register.
        self.asm.mov_rr(index, scratch, bound_size);
        self.asm.mul_ir(
            table_data.element_size.bytes() as i32,
            scratch,
            table_data.element_size,
        );
        self.asm.mov_mr(
            &self.address_at_reg(ptr_base, table_data.offset),
            ptr_base,
            self.ptr_size,
        );
        // Copy the value of the table base into a temporary register
        // so that we can use it later in case of a misspeculation.
        self.asm.mov_rr(ptr_base, tmp, self.ptr_size);
        // Calculate the address of the table element.
        self.asm.add_rr(scratch, ptr_base, self.ptr_size);
        if self.shared_flags.enable_table_access_spectre_mitigation() {
            // Perform a bounds check and override the value of the
            // table element address in case the index is out of bounds.
            self.asm.cmp_rr(bound, index, OperandSize::S32);
            self.asm.cmov(tmp, ptr_base, IntCmpKind::GeU, self.ptr_size);
        }
        context.free_reg(bound);
        context.free_reg(tmp);
        self.address_at_reg(ptr_base, 0)
    }

    fn table_size(&mut self, table_data: &TableData, context: &mut CodeGenContext) {
        let vmctx = <Self::ABI as ABI>::vmctx_reg();
        let scratch = regs::scratch();
        let size = context.any_gpr(self);

        if let Some(offset) = table_data.import_from {
            self.asm
                .mov_mr(&self.address_at_vmctx(offset), scratch, self.ptr_size);
        } else {
            self.asm.mov_rr(vmctx, scratch, self.ptr_size);
        };

        let size_addr = Address::offset(scratch, table_data.current_elems_offset);
        self.asm
            .mov_mr(&size_addr, size, table_data.current_elements_size);

        context.stack.push(TypedReg::i32(size).into());
    }

    fn memory_size(&mut self, heap_data: &crate::codegen::HeapData, context: &mut CodeGenContext) {
        let size_reg = context.any_gpr(self);
        let scratch = regs::scratch();
        let vmctx = <Self::ABI as ABI>::vmctx_reg();

        let base = if let Some(offset) = heap_data.import_from {
            self.asm
                .mov_mr(&self.address_at_vmctx(offset), scratch, self.ptr_size);
            scratch
        } else {
            vmctx
        };

        let size_addr = Address::offset(base, heap_data.current_length_offset);
        self.asm.mov_mr(&size_addr, size_reg, self.ptr_size);
        // Prepare the stack to emit a shift to get the size in pages rather
        // than in bytes.
        context
            .stack
            .push(TypedReg::new(heap_data.ty, size_reg).into());

        // Since the page size is a power-of-two, verify that 2^16, equals the
        // defined constant. This is mostly a safeguard in case the constant
        // value ever changes.
        let pow = 16;
        debug_assert_eq!(2u32.pow(pow), WASM_PAGE_SIZE);

        // Ensure that the constant is correctly typed according to the heap
        // type to reduce register pressure when emitting the shift operation.
        match heap_data.ty {
            WasmType::I32 => context.stack.push(Val::i32(pow as i32)),
            WasmType::I64 => context.stack.push(Val::i64(pow as i64)),
            _ => unreachable!(),
        }

        self.shift(context, ShiftKind::ShrU, heap_data.ty.into());
    }

    fn address_from_sp(&self, offset: SPOffset) -> Self::Address {
        Address::offset(regs::rsp(), self.sp_offset - offset.as_u32())
    }

    fn address_at_sp(&self, offset: SPOffset) -> Self::Address {
        Address::offset(regs::rsp(), offset.as_u32())
    }

    fn address_at_vmctx(&self, offset: u32) -> Self::Address {
        Address::offset(<Self::ABI as ABI>::vmctx_reg(), offset)
    }

    fn store_ptr(&mut self, src: Reg, dst: Self::Address) {
        self.store(src.into(), dst, self.ptr_size);
    }

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) {
        let scratch = <Self::ABI as ABI>::scratch_reg();
        let float_scratch = <Self::ABI as ABI>::float_scratch_reg();
        match src {
            RegImm::Imm(imm) => match imm {
                I::I32(v) => self.asm.mov_im(v as i32, &dst, size),
                I::I64(v) => match v.try_into() {
                    Ok(v) => self.asm.mov_im(v, &dst, size),
                    Err(_) => {
                        // If the immediate doesn't sign extend, use a scratch
                        // register.
                        self.asm.mov_ir(v, scratch, size);
                        self.asm.mov_rm(scratch, &dst, size);
                    }
                },
                I::F32(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    self.asm.xmm_mov_mr(&addr, float_scratch, size);
                    self.asm.xmm_mov_rm(float_scratch, &dst, size);
                }
                I::F64(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    self.asm.xmm_mov_mr(&addr, float_scratch, size);
                    self.asm.xmm_mov_rm(float_scratch, &dst, size);
                }
            },
            RegImm::Reg(reg) => {
                if reg.is_int() {
                    self.asm.mov_rm(reg, &dst, size);
                } else {
                    self.asm.xmm_mov_rm(reg, &dst, size);
                }
            }
        }
    }

    fn pop(&mut self, dst: Reg, size: OperandSize) {
        let current_sp = SPOffset::from_u32(self.sp_offset);
        match (dst.class(), size) {
            (RegClass::Int, OperandSize::S32) => {
                let addr = self.address_from_sp(current_sp);
                self.asm.mov_mr(&addr, dst, size);
                self.free_stack(size.bytes());
            }
            (RegClass::Int, OperandSize::S64) => {
                self.asm.pop_r(dst);
                self.decrement_sp(<Self::ABI as ABI>::word_bytes());
            }
            (RegClass::Float, _) => {
                let addr = self.address_from_sp(current_sp);
                self.asm.xmm_mov_mr(&addr, dst, size);
                self.free_stack(size.bytes());
            }
            _ => unreachable!(),
        }
    }

    fn call(
        &mut self,
        stack_args_size: u32,
        mut load_callee: impl FnMut(&mut Self) -> CalleeKind,
    ) -> u32 {
        let alignment: u32 = <Self::ABI as abi::ABI>::call_stack_align().into();
        let addend: u32 = <Self::ABI as abi::ABI>::arg_base_offset().into();
        let delta = calculate_frame_adjustment(self.sp_offset().as_u32(), addend, alignment);
        let aligned_args_size = align_to(stack_args_size, alignment);
        let total_stack = delta + aligned_args_size;
        self.reserve_stack(total_stack);
        let callee = load_callee(self);
        match callee {
            CalleeKind::Indirect(reg) => self.asm.call_with_reg(reg),
            CalleeKind::Direct(idx) => self.asm.call_with_index(idx),
            CalleeKind::Known(lib) => self.asm.call_with_lib(lib),
        };
        total_stack
    }

    fn load_ptr(&mut self, src: Self::Address, dst: Reg) {
        self.load(src, dst, self.ptr_size);
    }

    fn load_addr(&mut self, src: Self::Address, dst: Reg, size: OperandSize) {
        self.asm.lea(&src, dst, size);
    }

    fn load(&mut self, src: Address, dst: Reg, size: OperandSize) {
        if dst.is_int() {
            self.asm.mov_mr(&src, dst, size);
        } else {
            self.asm.xmm_mov_mr(&src, dst, size);
        }
    }

    fn sp_offset(&self) -> SPOffset {
        SPOffset::from_u32(self.sp_offset)
    }

    fn zero(&mut self, reg: Reg) {
        self.asm.xor_rr(reg, reg, OperandSize::S32);
    }

    fn mov(&mut self, src: RegImm, dst: Reg, size: OperandSize) {
        match (src, dst) {
            rr @ (RegImm::Reg(src), dst) => match (src.class(), dst.class()) {
                (RegClass::Int, RegClass::Int) => self.asm.mov_rr(src, dst, size),
                (RegClass::Float, RegClass::Float) => self.asm.xmm_mov_rr(src, dst, size),
                _ => Self::handle_invalid_operand_combination(rr.0, rr.1),
            },
            (RegImm::Imm(imm), dst) => match imm {
                I::I32(v) => self.asm.mov_ir(v as u64, dst, size),
                I::I64(v) => self.asm.mov_ir(v, dst, size),
                I::F32(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    self.asm.xmm_mov_mr(&addr, dst, size);
                }
                I::F64(v) => {
                    let addr = self.asm.add_constant(v.to_le_bytes().as_slice());
                    self.asm.xmm_mov_mr(&addr, dst, size);
                }
            },
        }
    }

    fn cmov(&mut self, src: Reg, dst: Reg, cc: IntCmpKind, size: OperandSize) {
        match (src.class(), dst.class()) {
            (RegClass::Int, RegClass::Int) => self.asm.cmov(src, dst, cc, size),
            (RegClass::Float, RegClass::Float) => self.asm.xmm_cmov(src, dst, cc, size),
            _ => Self::handle_invalid_operand_combination(src, dst),
        }
    }

    fn add(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), reg) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.add_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.add_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.add_rr(src, dst, size);
            }
        }
    }

    fn sub(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), reg) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.sub_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.sub_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.sub_rr(src, dst, size);
            }
        }
    }

    fn mul(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), reg) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.mul_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.mul_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.mul_rr(src, dst, size);
            }
        }
    }

    fn float_add(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        self.asm.xmm_add_rr(rhs, dst, size);
    }

    fn float_sub(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        self.asm.xmm_sub_rr(rhs, dst, size);
    }

    fn float_mul(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        self.asm.xmm_mul_rr(rhs, dst, size);
    }

    fn float_div(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        self.asm.xmm_div_rr(rhs, dst, size);
    }

    fn float_min(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        self.asm.xmm_min_seq(rhs, dst, size);
    }

    fn float_max(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        self.asm.xmm_max_seq(rhs, dst, size);
    }

    fn float_copysign(&mut self, dst: Reg, lhs: Reg, rhs: Reg, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        let scratch_gpr = regs::scratch();
        let scratch_xmm = regs::scratch_xmm();
        let sign_mask = match size {
            OperandSize::S32 => I::I32(0x80000000),
            OperandSize::S64 => I::I64(0x8000000000000000),
            OperandSize::S128 => unreachable!(),
        };
        self.load_constant(&sign_mask, scratch_gpr, size);
        self.asm.gpr_to_xmm(scratch_gpr, scratch_xmm, size);

        // Clear everything except sign bit in src.
        self.asm.xmm_and_rr(scratch_xmm, rhs, size);

        // Clear sign bit in dst using scratch to store result. Then copy the
        // result back to dst.
        self.asm.xmm_andn_rr(dst, scratch_xmm, size);
        self.asm.xmm_mov_rr(scratch_xmm, dst, size);

        // Copy sign bit from src to dst.
        self.asm.xmm_or_rr(rhs, dst, size);
    }

    fn float_neg(&mut self, dst: Reg, size: OperandSize) {
        assert_eq!(dst.class(), RegClass::Float);
        let mask = match size {
            OperandSize::S32 => I::I32(0x80000000),
            OperandSize::S64 => I::I64(0x8000000000000000),
            OperandSize::S128 => unreachable!(),
        };
        let scratch_gpr = regs::scratch();
        self.load_constant(&mask, scratch_gpr, size);
        let scratch_xmm = regs::scratch_xmm();
        self.asm.gpr_to_xmm(scratch_gpr, scratch_xmm, size);
        self.asm.xmm_xor_rr(scratch_xmm, dst, size);
    }

    fn float_abs(&mut self, dst: Reg, size: OperandSize) {
        assert_eq!(dst.class(), RegClass::Float);
        let mask = match size {
            OperandSize::S32 => I::I32(0x7fffffff),
            OperandSize::S64 => I::I64(0x7fffffffffffffff),
            OperandSize::S128 => unreachable!(),
        };
        let scratch_gpr = regs::scratch();
        self.load_constant(&mask, scratch_gpr, size);
        let scratch_xmm = regs::scratch_xmm();
        self.asm.gpr_to_xmm(scratch_gpr, scratch_xmm, size);
        self.asm.xmm_and_rr(scratch_xmm, dst, size);
    }

    fn float_round(&mut self, mode: RoundingMode, context: &mut CodeGenContext, size: OperandSize) {
        if self.flags.has_sse41() {
            let src = context.pop_to_reg(self, None);
            self.asm.xmm_rounds_rr(src.into(), src.into(), mode, size);
            context.stack.push(src.into());
        } else {
            FnCall::emit::<Self, Self::Ptr, _>(self, context, |context| {
                let b = match (&mode, size) {
                    (RoundingMode::Up, OperandSize::S32) => {
                        context.builtins.ceil_f32::<<Self as Masm>::ABI>()
                    }
                    (RoundingMode::Up, OperandSize::S64) => {
                        context.builtins.ceil_f64::<<Self as Masm>::ABI>()
                    }
                    (RoundingMode::Down, OperandSize::S32) => {
                        context.builtins.floor_f32::<<Self as Masm>::ABI>()
                    }
                    (RoundingMode::Down, OperandSize::S64) => {
                        context.builtins.floor_f64::<<Self as Masm>::ABI>()
                    }
                    (RoundingMode::Nearest, OperandSize::S32) => {
                        context.builtins.nearest_f32::<<Self as Masm>::ABI>()
                    }
                    (RoundingMode::Nearest, OperandSize::S64) => {
                        context.builtins.nearest_f64::<<Self as Masm>::ABI>()
                    }
                    (RoundingMode::Zero, OperandSize::S32) => {
                        context.builtins.trunc_f32::<<Self as Masm>::ABI>()
                    }
                    (RoundingMode::Zero, OperandSize::S64) => {
                        context.builtins.trunc_f64::<<Self as Masm>::ABI>()
                    }
                    (_, _) => unreachable!(),
                };
                Callee::Builtin(b)
            })
        }
    }

    fn float_sqrt(&mut self, dst: Reg, src: Reg, size: OperandSize) {
        self.asm.sqrt(src, dst, size);
    }

    fn and(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), reg) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.and_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.and_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.and_rr(src, dst, size);
            }
        }
    }

    fn or(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), reg) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.or_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.or_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.or_rr(src, dst, size);
            }
        }
    }

    fn xor(&mut self, dst: Reg, lhs: Reg, rhs: RegImm, size: OperandSize) {
        Self::ensure_two_argument_form(&dst, &lhs);
        match (rhs, dst) {
            (RegImm::Imm(imm), reg) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.xor_ir(v, reg, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.xor_rr(scratch, reg, size);
                }
            }

            (RegImm::Reg(src), dst) => {
                self.asm.xor_rr(src, dst, size);
            }
        }
    }

    fn shift(&mut self, context: &mut CodeGenContext, kind: ShiftKind, size: OperandSize) {
        let top = context.stack.peek().expect("value at stack top");

        if size == OperandSize::S32 && top.is_i32_const() {
            let val = context
                .stack
                .pop_i32_const()
                .expect("i32 const value at stack top");
            let typed_reg = context.pop_to_reg(self, None);

            self.asm.shift_ir(val as u8, typed_reg.into(), kind, size);

            context.stack.push(typed_reg.into());
        } else if size == OperandSize::S64 && top.is_i64_const() {
            let val = context
                .stack
                .pop_i64_const()
                .expect("i64 const value at stack top");
            let typed_reg = context.pop_to_reg(self, None);

            self.asm.shift_ir(val as u8, typed_reg.into(), kind, size);

            context.stack.push(typed_reg.into());
        } else {
            // Number of bits to shift must be in the CL register.
            let src = context.pop_to_reg(self, Some(regs::rcx()));
            let dst = context.pop_to_reg(self, None);

            self.asm.shift_rr(src.into(), dst.into(), kind, size);

            context.free_reg(src);
            context.stack.push(dst.into());
        }
    }

    fn div(&mut self, context: &mut CodeGenContext, kind: DivKind, size: OperandSize) {
        // Allocate rdx:rax.
        let rdx = context.reg(regs::rdx(), self);
        let rax = context.reg(regs::rax(), self);

        // Allocate the divisor, which can be any gpr.
        let divisor = context.pop_to_reg(self, None);

        // Mark rax as allocatable.
        context.free_reg(rax);
        // Move the top value to rax.
        let rax = context.pop_to_reg(self, Some(rax));
        self.asm.div(divisor.into(), (rax.into(), rdx), kind, size);

        // Free the divisor and rdx.
        context.free_reg(divisor);
        context.free_reg(rdx);

        // Push the quotient.
        context.stack.push(rax.into());
    }

    fn rem(&mut self, context: &mut CodeGenContext, kind: RemKind, size: OperandSize) {
        // Allocate rdx:rax.
        let rdx = context.reg(regs::rdx(), self);
        let rax = context.reg(regs::rax(), self);

        // Allocate the divisor, which can be any gpr.
        let divisor = context.pop_to_reg(self, None);

        // Mark rax as allocatable.
        context.free_reg(rax);
        // Move the top value to rax.
        let rax = context.pop_to_reg(self, Some(rax));
        self.asm.rem(divisor.reg, (rax.into(), rdx), kind, size);

        // Free the divisor and rax.
        context.free_reg(divisor);
        context.free_reg(rax);

        // Push the remainder.
        context.stack.push(Val::reg(rdx, divisor.ty));
    }

    fn epilogue(&mut self, locals_size: u32) {
        assert_eq!(self.sp_offset, locals_size);

        let rsp = rsp();
        if locals_size > 0 {
            self.asm.add_ir(locals_size as i32, rsp, OperandSize::S64);
        }
        self.asm.pop_r(rbp());
        self.asm.ret();
    }

    fn finalize(self) -> MachBufferFinalized<Final> {
        self.asm.finalize()
    }

    fn address_at_reg(&self, reg: Reg, offset: u32) -> Self::Address {
        Address::offset(reg, offset)
    }

    fn cmp(&mut self, src: RegImm, dst: Reg, size: OperandSize) {
        match src {
            RegImm::Imm(imm) => {
                if let Some(v) = imm.to_i32() {
                    self.asm.cmp_ir(v, dst, size);
                } else {
                    let scratch = regs::scratch();
                    self.load_constant(&imm, scratch, size);
                    self.asm.cmp_rr(scratch, dst, size);
                }
            }
            RegImm::Reg(src) => {
                self.asm.cmp_rr(src, dst, size);
            }
        }
    }

    fn cmp_with_set(&mut self, src: RegImm, dst: Reg, kind: IntCmpKind, size: OperandSize) {
        self.cmp(src, dst, size);
        self.asm.setcc(kind, dst);
    }

    fn float_cmp_with_set(
        &mut self,
        src1: Reg,
        src2: Reg,
        dst: Reg,
        kind: FloatCmpKind,
        size: OperandSize,
    ) {
        // Float comparisons needs to be ordered (that is, comparing with a NaN
        // should return 0) except for not equal which needs to be unordered.
        // We use ucomis{s, d} because comis{s, d} has an undefined result if
        // either operand is NaN. Since ucomis{s, d} is unordered, we need to
        // compensate to make the comparison ordered.  Ucomis{s, d} sets the
        // ZF, PF, and CF flags if there is an unordered result.
        let (src1, src2, set_kind) = match kind {
            FloatCmpKind::Eq => (src1, src2, IntCmpKind::Eq),
            FloatCmpKind::Ne => (src1, src2, IntCmpKind::Ne),
            FloatCmpKind::Gt => (src1, src2, IntCmpKind::GtU),
            FloatCmpKind::Ge => (src1, src2, IntCmpKind::GeU),
            // Reversing the operands and using the complementary comparison
            // avoids needing to perform an additional SETNP and AND
            // instruction.
            // SETNB and SETNBE check if the carry flag is unset (i.e., not
            // less than and not unordered) so we get the intended result
            // without having to look at the parity flag.
            FloatCmpKind::Lt => (src2, src1, IntCmpKind::GtU),
            FloatCmpKind::Le => (src2, src1, IntCmpKind::GeU),
        };
        self.asm.ucomis(src1, src2, size);
        self.asm.setcc(set_kind, dst);
        match kind {
            FloatCmpKind::Eq | FloatCmpKind::Gt | FloatCmpKind::Ge => {
                // Return false if either operand is NaN by ensuring PF is
                // unset.
                let scratch = regs::scratch();
                self.asm.setnp(scratch);
                self.asm.and_rr(scratch, dst, size);
            }
            FloatCmpKind::Ne => {
                // Return true if either operand is NaN by checking if PF is
                // set.
                let scratch = regs::scratch();
                self.asm.setp(scratch);
                self.asm.or_rr(scratch, dst, size);
            }
            FloatCmpKind::Lt | FloatCmpKind::Le => (),
        }
    }

    fn clz(&mut self, src: Reg, dst: Reg, size: OperandSize) {
        if self.flags.has_lzcnt() {
            self.asm.lzcnt(src, dst, size);
        } else {
            let scratch = regs::scratch();

            // Use the following approach:
            // dst = size.num_bits() - bsr(src) - is_not_zero
            //     = size.num.bits() + -bsr(src) - is_not_zero.
            self.asm.bsr(src.into(), dst.into(), size);
            self.asm.setcc(IntCmpKind::Ne, scratch.into());
            self.asm.neg(dst, dst, size);
            self.asm.add_ir(size.num_bits(), dst, size);
            self.asm.sub_rr(scratch, dst, size);
        }
    }

    fn ctz(&mut self, src: Reg, dst: Reg, size: OperandSize) {
        if self.flags.has_bmi1() {
            self.asm.tzcnt(src, dst, size);
        } else {
            let scratch = regs::scratch();

            // Use the following approach:
            // dst = bsf(src) + (is_zero * size.num_bits())
            //     = bsf(src) + (is_zero << size.log2()).
            // BSF outputs the correct value for every value except 0.
            // When the value is 0, BSF outputs 0, correct output for ctz is
            // the number of bits.
            self.asm.bsf(src.into(), dst.into(), size);
            self.asm.setcc(IntCmpKind::Eq, scratch.into());
            self.asm
                .shift_ir(size.log2(), scratch, ShiftKind::Shl, size);
            self.asm.add_rr(scratch, dst, size);
        }
    }

    fn get_label(&mut self) -> MachLabel {
        let buffer = self.asm.buffer_mut();
        buffer.get_label()
    }

    fn bind(&mut self, label: MachLabel) {
        let buffer = self.asm.buffer_mut();
        buffer.bind_label(label, &mut Default::default());
    }

    fn branch(
        &mut self,
        kind: IntCmpKind,
        lhs: RegImm,
        rhs: Reg,
        taken: MachLabel,
        size: OperandSize,
    ) {
        use IntCmpKind::*;

        match &(lhs, rhs) {
            (RegImm::Reg(rlhs), rrhs) => {
                // If the comparision kind is zero or not zero and both operands
                // are the same register, emit a test instruction. Else we emit
                // a normal comparison.
                if (kind == Eq || kind == Ne) && (rlhs == rrhs) {
                    self.asm.test_rr(*rrhs, *rlhs, size);
                } else {
                    self.cmp(lhs, rhs, size);
                }
            }
            _ => self.cmp(lhs, rhs, size),
        }
        self.asm.jmp_if(kind, taken);
    }

    fn jmp(&mut self, target: MachLabel) {
        self.asm.jmp(target);
    }

    fn popcnt(&mut self, context: &mut CodeGenContext, size: OperandSize) {
        let src = context.pop_to_reg(self, None);
        if self.flags.has_popcnt() && self.flags.has_sse42() {
            self.asm.popcnt(src.into(), size);
            context.stack.push(src.into());
        } else {
            // The fallback functionality here is based on `MacroAssembler::popcnt64` in:
            // https://searchfox.org/mozilla-central/source/js/src/jit/x64/MacroAssembler-x64-inl.h#495

            let tmp = context.any_gpr(self);
            let dst = src;
            let (masks, shift_amt) = match size {
                OperandSize::S64 => (
                    [
                        0x5555555555555555, // m1
                        0x3333333333333333, // m2
                        0x0f0f0f0f0f0f0f0f, // m4
                        0x0101010101010101, // h01
                    ],
                    56u8,
                ),
                // 32-bit popcount is the same, except the masks are half as
                // wide and we shift by 24 at the end rather than 56
                OperandSize::S32 => (
                    [0x55555555i64, 0x33333333i64, 0x0f0f0f0fi64, 0x01010101i64],
                    24u8,
                ),
                _ => unreachable!(),
            };
            self.asm.mov_rr(src.into(), tmp, size);

            // x -= (x >> 1) & m1;
            self.asm.shift_ir(1u8, dst.into(), ShiftKind::ShrU, size);
            let lhs = dst.reg;
            self.and(lhs, lhs, RegImm::i64(masks[0]), size);
            self.asm.sub_rr(dst.into(), tmp, size);

            // x = (x & m2) + ((x >> 2) & m2);
            self.asm.mov_rr(tmp, dst.into(), size);
            // Load `0x3333...` into the scratch reg once, allowing us to use
            // `and_rr` and avoid inadvertently loading it twice as with `and`
            let scratch = regs::scratch();
            self.load_constant(&I::i64(masks[1]), scratch, size);
            self.asm.and_rr(scratch, dst.into(), size);
            self.asm.shift_ir(2u8, tmp, ShiftKind::ShrU, size);
            self.asm.and_rr(scratch, tmp, size);
            self.asm.add_rr(dst.into(), tmp, size);

            // x = (x + (x >> 4)) & m4;
            self.asm.mov_rr(tmp.into(), dst.into(), size);
            self.asm.shift_ir(4u8, dst.into(), ShiftKind::ShrU, size);
            self.asm.add_rr(tmp, dst.into(), size);
            let lhs = dst.reg.into();
            self.and(lhs, lhs, RegImm::i64(masks[2]), size);

            // (x * h01) >> shift_amt
            let lhs = dst.reg.into();
            self.mul(lhs, lhs, RegImm::i64(masks[3]), size);
            self.asm
                .shift_ir(shift_amt, dst.into(), ShiftKind::ShrU, size);

            context.stack.push(dst.into());
            context.free_reg(tmp);
        }
    }

    fn wrap(&mut self, src: Reg, dst: Reg) {
        self.asm.mov_rr(src.into(), dst.into(), OperandSize::S32);
    }

    fn extend(&mut self, src: Reg, dst: Reg, kind: ExtendKind) {
        if let ExtendKind::I64ExtendI32U = kind {
            self.asm.movzx_rr(src, dst, kind);
        } else {
            self.asm.movsx_rr(src, dst, kind);
        }
    }

    fn signed_truncate(
        &mut self,
        src: Reg,
        dst: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        self.asm.cvt_float_to_sint_seq(
            src,
            dst,
            regs::scratch(),
            regs::scratch_xmm(),
            src_size,
            dst_size,
        );
    }

    fn unsigned_truncate(
        &mut self,
        src: Reg,
        dst: Reg,
        tmp_fpr: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        self.asm.cvt_float_to_uint_seq(
            src,
            dst,
            regs::scratch(),
            regs::scratch_xmm(),
            tmp_fpr,
            src_size,
            dst_size,
        );
    }

    fn signed_convert(&mut self, src: Reg, dst: Reg, src_size: OperandSize, dst_size: OperandSize) {
        self.asm.cvt_sint_to_float(src, dst, src_size, dst_size);
    }

    fn unsigned_convert(
        &mut self,
        src: Reg,
        dst: Reg,
        tmp_gpr: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        // Need to convert unsigned uint32 to uint64 for conversion instruction sequence.
        if let OperandSize::S32 = src_size {
            self.extend(src, src, ExtendKind::I64ExtendI32U);
        }

        self.asm
            .cvt_uint64_to_float_seq(src, dst, regs::scratch(), tmp_gpr, dst_size);
    }

    fn reinterpret_float_as_int(&mut self, src: Reg, dst: Reg, size: OperandSize) {
        self.asm.xmm_to_gpr(src, dst, size);
    }

    fn reinterpret_int_as_float(&mut self, src: Reg, dst: Reg, size: OperandSize) {
        self.asm.gpr_to_xmm(src.into(), dst, size);
    }

    fn demote(&mut self, src: Reg, dst: Reg) {
        self.asm
            .cvt_float_to_float(src.into(), dst.into(), OperandSize::S64, OperandSize::S32);
    }

    fn promote(&mut self, src: Reg, dst: Reg) {
        self.asm
            .cvt_float_to_float(src.into(), dst.into(), OperandSize::S32, OperandSize::S64);
    }

    fn unreachable(&mut self) {
        self.asm.trap(TrapCode::UnreachableCodeReached)
    }

    fn trapif(&mut self, cc: IntCmpKind, code: TrapCode) {
        self.asm.trapif(cc, code);
    }

    fn trapz(&mut self, src: Reg, code: TrapCode) {
        self.asm.test_rr(src, src, self.ptr_size);
        self.asm.trapif(IntCmpKind::Eq, code);
    }

    fn jmp_table(&mut self, targets: &[MachLabel], index: Reg, tmp: Reg) {
        // At least one default target.
        assert!(targets.len() >= 1);
        let default_index = targets.len() - 1;
        // Emit bounds check, by conditionally moving the max cases
        // into the given index reg if the contents of the index reg
        // are greater.
        let max = default_index;
        let size = OperandSize::S32;
        self.asm.mov_ir(max as u64, tmp, size);
        self.asm.cmp_rr(index, tmp, size);
        self.asm.cmov(tmp, index, IntCmpKind::LtU, size);

        let default = targets[default_index];
        let rest = &targets[0..default_index];
        let tmp1 = regs::scratch();
        self.asm.jmp_table(rest.into(), default, index, tmp1, tmp);
    }
}

impl MacroAssembler {
    /// Create an x64 MacroAssembler.
    pub fn new(
        ptr_size: impl PtrSize,
        shared_flags: settings::Flags,
        isa_flags: x64_settings::Flags,
    ) -> Self {
        Self {
            sp_offset: 0,
            asm: Assembler::new(shared_flags.clone(), isa_flags.clone()),
            flags: isa_flags,
            shared_flags,
            ptr_size: ptr_type_from_ptr_size(ptr_size.size()).into(),
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

    fn load_constant(&mut self, constant: &I, dst: Reg, size: OperandSize) {
        match constant {
            I::I32(v) => self.asm.mov_ir(*v as u64, dst, size),
            I::I64(v) => self.asm.mov_ir(*v, dst, size),
            _ => panic!(),
        }
    }

    fn handle_invalid_operand_combination<T>(src: impl Into<RegImm>, dst: impl Into<RegImm>) -> T {
        panic!(
            "Invalid operand combination; src={:?}, dst={:?}",
            src.into(),
            dst.into()
        );
    }

    fn ensure_two_argument_form(dst: &Reg, lhs: &Reg) {
        assert!(
            dst == lhs,
            "the destination and first source argument must be the same, dst={:?}, lhs={:?}",
            dst,
            lhs
        );
    }
}

use super::{
    ABI, RegAlloc,
    abi::Aarch64ABI,
    address::Address,
    asm::{Assembler, PatchableAddToReg},
    regs::{self, scratch_fpr_bitset, scratch_gpr_bitset},
};
use crate::{
    Result,
    abi::{self, align_to, calculate_frame_adjustment, local::LocalSlot, vmctx},
    bail,
    codegen::{CodeGenContext, CodeGenError, Emission, FuncEnv, ptr_type_from_ptr_size},
    format_err,
    isa::{
        CallingConvention,
        aarch64::abi::SHADOW_STACK_POINTER_SLOT_SIZE,
        reg::{Reg, WritableReg, writable},
    },
    masm::{
        CalleeKind, DivKind, Extend, ExtendKind, ExtractLaneKind, FloatCmpKind, FloatScratch,
        Imm as I, IntCmpKind, IntScratch, LoadKind, MacroAssembler as Masm, MulWideKind,
        OperandSize, RegImm, RemKind, ReplaceLaneKind, RmwOp, RoundingMode, SPOffset, Scratch,
        ScratchType, ShiftKind, SplatKind, StackSlot, StoreKind, TRUSTED_FLAGS, TrapCode,
        TruncKind, UNTRUSTED_FLAGS, V128AbsKind, V128AddKind, V128ConvertKind, V128ExtAddKind,
        V128ExtMulKind, V128ExtendKind, V128MaxKind, V128MinKind, V128MulKind, V128NarrowKind,
        V128NegKind, V128SubKind, V128TruncKind, VectorCompareKind, VectorEqualityKind, Zero,
    },
    stack::{TypedReg, Val},
};
use cranelift_codegen::{
    Final, MachBufferFinalized, MachLabel,
    binemit::CodeOffset,
    ir::{MemFlagsData, RelSourceLoc, SourceLoc, types},
    isa::aarch64,
    isa::aarch64::inst::{
        self, Cond, ExtendOp, Imm12, ImmLogic, ImmShift, SImm7Scaled, SImm9, ScalarSize,
        VecALUModOp, VecALUOp, VecExtendOp, VecLanesOp, VecMisc2, VecRRLongOp, VecRRNarrowOp,
        VecRRPairLongOp, VecRRRLongOp, VectorSize,
    },
    settings,
};
use regalloc2::RegClass;
use wasmtime_environ::{PtrSize, WasmValType};

/// Aarch64 MacroAssembler.
pub(crate) struct MacroAssembler {
    /// This value represents the maximum stack size seen while compiling the
    /// function. While the function is still being compiled its value will not
    /// be valid (the stack will grow and shrink as space is reserved and freed
    /// during compilation), but once all instructions have been seen this value
    /// will be the maximum stack usage seen.
    sp_max: u32,

    /// Add-with-immediate patchable instruction sequence used to add the
    /// constant stack max to a register.
    stack_max_use_add: Option<PatchableAddToReg>,

    /// Low level assembler.
    asm: Assembler,
    /// Stack pointer offset.
    sp_offset: u32,
    /// The target pointer size.
    ptr_size: OperandSize,
    /// Scratch register scope.
    scratch_scope: RegAlloc,
    /// Shared flags.
    shared_flags: settings::Flags,
}

impl MacroAssembler {
    /// Create an Aarch64 MacroAssembler.
    pub fn new(
        ptr_size: impl PtrSize,
        shared_flags: settings::Flags,
        isa_flags: aarch64::settings::Flags,
    ) -> Result<Self> {
        Ok(Self {
            sp_max: 0,
            stack_max_use_add: None,
            asm: Assembler::new(shared_flags.clone(), isa_flags),
            sp_offset: 0u32,
            ptr_size: ptr_type_from_ptr_size(ptr_size.size()).try_into()?,
            scratch_scope: RegAlloc::from(scratch_gpr_bitset(), scratch_fpr_bitset()),
            shared_flags,
        })
    }

    /// Add the maximum stack used to a register, recording an obligation to update the
    /// add-with-immediate instruction emitted to use the real stack max when the masm is being
    /// finalized.
    fn add_stack_max(&mut self, reg: WritableReg, tmp: WritableReg) {
        assert!(self.stack_max_use_add.is_none());
        let patch = PatchableAddToReg::new(reg, tmp, self.asm.buffer_mut());
        self.stack_max_use_add.replace(patch);
    }

    /// Ensures that the stack pointer remains 16-byte aligned for the duration
    /// of the provided function. This alignment is necessary for AArch64
    /// compliance, particularly for signal handlers that may be invoked
    /// during execution. While the compiler doesn't directly use the stack
    /// pointer for memory addressing, maintaining this alignment is crucial
    /// to prevent issues when handling signals.
    pub fn with_aligned_sp<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Self) -> Result<T>,
    {
        let mut aligned = false;
        let alignment: u32 = <Aarch64ABI as ABI>::call_stack_align().into();
        let addend: u32 = <Aarch64ABI as ABI>::initial_frame_size().into();
        let delta = calculate_frame_adjustment(self.sp_offset()?.as_u32(), addend, alignment);
        if delta != 0 {
            self.sub(
                writable!(regs::sp()),
                // Since we don't need to synchronize the shadow stack pointer
                // when freeing stack space [^1], the stack pointer may become
                // out of sync with the primary shadow stack pointer. Therefore,
                // we use the shadow stack pointer as the reference for
                // calculating any alignment delta (self.sp_offset).
                //
                // [1]: This approach avoids an unnecessary move instruction and
                // maintains the invariant of not accessing memory below the
                // current stack pointer, preventing issues with signal handlers
                // and interrupts.
                regs::shadow_sp(),
                RegImm::i32(delta as i32),
                OperandSize::S64,
            )?;

            aligned = true;
        }

        let res = f(self)?;

        if aligned {
            self.move_shadow_sp_to_sp();
        }

        Ok(res)
    }
}

impl Masm for MacroAssembler {
    type Address = Address;
    type Ptr = u8;
    type ABI = Aarch64ABI;

    fn frame_setup(&mut self) -> Result<()> {
        let lr = regs::lr();
        let fp = regs::fp();
        let sp = regs::sp();

        let offset = SImm7Scaled::maybe_from_i64(-16, types::I64)
            .expect("Frame pointer offset of -16 is valid for pair addressing");
        let addr = Address::pre_indexed_from_sp_for_pair(offset);
        self.asm.stp(fp, lr, addr.to_pair_addressing_mode());
        self.asm.mov_rr(sp, writable!(fp), OperandSize::S64);

        let offset = SImm9::maybe_from_i64(-(SHADOW_STACK_POINTER_SLOT_SIZE as i64))
            .expect("Shadow stack pointer slot size is valid for single addressing");
        let addr = Address::pre_indexed_from_sp(offset);
        addr.to_addressing_mode(self, OperandSize::S64, |masm, mem| {
            masm.asm
                .str(regs::shadow_sp(), mem, OperandSize::S64, TRUSTED_FLAGS);
            Ok(())
        })?;

        self.move_sp_to_shadow_sp();
        Ok(())
    }

    fn check_stack(&mut self, vmctx: Reg) -> Result<()> {
        let ptr_size_u8: u8 = self.ptr_size.bytes().try_into().unwrap();

        // The PatchableAddToReg construct on aarch64 is not a single
        // add-immediate instruction, but a 3-instruction sequence that loads an
        // immediate using 2 mov-immediate instructions into _another_ scratch
        // register before adding it into the target scratch register.
        //
        // In other words, to make this work we use _two_ scratch registers, one
        // to hold the limit we're calculating and one helper that's just used
        // to load the immediate.
        //
        // Luckily on aarch64 we have 2 available scratch registers, ip0 and
        // ip1.
        // NB that this in this case, we manually allocate the scratch registers
        // as precision when it comes to its usage is

        let ptr_size = self.ptr_size;
        self.with_aligned_sp(|masm| {
            masm.with_scratch::<IntScratch, _>(|masm, scratch_stk_limit| {
                masm.with_scratch::<IntScratch, _>(|masm, scratch_tmp| {
                    masm.load_ptr(
                        masm.address_at_reg(vmctx, ptr_size_u8.vmcontext_store_context().into())?,
                        scratch_stk_limit.writable(),
                    )?;

                    masm.load_ptr(
                        Address::offset(
                            scratch_stk_limit.inner(),
                            ptr_size_u8.vmstore_context_stack_limit().into(),
                        ),
                        scratch_stk_limit.writable(),
                    )?;

                    masm.add_stack_max(scratch_stk_limit.writable(), scratch_tmp.writable());

                    // Aarch can only do a cmp with sp in the first operand, which means we
                    // use a less-than comparison, not a greater-than (stack grows down).
                    masm.cmp(regs::sp(), scratch_stk_limit.inner().into(), ptr_size)?;
                    masm.asm
                        .trapif(IntCmpKind::LtU.into(), TrapCode::STACK_OVERFLOW);

                    Ok(())
                })
            })
        })
    }

    fn frame_restore(&mut self) -> Result<()> {
        debug_assert_eq!(self.sp_offset, 0);

        // Sync the real stack pointer with the value of the shadow stack
        // pointer.
        self.move_shadow_sp_to_sp();

        // Pop the shadow stack pointer. It's assumed that at this point
        // `sp_offset` is 0 and therefore the real stack pointer should be
        // 16-byte aligned.
        let offset = SImm9::maybe_from_i64(SHADOW_STACK_POINTER_SLOT_SIZE as i64)
            .expect("Shadow stack pointer slot size is valid for single addressing");
        let addr = Address::post_indexed_from_sp(offset);
        addr.to_addressing_mode(self, OperandSize::S64, |masm, mem| {
            masm.asm.uload(
                mem,
                writable!(regs::shadow_sp()),
                OperandSize::S64,
                TRUSTED_FLAGS,
            );
            Ok(())
        })?;

        // Restore the link register and frame pointer.
        let lr = regs::lr();
        let fp = regs::fp();
        let offset = SImm7Scaled::maybe_from_i64(16, types::I64)
            .expect("Frame pointer offset 16 is valid for pair addressing");
        let addr = Address::post_indexed_from_sp_for_pair(offset);

        self.asm.ldp(fp, lr, addr.to_pair_addressing_mode());
        self.asm.ret();
        Ok(())
    }

    fn reserve_stack(&mut self, bytes: u32) -> Result<()> {
        if bytes == 0 {
            return Ok(());
        }

        let ssp = regs::shadow_sp();

        match Imm12::maybe_from_u64(bytes as u64) {
            Some(v) => self.asm.sub_ir(v, ssp, writable!(ssp), OperandSize::S64),
            None => {
                self.with_scratch::<IntScratch, _>(|masm, scratch| {
                    masm.asm
                        .mov_ir(scratch.writable(), I::I64(bytes as u64), OperandSize::S64);
                    masm.asm
                        .sub_rrr(scratch.inner(), ssp, writable!(ssp), OperandSize::S64);
                });
            }
        }

        // Even though we're using the shadow stack pointer to reserve stack, we
        // must ensure that the real stack pointer reflects the stack claimed so
        // far; we can't use stack memory below the real stack pointer as it
        // could be clobbered by interrupts or signal handlers.
        self.move_shadow_sp_to_sp();

        self.increment_sp(bytes);
        Ok(())
    }

    fn free_stack(&mut self, bytes: u32) -> Result<()> {
        if bytes == 0 {
            return Ok(());
        }

        let ssp = regs::shadow_sp();
        match Imm12::maybe_from_u64(bytes as u64) {
            Some(v) => self.asm.add_ir(v, ssp, writable!(ssp), OperandSize::S64),
            None => {
                self.with_scratch::<IntScratch, _>(|masm, scratch| {
                    masm.asm
                        .mov_ir(scratch.writable(), I::I64(bytes as u64), OperandSize::S64);
                    masm.asm
                        .add_rrr(ssp, scratch.inner(), writable!(ssp), OperandSize::S64);
                });
            }
        }

        // We must ensure that the real stack pointer reflects the offset
        // tracked by `self.sp_offset`, we use such value to calculate
        // alignment, which is crucial for calls.
        //
        // As an optimization: this synchronization doesn't need to happen all
        // the time, in theory we could ensure to sync the shadow stack pointer
        // with the stack pointer when alignment is required, like at callsites.
        // This is the simplest approach at the time of writing, which
        // integrates well with the rest of the aarch64 infrastructure.
        self.move_shadow_sp_to_sp();

        self.decrement_sp(bytes);
        Ok(())
    }

    fn reset_stack_pointer(&mut self, offset: SPOffset) -> Result<()> {
        self.sp_offset = offset.as_u32();
        Ok(())
    }

    fn local_address(&mut self, local: &LocalSlot) -> Result<Address> {
        let (reg, offset) = local
            .addressed_from_sp()
            .then(|| {
                let offset = self.sp_offset.checked_sub(local.offset).expect(&format!(
                    "Invalid local offset = {}; sp offset = {}",
                    local.offset, self.sp_offset
                ));
                (regs::shadow_sp(), offset)
            })
            .unwrap_or((regs::fp(), local.offset));

        Ok(Address::offset(reg, offset as i64))
    }

    fn address_from_sp(&self, offset: SPOffset) -> Result<Self::Address> {
        Ok(Address::from_shadow_sp(
            (self.sp_offset - offset.as_u32()) as i64,
        ))
    }

    fn address_at_sp(&self, offset: SPOffset) -> Result<Self::Address> {
        Ok(Address::from_shadow_sp(offset.as_u32() as i64))
    }

    fn address_at_vmctx(&self, offset: u32) -> Result<Self::Address> {
        Ok(Address::offset(vmctx!(Self), offset as i64))
    }

    fn store_ptr(&mut self, src: Reg, dst: Self::Address) -> Result<()> {
        self.store(src.into(), dst, self.ptr_size)
    }

    fn store(&mut self, src: RegImm, dst: Address, size: OperandSize) -> Result<()> {
        match src {
            RegImm::Imm(v) => {
                match v {
                    I::I32(_) | I::I64(_) => {
                        self.with_scratch::<IntScratch, _>(|masm, scratch| -> Result<()> {
                            masm.asm.mov_ir(scratch.writable(), v, v.size());
                            dst.to_addressing_mode(masm, size, |masm, mem| {
                                masm.asm.str(scratch.inner(), mem, size, TRUSTED_FLAGS);
                                Ok(())
                            })
                        })?;
                    }
                    imm @ (I::F32(_) | I::F64(_) | I::V128(_)) => {
                        self.with_scratch::<FloatScratch, _>(|masm, scratch| -> Result<()> {
                            masm.asm.mov_ir(scratch.writable(), imm, imm.size());
                            dst.to_addressing_mode(masm, size, |masm, mem| {
                                masm.asm.str(scratch.inner(), mem, size, TRUSTED_FLAGS);
                                Ok(())
                            })
                        })?;
                    }
                };
                Ok(())
            }
            RegImm::Reg(r) => dst.to_addressing_mode(self, size, |masm, mem| {
                masm.asm.str(r, mem, size, TRUSTED_FLAGS);
                Ok(())
            }),
        }
    }

    fn wasm_store(&mut self, src: Reg, dst: Self::Address, op_kind: StoreKind) -> Result<()> {
        self.with_aligned_sp(|masm| match op_kind {
            StoreKind::Operand(size) => dst.to_addressing_mode(masm, size, |masm, mem| {
                masm.asm.str(src, mem, size, UNTRUSTED_FLAGS);
                Ok(())
            }),
            StoreKind::Atomic(_size) => {
                Err(format_err!(CodeGenError::unimplemented_masm_instruction()))
            }
            StoreKind::VectorLane(_selector) => {
                Err(format_err!(CodeGenError::unimplemented_masm_instruction()))
            }
        })
    }

    fn with_scratch<T: ScratchType, R>(&mut self, f: impl FnOnce(&mut Self, Scratch) -> R) -> R {
        let r = self
            .scratch_scope
            .reg_for_class(T::reg_class(), &mut |_| Ok(()))
            .expect("Scratch register to be available");

        let ret = f(self, Scratch::new(r));

        self.scratch_scope.free(r);
        ret
    }

    fn call(
        &mut self,
        stack_args_size: u32,
        mut load_callee: impl FnMut(&mut Self) -> Result<(CalleeKind, CallingConvention)>,
    ) -> Result<u32> {
        let alignment: u32 = <Self::ABI as abi::ABI>::call_stack_align().into();
        let addend: u32 = <Self::ABI as abi::ABI>::initial_frame_size().into();
        let delta = calculate_frame_adjustment(self.sp_offset()?.as_u32(), addend, alignment);
        let aligned_args_size = align_to(stack_args_size, alignment);
        let total_stack = delta + aligned_args_size;
        self.reserve_stack(total_stack)?;
        let (callee, call_conv) = load_callee(self)?;
        match callee {
            CalleeKind::Indirect(reg) => self.asm.call_with_reg(reg, call_conv),
            CalleeKind::Direct(idx) => self.asm.call_with_name(idx, call_conv),
        }

        Ok(total_stack)
    }

    fn load(&mut self, src: Address, dst: WritableReg, size: OperandSize) -> Result<()> {
        src.to_addressing_mode(self, size, |masm, mem| {
            Ok(masm.asm.uload(mem, dst, size, TRUSTED_FLAGS))
        })
    }

    fn load_ptr(&mut self, src: Self::Address, dst: WritableReg) -> Result<()> {
        self.load(src, dst, self.ptr_size)
    }

    fn wasm_load(&mut self, src: Self::Address, dst: WritableReg, kind: LoadKind) -> Result<()> {
        let size = kind.derive_operand_size();
        self.with_aligned_sp(|masm| match &kind {
            LoadKind::Operand(_) => src.to_addressing_mode(masm, size, |masm, mem| {
                Ok(masm.asm.uload(mem, dst, size, UNTRUSTED_FLAGS))
            }),
            LoadKind::Splat(_) => bail!(CodeGenError::UnimplementedWasmLoadKind),
            LoadKind::ScalarExtend(extend_kind) => {
                if extend_kind.signed() {
                    src.to_addressing_mode(masm, size, |masm, mem| {
                        masm.asm.sload(mem, dst, size, UNTRUSTED_FLAGS);
                        Ok(())
                    })
                } else {
                    src.to_addressing_mode(masm, size, |masm, mem| {
                        // unlike x64, unused bits are set to zero so we don't need to extend
                        masm.asm.uload(mem, dst, size, UNTRUSTED_FLAGS);
                        Ok(())
                    })
                }
            }
            LoadKind::VectorExtend(_vector_extend_kind) => {
                bail!(CodeGenError::UnimplementedWasmLoadKind)
            }
            LoadKind::VectorLane(_selector) => {
                bail!(CodeGenError::unimplemented_masm_instruction())
            }
            LoadKind::Atomic(_, _) => bail!(CodeGenError::unimplemented_masm_instruction()),
            LoadKind::VectorZero(_size) => {
                bail!(CodeGenError::UnimplementedWasmLoadKind)
            }
        })
    }

    fn compute_addr(
        &mut self,
        src: Self::Address,
        dst: WritableReg,
        size: OperandSize,
    ) -> Result<()> {
        let (base, offset) = src.unwrap_offset();
        self.add_ir(dst, base, I::i64(offset), size)
    }

    fn pop(&mut self, dst: WritableReg, size: OperandSize) -> Result<()> {
        let addr = self.address_from_sp(SPOffset::from_u32(self.sp_offset))?;
        addr.to_addressing_mode(self, size, |masm, mem| {
            masm.asm.uload(mem, dst, size, TRUSTED_FLAGS);
            Ok(())
        })?;
        self.free_stack(size.bytes())
    }

    fn sp_offset(&self) -> Result<SPOffset> {
        Ok(SPOffset::from_u32(self.sp_offset))
    }

    fn finalize(mut self, base: Option<SourceLoc>) -> Result<MachBufferFinalized<Final>> {
        if let Some(patch) = self.stack_max_use_add {
            patch.finalize(i32::try_from(self.sp_max).unwrap(), self.asm.buffer_mut());
        }

        Ok(self.asm.finalize(base))
    }

    fn mov(&mut self, dst: WritableReg, src: RegImm, size: OperandSize) -> Result<()> {
        match (src, dst) {
            (RegImm::Imm(v), _) => match v {
                I::I32(_) | I::I64(_) => {
                    self.asm.mov_ir(dst, v, v.size());
                    Ok(())
                }
                imm @ (I::F32(_) | I::F64(_) | I::V128(_)) => {
                    self.asm.mov_ir(dst, imm, imm.size());
                    Ok(())
                }
            },
            (RegImm::Reg(rs), rd) => match (rs.class(), rd.to_reg().class()) {
                (RegClass::Int, RegClass::Int) => Ok(self.asm.mov_rr(rs, rd, size)),
                (RegClass::Float, RegClass::Float) => Ok(self.asm.fmov_rr(rs, rd, size)),
                (RegClass::Int, RegClass::Float) => Ok(self.asm.mov_to_fpu(rs, rd, size)),
                _ => bail!(CodeGenError::invalid_operand_combination()),
            },
        }
    }

    fn cmov(
        &mut self,
        dst: WritableReg,
        src: Reg,
        cc: IntCmpKind,
        size: OperandSize,
    ) -> Result<()> {
        match (src.class(), dst.to_reg().class()) {
            (RegClass::Int, RegClass::Int) => self.asm.csel(src, dst.to_reg(), dst, Cond::from(cc)),
            (RegClass::Float, RegClass::Float) => {
                self.asm
                    .fpu_csel(src, dst.to_reg(), dst, Cond::from(cc), size)
            }
            _ => return Err(format_err!(CodeGenError::invalid_operand_combination())),
        }

        Ok(())
    }

    fn add(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => self.add_ir(rd, rn, v, size),

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.add_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn add_uextend(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        from_size: OperandSize,
        size: OperandSize,
    ) -> Result<()> {
        assert!(from_size.num_bits() <= size.num_bits());
        let extendop = match from_size {
            OperandSize::S8 => ExtendOp::UXTB,
            OperandSize::S16 => ExtendOp::UXTH,
            OperandSize::S32 => ExtendOp::UXTW,
            OperandSize::S64 => ExtendOp::UXTX,
            OperandSize::S128 => {
                return Err(format_err!(CodeGenError::invalid_operand_combination()));
            }
        };

        self.asm.add_rrr_with_extend(rhs, lhs, dst, size, extendop);
        Ok(())
    }

    fn checked_uadd(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: RegImm,
        size: OperandSize,
        trap: TrapCode,
    ) -> Result<()> {
        // Similar to all the other potentially-trapping operations, we need to
        // ensure that the real SP is 16-byte aligned in case control flow is
        // transferred to a signal handler.
        self.with_aligned_sp(|masm| {
            // NB: we don't use `Self::add_ir` since we explicitly
            // want to emit the add variant which sets overflow
            // flags.
            match rhs {
                RegImm::Reg(rm) => {
                    masm.asm.adds_rrr(rm, lhs, dst, size);
                }
                RegImm::Imm(rhs) => {
                    let imm = rhs.unwrap_as_u64();
                    match Imm12::maybe_from_u64(imm) {
                        Some(imm12) => masm.asm.adds_ir(imm12, lhs, dst, size),
                        None => {
                            masm.with_scratch::<IntScratch, _>(|masm, scratch| {
                                masm.asm.mov_ir(scratch.writable(), rhs, rhs.size());
                                masm.asm.adds_rrr(scratch.inner(), lhs, dst, size);
                            });
                        }
                    }
                }
            }
            masm.asm.trapif(Cond::Hs, trap);
            Ok(())
        })
    }

    fn sub(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = v.unwrap_as_u64();
                match Imm12::maybe_from_u64(imm) {
                    Some(imm12) => self.asm.sub_ir(imm12, rn, rd, size),
                    None => {
                        self.with_scratch::<IntScratch, _>(|masm, scratch| {
                            masm.asm.mov_ir(scratch.writable(), v, v.size());
                            masm.asm.sub_rrr(scratch.inner(), rn, rd, size);
                        });
                    }
                };

                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.sub_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn mul(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => self.with_scratch::<IntScratch, _>(|masm, scratch| {
                masm.asm.mov_ir(scratch.writable(), v, v.size());
                masm.asm.mul_rrr(scratch.inner(), rn, rd, size);
                Ok(())
            }),

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.mul_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn float_add(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fadd_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_sub(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fsub_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_mul(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fmul_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_div(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fdiv_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_min(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fmin_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_max(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()> {
        self.asm.fmax_rrr(rhs, lhs, dst, size);
        Ok(())
    }

    fn float_copysign(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        size: OperandSize,
    ) -> Result<()> {
        let max_shift = match size {
            OperandSize::S32 => 0x1f,
            OperandSize::S64 => 0x3f,
            _ => bail!(CodeGenError::unexpected_operand_size()),
        };
        self.asm.fushr_rri(rhs, writable!(rhs), max_shift, size);
        self.asm.fsli_rri_mod(lhs, rhs, dst, max_shift, size);
        Ok(())
    }

    fn float_neg(&mut self, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.fneg_rr(dst.to_reg(), dst, size);
        Ok(())
    }

    fn float_abs(&mut self, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.fabs_rr(dst.to_reg(), dst, size);
        Ok(())
    }

    fn float_round<
        F: FnMut(&mut FuncEnv<Self::Ptr>, &mut CodeGenContext<Emission>, &mut Self) -> Result<()>,
    >(
        &mut self,
        mode: RoundingMode,
        _env: &mut FuncEnv<Self::Ptr>,
        context: &mut CodeGenContext<Emission>,
        size: OperandSize,
        _fallback: F,
    ) -> Result<()> {
        let src = context.pop_to_reg(self, None)?;
        self.asm
            .fround_rr(src.into(), writable!(src.into()), mode, size);
        context.stack.push(src.into());
        Ok(())
    }

    fn float_sqrt(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()> {
        self.asm.fsqrt_rr(src, dst, size);
        Ok(())
    }

    fn maybe_canonicalize_nan(&mut self, reg: WritableReg, size: OperandSize) -> Result<()> {
        if !self.shared_flags.enable_nan_canonicalization() {
            return Ok(());
        }

        let done_label = self.asm.buffer_mut().get_label();

        self.asm.fcmp(reg.to_reg(), reg.to_reg(), size);
        self.asm.jmp_if(Cond::Vc, done_label);

        let canonical_nan = match size {
            OperandSize::S32 => crate::masm::CANONICAL_NAN_F32,
            OperandSize::S64 => crate::masm::CANONICAL_NAN_F64,
            _ => bail!(CodeGenError::unexpected_operand_size()),
        };
        let constant = self.asm.add_constant(canonical_nan);
        self.asm.uload(
            inst::AMode::Const { addr: constant },
            reg,
            size,
            TRUSTED_FLAGS,
        );

        self.asm
            .buffer_mut()
            .bind_label(done_label, &mut Default::default());
        Ok(())
    }

    fn maybe_canonicalize_v128_nan(
        &mut self,
        _reg: WritableReg,
        _lane_size: OperandSize,
    ) -> Result<()> {
        if !self.shared_flags.enable_nan_canonicalization() {
            return Ok(());
        }
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn and(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = v.unwrap_as_u64();
                let csize: inst::OperandSize = size.into();

                match ImmLogic::maybe_from_u64(imm, csize.to_ty()) {
                    Some(imml) => self.asm.and_ir(imml, rn, rd, size),
                    None => {
                        self.with_scratch::<IntScratch, _>(|masm, scratch| {
                            masm.asm.mov_ir(scratch.writable(), v, v.size());
                            masm.asm.and_rrr(scratch.inner(), rn, rd, size);
                        });
                    }
                };

                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.and_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn or(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = v.unwrap_as_u64();
                let csize: inst::OperandSize = size.into();

                match ImmLogic::maybe_from_u64(imm, csize.to_ty()) {
                    Some(imml) => self.asm.or_ir(imml, rn, rd, size),
                    None => {
                        self.with_scratch::<IntScratch, _>(|masm, scratch| {
                            masm.asm.mov_ir(scratch.writable(), v, v.size());
                            masm.asm.or_rrr(scratch.inner(), rn, rd, size);
                        });
                    }
                };

                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.or_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn xor(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()> {
        match (rhs, lhs, dst) {
            (RegImm::Imm(v), rn, rd) => {
                let imm = v.unwrap_as_u64();
                let csize: inst::OperandSize = size.into();

                match ImmLogic::maybe_from_u64(imm, csize.to_ty()) {
                    Some(imml) => self.asm.xor_ir(imml, rn, rd, size),
                    None => {
                        self.with_scratch::<IntScratch, _>(|masm, scratch| {
                            masm.asm.mov_ir(scratch.writable(), v, v.size());
                            masm.asm.xor_rrr(scratch.inner(), rn, rd, size);
                        });
                    }
                };
                Ok(())
            }

            (RegImm::Reg(rm), rn, rd) => {
                self.asm.xor_rrr(rm, rn, rd, size);
                Ok(())
            }
        }
    }

    fn shift_ir(
        &mut self,
        dst: WritableReg,
        imm: I,
        lhs: Reg,
        kind: ShiftKind,
        size: OperandSize,
    ) -> Result<()> {
        match ImmShift::maybe_from_u64(imm.unwrap_as_u64()) {
            // Immediate Ranges:
            //   32-bit variant: 0-31
            //   64-bit variant: 0-63
            Some(imml) if imml.value() < size.num_bits() => {
                self.asm.shift_ir(imml, lhs, dst, kind, size)
            }
            _ => {
                self.with_scratch::<IntScratch, _>(|masm, scratch| {
                    masm.asm.mov_ir(scratch.writable(), imm, imm.size());
                    masm.asm.shift_rrr(scratch.inner(), lhs, dst, kind, size);
                });
            }
        };
        Ok(())
    }

    fn shift(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: ShiftKind,
        size: OperandSize,
    ) -> Result<()> {
        let src = context.pop_to_reg(self, None)?;
        let dst = context.pop_to_reg(self, None)?;

        self.asm
            .shift_rrr(src.into(), dst.into(), writable!(dst.into()), kind, size);

        context.free_reg(src);
        context.stack.push(dst.into());

        Ok(())
    }

    fn div(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: DivKind,
        size: OperandSize,
    ) -> Result<()> {
        context.binop(self, size, |this, dividend, divisor, size| {
            this.with_aligned_sp(|this| {
                this.asm
                    .div_rrr(divisor, dividend, writable!(dividend), kind, size);
                Ok(())
            })?;
            match size {
                OperandSize::S32 => Ok(TypedReg::new(WasmValType::I32, dividend)),
                OperandSize::S64 => Ok(TypedReg::new(WasmValType::I64, dividend)),
                _ => Err(format_err!(CodeGenError::unexpected_operand_size())),
            }
        })
    }

    fn rem(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: RemKind,
        size: OperandSize,
    ) -> Result<()> {
        context.binop(self, size, |this, dividend, divisor, size| {
            this.with_aligned_sp(|this| {
                this.with_scratch::<IntScratch, _>(|masm, scratch| {
                    masm.asm.rem_rrr(
                        divisor,
                        dividend,
                        writable!(dividend),
                        scratch.writable(),
                        kind,
                        size,
                    );
                });
                Ok(())
            })?;
            match size {
                OperandSize::S32 => Ok(TypedReg::new(WasmValType::I32, dividend)),
                OperandSize::S64 => Ok(TypedReg::new(WasmValType::I64, dividend)),
                _ => Err(format_err!(CodeGenError::unexpected_operand_size())),
            }
        })
    }

    fn zero(&mut self, reg: WritableReg) -> Result<()> {
        self.asm.mov_ir(reg, I::i64(0), OperandSize::S64);
        Ok(())
    }

    fn popcnt(&mut self, context: &mut CodeGenContext<Emission>, size: OperandSize) -> Result<()> {
        let src = context.pop_to_reg(self, None)?;
        self.with_scratch::<FloatScratch, _>(|masm, tmp| {
            masm.asm.mov_to_fpu(src.into(), tmp.writable(), size);
            masm.asm.cnt(tmp.writable());
            masm.asm
                .addv(tmp.inner(), tmp.writable(), VectorSize::Size8x8);
            masm.asm
                .mov_from_vec(tmp.inner(), writable!(src.into()), 0, OperandSize::S8);
        });
        context.stack.push(src.into());
        Ok(())
    }

    fn signed_truncate(
        &mut self,
        dst: WritableReg,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
    ) -> Result<()> {
        self.with_aligned_sp(|masm| {
            masm.with_scratch::<FloatScratch, _>(|masm, scratch| {
                masm.asm
                    .fpu_to_int(dst, src, scratch.writable(), src_size, dst_size, kind, true);
            });
            Ok(())
        })
    }

    fn unsigned_truncate(
        &mut self,
        ctx: &mut CodeGenContext<Emission>,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
    ) -> Result<()> {
        let dst_ty = match dst_size {
            OperandSize::S32 => WasmValType::I32,
            OperandSize::S64 => WasmValType::I64,
            _ => bail!(CodeGenError::unexpected_operand_size()),
        };

        ctx.convert_op(self, dst_ty, |masm, dst, src, dst_size| {
            masm.with_aligned_sp(|masm| {
                masm.with_scratch::<FloatScratch, _>(|masm, scratch| {
                    masm.asm.fpu_to_int(
                        writable!(dst),
                        src,
                        scratch.writable(),
                        src_size,
                        dst_size,
                        kind,
                        false,
                    );
                    Ok(())
                })
            })
        })
    }

    fn signed_convert(
        &mut self,
        dst: WritableReg,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) -> Result<()> {
        self.asm.cvt_sint_to_float(src, dst, src_size, dst_size);
        Ok(())
    }

    fn unsigned_convert(
        &mut self,
        dst: WritableReg,
        src: Reg,
        _tmp_gpr: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) -> Result<()> {
        self.asm.cvt_uint_to_float(src, dst, src_size, dst_size);
        Ok(())
    }

    fn reinterpret_float_as_int(
        &mut self,
        dst: WritableReg,
        src: Reg,
        size: OperandSize,
    ) -> Result<()> {
        self.asm.mov_from_vec(src, dst, 0, size);
        Ok(())
    }

    fn reinterpret_int_as_float(
        &mut self,
        dst: WritableReg,
        src: Reg,
        size: OperandSize,
    ) -> Result<()> {
        self.asm.mov_to_fpu(src, dst, size);
        Ok(())
    }

    fn demote(&mut self, dst: WritableReg, src: Reg) -> Result<()> {
        self.asm
            .cvt_float_to_float(src, dst, OperandSize::S64, OperandSize::S32);
        Ok(())
    }

    fn promote(&mut self, dst: WritableReg, src: Reg) -> Result<()> {
        self.asm
            .cvt_float_to_float(src, dst, OperandSize::S32, OperandSize::S64);
        Ok(())
    }

    fn push(&mut self, reg: Reg, size: OperandSize) -> Result<StackSlot> {
        self.reserve_stack(size.bytes())?;
        let address = self.address_from_sp(SPOffset::from_u32(self.sp_offset))?;
        address.to_addressing_mode(self, size, |masm, mem| {
            masm.asm.str(reg, mem, size, TRUSTED_FLAGS);
            Ok(())
        })?;

        Ok(StackSlot {
            offset: SPOffset::from_u32(self.sp_offset),
            size: size.bytes(),
        })
    }

    fn address_at_reg(&self, reg: Reg, offset: u32) -> Result<Self::Address> {
        Ok(Address::offset(reg, offset as i64))
    }

    fn cmp_with_set(
        &mut self,
        dst: WritableReg,
        src: RegImm,
        kind: IntCmpKind,
        size: OperandSize,
    ) -> Result<()> {
        self.cmp(dst.to_reg(), src, size)?;
        self.asm.cset(dst, kind.into());
        Ok(())
    }

    fn cmp(&mut self, src1: Reg, src2: RegImm, size: OperandSize) -> Result<()> {
        match src2 {
            RegImm::Reg(src2) => {
                self.asm.subs_rrr(src2, src1, writable!(regs::zero()), size);
                Ok(())
            }
            RegImm::Imm(v) => {
                let val = v.unwrap_as_u64();
                match Imm12::maybe_from_u64(val) {
                    Some(imm12) => self.asm.subs_ir(imm12, src1, size),
                    None => {
                        self.with_scratch::<IntScratch, _>(|masm, scratch| {
                            masm.asm.mov_ir(scratch.writable(), v, v.size());
                            masm.asm
                                .subs_rrr(scratch.inner(), src1, writable!(regs::zero()), size);
                        });
                    }
                };
                Ok(())
            }
        }
    }

    fn float_cmp_with_set(
        &mut self,
        dst: WritableReg,
        src1: Reg,
        src2: Reg,
        kind: FloatCmpKind,
        size: OperandSize,
    ) -> Result<()> {
        self.asm.fcmp(src1, src2, size);
        self.asm.cset(dst, kind.into());
        Ok(())
    }

    fn clz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()> {
        self.asm.clz(src, dst, size);
        Ok(())
    }

    fn ctz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()> {
        self.with_scratch::<IntScratch, _>(|masm, scratch| {
            masm.asm.rbit(src, scratch.writable(), size);
            masm.asm.clz(scratch.inner(), dst, size);
            Ok(())
        })
    }

    fn wrap(&mut self, dst: WritableReg, src: Reg) -> Result<()> {
        self.asm.mov_rr(src, dst, OperandSize::S32);
        Ok(())
    }

    fn extend(&mut self, dst: WritableReg, src: Reg, kind: ExtendKind) -> Result<()> {
        self.asm.extend(src, dst, kind);
        Ok(())
    }

    fn get_label(&mut self) -> Result<MachLabel> {
        Ok(self.asm.get_label())
    }

    fn bind(&mut self, label: MachLabel) -> Result<()> {
        let buffer = self.asm.buffer_mut();
        buffer.bind_label(label, &mut Default::default());
        Ok(())
    }

    fn branch(
        &mut self,
        kind: IntCmpKind,
        lhs: Reg,
        rhs: RegImm,
        taken: MachLabel,
        size: OperandSize,
    ) -> Result<()> {
        use IntCmpKind::*;

        match &(lhs, rhs) {
            (rlhs, RegImm::Reg(rrhs)) => {
                // If the comparison kind is zero or not zero and both operands
                // are the same register, emit an ands instruction. Else we emit
                // a normal comparison.
                if (kind == Eq || kind == Ne) && (rlhs == rrhs) {
                    self.asm.ands_rr(*rlhs, *rrhs, size);
                } else {
                    self.cmp(lhs, rhs, size)?;
                }
            }
            _ => self.cmp(lhs, rhs, size)?,
        }
        self.asm.jmp_if(kind.into(), taken);
        Ok(())
    }

    fn jmp(&mut self, target: MachLabel) -> Result<()> {
        self.asm.jmp(target);
        Ok(())
    }

    fn unreachable(&mut self) -> Result<()> {
        self.with_aligned_sp(|masm| {
            masm.asm.udf(wasmtime_cranelift::TRAP_UNREACHABLE);
            Ok(())
        })
    }

    fn jmp_table(&mut self, targets: &[MachLabel], index: Reg, tmp: Reg) -> Result<()> {
        // At least one default target.
        debug_assert!(targets.len() >= 1);
        let default_index = targets.len() - 1;
        let max = default_index;
        self.asm.mov_ir(
            writable!(tmp),
            I::i32(i32::try_from(max).unwrap()),
            OperandSize::S32,
        );
        // NB: We only emit the comparison instruction, since
        // `Assembler::jmp_table` (and the underlying Cranelift
        // instruction) will emit spectre mitigation and bounds
        // checks.
        self.asm
            .subs_rrr(tmp, index, writable!(regs::zero()), OperandSize::S32);
        let default = targets[default_index];
        let rest = &targets[0..default_index];
        self.with_scratch::<IntScratch, _>(|masm, scratch| {
            masm.asm
                .jmp_table(rest, default, index, scratch.inner(), tmp);
            Ok(())
        })
    }

    fn trap(&mut self, code: TrapCode) -> Result<()> {
        self.with_aligned_sp(|masm| {
            masm.asm.udf(code);
            Ok(())
        })
    }

    fn trapz(&mut self, src: Reg, code: TrapCode) -> Result<()> {
        self.with_aligned_sp(|masm| {
            masm.asm.trapz(src, code, OperandSize::S64);
            Ok(())
        })
    }

    fn trapif(&mut self, cc: IntCmpKind, code: TrapCode) -> Result<()> {
        self.with_aligned_sp(|masm| {
            masm.asm.trapif(cc.into(), code);
            Ok(())
        })
    }

    fn start_source_loc(&mut self, loc: RelSourceLoc) -> Result<(CodeOffset, RelSourceLoc)> {
        Ok(self.asm.buffer_mut().start_srcloc(loc))
    }

    fn end_source_loc(&mut self) -> Result<()> {
        self.asm.buffer_mut().end_srcloc();
        Ok(())
    }

    fn current_code_offset(&self) -> Result<CodeOffset> {
        Ok(self.asm.buffer().cur_offset())
    }

    fn add128(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs_lo: Reg,
        lhs_hi: Reg,
        rhs_lo: Reg,
        rhs_hi: Reg,
    ) -> Result<()> {
        self.asm.adds_rrr(rhs_lo, lhs_lo, dst_lo, OperandSize::S64);
        self.asm.adc_rrr(rhs_hi, lhs_hi, dst_hi, OperandSize::S64);
        Ok(())
    }

    fn sub128(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs_lo: Reg,
        lhs_hi: Reg,
        rhs_lo: Reg,
        rhs_hi: Reg,
    ) -> Result<()> {
        self.asm.subs_rrr(rhs_lo, lhs_lo, dst_lo, OperandSize::S64);
        self.asm.sbc_rrr(rhs_hi, lhs_hi, dst_hi, OperandSize::S64);
        Ok(())
    }

    fn mul_wide(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: MulWideKind,
    ) -> Result<()> {
        let rhs = context.pop_to_reg(self, None)?;
        let lhs = context.pop_to_reg(self, None)?;
        let dst_hi = context.any_gpr(self)?;

        // Emit the high-half multiply first since the low-half multiply may
        // alias `lhs` or `rhs` as its destination.
        match kind {
            MulWideKind::Signed => self.asm.smulh_rrr(rhs.reg, lhs.reg, writable!(dst_hi)),
            MulWideKind::Unsigned => self.asm.umulh_rrr(rhs.reg, lhs.reg, writable!(dst_hi)),
        }
        self.asm
            .mul_rrr(rhs.reg, lhs.reg, writable!(lhs.reg), OperandSize::S64);

        context.free_reg(rhs);
        context.stack.push(lhs.into());
        context.stack.push(Val::Reg(TypedReg::i64(dst_hi)));
        Ok(())
    }

    fn splat(&mut self, context: &mut CodeGenContext<Emission>, size: SplatKind) -> Result<()> {
        let src = context.pop_to_reg(self, None)?;
        let dst = writable!(context.any_fpr(self)?);

        match size {
            SplatKind::I8x16 => {
                self.asm.vec_dup(src.reg, dst, VectorSize::Size8x16);
            }
            SplatKind::I16x8 => {
                self.asm.vec_dup(src.reg, dst, VectorSize::Size16x8);
            }
            SplatKind::I32x4 => {
                self.asm.vec_dup(src.reg, dst, VectorSize::Size32x4);
            }
            SplatKind::I64x2 => {
                self.asm.vec_dup(src.reg, dst, VectorSize::Size64x2);
            }
            SplatKind::F32x4 => {
                self.asm.vec_dup_elem(src.reg, dst, VectorSize::Size32x4, 0);
            }
            SplatKind::F64x2 => {
                self.asm.vec_dup_elem(src.reg, dst, VectorSize::Size64x2, 0);
            }
        }
        context.free_reg(src);
        context.stack.push(TypedReg::v128(dst.to_reg()).into());
        Ok(())
    }

    fn shuffle(&mut self, _dst: WritableReg, _lhs: Reg, _rhs: Reg, _lanes: [u8; 16]) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn swizzle(&mut self, _dst: WritableReg, _lhs: Reg, _rhs: Reg) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn atomic_rmw(
        &mut self,
        _context: &mut CodeGenContext<Emission>,
        _addr: Self::Address,
        _size: OperandSize,
        _op: RmwOp,
        _flags: MemFlagsData,
        _extend: Option<Extend<Zero>>,
    ) -> Result<()> {
        Err(format_err!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn extract_lane(
        &mut self,
        src: Reg,
        dst: WritableReg,
        lane: u8,
        kind: ExtractLaneKind,
    ) -> Result<()> {
        match kind {
            ExtractLaneKind::I8x16S => {
                self.asm
                    .mov_from_vec_signed(src, dst, lane, VectorSize::Size8x16, OperandSize::S32)
            }
            ExtractLaneKind::I16x8S => {
                self.asm
                    .mov_from_vec_signed(src, dst, lane, VectorSize::Size16x8, OperandSize::S32)
            }
            ExtractLaneKind::I8x16U => self.asm.mov_from_vec(src, dst, lane, OperandSize::S8),
            ExtractLaneKind::I16x8U => self.asm.mov_from_vec(src, dst, lane, OperandSize::S16),
            ExtractLaneKind::I32x4 => self.asm.mov_from_vec(src, dst, lane, OperandSize::S32),
            ExtractLaneKind::I64x2 => self.asm.mov_from_vec(src, dst, lane, OperandSize::S64),
            ExtractLaneKind::F32x4 => {
                self.asm
                    .fpu_move_from_vec(src, dst, lane, VectorSize::Size32x4)
            }
            ExtractLaneKind::F64x2 => {
                self.asm
                    .fpu_move_from_vec(src, dst, lane, VectorSize::Size64x2)
            }
        }
        Ok(())
    }

    fn replace_lane(
        &mut self,
        src: RegImm,
        dst: WritableReg,
        lane: u8,
        kind: ReplaceLaneKind,
    ) -> Result<()> {
        let size = match kind {
            ReplaceLaneKind::I8x16 => VectorSize::Size8x16,
            ReplaceLaneKind::I16x8 => VectorSize::Size16x8,
            ReplaceLaneKind::I32x4 => VectorSize::Size32x4,
            ReplaceLaneKind::I64x2 => VectorSize::Size64x2,
            ReplaceLaneKind::F32x4 => VectorSize::Size32x4,
            ReplaceLaneKind::F64x2 => VectorSize::Size64x2,
        };
        match kind {
            ReplaceLaneKind::I8x16
            | ReplaceLaneKind::I16x8
            | ReplaceLaneKind::I32x4
            | ReplaceLaneKind::I64x2 => match src {
                RegImm::Reg(reg) => self.asm.mov_to_vec(reg, dst, lane, size),
                RegImm::Imm(imm) => {
                    self.with_scratch::<IntScratch, _>(|masm, scratch| {
                        masm.asm.mov_ir(scratch.writable(), imm, imm.size());
                        masm.asm.mov_to_vec(scratch.inner(), dst, lane, size);
                    });
                }
            },
            ReplaceLaneKind::F32x4 | ReplaceLaneKind::F64x2 => match src {
                RegImm::Reg(reg) => self.asm.vec_mov_element(reg, dst, lane, 0, size),
                RegImm::Imm(imm) => {
                    self.with_scratch::<FloatScratch, _>(|masm, scratch| {
                        masm.asm.mov_ir(scratch.writable(), imm, imm.size());
                        masm.asm
                            .vec_mov_element(scratch.inner(), dst, lane, 0, size);
                    });
                }
            },
        }
        Ok(())
    }

    fn atomic_cas(
        &mut self,
        _context: &mut CodeGenContext<Emission>,
        _addr: Self::Address,
        _size: OperandSize,
        _flags: MemFlagsData,
        _extend: Option<Extend<Zero>>,
    ) -> Result<()> {
        Err(format_err!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_eq(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        kind: VectorEqualityKind,
    ) -> Result<()> {
        match kind {
            VectorEqualityKind::I8x16 => {
                self.asm
                    .vec_rrr(VecALUOp::Cmeq, lhs, rhs, dst, VectorSize::Size8x16);
            }
            VectorEqualityKind::I16x8 => {
                self.asm
                    .vec_rrr(VecALUOp::Cmeq, lhs, rhs, dst, VectorSize::Size16x8);
            }
            VectorEqualityKind::I32x4 => {
                self.asm
                    .vec_rrr(VecALUOp::Cmeq, lhs, rhs, dst, VectorSize::Size32x4);
            }
            VectorEqualityKind::I64x2 => {
                self.asm
                    .vec_rrr(VecALUOp::Cmeq, lhs, rhs, dst, VectorSize::Size64x2);
            }
            VectorEqualityKind::F32x4 => {
                self.asm
                    .vec_rrr(VecALUOp::Fcmeq, lhs, rhs, dst, VectorSize::Size32x4);
            }
            VectorEqualityKind::F64x2 => {
                self.asm
                    .vec_rrr(VecALUOp::Fcmeq, lhs, rhs, dst, VectorSize::Size64x2);
            }
        }
        Ok(())
    }

    fn v128_ne(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        kind: VectorEqualityKind,
    ) -> Result<()> {
        match kind {
            VectorEqualityKind::I8x16 => {
                self.asm
                    .vec_rrr(VecALUOp::Cmeq, lhs, rhs, dst, VectorSize::Size8x16);
                self.asm
                    .vec_misc(VecMisc2::Not, dst.to_reg(), dst, VectorSize::Size8x16);
            }
            VectorEqualityKind::I16x8 => {
                self.asm
                    .vec_rrr(VecALUOp::Cmeq, lhs, rhs, dst, VectorSize::Size16x8);
                self.asm
                    .vec_misc(VecMisc2::Not, dst.to_reg(), dst, VectorSize::Size16x8);
            }
            VectorEqualityKind::I32x4 => {
                self.asm
                    .vec_rrr(VecALUOp::Cmeq, lhs, rhs, dst, VectorSize::Size32x4);
                self.asm
                    .vec_misc(VecMisc2::Not, dst.to_reg(), dst, VectorSize::Size32x4);
            }
            VectorEqualityKind::I64x2 => {
                self.asm
                    .vec_rrr(VecALUOp::Cmeq, lhs, rhs, dst, VectorSize::Size64x2);
                self.asm
                    .vec_misc(VecMisc2::Not, dst.to_reg(), dst, VectorSize::Size64x2);
            }
            VectorEqualityKind::F32x4 => {
                self.asm
                    .vec_rrr(VecALUOp::Fcmeq, lhs, rhs, dst, VectorSize::Size32x4);
                self.asm
                    .vec_misc(VecMisc2::Not, dst.to_reg(), dst, VectorSize::Size32x4);
            }
            VectorEqualityKind::F64x2 => {
                self.asm
                    .vec_rrr(VecALUOp::Fcmeq, lhs, rhs, dst, VectorSize::Size64x2);
                self.asm
                    .vec_misc(VecMisc2::Not, dst.to_reg(), dst, VectorSize::Size64x2);
            }
        }
        Ok(())
    }

    fn v128_lt(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        kind: VectorCompareKind,
    ) -> Result<()> {
        // aarch64 lacks vector less-than; swap operands and use greater-than.
        let (op, size) = match kind {
            VectorCompareKind::I8x16S => (VecALUOp::Cmgt, VectorSize::Size8x16),
            VectorCompareKind::I8x16U => (VecALUOp::Cmhi, VectorSize::Size8x16),
            VectorCompareKind::I16x8S => (VecALUOp::Cmgt, VectorSize::Size16x8),
            VectorCompareKind::I16x8U => (VecALUOp::Cmhi, VectorSize::Size16x8),
            VectorCompareKind::I32x4S => (VecALUOp::Cmgt, VectorSize::Size32x4),
            VectorCompareKind::I32x4U => (VecALUOp::Cmhi, VectorSize::Size32x4),
            VectorCompareKind::I64x2S => (VecALUOp::Cmgt, VectorSize::Size64x2),
            VectorCompareKind::F32x4 => (VecALUOp::Fcmgt, VectorSize::Size32x4),
            VectorCompareKind::F64x2 => (VecALUOp::Fcmgt, VectorSize::Size64x2),
        };
        self.asm.vec_rrr(op, rhs, lhs, dst, size);
        Ok(())
    }

    fn v128_le(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        kind: VectorCompareKind,
    ) -> Result<()> {
        // aarch64 lacks vector less-than; swap operands and use greater-than.
        let (op, size) = match kind {
            VectorCompareKind::I8x16S => (VecALUOp::Cmge, VectorSize::Size8x16),
            VectorCompareKind::I8x16U => (VecALUOp::Cmhs, VectorSize::Size8x16),
            VectorCompareKind::I16x8S => (VecALUOp::Cmge, VectorSize::Size16x8),
            VectorCompareKind::I16x8U => (VecALUOp::Cmhs, VectorSize::Size16x8),
            VectorCompareKind::I32x4S => (VecALUOp::Cmge, VectorSize::Size32x4),
            VectorCompareKind::I32x4U => (VecALUOp::Cmhs, VectorSize::Size32x4),
            VectorCompareKind::I64x2S => (VecALUOp::Cmge, VectorSize::Size64x2),
            VectorCompareKind::F32x4 => (VecALUOp::Fcmge, VectorSize::Size32x4),
            VectorCompareKind::F64x2 => (VecALUOp::Fcmge, VectorSize::Size64x2),
        };
        self.asm.vec_rrr(op, rhs, lhs, dst, size);
        Ok(())
    }

    fn v128_gt(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        kind: VectorCompareKind,
    ) -> Result<()> {
        let (op, size) = match kind {
            VectorCompareKind::I8x16S => (VecALUOp::Cmgt, VectorSize::Size8x16),
            VectorCompareKind::I8x16U => (VecALUOp::Cmhi, VectorSize::Size8x16),
            VectorCompareKind::I16x8S => (VecALUOp::Cmgt, VectorSize::Size16x8),
            VectorCompareKind::I16x8U => (VecALUOp::Cmhi, VectorSize::Size16x8),
            VectorCompareKind::I32x4S => (VecALUOp::Cmgt, VectorSize::Size32x4),
            VectorCompareKind::I32x4U => (VecALUOp::Cmhi, VectorSize::Size32x4),
            VectorCompareKind::I64x2S => (VecALUOp::Cmgt, VectorSize::Size64x2),
            VectorCompareKind::F32x4 => (VecALUOp::Fcmgt, VectorSize::Size32x4),
            VectorCompareKind::F64x2 => (VecALUOp::Fcmgt, VectorSize::Size64x2),
        };
        self.asm.vec_rrr(op, lhs, rhs, dst, size);
        Ok(())
    }

    fn v128_ge(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        kind: VectorCompareKind,
    ) -> Result<()> {
        let (op, size) = match kind {
            VectorCompareKind::I8x16S => (VecALUOp::Cmge, VectorSize::Size8x16),
            VectorCompareKind::I8x16U => (VecALUOp::Cmhs, VectorSize::Size8x16),
            VectorCompareKind::I16x8S => (VecALUOp::Cmge, VectorSize::Size16x8),
            VectorCompareKind::I16x8U => (VecALUOp::Cmhs, VectorSize::Size16x8),
            VectorCompareKind::I32x4S => (VecALUOp::Cmge, VectorSize::Size32x4),
            VectorCompareKind::I32x4U => (VecALUOp::Cmhs, VectorSize::Size32x4),
            VectorCompareKind::I64x2S => (VecALUOp::Cmge, VectorSize::Size64x2),
            VectorCompareKind::F32x4 => (VecALUOp::Fcmge, VectorSize::Size32x4),
            VectorCompareKind::F64x2 => (VecALUOp::Fcmge, VectorSize::Size64x2),
        };
        self.asm.vec_rrr(op, lhs, rhs, dst, size);
        Ok(())
    }

    fn v128_not(&mut self, dst: WritableReg) -> Result<()> {
        self.asm
            .vec_misc(VecMisc2::Not, dst.to_reg(), dst, VectorSize::Size32x4);
        Ok(())
    }

    fn fence(&mut self) -> Result<()> {
        Err(format_err!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_and(&mut self, src1: Reg, src2: Reg, dst: WritableReg) -> Result<()> {
        self.asm
            .vec_rrr(VecALUOp::And, src1, src2, dst, VectorSize::Size32x4);
        Ok(())
    }

    fn v128_and_not(&mut self, src1: Reg, src2: Reg, dst: WritableReg) -> Result<()> {
        self.asm
            .vec_rrr(VecALUOp::Bic, src2, src1, dst, VectorSize::Size32x4);
        Ok(())
    }

    fn v128_or(&mut self, src1: Reg, src2: Reg, dst: WritableReg) -> Result<()> {
        self.asm
            .vec_rrr(VecALUOp::Orr, src1, src2, dst, VectorSize::Size32x4);
        Ok(())
    }

    fn v128_xor(&mut self, src1: Reg, src2: Reg, dst: WritableReg) -> Result<()> {
        self.asm
            .vec_rrr(VecALUOp::Eor, src1, src2, dst, VectorSize::Size32x4);
        Ok(())
    }

    fn v128_bitselect(&mut self, src1: Reg, src2: Reg, mask: Reg, dst: WritableReg) -> Result<()> {
        self.asm.fmov_rr(mask, dst, OperandSize::S128);
        self.asm
            .vec_rrr_mod(VecALUModOp::Bsl, src1, src2, dst, VectorSize::Size32x4);
        Ok(())
    }

    fn v128_any_true(&mut self, src: Reg, dst: WritableReg) -> Result<()> {
        self.with_scratch::<FloatScratch, _>(|masm, tmp| {
            masm.asm.vec_rrr(
                VecALUOp::Umaxp,
                src,
                src,
                tmp.writable(),
                VectorSize::Size32x4,
            );
            masm.asm.mov_from_vec(tmp.inner(), dst, 0, OperandSize::S64);
        });
        self.asm.subs_ir(
            Imm12::maybe_from_u64(0).unwrap(),
            dst.to_reg(),
            OperandSize::S64,
        );
        self.asm.cset(dst, Cond::Ne);
        Ok(())
    }

    fn v128_convert(&mut self, src: Reg, dst: WritableReg, kind: V128ConvertKind) -> Result<()> {
        match kind {
            V128ConvertKind::I32x4S => {
                self.asm
                    .vec_misc(VecMisc2::Scvtf, src, dst, VectorSize::Size32x4);
            }
            V128ConvertKind::I32x4U => {
                self.asm
                    .vec_misc(VecMisc2::Ucvtf, src, dst, VectorSize::Size32x4);
            }
            V128ConvertKind::I32x4LowS => {
                self.asm
                    .vec_extend(VecExtendOp::Sxtl, src, dst, false, ScalarSize::Size64);
                self.asm
                    .vec_misc(VecMisc2::Scvtf, dst.to_reg(), dst, VectorSize::Size64x2);
            }
            V128ConvertKind::I32x4LowU => {
                self.asm
                    .vec_extend(VecExtendOp::Uxtl, src, dst, false, ScalarSize::Size64);
                self.asm
                    .vec_misc(VecMisc2::Ucvtf, dst.to_reg(), dst, VectorSize::Size64x2);
            }
        }
        Ok(())
    }

    fn v128_narrow(
        &mut self,
        src1: Reg,
        src2: Reg,
        dst: WritableReg,
        kind: V128NarrowKind,
    ) -> Result<()> {
        let (op, lane_size) = match kind {
            V128NarrowKind::I16x8S => (VecRRNarrowOp::Sqxtn, ScalarSize::Size8),
            V128NarrowKind::I16x8U => (VecRRNarrowOp::Sqxtun, ScalarSize::Size8),
            V128NarrowKind::I32x4S => (VecRRNarrowOp::Sqxtn, ScalarSize::Size16),
            V128NarrowKind::I32x4U => (VecRRNarrowOp::Sqxtun, ScalarSize::Size16),
        };
        debug_assert!(dst.to_reg() != src2);
        self.asm.vec_narrow(op, src1, dst, false, lane_size);
        self.asm.vec_narrow(op, src2, dst, true, lane_size);
        Ok(())
    }

    fn v128_demote(&mut self, src: Reg, dst: WritableReg) -> Result<()> {
        self.asm
            .vec_narrow(VecRRNarrowOp::Fcvtn, src, dst, false, ScalarSize::Size32);
        Ok(())
    }

    fn v128_promote(&mut self, src: Reg, dst: WritableReg) -> Result<()> {
        self.asm.vec_rr_long(VecRRLongOp::Fcvtl32, src, dst, false);
        Ok(())
    }

    fn v128_extend(&mut self, src: Reg, dst: WritableReg, kind: V128ExtendKind) -> Result<()> {
        use VecExtendOp::{Sxtl, Uxtl};
        let (op, high_half, lane_size) = match kind {
            V128ExtendKind::LowI8x16S => (Sxtl, false, ScalarSize::Size16),
            V128ExtendKind::HighI8x16S => (Sxtl, true, ScalarSize::Size16),
            V128ExtendKind::LowI8x16U => (Uxtl, false, ScalarSize::Size16),
            V128ExtendKind::HighI8x16U => (Uxtl, true, ScalarSize::Size16),
            V128ExtendKind::LowI16x8S => (Sxtl, false, ScalarSize::Size32),
            V128ExtendKind::HighI16x8S => (Sxtl, true, ScalarSize::Size32),
            V128ExtendKind::LowI16x8U => (Uxtl, false, ScalarSize::Size32),
            V128ExtendKind::HighI16x8U => (Uxtl, true, ScalarSize::Size32),
            V128ExtendKind::LowI32x4S => (Sxtl, false, ScalarSize::Size64),
            V128ExtendKind::HighI32x4S => (Sxtl, true, ScalarSize::Size64),
            V128ExtendKind::LowI32x4U => (Uxtl, false, ScalarSize::Size64),
            V128ExtendKind::HighI32x4U => (Uxtl, true, ScalarSize::Size64),
        };
        self.asm.vec_extend(op, src, dst, high_half, lane_size);
        Ok(())
    }

    fn v128_add(&mut self, lhs: Reg, rhs: Reg, dst: WritableReg, kind: V128AddKind) -> Result<()> {
        match kind {
            V128AddKind::F32x4 => {
                self.asm
                    .vec_rrr(VecALUOp::Fadd, lhs, rhs, dst, VectorSize::Size32x4);
            }
            V128AddKind::F64x2 => {
                self.asm
                    .vec_rrr(VecALUOp::Fadd, lhs, rhs, dst, VectorSize::Size64x2);
            }
            V128AddKind::I8x16 => {
                self.asm
                    .vec_rrr(VecALUOp::Add, lhs, rhs, dst, VectorSize::Size8x16);
            }
            V128AddKind::I8x16SatS => {
                self.asm
                    .vec_rrr(VecALUOp::Sqadd, lhs, rhs, dst, VectorSize::Size8x16);
            }
            V128AddKind::I8x16SatU => {
                self.asm
                    .vec_rrr(VecALUOp::Uqadd, lhs, rhs, dst, VectorSize::Size8x16);
            }
            V128AddKind::I16x8 => {
                self.asm
                    .vec_rrr(VecALUOp::Add, lhs, rhs, dst, VectorSize::Size16x8);
            }
            V128AddKind::I16x8SatS => {
                self.asm
                    .vec_rrr(VecALUOp::Sqadd, lhs, rhs, dst, VectorSize::Size16x8);
            }
            V128AddKind::I16x8SatU => {
                self.asm
                    .vec_rrr(VecALUOp::Uqadd, lhs, rhs, dst, VectorSize::Size16x8);
            }
            V128AddKind::I32x4 => {
                self.asm
                    .vec_rrr(VecALUOp::Add, lhs, rhs, dst, VectorSize::Size32x4);
            }
            V128AddKind::I64x2 => {
                self.asm
                    .vec_rrr(VecALUOp::Add, lhs, rhs, dst, VectorSize::Size64x2);
            }
        }
        Ok(())
    }

    fn v128_sub(&mut self, lhs: Reg, rhs: Reg, dst: WritableReg, kind: V128SubKind) -> Result<()> {
        match kind {
            V128SubKind::F32x4 => {
                self.asm
                    .vec_rrr(VecALUOp::Fsub, lhs, rhs, dst, VectorSize::Size32x4);
            }
            V128SubKind::F64x2 => {
                self.asm
                    .vec_rrr(VecALUOp::Fsub, lhs, rhs, dst, VectorSize::Size64x2);
            }
            V128SubKind::I8x16 => {
                self.asm
                    .vec_rrr(VecALUOp::Sub, lhs, rhs, dst, VectorSize::Size8x16);
            }
            V128SubKind::I8x16SatS => {
                self.asm
                    .vec_rrr(VecALUOp::Sqsub, lhs, rhs, dst, VectorSize::Size8x16);
            }
            V128SubKind::I8x16SatU => {
                self.asm
                    .vec_rrr(VecALUOp::Uqsub, lhs, rhs, dst, VectorSize::Size8x16);
            }
            V128SubKind::I16x8 => {
                self.asm
                    .vec_rrr(VecALUOp::Sub, lhs, rhs, dst, VectorSize::Size16x8);
            }
            V128SubKind::I16x8SatS => {
                self.asm
                    .vec_rrr(VecALUOp::Sqsub, lhs, rhs, dst, VectorSize::Size16x8);
            }
            V128SubKind::I16x8SatU => {
                self.asm
                    .vec_rrr(VecALUOp::Uqsub, lhs, rhs, dst, VectorSize::Size16x8);
            }
            V128SubKind::I32x4 => {
                self.asm
                    .vec_rrr(VecALUOp::Sub, lhs, rhs, dst, VectorSize::Size32x4);
            }
            V128SubKind::I64x2 => {
                self.asm
                    .vec_rrr(VecALUOp::Sub, lhs, rhs, dst, VectorSize::Size64x2);
            }
        }
        Ok(())
    }

    fn v128_mul(
        &mut self,
        _context: &mut CodeGenContext<Emission>,
        _kind: V128MulKind,
    ) -> Result<()> {
        Err(format_err!(CodeGenError::unimplemented_masm_instruction()))
    }

    fn v128_abs(&mut self, src: Reg, dst: WritableReg, kind: V128AbsKind) -> Result<()> {
        let (misc_op, size) = match kind {
            V128AbsKind::I8x16 => (VecMisc2::Abs, VectorSize::Size8x16),
            V128AbsKind::I16x8 => (VecMisc2::Abs, VectorSize::Size16x8),
            V128AbsKind::I32x4 => (VecMisc2::Abs, VectorSize::Size32x4),
            V128AbsKind::I64x2 => (VecMisc2::Abs, VectorSize::Size64x2),
            V128AbsKind::F32x4 => (VecMisc2::Fabs, VectorSize::Size32x4),
            V128AbsKind::F64x2 => (VecMisc2::Fabs, VectorSize::Size64x2),
        };
        self.asm.vec_misc(misc_op, src, dst, size);
        Ok(())
    }

    fn v128_neg(&mut self, op: WritableReg, kind: V128NegKind) -> Result<()> {
        let (misc_op, size) = match kind {
            V128NegKind::I8x16 => (VecMisc2::Neg, VectorSize::Size8x16),
            V128NegKind::I16x8 => (VecMisc2::Neg, VectorSize::Size16x8),
            V128NegKind::I32x4 => (VecMisc2::Neg, VectorSize::Size32x4),
            V128NegKind::I64x2 => (VecMisc2::Neg, VectorSize::Size64x2),
            V128NegKind::F32x4 => (VecMisc2::Fneg, VectorSize::Size32x4),
            V128NegKind::F64x2 => (VecMisc2::Fneg, VectorSize::Size64x2),
        };
        self.asm.vec_misc(misc_op, op.to_reg(), op, size);
        Ok(())
    }

    fn v128_shift(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        lane_width: OperandSize,
        shift_kind: ShiftKind,
    ) -> Result<()> {
        let shift_amount = context.pop_to_reg(self, None)?.reg;
        let operand = context.pop_to_reg(self, None)?.reg;
        let amount_mask = lane_width.num_bits() - 1;
        self.and(
            writable!(shift_amount),
            shift_amount,
            RegImm::i32(amount_mask as i32),
            OperandSize::S32,
        )?;

        let size = match lane_width {
            OperandSize::S8 => VectorSize::Size8x16,
            OperandSize::S16 => VectorSize::Size16x8,
            OperandSize::S32 => VectorSize::Size32x4,
            OperandSize::S64 => VectorSize::Size64x2,
            _ => bail!(CodeGenError::unexpected_operand_size()),
        };

        let (op, negate) = match shift_kind {
            ShiftKind::Shl => (VecALUOp::Sshl, false),
            ShiftKind::ShrS => (VecALUOp::Sshl, true),
            ShiftKind::ShrU => (VecALUOp::Ushl, true),
            ShiftKind::Rotl | ShiftKind::Rotr => {
                bail!(CodeGenError::unimplemented_masm_instruction())
            }
        };

        if negate {
            self.asm
                .neg_rr(shift_amount, writable!(shift_amount), OperandSize::S64);
        }

        self.with_scratch::<FloatScratch, _>(|masm, tmp| {
            masm.asm.vec_dup(shift_amount, tmp.writable(), size);
            masm.asm
                .vec_rrr(op, operand, tmp.inner(), writable!(operand), size);
        });

        context.free_reg(shift_amount);
        context.stack.push(TypedReg::v128(operand).into());
        Ok(())
    }

    fn v128_q15mulr_sat_s(
        &mut self,
        _lhs: Reg,
        _rhs: Reg,
        _dst: WritableReg,
        _size: OperandSize,
    ) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn v128_all_true(&mut self, src: Reg, dst: WritableReg, size: OperandSize) -> Result<()> {
        match size {
            OperandSize::S8 | OperandSize::S16 | OperandSize::S32 => {
                self.with_scratch::<FloatScratch, _>(|masm, tmp| {
                    masm.asm.vec_lanes(
                        VecLanesOp::Uminv,
                        src,
                        tmp.writable(),
                        VectorSize::from_lane_size(size.into(), true),
                    );
                    masm.asm.mov_from_vec(tmp.inner(), dst, 0, OperandSize::S64);
                });
                self.asm.subs_ir(
                    Imm12::maybe_from_u64(0).unwrap(),
                    dst.to_reg(),
                    OperandSize::S64,
                );
                self.asm.cset(dst, Cond::Ne);
            }
            OperandSize::S64 => {
                self.with_scratch::<FloatScratch, _>(|masm, tmp| {
                    masm.asm
                        .vec_misc(VecMisc2::Cmeq0, src, tmp.writable(), VectorSize::Size64x2);
                    masm.asm.vec_rrr(
                        VecALUOp::Addp,
                        tmp.inner(),
                        tmp.inner(),
                        tmp.writable(),
                        VectorSize::Size64x2,
                    );
                    masm.asm.fcmp(tmp.inner(), tmp.inner(), OperandSize::S64);
                });
                self.asm.cset(dst, Cond::Eq);
            }
            OperandSize::S128 => bail!(CodeGenError::unexpected_operand_size()),
        }
        Ok(())
    }

    fn v128_bitmask(&mut self, _src: Reg, _dst: WritableReg, _size: OperandSize) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn v128_trunc(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: V128TruncKind,
    ) -> Result<()> {
        let reg = writable!(context.pop_to_reg(self, None)?.reg);
        match kind {
            V128TruncKind::F32x4 | V128TruncKind::F64x2 => {
                self.asm.vec_misc(
                    VecMisc2::Frintz,
                    reg.to_reg(),
                    reg,
                    VectorSize::from_lane_size(kind.dst_lane_size().into(), true),
                );
            }
            V128TruncKind::I32x4FromF32x4S => {
                self.asm
                    .vec_misc(VecMisc2::Fcvtzs, reg.to_reg(), reg, VectorSize::Size32x4);
            }
            V128TruncKind::I32x4FromF32x4U => {
                self.asm
                    .vec_misc(VecMisc2::Fcvtzu, reg.to_reg(), reg, VectorSize::Size32x4);
            }
            V128TruncKind::I32x4FromF64x2SZero => {
                self.asm
                    .vec_misc(VecMisc2::Fcvtzs, reg.to_reg(), reg, VectorSize::Size64x2);
                self.asm.vec_narrow(
                    VecRRNarrowOp::Sqxtn,
                    reg.to_reg(),
                    reg,
                    false,
                    ScalarSize::Size32,
                );
            }
            V128TruncKind::I32x4FromF64x2UZero => {
                self.asm
                    .vec_misc(VecMisc2::Fcvtzu, reg.to_reg(), reg, VectorSize::Size64x2);
                self.asm.vec_narrow(
                    VecRRNarrowOp::Uqxtn,
                    reg.to_reg(),
                    reg,
                    false,
                    ScalarSize::Size32,
                );
            }
        }
        context.stack.push(TypedReg::v128(reg.to_reg()).into());
        Ok(())
    }

    fn v128_min(
        &mut self,
        src1: Reg,
        src2: Reg,
        dst: WritableReg,
        kind: V128MinKind,
    ) -> Result<()> {
        let (op, size) = match kind {
            V128MinKind::F32x4 | V128MinKind::F64x2 => {
                bail!(CodeGenError::unimplemented_masm_instruction())
            }
            V128MinKind::I8x16S => (VecALUOp::Smin, VectorSize::Size8x16),
            V128MinKind::I8x16U => (VecALUOp::Umin, VectorSize::Size8x16),
            V128MinKind::I16x8S => (VecALUOp::Smin, VectorSize::Size16x8),
            V128MinKind::I16x8U => (VecALUOp::Umin, VectorSize::Size16x8),
            V128MinKind::I32x4S => (VecALUOp::Smin, VectorSize::Size32x4),
            V128MinKind::I32x4U => (VecALUOp::Umin, VectorSize::Size32x4),
        };
        self.asm.vec_rrr(op, src1, src2, dst, size);
        Ok(())
    }

    fn v128_max(
        &mut self,
        src1: Reg,
        src2: Reg,
        dst: WritableReg,
        kind: V128MaxKind,
    ) -> Result<()> {
        let (op, size) = match kind {
            V128MaxKind::F32x4 | V128MaxKind::F64x2 => {
                bail!(CodeGenError::unimplemented_masm_instruction())
            }
            V128MaxKind::I8x16S => (VecALUOp::Smax, VectorSize::Size8x16),
            V128MaxKind::I8x16U => (VecALUOp::Umax, VectorSize::Size8x16),
            V128MaxKind::I16x8S => (VecALUOp::Smax, VectorSize::Size16x8),
            V128MaxKind::I16x8U => (VecALUOp::Umax, VectorSize::Size16x8),
            V128MaxKind::I32x4S => (VecALUOp::Smax, VectorSize::Size32x4),
            V128MaxKind::I32x4U => (VecALUOp::Umax, VectorSize::Size32x4),
        };
        self.asm.vec_rrr(op, src1, src2, dst, size);
        Ok(())
    }

    fn v128_extmul(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: V128ExtMulKind,
    ) -> Result<()> {
        let (op, high_half) = match kind {
            V128ExtMulKind::LowI8x16S => (VecRRRLongOp::Smull8, false),
            V128ExtMulKind::HighI8x16S => (VecRRRLongOp::Smull8, true),
            V128ExtMulKind::LowI8x16U => (VecRRRLongOp::Umull8, false),
            V128ExtMulKind::HighI8x16U => (VecRRRLongOp::Umull8, true),
            V128ExtMulKind::LowI16x8S => (VecRRRLongOp::Smull16, false),
            V128ExtMulKind::HighI16x8S => (VecRRRLongOp::Smull16, true),
            V128ExtMulKind::LowI16x8U => (VecRRRLongOp::Umull16, false),
            V128ExtMulKind::HighI16x8U => (VecRRRLongOp::Umull16, true),
            V128ExtMulKind::LowI32x4S => (VecRRRLongOp::Smull32, false),
            V128ExtMulKind::HighI32x4S => (VecRRRLongOp::Smull32, true),
            V128ExtMulKind::LowI32x4U => (VecRRRLongOp::Umull32, false),
            V128ExtMulKind::HighI32x4U => (VecRRRLongOp::Umull32, true),
        };
        let rhs = context.pop_to_reg(self, None)?;
        let lhs = context.pop_to_reg(self, None)?;
        self.asm
            .vec_rrr_long(op, lhs.reg, rhs.reg, writable!(lhs.reg), high_half);
        context.free_reg(rhs);
        context.stack.push(TypedReg::v128(lhs.reg).into());
        Ok(())
    }

    fn v128_extadd_pairwise(
        &mut self,
        src: Reg,
        dst: WritableReg,
        kind: V128ExtAddKind,
    ) -> Result<()> {
        let op = match kind {
            V128ExtAddKind::I8x16S => VecRRPairLongOp::Saddlp8,
            V128ExtAddKind::I8x16U => VecRRPairLongOp::Uaddlp8,
            V128ExtAddKind::I16x8S => VecRRPairLongOp::Saddlp16,
            V128ExtAddKind::I16x8U => VecRRPairLongOp::Uaddlp16,
        };
        self.asm.vec_rr_pair_long(op, src, dst);
        Ok(())
    }

    fn v128_dot(&mut self, _lhs: Reg, _rhs: Reg, _dst: WritableReg) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn v128_popcnt(&mut self, context: &mut CodeGenContext<Emission>) -> Result<()> {
        let reg = writable!(context.pop_to_reg(self, None)?.reg);
        self.asm
            .vec_misc(VecMisc2::Cnt, reg.to_reg(), reg, VectorSize::Size8x16);
        context.stack.push(TypedReg::v128(reg.to_reg()).into());
        Ok(())
    }

    fn v128_avgr(&mut self, lhs: Reg, rhs: Reg, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.vec_rrr(
            VecALUOp::Urhadd,
            lhs,
            rhs,
            dst,
            VectorSize::from_lane_size(size.into(), true),
        );
        Ok(())
    }

    fn v128_div(
        &mut self,
        _lhs: Reg,
        _rhs: Reg,
        _dst: WritableReg,
        _size: OperandSize,
    ) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn v128_sqrt(&mut self, src: Reg, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.vec_misc(
            VecMisc2::Fsqrt,
            src,
            dst,
            VectorSize::from_lane_size(size.into(), true),
        );
        Ok(())
    }

    fn v128_ceil(&mut self, src: Reg, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.vec_misc(
            VecMisc2::Frintp,
            src,
            dst,
            VectorSize::from_lane_size(size.into(), true),
        );
        Ok(())
    }

    fn v128_floor(&mut self, src: Reg, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.vec_misc(
            VecMisc2::Frintm,
            src,
            dst,
            VectorSize::from_lane_size(size.into(), true),
        );
        Ok(())
    }

    fn v128_nearest(&mut self, src: Reg, dst: WritableReg, size: OperandSize) -> Result<()> {
        self.asm.vec_misc(
            VecMisc2::Frintn,
            src,
            dst,
            VectorSize::from_lane_size(size.into(), true),
        );
        Ok(())
    }

    fn v128_pmin(
        &mut self,
        _lhs: Reg,
        _rhs: Reg,
        _dst: WritableReg,
        _size: OperandSize,
    ) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }

    fn v128_pmax(
        &mut self,
        _lhs: Reg,
        _rhs: Reg,
        _dst: WritableReg,
        _size: OperandSize,
    ) -> Result<()> {
        bail!(CodeGenError::unimplemented_masm_instruction())
    }
}

impl MacroAssembler {
    fn increment_sp(&mut self, bytes: u32) {
        self.sp_offset += bytes;

        // NOTE: we use `max` here to track the largest stack allocation in `sp_max`. Once we have
        // seen the entire function, this value will represent the maximum size for the stack
        // frame.
        self.sp_max = self.sp_max.max(self.sp_offset);
    }

    fn decrement_sp(&mut self, bytes: u32) {
        self.sp_offset -= bytes;
    }

    // Copies the value of the stack pointer to the shadow stack
    // pointer: mov x28, sp

    // This function is called at the epilogue.
    fn move_sp_to_shadow_sp(&mut self) {
        let sp = regs::sp();
        let shadow_sp = regs::shadow_sp();
        self.asm.mov_rr(sp, writable!(shadow_sp), OperandSize::S64);
    }

    /// Helper to add an immediate to a register.
    fn add_ir(&mut self, dst: WritableReg, lhs: Reg, rhs: I, size: OperandSize) -> Result<()> {
        let imm = rhs.unwrap_as_u64();
        match Imm12::maybe_from_u64(imm) {
            Some(imm12) => self.asm.add_ir(imm12, lhs, dst, size),
            None => {
                self.with_scratch::<IntScratch, _>(|masm, scratch| {
                    masm.asm.mov_ir(scratch.writable(), rhs, rhs.size());
                    masm.asm.add_rrr(scratch.inner(), lhs, dst, size);
                });
            }
        };
        Ok(())
    }

    // Copies the value of the shadow stack pointer to the stack pointer: mov
    // sp, x28.
    //
    // This function is usually called when the space is claimed, e.g., via
    // a push, when stack space is reserved explicitly or after emitting code
    // that requires explicit stack pointer alignment (code that could result in
    // signal handling).
    //
    // This ensures the stack pointer always reflects the allocated stack space,
    // otherwise any space below the stack pointer could get clobbered with
    // interrupts and signal handlers.
    //
    // This function must also be called at the function epilogue, since the
    // stack pointer is used to restore the current function frame.
    fn move_shadow_sp_to_sp(&mut self) {
        let shadow_sp = regs::shadow_sp();
        let sp = writable!(regs::sp());
        let imm = Imm12::maybe_from_u64(0).unwrap();
        self.asm.add_ir(imm, shadow_sp, sp, OperandSize::S64);
    }
}

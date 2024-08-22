//! ISLE integration glue code for s390x lowering.

// Pull in the ISLE generated code.
pub mod generated_code;

use crate::ir::ExternalName;
// Types that the generated ISLE code uses via `use super::*`.
use crate::isa::s390x::abi::{S390xMachineDeps, REG_SAVE_AREA_SIZE};
use crate::isa::s390x::inst::{
    gpr, stack_reg, writable_gpr, zero_reg, CallIndInfo, CallInfo, Cond, Inst as MInst, LaneOrder,
    MemArg, MemArgPair, RegPair, ReturnCallIndInfo, ReturnCallInfo, SymbolReloc, UImm12,
    UImm16Shifted, UImm32Shifted, WritableRegPair,
};
use crate::isa::s390x::S390xBackend;
use crate::machinst::isle::*;
use crate::machinst::{MachLabel, Reg};
use crate::{
    ir::{
        condcodes::*, immediates::*, types::*, ArgumentExtension, ArgumentPurpose, AtomicRmwOp,
        BlockCall, Endianness, Inst, InstructionData, KnownSymbol, LibCall, MemFlags, Opcode,
        TrapCode, Value, ValueList,
    },
    isa::CallConv,
    machinst::abi::ABIMachineSpec,
    machinst::{
        ArgPair, CallArgList, CallArgPair, CallRetList, CallRetPair, InstOutput, MachInst,
        VCodeConstant, VCodeConstantData,
    },
};
use regalloc2::PReg;
use smallvec::smallvec;
use std::boxed::Box;
use std::cell::Cell;
use std::vec::Vec;

/// Information describing a library call to be emitted.
pub struct LibCallInfo {
    libcall: LibCall,
    uses: CallArgList,
    defs: CallRetList,
    tls_symbol: Option<SymbolReloc>,
}

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;
type BoxReturnCallInfo = Box<ReturnCallInfo>;
type BoxReturnCallIndInfo = Box<ReturnCallIndInfo>;
type VecMachLabel = Vec<MachLabel>;
type BoxExternalName = Box<ExternalName>;
type BoxSymbolReloc = Box<SymbolReloc>;
type VecMInst = Vec<MInst>;
type VecMInstBuilder = Cell<Vec<MInst>>;
type VecArgPair = Vec<ArgPair>;
type CallArgListBuilder = Cell<CallArgList>;

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &S390xBackend,
    inst: Inst,
) -> Option<InstOutput> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = IsleContext { lower_ctx, backend };
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

/// The main entry point for branch lowering with ISLE.
pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &S390xBackend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = IsleContext { lower_ctx, backend };
    generated_code::constructor_lower_branch(&mut isle_ctx, branch, targets)
}

impl generated_code::Context for IsleContext<'_, '_, MInst, S390xBackend> {
    isle_lower_prelude_methods!();

    fn gen_return_call(
        &mut self,
        callee_sig: SigRef,
        callee: ExternalName,
        distance: RelocDistance,
        args: ValueSlice,
    ) -> InstOutput {
        let _ = (callee_sig, callee, distance, args);
        todo!()
    }

    fn gen_return_call_indirect(
        &mut self,
        callee_sig: SigRef,
        callee: Value,
        args: ValueSlice,
    ) -> InstOutput {
        let _ = (callee_sig, callee, args);
        todo!()
    }

    #[inline]
    fn args_builder_new(&mut self) -> CallArgListBuilder {
        Cell::new(CallArgList::new())
    }

    #[inline]
    fn args_builder_push(
        &mut self,
        builder: &CallArgListBuilder,
        vreg: Reg,
        preg: RealReg,
    ) -> Unit {
        let mut args = builder.take();
        args.push(CallArgPair {
            vreg,
            preg: preg.into(),
        });
        builder.set(args);
    }

    #[inline]
    fn args_builder_finish(&mut self, builder: &CallArgListBuilder) -> CallArgList {
        builder.take()
    }

    fn defs_init(&mut self, abi: Sig) -> CallRetList {
        // Allocate writable registers for all retval regs, except for StructRet args.
        let mut defs = smallvec![];
        for i in 0..self.lower_ctx.sigs().num_rets(abi) {
            if let &ABIArg::Slots {
                ref slots, purpose, ..
            } = &self.lower_ctx.sigs().get_ret(abi, i)
            {
                if purpose == ArgumentPurpose::StructReturn {
                    continue;
                }
                for slot in slots {
                    match slot {
                        &ABIArgSlot::Reg { reg, ty, .. } => {
                            let value_regs = self.lower_ctx.alloc_tmp(ty);
                            defs.push(CallRetPair {
                                vreg: value_regs.only_reg().unwrap(),
                                preg: reg.into(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        defs
    }

    fn defs_lookup(&mut self, defs: &CallRetList, reg: RealReg) -> Reg {
        let reg = Reg::from(reg);
        for def in defs {
            if def.preg == reg {
                return def.vreg.to_reg();
            }
        }
        unreachable!()
    }

    fn abi_sig(&mut self, sig_ref: SigRef) -> Sig {
        self.lower_ctx.sigs().abi_sig_for_sig_ref(sig_ref)
    }

    fn abi_first_ret(&mut self, sig_ref: SigRef, abi: Sig) -> usize {
        // Return the index of the first actual return value, excluding
        // any StructReturn that might have been added to Sig.
        let sig = &self.lower_ctx.dfg().signatures[sig_ref];
        self.lower_ctx.sigs().num_rets(abi) - sig.returns.len()
    }

    fn abi_lane_order(&mut self, abi: Sig) -> LaneOrder {
        lane_order_for_call_conv(self.lower_ctx.sigs()[abi].call_conv())
    }

    fn abi_call_stack_args(&mut self, abi: Sig) -> MemArg {
        let sig_data = &self.lower_ctx.sigs()[abi];
        if sig_data.call_conv() != CallConv::Tail {
            // System ABI: outgoing arguments are at the bottom of the
            // caller's frame (register save area included in offsets).
            let arg_space = sig_data.sized_stack_arg_space() as u32;
            self.lower_ctx
                .abi_mut()
                .accumulate_outgoing_args_size(arg_space);
            MemArg::reg_plus_off(stack_reg(), 0, MemFlags::trusted())
        } else {
            // Tail-call ABI: outgoing arguments are at the top of the
            // callee's frame; so we need to allocate that bit of this
            // frame here, including a backchain copy if needed.
            let arg_space = sig_data.sized_stack_arg_space() as u32;
            if arg_space > 0 {
                if self.backend.flags.preserve_frame_pointers() {
                    let tmp = self.lower_ctx.alloc_tmp(I64).only_reg().unwrap();
                    let src_mem = MemArg::reg(stack_reg(), MemFlags::trusted());
                    let dst_mem = MemArg::reg(stack_reg(), MemFlags::trusted());
                    self.emit(&MInst::Load64 {
                        rd: tmp,
                        mem: src_mem,
                    });
                    self.emit(&MInst::AllocateArgs { size: arg_space });
                    self.emit(&MInst::Store64 {
                        rd: tmp.to_reg(),
                        mem: dst_mem,
                    });
                } else {
                    self.emit(&MInst::AllocateArgs { size: arg_space });
                }
            }
            MemArg::reg_plus_off(stack_reg(), arg_space.into(), MemFlags::trusted())
        }
    }

    fn abi_call_stack_rets(&mut self, abi: Sig) -> MemArg {
        let sig_data = &self.lower_ctx.sigs()[abi];
        if sig_data.call_conv() != CallConv::Tail {
            // System ABI: buffer for outgoing return values is just above
            // the outgoing arguments.
            let arg_space = sig_data.sized_stack_arg_space() as u32;
            let ret_space = sig_data.sized_stack_ret_space() as u32;
            self.lower_ctx
                .abi_mut()
                .accumulate_outgoing_args_size(arg_space + ret_space);
            MemArg::reg_plus_off(stack_reg(), arg_space.into(), MemFlags::trusted())
        } else {
            // Tail-call ABI: buffer for outgoing return values is at the
            // bottom of the caller's frame (above the register save area).
            let ret_space = sig_data.sized_stack_ret_space() as u32;
            self.lower_ctx
                .abi_mut()
                .accumulate_outgoing_args_size(REG_SAVE_AREA_SIZE + ret_space);
            MemArg::NominalSPOffset {
                off: REG_SAVE_AREA_SIZE as i64,
            }
        }
    }

    fn abi_return_call_stack_args(&mut self, abi: Sig) -> MemArg {
        // Tail calls: outgoing arguments at the top of the caller's frame.
        // Create a backchain copy if needed to ensure correct stack unwinding
        // during the tail-call sequence.
        let sig_data = &self.lower_ctx.sigs()[abi];
        let arg_space = sig_data.sized_stack_arg_space() as u32;
        self.lower_ctx
            .abi_mut()
            .accumulate_tail_args_size(arg_space);
        if arg_space > 0 {
            let tmp = self.lower_ctx.alloc_tmp(I64).only_reg().unwrap();
            let src_mem = MemArg::InitialSPOffset { off: 0 };
            let dst_mem = MemArg::InitialSPOffset {
                off: -(arg_space as i64),
            };
            self.emit(&MInst::Load64 {
                rd: tmp,
                mem: src_mem,
            });
            self.emit(&MInst::Store64 {
                rd: tmp.to_reg(),
                mem: dst_mem,
            });
        }
        MemArg::InitialSPOffset { off: 0 }
    }

    fn abi_call_info(
        &mut self,
        abi: Sig,
        name: ExternalName,
        uses: &CallArgList,
        defs: &CallRetList,
    ) -> BoxCallInfo {
        let sig_data = &self.lower_ctx.sigs()[abi];
        // Get clobbers: all caller-saves. These may include return value
        // regs, which we will remove from the clobber set later.
        let clobbers = S390xMachineDeps::get_regs_clobbered_by_call(sig_data.call_conv());
        let callee_pop_size = if sig_data.call_conv() == CallConv::Tail {
            sig_data.sized_stack_arg_space() as u32
        } else {
            0
        };
        Box::new(CallInfo {
            dest: name.clone(),
            uses: uses.clone(),
            defs: defs.clone(),
            clobbers,
            callee_pop_size,
            caller_callconv: self.lower_ctx.abi().call_conv(self.lower_ctx.sigs()),
            callee_callconv: self.lower_ctx.sigs()[abi].call_conv(),
            tls_symbol: None,
        })
    }

    fn abi_call_ind_info(
        &mut self,
        abi: Sig,
        target: Reg,
        uses: &CallArgList,
        defs: &CallRetList,
    ) -> BoxCallIndInfo {
        let sig_data = &self.lower_ctx.sigs()[abi];
        // Get clobbers: all caller-saves. These may include return value
        // regs, which we will remove from the clobber set later.
        let clobbers = S390xMachineDeps::get_regs_clobbered_by_call(sig_data.call_conv());
        let callee_pop_size = if sig_data.call_conv() == CallConv::Tail {
            sig_data.sized_stack_arg_space() as u32
        } else {
            0
        };
        Box::new(CallIndInfo {
            rn: target,
            uses: uses.clone(),
            defs: defs.clone(),
            clobbers,
            callee_pop_size,
            caller_callconv: self.lower_ctx.abi().call_conv(self.lower_ctx.sigs()),
            callee_callconv: self.lower_ctx.sigs()[abi].call_conv(),
        })
    }

    fn abi_return_call_info(
        &mut self,
        abi: Sig,
        name: ExternalName,
        uses: &CallArgList,
    ) -> BoxReturnCallInfo {
        let sig_data = &self.lower_ctx.sigs()[abi];
        let callee_pop_size = sig_data.sized_stack_arg_space() as u32;
        Box::new(ReturnCallInfo {
            dest: name.clone(),
            uses: uses.clone(),
            callee_pop_size,
        })
    }

    fn abi_return_call_ind_info(
        &mut self,
        abi: Sig,
        target: Reg,
        uses: &CallArgList,
    ) -> BoxReturnCallIndInfo {
        let sig_data = &self.lower_ctx.sigs()[abi];
        let callee_pop_size = sig_data.sized_stack_arg_space() as u32;
        Box::new(ReturnCallIndInfo {
            rn: target,
            uses: uses.clone(),
            callee_pop_size,
        })
    }

    fn lib_call_info_memcpy(&mut self, dst: Reg, src: Reg, len: Reg) -> LibCallInfo {
        LibCallInfo {
            libcall: LibCall::Memcpy,
            uses: smallvec![
                CallArgPair {
                    vreg: dst,
                    preg: gpr(2),
                },
                CallArgPair {
                    vreg: src,
                    preg: gpr(3),
                },
                CallArgPair {
                    vreg: len,
                    preg: gpr(4),
                },
            ],
            defs: smallvec![],
            tls_symbol: None,
        }
    }

    fn lib_call_info_tls_get_offset(
        &mut self,
        tls_offset: WritableReg,
        got: Reg,
        got_offset: Reg,
        tls_symbol: &SymbolReloc,
    ) -> LibCallInfo {
        LibCallInfo {
            libcall: LibCall::ElfTlsGetOffset,
            uses: smallvec![
                CallArgPair {
                    vreg: got,
                    preg: gpr(12),
                },
                CallArgPair {
                    vreg: got_offset,
                    preg: gpr(2),
                },
            ],
            defs: smallvec![CallRetPair {
                vreg: tls_offset,
                preg: gpr(2),
            },],
            tls_symbol: Some(tls_symbol.clone()),
        }
    }

    fn lib_call_info(&mut self, info: &LibCallInfo) -> BoxCallInfo {
        let caller_callconv = self.lower_ctx.abi().call_conv(self.lower_ctx.sigs());
        let callee_callconv = CallConv::for_libcall(&self.backend.flags, caller_callconv);

        // Clobbers are defined by the calling convention. We will remove return value regs later.
        let clobbers = S390xMachineDeps::get_regs_clobbered_by_call(callee_callconv);

        // Libcalls only require the register save area.
        self.lower_ctx
            .abi_mut()
            .accumulate_outgoing_args_size(REG_SAVE_AREA_SIZE);

        Box::new(CallInfo {
            dest: ExternalName::LibCall(info.libcall),
            uses: info.uses.clone(),
            defs: info.defs.clone(),
            clobbers,
            callee_pop_size: 0,
            caller_callconv,
            callee_callconv,
            tls_symbol: info.tls_symbol.clone(),
        })
    }

    #[inline]
    fn box_symbol_reloc(&mut self, symbol_reloc: &SymbolReloc) -> BoxSymbolReloc {
        Box::new(symbol_reloc.clone())
    }

    #[inline]
    fn mie2_enabled(&mut self, _: Type) -> Option<()> {
        if self.backend.isa_flags.has_mie2() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn mie2_disabled(&mut self, _: Type) -> Option<()> {
        if !self.backend.isa_flags.has_mie2() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn vxrs_ext2_enabled(&mut self, _: Type) -> Option<()> {
        if self.backend.isa_flags.has_vxrs_ext2() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn vxrs_ext2_disabled(&mut self, _: Type) -> Option<()> {
        if !self.backend.isa_flags.has_vxrs_ext2() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn writable_gpr(&mut self, regno: u8) -> WritableReg {
        writable_gpr(regno)
    }

    #[inline]
    fn zero_reg(&mut self) -> Reg {
        zero_reg()
    }

    #[inline]
    fn gpr32_ty(&mut self, ty: Type) -> Option<Type> {
        match ty {
            I8 | I16 | I32 => Some(ty),
            _ => None,
        }
    }

    #[inline]
    fn gpr64_ty(&mut self, ty: Type) -> Option<Type> {
        match ty {
            I64 => Some(ty),
            _ => None,
        }
    }

    #[inline]
    fn vr128_ty(&mut self, ty: Type) -> Option<Type> {
        match ty {
            I128 => Some(ty),
            _ if ty.is_vector() && ty.bits() == 128 => Some(ty),
            _ => None,
        }
    }

    #[inline]
    fn uimm32shifted(&mut self, n: u32, shift: u8) -> UImm32Shifted {
        UImm32Shifted::maybe_with_shift(n, shift).unwrap()
    }

    #[inline]
    fn uimm16shifted(&mut self, n: u16, shift: u8) -> UImm16Shifted {
        UImm16Shifted::maybe_with_shift(n, shift).unwrap()
    }

    #[inline]
    fn i64_nonequal(&mut self, val: i64, cmp: i64) -> Option<i64> {
        if val != cmp {
            Some(val)
        } else {
            None
        }
    }

    #[inline]
    fn u64_pair_split(&mut self, n: u128) -> (u64, u64) {
        ((n >> 64) as u64, n as u64)
    }

    #[inline]
    fn u64_pair_concat(&mut self, hi: u64, lo: u64) -> u128 {
        (hi as u128) << 64 | (lo as u128)
    }

    #[inline]
    fn u32_pair_split(&mut self, n: u64) -> (u32, u32) {
        ((n >> 32) as u32, n as u32)
    }

    #[inline]
    fn u32_pair_concat(&mut self, hi: u32, lo: u32) -> u64 {
        (hi as u64) << 32 | (lo as u64)
    }

    #[inline]
    fn u16_pair_split(&mut self, n: u32) -> (u16, u16) {
        ((n >> 16) as u16, n as u16)
    }

    #[inline]
    fn u16_pair_concat(&mut self, hi: u16, lo: u16) -> u32 {
        (hi as u32) << 16 | (lo as u32)
    }

    #[inline]
    fn u8_pair_split(&mut self, n: u16) -> (u8, u8) {
        ((n >> 8) as u8, n as u8)
    }

    #[inline]
    fn u8_pair_concat(&mut self, hi: u8, lo: u8) -> u16 {
        (hi as u16) << 8 | (lo as u16)
    }

    #[inline]
    fn u8_as_u16(&mut self, n: u8) -> u16 {
        n as u16
    }

    #[inline]
    fn u64_truncate_to_u32(&mut self, n: u64) -> u32 {
        n as u32
    }

    #[inline]
    fn u64_as_i16(&mut self, n: u64) -> i16 {
        n as i16
    }

    #[inline]
    fn u64_nonzero_hipart(&mut self, n: u64) -> Option<u64> {
        let part = n & 0xffff_ffff_0000_0000;
        if part != 0 {
            Some(part)
        } else {
            None
        }
    }

    #[inline]
    fn u64_nonzero_lopart(&mut self, n: u64) -> Option<u64> {
        let part = n & 0x0000_0000_ffff_ffff;
        if part != 0 {
            Some(part)
        } else {
            None
        }
    }

    #[inline]
    fn i32_from_u64(&mut self, n: u64) -> Option<i32> {
        if let Ok(imm) = i32::try_from(n as i64) {
            Some(imm)
        } else {
            None
        }
    }

    #[inline]
    fn i16_from_u64(&mut self, n: u64) -> Option<i16> {
        if let Ok(imm) = i16::try_from(n as i64) {
            Some(imm)
        } else {
            None
        }
    }

    #[inline]
    fn i16_from_u32(&mut self, n: u32) -> Option<i16> {
        if let Ok(imm) = i16::try_from(n as i32) {
            Some(imm)
        } else {
            None
        }
    }

    #[inline]
    fn uimm32shifted_from_u64(&mut self, n: u64) -> Option<UImm32Shifted> {
        UImm32Shifted::maybe_from_u64(n)
    }

    #[inline]
    fn uimm16shifted_from_u64(&mut self, n: u64) -> Option<UImm16Shifted> {
        UImm16Shifted::maybe_from_u64(n)
    }

    #[inline]
    fn lane_order(&mut self) -> LaneOrder {
        lane_order_for_call_conv(self.lower_ctx.abi().call_conv(self.lower_ctx.sigs()))
    }

    #[inline]
    fn be_lane_idx(&mut self, ty: Type, idx: u8) -> u8 {
        match self.lane_order() {
            LaneOrder::LittleEndian => ty.lane_count() as u8 - 1 - idx,
            LaneOrder::BigEndian => idx,
        }
    }

    #[inline]
    fn be_vec_const(&mut self, ty: Type, n: u128) -> u128 {
        match self.lane_order() {
            LaneOrder::LittleEndian => n,
            LaneOrder::BigEndian => {
                let lane_count = ty.lane_count();
                let lane_bits = ty.lane_bits();
                let lane_mask = (1u128 << lane_bits) - 1;
                let mut n_le = n;
                let mut n_be = 0u128;
                for _ in 0..lane_count {
                    n_be = (n_be << lane_bits) | (n_le & lane_mask);
                    n_le = n_le >> lane_bits;
                }
                n_be
            }
        }
    }

    #[inline]
    fn lane_byte_mask(&mut self, ty: Type, idx: u8) -> u16 {
        let lane_bytes = (ty.lane_bits() / 8) as u8;
        let lane_mask = (1u16 << lane_bytes) - 1;
        lane_mask << (16 - ((idx + 1) * lane_bytes))
    }

    #[inline]
    fn shuffle_mask_from_u128(&mut self, idx: u128) -> (u128, u16) {
        let bytes = match self.lane_order() {
            LaneOrder::LittleEndian => idx.to_be_bytes().map(|x| {
                if x < 16 {
                    15 - x
                } else if x < 32 {
                    47 - x
                } else {
                    128
                }
            }),
            LaneOrder::BigEndian => idx.to_le_bytes().map(|x| if x < 32 { x } else { 128 }),
        };
        let and_mask = bytes.iter().fold(0, |acc, &x| (acc << 1) | (x < 32) as u16);
        let permute_mask = u128::from_be_bytes(bytes);
        (permute_mask, and_mask)
    }

    #[inline]
    fn u64_from_value(&mut self, val: Value) -> Option<u64> {
        let inst = self.lower_ctx.dfg().value_def(val).inst()?;
        let constant = self.lower_ctx.get_constant(inst)?;
        let ty = self.lower_ctx.output_ty(inst, 0);
        Some(zero_extend_to_u64(constant, self.ty_bits(ty)))
    }

    #[inline]
    fn u64_from_inverted_value(&mut self, val: Value) -> Option<u64> {
        let inst = self.lower_ctx.dfg().value_def(val).inst()?;
        let constant = self.lower_ctx.get_constant(inst)?;
        let ty = self.lower_ctx.output_ty(inst, 0);
        Some(zero_extend_to_u64(!constant, self.ty_bits(ty)))
    }

    #[inline]
    fn u32_from_value(&mut self, val: Value) -> Option<u32> {
        let constant = self.u64_from_value(val)?;
        let imm = u32::try_from(constant).ok()?;
        Some(imm)
    }

    #[inline]
    fn u8_from_value(&mut self, val: Value) -> Option<u8> {
        let constant = self.u64_from_value(val)?;
        let imm = u8::try_from(constant).ok()?;
        Some(imm)
    }

    #[inline]
    fn u64_from_signed_value(&mut self, val: Value) -> Option<u64> {
        let inst = self.lower_ctx.dfg().value_def(val).inst()?;
        let constant = self.lower_ctx.get_constant(inst)?;
        let ty = self.lower_ctx.output_ty(inst, 0);
        Some(sign_extend_to_u64(constant, self.ty_bits(ty)))
    }

    #[inline]
    fn i64_from_value(&mut self, val: Value) -> Option<i64> {
        let constant = self.u64_from_signed_value(val)? as i64;
        Some(constant)
    }

    #[inline]
    fn i32_from_value(&mut self, val: Value) -> Option<i32> {
        let constant = self.u64_from_signed_value(val)? as i64;
        let imm = i32::try_from(constant).ok()?;
        Some(imm)
    }

    #[inline]
    fn i16_from_value(&mut self, val: Value) -> Option<i16> {
        let constant = self.u64_from_signed_value(val)? as i64;
        let imm = i16::try_from(constant).ok()?;
        Some(imm)
    }

    #[inline]
    fn i16_from_swapped_value(&mut self, val: Value) -> Option<i16> {
        let constant = self.u64_from_signed_value(val)? as i64;
        let imm = i16::try_from(constant).ok()?;
        Some(imm.swap_bytes())
    }

    #[inline]
    fn i64_from_negated_value(&mut self, val: Value) -> Option<i64> {
        let constant = self.u64_from_signed_value(val)? as i64;
        let imm = constant.wrapping_neg();
        Some(imm)
    }

    #[inline]
    fn i32_from_negated_value(&mut self, val: Value) -> Option<i32> {
        let constant = self.u64_from_signed_value(val)? as i64;
        let imm = i32::try_from(constant.wrapping_neg()).ok()?;
        Some(imm)
    }

    #[inline]
    fn i16_from_negated_value(&mut self, val: Value) -> Option<i16> {
        let constant = self.u64_from_signed_value(val)? as i64;
        let imm = i16::try_from(constant.wrapping_neg()).ok()?;
        Some(imm)
    }

    #[inline]
    fn uimm16shifted_from_value(&mut self, val: Value) -> Option<UImm16Shifted> {
        let constant = self.u64_from_value(val)?;
        UImm16Shifted::maybe_from_u64(constant)
    }

    #[inline]
    fn uimm32shifted_from_value(&mut self, val: Value) -> Option<UImm32Shifted> {
        let constant = self.u64_from_value(val)?;
        UImm32Shifted::maybe_from_u64(constant)
    }

    #[inline]
    fn uimm16shifted_from_inverted_value(&mut self, val: Value) -> Option<UImm16Shifted> {
        let constant = self.u64_from_inverted_value(val)?;
        let imm = UImm16Shifted::maybe_from_u64(constant)?;
        Some(imm.negate_bits())
    }

    #[inline]
    fn uimm32shifted_from_inverted_value(&mut self, val: Value) -> Option<UImm32Shifted> {
        let constant = self.u64_from_inverted_value(val)?;
        let imm = UImm32Shifted::maybe_from_u64(constant)?;
        Some(imm.negate_bits())
    }

    #[inline]
    fn len_minus_one(&mut self, len: u64) -> Option<u8> {
        if len > 0 && len <= 256 {
            Some((len - 1) as u8)
        } else {
            None
        }
    }

    #[inline]
    fn mask_amt_imm(&mut self, ty: Type, amt: i64) -> u8 {
        let mask = ty.lane_bits() - 1;
        (amt as u8) & (mask as u8)
    }

    #[inline]
    fn mask_as_cond(&mut self, mask: u8) -> Cond {
        Cond::from_mask(mask)
    }

    #[inline]
    fn intcc_as_cond(&mut self, cc: &IntCC) -> Cond {
        Cond::from_intcc(*cc)
    }

    #[inline]
    fn floatcc_as_cond(&mut self, cc: &FloatCC) -> Cond {
        Cond::from_floatcc(*cc)
    }

    #[inline]
    fn invert_cond(&mut self, cond: &Cond) -> Cond {
        Cond::invert(*cond)
    }

    #[inline]
    fn signed(&mut self, cc: &IntCC) -> Option<()> {
        if condcode_is_signed(*cc) {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn unsigned(&mut self, cc: &IntCC) -> Option<()> {
        if !condcode_is_signed(*cc) {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn zero_offset(&mut self) -> Offset32 {
        Offset32::new(0)
    }

    #[inline]
    fn i64_from_offset(&mut self, off: Offset32) -> i64 {
        i64::from(off)
    }

    #[inline]
    fn fcvt_to_uint_ub32(&mut self, size: u8) -> u64 {
        (2.0_f32).powi(size.into()).to_bits() as u64
    }

    #[inline]
    fn fcvt_to_uint_lb32(&mut self) -> u64 {
        (-1.0_f32).to_bits() as u64
    }

    #[inline]
    fn fcvt_to_uint_ub64(&mut self, size: u8) -> u64 {
        (2.0_f64).powi(size.into()).to_bits()
    }

    #[inline]
    fn fcvt_to_uint_lb64(&mut self) -> u64 {
        (-1.0_f64).to_bits()
    }

    #[inline]
    fn fcvt_to_sint_ub32(&mut self, size: u8) -> u64 {
        (2.0_f32).powi((size - 1).into()).to_bits() as u64
    }

    #[inline]
    fn fcvt_to_sint_lb32(&mut self, size: u8) -> u64 {
        let lb = (-2.0_f32).powi((size - 1).into());
        std::cmp::max(lb.to_bits() + 1, (lb - 1.0).to_bits()) as u64
    }

    #[inline]
    fn fcvt_to_sint_ub64(&mut self, size: u8) -> u64 {
        (2.0_f64).powi((size - 1).into()).to_bits()
    }

    #[inline]
    fn fcvt_to_sint_lb64(&mut self, size: u8) -> u64 {
        let lb = (-2.0_f64).powi((size - 1).into());
        std::cmp::max(lb.to_bits() + 1, (lb - 1.0).to_bits())
    }

    #[inline]
    fn littleendian(&mut self, flags: MemFlags) -> Option<()> {
        let endianness = flags.endianness(Endianness::Big);
        if endianness == Endianness::Little {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn bigendian(&mut self, flags: MemFlags) -> Option<()> {
        let endianness = flags.endianness(Endianness::Big);
        if endianness == Endianness::Big {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn memflags_trusted(&mut self) -> MemFlags {
        MemFlags::trusted()
    }

    #[inline]
    fn memarg_flags(&mut self, mem: &MemArg) -> MemFlags {
        mem.get_flags()
    }

    #[inline]
    fn memarg_offset(&mut self, base: &MemArg, offset: i64) -> MemArg {
        MemArg::offset(base, offset)
    }

    #[inline]
    fn memarg_reg_plus_reg(&mut self, x: Reg, y: Reg, bias: u8, flags: MemFlags) -> MemArg {
        MemArg::BXD12 {
            base: x,
            index: y,
            disp: UImm12::maybe_from_u64(bias as u64).unwrap(),
            flags,
        }
    }

    #[inline]
    fn memarg_reg_plus_off(&mut self, reg: Reg, off: i64, bias: u8, flags: MemFlags) -> MemArg {
        MemArg::reg_plus_off(reg, off + (bias as i64), flags)
    }

    #[inline]
    fn memarg_symbol(&mut self, name: ExternalName, offset: i32, flags: MemFlags) -> MemArg {
        MemArg::Symbol {
            name: Box::new(name),
            offset,
            flags,
        }
    }

    #[inline]
    fn memarg_got(&mut self) -> MemArg {
        MemArg::Symbol {
            name: Box::new(ExternalName::KnownSymbol(KnownSymbol::ElfGlobalOffsetTable)),
            offset: 0,
            flags: MemFlags::trusted(),
        }
    }

    #[inline]
    fn memarg_symbol_offset_sum(&mut self, off1: i64, off2: i64) -> Option<i32> {
        let off = i32::try_from(off1 + off2).ok()?;
        if off & 1 == 0 {
            Some(off)
        } else {
            None
        }
    }

    #[inline]
    fn memarg_pair_from_memarg(&mut self, mem: &MemArg) -> Option<MemArgPair> {
        MemArgPair::maybe_from_memarg(mem)
    }

    #[inline]
    fn memarg_pair_from_reg(&mut self, reg: Reg, flags: MemFlags) -> MemArgPair {
        MemArgPair {
            base: reg,
            disp: UImm12::zero(),
            flags,
        }
    }

    #[inline]
    fn memarg_frame_pointer_offset(&mut self) -> MemArg {
        // The frame pointer (back chain) is stored directly at SP.
        MemArg::NominalSPOffset { off: 0 }
    }

    #[inline]
    fn memarg_return_address_offset(&mut self) -> MemArg {
        // The return address is stored 14 pointer-sized slots above the initial SP.
        MemArg::InitialSPOffset { off: 14 * 8 }
    }

    #[inline]
    fn inst_builder_new(&mut self) -> VecMInstBuilder {
        Cell::new(Vec::<MInst>::new())
    }

    #[inline]
    fn inst_builder_push(&mut self, builder: &VecMInstBuilder, inst: &MInst) -> Unit {
        let mut vec = builder.take();
        vec.push(inst.clone());
        builder.set(vec);
    }

    #[inline]
    fn inst_builder_finish(&mut self, builder: &VecMInstBuilder) -> Vec<MInst> {
        builder.take()
    }

    #[inline]
    fn real_reg(&mut self, reg: WritableReg) -> Option<WritableReg> {
        if reg.to_reg().is_real() {
            Some(reg)
        } else {
            None
        }
    }

    #[inline]
    fn same_reg(&mut self, dst: WritableReg, src: Reg) -> Option<Reg> {
        if dst.to_reg() == src {
            Some(src)
        } else {
            None
        }
    }

    #[inline]
    fn sinkable_inst(&mut self, val: Value) -> Option<Inst> {
        self.is_sinkable_inst(val)
    }

    #[inline]
    fn emit(&mut self, inst: &MInst) -> Unit {
        self.lower_ctx.emit(inst.clone());
    }

    #[inline]
    fn preg_stack(&mut self) -> PReg {
        stack_reg().to_real_reg().unwrap().into()
    }

    #[inline]
    fn preg_gpr_0(&mut self) -> PReg {
        gpr(0).to_real_reg().unwrap().into()
    }

    #[inline]
    fn writable_regpair(&mut self, hi: WritableReg, lo: WritableReg) -> WritableRegPair {
        WritableRegPair { hi, lo }
    }

    #[inline]
    fn writable_regpair_hi(&mut self, w: WritableRegPair) -> WritableReg {
        w.hi
    }

    #[inline]
    fn writable_regpair_lo(&mut self, w: WritableRegPair) -> WritableReg {
        w.lo
    }

    #[inline]
    fn regpair(&mut self, hi: Reg, lo: Reg) -> RegPair {
        RegPair { hi, lo }
    }

    #[inline]
    fn regpair_hi(&mut self, w: RegPair) -> Reg {
        w.hi
    }

    #[inline]
    fn regpair_lo(&mut self, w: RegPair) -> Reg {
        w.lo
    }
}

/// Lane order to be used for a given calling convention.
#[inline]
fn lane_order_for_call_conv(call_conv: CallConv) -> LaneOrder {
    match call_conv {
        CallConv::WasmtimeSystemV => LaneOrder::LittleEndian,
        CallConv::Tail => LaneOrder::LittleEndian,
        _ => LaneOrder::BigEndian,
    }
}

/// Zero-extend the low `from_bits` bits of `value` to a full u64.
#[inline]
fn zero_extend_to_u64(value: u64, from_bits: u8) -> u64 {
    assert!(from_bits <= 64);
    if from_bits >= 64 {
        value
    } else {
        value & ((1u64 << from_bits) - 1)
    }
}

/// Sign-extend the low `from_bits` bits of `value` to a full u64.
#[inline]
fn sign_extend_to_u64(value: u64, from_bits: u8) -> u64 {
    assert!(from_bits <= 64);
    if from_bits >= 64 {
        value
    } else {
        (((value << (64 - from_bits)) as i64) >> (64 - from_bits)) as u64
    }
}

/// Determines whether this condcode interprets inputs as signed or
/// unsigned.  See the documentation for the `icmp` instruction in
/// cranelift-codegen/meta/src/shared/instructions.rs for further insights
/// into this.
#[inline]
fn condcode_is_signed(cc: IntCC) -> bool {
    match cc {
        IntCC::Equal => false,
        IntCC::NotEqual => false,
        IntCC::SignedGreaterThanOrEqual => true,
        IntCC::SignedGreaterThan => true,
        IntCC::SignedLessThanOrEqual => true,
        IntCC::SignedLessThan => true,
        IntCC::UnsignedGreaterThanOrEqual => false,
        IntCC::UnsignedGreaterThan => false,
        IntCC::UnsignedLessThanOrEqual => false,
        IntCC::UnsignedLessThan => false,
    }
}

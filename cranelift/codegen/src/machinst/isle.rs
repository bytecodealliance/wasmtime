use crate::ir::{BlockCall, Value, ValueList};
use alloc::boxed::Box;
use alloc::vec::Vec;
use smallvec::SmallVec;
use std::cell::Cell;

pub use super::MachLabel;
use super::RetPair;
pub use crate::ir::{condcodes::CondCode, *};
pub use crate::isa::{unwind::UnwindInst, TargetIsa};
pub use crate::machinst::{
    ABIArg, ABIArgSlot, ABIMachineSpec, CallSite, InputSourceInst, Lower, LowerBackend, RealReg,
    Reg, RelocDistance, Sig, VCodeInst, Writable,
};
pub use crate::settings::{StackSwitchModel, TlsModel};

pub type Unit = ();
pub type ValueSlice = (ValueList, usize);
pub type ValueArray2 = [Value; 2];
pub type ValueArray3 = [Value; 3];
pub type BlockArray2 = [BlockCall; 2];
pub type WritableReg = Writable<Reg>;
pub type VecRetPair = Vec<RetPair>;
pub type VecMask = Vec<u8>;
pub type ValueRegs = crate::machinst::ValueRegs<Reg>;
pub type WritableValueRegs = crate::machinst::ValueRegs<WritableReg>;
pub type InstOutput = SmallVec<[ValueRegs; 2]>;
pub type InstOutputBuilder = Cell<InstOutput>;
pub type BoxExternalName = Box<ExternalName>;
pub type Range = (usize, usize);
pub type MachLabelSlice = [MachLabel];
pub type BoxVecMachLabel = Box<Vec<MachLabel>>;

pub enum RangeView {
    Empty,
    NonEmpty { index: usize, rest: Range },
}

/// Helper macro to define methods in `prelude.isle` within `impl Context for
/// ...` for each backend. These methods are shared amongst all backends.
#[macro_export]
#[doc(hidden)]
macro_rules! isle_lower_prelude_methods {
    () => {
        crate::isle_lower_prelude_methods!(MInst);
    };
    ($inst:ty) => {
        crate::isle_common_prelude_methods!();

        #[inline]
        fn value_type(&mut self, val: Value) -> Type {
            self.lower_ctx.dfg().value_type(val)
        }

        #[inline]
        fn value_reg(&mut self, reg: Reg) -> ValueRegs {
            ValueRegs::one(reg)
        }

        #[inline]
        fn value_regs(&mut self, r1: Reg, r2: Reg) -> ValueRegs {
            ValueRegs::two(r1, r2)
        }

        #[inline]
        fn writable_value_regs(&mut self, r1: WritableReg, r2: WritableReg) -> WritableValueRegs {
            WritableValueRegs::two(r1, r2)
        }

        #[inline]
        fn writable_value_reg(&mut self, r: WritableReg) -> WritableValueRegs {
            WritableValueRegs::one(r)
        }

        #[inline]
        fn value_regs_invalid(&mut self) -> ValueRegs {
            ValueRegs::invalid()
        }

        #[inline]
        fn output_none(&mut self) -> InstOutput {
            smallvec::smallvec![]
        }

        #[inline]
        fn output(&mut self, regs: ValueRegs) -> InstOutput {
            smallvec::smallvec![regs]
        }

        #[inline]
        fn output_pair(&mut self, r1: ValueRegs, r2: ValueRegs) -> InstOutput {
            smallvec::smallvec![r1, r2]
        }

        #[inline]
        fn output_builder_new(&mut self) -> InstOutputBuilder {
            std::cell::Cell::new(InstOutput::new())
        }

        #[inline]
        fn output_builder_push(&mut self, builder: &InstOutputBuilder, regs: ValueRegs) -> Unit {
            let mut vec = builder.take();
            vec.push(regs);
            builder.set(vec);
        }

        #[inline]
        fn output_builder_finish(&mut self, builder: &InstOutputBuilder) -> InstOutput {
            builder.take()
        }

        #[inline]
        fn temp_writable_reg(&mut self, ty: Type) -> WritableReg {
            let value_regs = self.lower_ctx.alloc_tmp(ty);
            value_regs.only_reg().unwrap()
        }

        #[inline]
        fn is_valid_reg(&mut self, reg: Reg) -> bool {
            use crate::machinst::valueregs::InvalidSentinel;
            !reg.is_invalid_sentinel()
        }

        #[inline]
        fn invalid_reg(&mut self) -> Reg {
            use crate::machinst::valueregs::InvalidSentinel;
            Reg::invalid_sentinel()
        }

        #[inline]
        fn mark_value_used(&mut self, val: Value) {
            self.lower_ctx.increment_lowered_uses(val);
        }

        #[inline]
        fn put_in_reg(&mut self, val: Value) -> Reg {
            self.put_in_regs(val).only_reg().unwrap()
        }

        #[inline]
        fn put_in_regs(&mut self, val: Value) -> ValueRegs {
            self.lower_ctx.put_value_in_regs(val)
        }

        #[inline]
        fn ensure_in_vreg(&mut self, reg: Reg, ty: Type) -> Reg {
            self.lower_ctx.ensure_in_vreg(reg, ty)
        }

        #[inline]
        fn value_regs_get(&mut self, regs: ValueRegs, i: usize) -> Reg {
            regs.regs()[i]
        }

        #[inline]
        fn value_regs_len(&mut self, regs: ValueRegs) -> usize {
            regs.regs().len()
        }

        #[inline]
        fn value_list_slice(&mut self, list: ValueList) -> ValueSlice {
            (list, 0)
        }

        #[inline]
        fn value_slice_empty(&mut self, slice: ValueSlice) -> Option<()> {
            let (list, off) = slice;
            if off >= list.len(&self.lower_ctx.dfg().value_lists) {
                Some(())
            } else {
                None
            }
        }

        #[inline]
        fn value_slice_unwrap(&mut self, slice: ValueSlice) -> Option<(Value, ValueSlice)> {
            let (list, off) = slice;
            if let Some(val) = list.get(off, &self.lower_ctx.dfg().value_lists) {
                Some((val, (list, off + 1)))
            } else {
                None
            }
        }

        #[inline]
        fn value_slice_len(&mut self, slice: ValueSlice) -> usize {
            let (list, off) = slice;
            list.len(&self.lower_ctx.dfg().value_lists) - off
        }

        #[inline]
        fn value_slice_get(&mut self, slice: ValueSlice, idx: usize) -> Value {
            let (list, off) = slice;
            list.get(off + idx, &self.lower_ctx.dfg().value_lists)
                .unwrap()
        }

        #[inline]
        fn writable_reg_to_reg(&mut self, r: WritableReg) -> Reg {
            r.to_reg()
        }

        #[inline]
        fn inst_results(&mut self, inst: Inst) -> ValueSlice {
            (self.lower_ctx.dfg().inst_results_list(inst), 0)
        }

        #[inline]
        fn first_result(&mut self, inst: Inst) -> Option<Value> {
            self.lower_ctx.dfg().inst_results(inst).first().copied()
        }

        #[inline]
        fn inst_data(&mut self, inst: Inst) -> InstructionData {
            self.lower_ctx.dfg().insts[inst]
        }

        #[inline]
        fn def_inst(&mut self, val: Value) -> Option<Inst> {
            self.lower_ctx.dfg().value_def(val).inst()
        }

        #[inline]
        fn i64_from_iconst(&mut self, val: Value) -> Option<i64> {
            let inst = self.def_inst(val)?;
            let constant = match self.lower_ctx.data(inst) {
                InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } => imm.bits(),
                _ => return None,
            };
            let ty = self.lower_ctx.output_ty(inst, 0);
            let shift_amt = std::cmp::max(0, 64 - self.ty_bits(ty));
            Some((constant << shift_amt) >> shift_amt)
        }

        fn i32_from_iconst(&mut self, val: Value) -> Option<i32> {
            self.i64_from_iconst(val)?.try_into().ok()
        }

        fn zero_value(&mut self, value: Value) -> Option<Value> {
            let insn = self.def_inst(value);
            if insn.is_some() {
                let insn = insn.unwrap();
                let inst_data = self.lower_ctx.data(insn);
                match inst_data {
                    InstructionData::Unary {
                        opcode: Opcode::Splat,
                        arg,
                    } => {
                        let arg = arg.clone();
                        return self.zero_value(arg);
                    }
                    InstructionData::UnaryConst {
                        opcode: Opcode::Vconst | Opcode::F128const,
                        constant_handle,
                    } => {
                        let constant_data =
                            self.lower_ctx.get_constant_data(*constant_handle).clone();
                        if constant_data.into_vec().iter().any(|&x| x != 0) {
                            return None;
                        } else {
                            return Some(value);
                        }
                    }
                    InstructionData::UnaryImm { imm, .. } => {
                        if imm.bits() == 0 {
                            return Some(value);
                        } else {
                            return None;
                        }
                    }
                    InstructionData::UnaryIeee16 { imm, .. } => {
                        if imm.bits() == 0 {
                            return Some(value);
                        } else {
                            return None;
                        }
                    }
                    InstructionData::UnaryIeee32 { imm, .. } => {
                        if imm.bits() == 0 {
                            return Some(value);
                        } else {
                            return None;
                        }
                    }
                    InstructionData::UnaryIeee64 { imm, .. } => {
                        if imm.bits() == 0 {
                            return Some(value);
                        } else {
                            return None;
                        }
                    }
                    _ => None,
                }
            } else {
                None
            }
        }

        #[inline]
        fn tls_model(&mut self, _: Type) -> TlsModel {
            self.backend.flags().tls_model()
        }

        #[inline]
        fn tls_model_is_elf_gd(&mut self) -> Option<()> {
            if self.backend.flags().tls_model() == TlsModel::ElfGd {
                Some(())
            } else {
                None
            }
        }

        #[inline]
        fn tls_model_is_macho(&mut self) -> Option<()> {
            if self.backend.flags().tls_model() == TlsModel::Macho {
                Some(())
            } else {
                None
            }
        }

        #[inline]
        fn tls_model_is_coff(&mut self) -> Option<()> {
            if self.backend.flags().tls_model() == TlsModel::Coff {
                Some(())
            } else {
                None
            }
        }

        #[inline]
        fn preserve_frame_pointers(&mut self) -> Option<()> {
            if self.backend.flags().preserve_frame_pointers() {
                Some(())
            } else {
                None
            }
        }

        #[inline]
        fn stack_switch_model(&mut self) -> Option<StackSwitchModel> {
            Some(self.backend.flags().stack_switch_model())
        }

        #[inline]
        fn func_ref_data(&mut self, func_ref: FuncRef) -> (SigRef, ExternalName, RelocDistance) {
            let funcdata = &self.lower_ctx.dfg().ext_funcs[func_ref];
            let reloc_distance = if funcdata.colocated {
                RelocDistance::Near
            } else {
                RelocDistance::Far
            };
            (funcdata.signature, funcdata.name.clone(), reloc_distance)
        }

        #[inline]
        fn box_external_name(&mut self, extname: ExternalName) -> BoxExternalName {
            Box::new(extname)
        }

        #[inline]
        fn symbol_value_data(
            &mut self,
            global_value: GlobalValue,
        ) -> Option<(ExternalName, RelocDistance, i64)> {
            let (name, reloc, offset) = self.lower_ctx.symbol_value_data(global_value)?;
            Some((name.clone(), reloc, offset))
        }

        #[inline]
        fn reloc_distance_near(&mut self, dist: RelocDistance) -> Option<()> {
            if dist == RelocDistance::Near {
                Some(())
            } else {
                None
            }
        }

        #[inline]
        fn u128_from_immediate(&mut self, imm: Immediate) -> Option<u128> {
            let bytes = self.lower_ctx.get_immediate_data(imm).as_slice();
            Some(u128::from_le_bytes(bytes.try_into().ok()?))
        }

        #[inline]
        fn vconst_from_immediate(&mut self, imm: Immediate) -> Option<VCodeConstant> {
            Some(self.lower_ctx.use_constant(VCodeConstantData::Generated(
                self.lower_ctx.get_immediate_data(imm).clone(),
            )))
        }

        #[inline]
        fn vec_mask_from_immediate(&mut self, imm: Immediate) -> Option<VecMask> {
            let data = self.lower_ctx.get_immediate_data(imm);
            if data.len() == 16 {
                Some(Vec::from(data.as_slice()))
            } else {
                None
            }
        }

        #[inline]
        fn u64_from_constant(&mut self, constant: Constant) -> Option<u64> {
            let bytes = self.lower_ctx.get_constant_data(constant).as_slice();
            Some(u64::from_le_bytes(bytes.try_into().ok()?))
        }

        #[inline]
        fn u128_from_constant(&mut self, constant: Constant) -> Option<u128> {
            let bytes = self.lower_ctx.get_constant_data(constant).as_slice();
            Some(u128::from_le_bytes(bytes.try_into().ok()?))
        }

        #[inline]
        fn emit_u64_le_const(&mut self, value: u64) -> VCodeConstant {
            let data = VCodeConstantData::U64(value.to_le_bytes());
            self.lower_ctx.use_constant(data)
        }

        #[inline]
        fn emit_u128_le_const(&mut self, value: u128) -> VCodeConstant {
            let data = VCodeConstantData::Generated(value.to_le_bytes().as_slice().into());
            self.lower_ctx.use_constant(data)
        }

        #[inline]
        fn const_to_vconst(&mut self, constant: Constant) -> VCodeConstant {
            self.lower_ctx.use_constant(VCodeConstantData::Pool(
                constant,
                self.lower_ctx.get_constant_data(constant).clone(),
            ))
        }

        fn only_writable_reg(&mut self, regs: WritableValueRegs) -> Option<WritableReg> {
            regs.only_reg()
        }

        fn writable_regs_get(&mut self, regs: WritableValueRegs, idx: usize) -> WritableReg {
            regs.regs()[idx]
        }

        fn abi_num_args(&mut self, abi: Sig) -> usize {
            self.lower_ctx.sigs().num_args(abi)
        }

        fn abi_get_arg(&mut self, abi: Sig, idx: usize) -> ABIArg {
            self.lower_ctx.sigs().get_arg(abi, idx)
        }

        fn abi_num_rets(&mut self, abi: Sig) -> usize {
            self.lower_ctx.sigs().num_rets(abi)
        }

        fn abi_get_ret(&mut self, abi: Sig, idx: usize) -> ABIArg {
            self.lower_ctx.sigs().get_ret(abi, idx)
        }

        fn abi_ret_arg(&mut self, abi: Sig) -> Option<ABIArg> {
            self.lower_ctx.sigs().get_ret_arg(abi)
        }

        fn abi_no_ret_arg(&mut self, abi: Sig) -> Option<()> {
            if let Some(_) = self.lower_ctx.sigs().get_ret_arg(abi) {
                None
            } else {
                Some(())
            }
        }

        fn abi_sized_stack_arg_space(&mut self, abi: Sig) -> i64 {
            self.lower_ctx.sigs()[abi].sized_stack_arg_space()
        }

        fn abi_sized_stack_ret_space(&mut self, abi: Sig) -> i64 {
            self.lower_ctx.sigs()[abi].sized_stack_ret_space()
        }

        fn abi_arg_only_slot(&mut self, arg: &ABIArg) -> Option<ABIArgSlot> {
            match arg {
                &ABIArg::Slots { ref slots, .. } => {
                    if slots.len() == 1 {
                        Some(slots[0])
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn abi_arg_struct_pointer(&mut self, arg: &ABIArg) -> Option<(ABIArgSlot, i64, u64)> {
            match arg {
                &ABIArg::StructArg {
                    pointer,
                    offset,
                    size,
                    ..
                } => {
                    if let Some(pointer) = pointer {
                        Some((pointer, offset, size))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        fn abi_arg_implicit_pointer(&mut self, arg: &ABIArg) -> Option<(ABIArgSlot, i64, Type)> {
            match arg {
                &ABIArg::ImplicitPtrArg {
                    pointer,
                    offset,
                    ty,
                    ..
                } => Some((pointer, offset, ty)),
                _ => None,
            }
        }

        fn abi_unwrap_ret_area_ptr(&mut self) -> Reg {
            self.lower_ctx.abi().ret_area_ptr().unwrap()
        }

        fn abi_stackslot_addr(
            &mut self,
            dst: WritableReg,
            stack_slot: StackSlot,
            offset: Offset32,
        ) -> MInst {
            let offset = u32::try_from(i32::from(offset)).unwrap();
            self.lower_ctx
                .abi()
                .sized_stackslot_addr(stack_slot, offset, dst)
                .into()
        }

        fn abi_dynamic_stackslot_addr(
            &mut self,
            dst: WritableReg,
            stack_slot: DynamicStackSlot,
        ) -> MInst {
            assert!(self
                .lower_ctx
                .abi()
                .dynamic_stackslot_offsets()
                .is_valid(stack_slot));
            self.lower_ctx
                .abi()
                .dynamic_stackslot_addr(stack_slot, dst)
                .into()
        }

        fn real_reg_to_reg(&mut self, reg: RealReg) -> Reg {
            Reg::from(reg)
        }

        fn real_reg_to_writable_reg(&mut self, reg: RealReg) -> WritableReg {
            Writable::from_reg(Reg::from(reg))
        }

        fn is_sinkable_inst(&mut self, val: Value) -> Option<Inst> {
            let input = self.lower_ctx.get_value_as_source_or_const(val);

            if let InputSourceInst::UniqueUse(inst, _) = input.inst {
                Some(inst)
            } else {
                None
            }
        }

        #[inline]
        fn sink_inst(&mut self, inst: Inst) {
            self.lower_ctx.sink_inst(inst);
        }

        #[inline]
        fn maybe_uextend(&mut self, value: Value) -> Option<Value> {
            if let Some(def_inst) = self.def_inst(value) {
                if let InstructionData::Unary {
                    opcode: Opcode::Uextend,
                    arg,
                } = self.lower_ctx.data(def_inst)
                {
                    return Some(*arg);
                }
            }

            Some(value)
        }

        #[inline]
        fn uimm8(&mut self, x: Imm64) -> Option<u8> {
            let x64: i64 = x.into();
            let x8: u8 = x64.try_into().ok()?;
            Some(x8)
        }

        #[inline]
        fn preg_to_reg(&mut self, preg: PReg) -> Reg {
            preg.into()
        }

        #[inline]
        fn gen_move(&mut self, ty: Type, dst: WritableReg, src: Reg) -> MInst {
            <$inst>::gen_move(dst, src, ty).into()
        }

        /// Generate the return instruction.
        fn gen_return(&mut self, (list, off): ValueSlice) {
            let rets = (off..list.len(&self.lower_ctx.dfg().value_lists))
                .map(|ix| {
                    let val = list.get(ix, &self.lower_ctx.dfg().value_lists).unwrap();
                    self.put_in_regs(val)
                })
                .collect();
            self.lower_ctx.gen_return(rets);
        }

        /// Same as `shuffle32_from_imm`, but for 64-bit lane shuffles.
        fn shuffle64_from_imm(&mut self, imm: Immediate) -> Option<(u8, u8)> {
            use crate::machinst::isle::shuffle_imm_as_le_lane_idx;

            let bytes = self.lower_ctx.get_immediate_data(imm).as_slice();
            Some((
                shuffle_imm_as_le_lane_idx(8, &bytes[0..8])?,
                shuffle_imm_as_le_lane_idx(8, &bytes[8..16])?,
            ))
        }

        /// Attempts to interpret the shuffle immediate `imm` as a shuffle of
        /// 32-bit lanes, returning four integers, each of which is less than 8,
        /// which represents a permutation of 32-bit lanes as specified by
        /// `imm`.
        ///
        /// For example the shuffle immediate
        ///
        /// `0 1 2 3 8 9 10 11 16 17 18 19 24 25 26 27`
        ///
        /// would return `Some((0, 2, 4, 6))`.
        fn shuffle32_from_imm(&mut self, imm: Immediate) -> Option<(u8, u8, u8, u8)> {
            use crate::machinst::isle::shuffle_imm_as_le_lane_idx;

            let bytes = self.lower_ctx.get_immediate_data(imm).as_slice();
            Some((
                shuffle_imm_as_le_lane_idx(4, &bytes[0..4])?,
                shuffle_imm_as_le_lane_idx(4, &bytes[4..8])?,
                shuffle_imm_as_le_lane_idx(4, &bytes[8..12])?,
                shuffle_imm_as_le_lane_idx(4, &bytes[12..16])?,
            ))
        }

        /// Same as `shuffle32_from_imm`, but for 16-bit lane shuffles.
        fn shuffle16_from_imm(
            &mut self,
            imm: Immediate,
        ) -> Option<(u8, u8, u8, u8, u8, u8, u8, u8)> {
            use crate::machinst::isle::shuffle_imm_as_le_lane_idx;
            let bytes = self.lower_ctx.get_immediate_data(imm).as_slice();
            Some((
                shuffle_imm_as_le_lane_idx(2, &bytes[0..2])?,
                shuffle_imm_as_le_lane_idx(2, &bytes[2..4])?,
                shuffle_imm_as_le_lane_idx(2, &bytes[4..6])?,
                shuffle_imm_as_le_lane_idx(2, &bytes[6..8])?,
                shuffle_imm_as_le_lane_idx(2, &bytes[8..10])?,
                shuffle_imm_as_le_lane_idx(2, &bytes[10..12])?,
                shuffle_imm_as_le_lane_idx(2, &bytes[12..14])?,
                shuffle_imm_as_le_lane_idx(2, &bytes[14..16])?,
            ))
        }

        fn safe_divisor_from_imm64(&mut self, ty: Type, val: Imm64) -> Option<u64> {
            let minus_one = if ty.bytes() == 8 {
                -1
            } else {
                (1 << (ty.bytes() * 8)) - 1
            };
            let bits = val.bits() & minus_one;
            if bits == 0 || bits == minus_one {
                None
            } else {
                Some(bits as u64)
            }
        }

        fn single_target(&mut self, targets: &MachLabelSlice) -> Option<MachLabel> {
            if targets.len() == 1 {
                Some(targets[0])
            } else {
                None
            }
        }

        fn two_targets(&mut self, targets: &MachLabelSlice) -> Option<(MachLabel, MachLabel)> {
            if targets.len() == 2 {
                Some((targets[0], targets[1]))
            } else {
                None
            }
        }

        fn jump_table_targets(
            &mut self,
            targets: &MachLabelSlice,
        ) -> Option<(MachLabel, BoxVecMachLabel)> {
            use std::boxed::Box;
            if targets.is_empty() {
                return None;
            }

            let default_label = targets[0];
            let jt_targets = Box::new(targets[1..].to_vec());
            Some((default_label, jt_targets))
        }

        fn jump_table_size(&mut self, targets: &BoxVecMachLabel) -> u32 {
            targets.len() as u32
        }

        fn add_range_fact(&mut self, reg: Reg, bits: u16, min: u64, max: u64) -> Reg {
            self.lower_ctx.add_range_fact(reg, bits, min, max);
            reg
        }
    };
}

/// Returns the `size`-byte lane referred to by the shuffle immediate specified
/// in `bytes`.
///
/// This helper is used by `shuffleNN_from_imm` above and is used to interpret a
/// byte-based shuffle as a higher-level shuffle of bigger lanes. This will see
/// if the `bytes` specified, which must have `size` length, specifies a lane in
/// vectors aligned to a `size`-byte boundary.
///
/// Returns `None` if `bytes` doesn't specify a `size`-byte lane aligned
/// appropriately, or returns `Some(n)` where `n` is the index of the lane being
/// shuffled.
pub fn shuffle_imm_as_le_lane_idx(size: u8, bytes: &[u8]) -> Option<u8> {
    assert_eq!(bytes.len(), usize::from(size));

    // The first index in `bytes` must be aligned to a `size` boundary for the
    // bytes to be a valid specifier for a lane of `size` bytes.
    if bytes[0] % size != 0 {
        return None;
    }

    // Afterwards the bytes must all be one larger than the prior to specify a
    // contiguous sequence of bytes that's being shuffled. Basically `bytes`
    // must refer to the entire `size`-byte lane, in little-endian order.
    for i in 0..size - 1 {
        let idx = usize::from(i);
        if bytes[idx] + 1 != bytes[idx + 1] {
            return None;
        }
    }

    // All of the `bytes` are in-order, meaning that this is a valid shuffle
    // immediate to specify a lane of `size` bytes. The index, when viewed as
    // `size`-byte immediates, will be the first byte divided by the byte size.
    Some(bytes[0] / size)
}

/// Helpers specifically for machines that use `abi::CallSite`.
#[macro_export]
#[doc(hidden)]
macro_rules! isle_prelude_caller_methods {
    ($abispec:ty, $abicaller:ty) => {
        fn gen_call(
            &mut self,
            sig_ref: SigRef,
            extname: ExternalName,
            dist: RelocDistance,
            args @ (inputs, off): ValueSlice,
        ) -> InstOutput {
            let caller_conv = self.lower_ctx.abi().call_conv(self.lower_ctx.sigs());
            let sig = &self.lower_ctx.dfg().signatures[sig_ref];
            let num_rets = sig.returns.len();
            let caller = <$abicaller>::from_func(
                self.lower_ctx.sigs(),
                sig_ref,
                &extname,
                IsTailCall::No,
                dist,
                caller_conv,
                self.backend.flags().clone(),
            );

            assert_eq!(
                inputs.len(&self.lower_ctx.dfg().value_lists) - off,
                sig.params.len()
            );

            crate::machinst::isle::gen_call_common(&mut self.lower_ctx, num_rets, caller, args)
        }

        fn gen_call_indirect(
            &mut self,
            sig_ref: SigRef,
            val: Value,
            args @ (inputs, off): ValueSlice,
        ) -> InstOutput {
            let caller_conv = self.lower_ctx.abi().call_conv(self.lower_ctx.sigs());
            let ptr = self.put_in_reg(val);
            let sig = &self.lower_ctx.dfg().signatures[sig_ref];
            let num_rets = sig.returns.len();
            let caller = <$abicaller>::from_ptr(
                self.lower_ctx.sigs(),
                sig_ref,
                ptr,
                IsTailCall::No,
                caller_conv,
                self.backend.flags().clone(),
            );

            assert_eq!(
                inputs.len(&self.lower_ctx.dfg().value_lists) - off,
                sig.params.len()
            );

            crate::machinst::isle::gen_call_common(&mut self.lower_ctx, num_rets, caller, args)
        }
    };
}

fn gen_call_common_args<M: ABIMachineSpec>(
    ctx: &mut Lower<'_, M::I>,
    call_site: &mut CallSite<M>,
    (inputs, off): ValueSlice,
) {
    let num_args = call_site.num_args(ctx.sigs());

    assert_eq!(inputs.len(&ctx.dfg().value_lists) - off, num_args);
    let mut arg_regs = vec![];
    for i in 0..num_args {
        let input = inputs.get(off + i, &ctx.dfg().value_lists).unwrap();
        arg_regs.push(ctx.put_value_in_regs(input));
    }
    for (i, arg_regs) in arg_regs.iter().enumerate() {
        call_site.emit_copy_regs_to_buffer(ctx, i, *arg_regs);
    }
    for (i, arg_regs) in arg_regs.iter().enumerate() {
        call_site.gen_arg(ctx, i, *arg_regs);
    }
}

pub fn gen_call_common<M: ABIMachineSpec>(
    ctx: &mut Lower<'_, M::I>,
    num_rets: usize,
    mut caller: CallSite<M>,
    args: ValueSlice,
) -> InstOutput {
    gen_call_common_args(ctx, &mut caller, args);

    // Handle retvals prior to emitting call, so the
    // constraints are on the call instruction; but buffer the
    // instructions till after the call.
    let mut outputs = InstOutput::new();
    let mut retval_insts = crate::machinst::abi::SmallInstVec::new();
    // We take the *last* `num_rets` returns of the sig:
    // this skips a StructReturn, if any, that is present.
    let sigdata_num_rets = caller.num_rets(ctx.sigs());
    debug_assert!(num_rets <= sigdata_num_rets);
    for i in (sigdata_num_rets - num_rets)..sigdata_num_rets {
        let (retval_inst, retval_regs) = caller.gen_retval(ctx, i);
        retval_insts.extend(retval_inst.into_iter());
        outputs.push(retval_regs);
    }

    caller.emit_call(ctx);

    for inst in retval_insts {
        ctx.emit(inst);
    }

    outputs
}

/// This structure is used to implement the ISLE-generated `Context` trait and
/// internally has a temporary reference to a machinst `LowerCtx`.
pub(crate) struct IsleContext<'a, 'b, I, B>
where
    I: VCodeInst,
    B: LowerBackend,
{
    pub lower_ctx: &'a mut Lower<'b, I>,
    pub backend: &'a B,
}

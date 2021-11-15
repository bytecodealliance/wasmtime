//! ISLE integration glue code for x64 lowering.

// Pull in the ISLE generated code.
mod generated_code;

// Types that the generated ISLE code uses via `use super::*`.
use super::{
    is_mergeable_load, lower_to_amode, AluRmiROpcode, Inst as MInst, OperandSize, Reg, RegMemImm,
    Writable,
};
use crate::isa::x64::inst::args::SyntheticAmode;
use crate::isa::x64::settings as x64_settings;
use crate::{
    ir::{immediates::*, types::*, Inst, InstructionData, Opcode, Value, ValueList},
    isa::x64::inst::{
        args::{Avx512Opcode, CmpOpcode, ExtMode, Imm8Reg, RegMem, ShiftKind, SseOpcode, CC},
        x64_map_regs, RegMapper,
    },
    machinst::{get_output_reg, InsnInput, InsnOutput, LowerCtx},
};
use smallvec::SmallVec;
use std::convert::TryFrom;

type Unit = ();
type ValueSlice<'a> = &'a [Value];
type ValueArray2 = [Value; 2];
type ValueArray3 = [Value; 3];
type WritableReg = Writable<Reg>;
type ValueRegs = crate::machinst::ValueRegs<Reg>;

pub struct SinkableLoad {
    inst: Inst,
    addr_input: InsnInput,
    offset: i32,
}

#[derive(Default)]
struct RegRenamer {
    // Map of `(old, new)` register names. Use a `SmallVec` because we typically
    // only have one or two renamings.
    renames: SmallVec<[(Reg, Reg); 2]>,
}

impl RegRenamer {
    fn add_rename(&mut self, old: Reg, new: Reg) {
        self.renames.push((old, new));
    }

    fn get_rename(&self, reg: Reg) -> Option<Reg> {
        self.renames
            .iter()
            .find(|(old, _)| reg == *old)
            .map(|(_, new)| *new)
    }
}

impl RegMapper for RegRenamer {
    fn get_use(&self, reg: Reg) -> Option<Reg> {
        self.get_rename(reg)
    }

    fn get_def(&self, reg: Reg) -> Option<Reg> {
        self.get_rename(reg)
    }

    fn get_mod(&self, reg: Reg) -> Option<Reg> {
        self.get_rename(reg)
    }
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower<C>(
    lower_ctx: &mut C,
    isa_flags: &x64_settings::Flags,
    outputs: &[InsnOutput],
    inst: Inst,
) -> Result<(), ()>
where
    C: LowerCtx<I = MInst>,
{
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = IsleContext::new(lower_ctx, isa_flags);

    let temp_regs = generated_code::constructor_lower(&mut isle_ctx, inst).ok_or(())?;
    let mut temp_regs = temp_regs.regs().iter();

    // The ISLE generated code emits its own registers to define the
    // instruction's lowered values in. We rename those registers to the
    // registers they were assigned when their value was used as an operand in
    // earlier lowerings.
    let mut renamer = RegRenamer::default();
    for output in outputs {
        let dsts = get_output_reg(isle_ctx.lower_ctx, *output);
        for (temp, dst) in temp_regs.by_ref().zip(dsts.regs()) {
            renamer.add_rename(*temp, dst.to_reg());
        }
    }

    for mut inst in isle_ctx.into_emitted_insts() {
        x64_map_regs(&mut inst, &renamer);
        lower_ctx.emit(inst);
    }

    Ok(())
}

pub struct IsleContext<'a, C> {
    lower_ctx: &'a mut C,
    isa_flags: &'a x64_settings::Flags,
    emitted_insts: SmallVec<[MInst; 6]>,
}

impl<'a, C> IsleContext<'a, C> {
    pub fn new(lower_ctx: &'a mut C, isa_flags: &'a x64_settings::Flags) -> Self {
        IsleContext {
            lower_ctx,
            isa_flags,
            emitted_insts: SmallVec::new(),
        }
    }

    pub fn into_emitted_insts(self) -> SmallVec<[MInst; 6]> {
        self.emitted_insts
    }
}

impl<'a, C> generated_code::Context for IsleContext<'a, C>
where
    C: LowerCtx<I = MInst>,
{
    #[inline]
    fn unpack_value_array_2(&mut self, arr: &ValueArray2) -> (Value, Value) {
        let [a, b] = *arr;
        (a, b)
    }

    #[inline]
    fn pack_value_array_2(&mut self, a: Value, b: Value) -> ValueArray2 {
        [a, b]
    }

    #[inline]
    fn unpack_value_array_3(&mut self, arr: &ValueArray3) -> (Value, Value, Value) {
        let [a, b, c] = *arr;
        (a, b, c)
    }

    #[inline]
    fn pack_value_array_3(&mut self, a: Value, b: Value, c: Value) -> ValueArray3 {
        [a, b, c]
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
    fn temp_writable_reg(&mut self, ty: Type) -> WritableReg {
        let value_regs = self.lower_ctx.alloc_tmp(ty);
        value_regs.only_reg().unwrap()
    }

    #[inline]
    fn invalid_reg(&mut self) -> Reg {
        Reg::invalid()
    }

    #[inline]
    fn put_in_reg(&mut self, val: Value) -> Reg {
        self.lower_ctx.put_value_in_regs(val).only_reg().unwrap()
    }

    #[inline]
    fn put_in_regs(&mut self, val: Value) -> ValueRegs {
        self.lower_ctx.put_value_in_regs(val)
    }

    #[inline]
    fn value_regs_get(&mut self, regs: ValueRegs, i: usize) -> Reg {
        regs.regs()[i]
    }

    #[inline]
    fn u8_as_u64(&mut self, x: u8) -> u64 {
        x.into()
    }

    #[inline]
    fn u16_as_u64(&mut self, x: u16) -> u64 {
        x.into()
    }

    #[inline]
    fn u32_as_u64(&mut self, x: u32) -> u64 {
        x.into()
    }

    #[inline]
    fn ty_bits(&mut self, ty: Type) -> u16 {
        ty.bits()
    }

    #[inline]
    fn fits_in_64(&mut self, ty: Type) -> Option<Type> {
        if ty.bits() <= 64 {
            Some(ty)
        } else {
            None
        }
    }

    #[inline]
    fn value_list_slice(&mut self, list: ValueList) -> ValueSlice {
        list.as_slice(&self.lower_ctx.dfg().value_lists)
    }

    #[inline]
    fn unwrap_head_value_list_1(&mut self, list: ValueList) -> (Value, ValueSlice) {
        match self.value_list_slice(list) {
            [head, tail @ ..] => (*head, tail),
            _ => out_of_line_panic("`unwrap_head_value_list_1` on empty `ValueList`"),
        }
    }

    #[inline]
    fn unwrap_head_value_list_2(&mut self, list: ValueList) -> (Value, Value, ValueSlice) {
        match self.value_list_slice(list) {
            [head1, head2, tail @ ..] => (*head1, *head2, tail),
            _ => out_of_line_panic(
                "`unwrap_head_value_list_2` on list without at least two elements",
            ),
        }
    }

    #[inline]
    fn writable_reg_to_reg(&mut self, r: WritableReg) -> Reg {
        r.to_reg()
    }

    #[inline]
    fn u64_from_imm64(&mut self, imm: Imm64) -> u64 {
        imm.bits() as u64
    }

    #[inline]
    fn inst_results(&mut self, inst: Inst) -> ValueSlice {
        self.lower_ctx.dfg().inst_results(inst)
    }

    #[inline]
    fn first_result(&mut self, inst: Inst) -> Option<Value> {
        self.lower_ctx.dfg().inst_results(inst).first().copied()
    }

    #[inline]
    fn inst_data(&mut self, inst: Inst) -> InstructionData {
        self.lower_ctx.dfg()[inst].clone()
    }

    #[inline]
    fn value_type(&mut self, val: Value) -> Type {
        self.lower_ctx.dfg().value_type(val)
    }

    #[inline]
    fn multi_lane(&mut self, ty: Type) -> Option<(u8, u16)> {
        if ty.lane_count() > 1 {
            Some((ty.lane_bits(), ty.lane_count()))
        } else {
            None
        }
    }

    #[inline]
    fn def_inst(&mut self, val: Value) -> Option<Inst> {
        self.lower_ctx.dfg().value_def(val).inst()
    }

    #[inline]
    fn operand_size_of_type(&mut self, ty: Type) -> OperandSize {
        if ty.bits() == 64 {
            OperandSize::Size64
        } else {
            OperandSize::Size32
        }
    }

    fn put_in_reg_mem(&mut self, val: Value) -> RegMem {
        let inputs = self.lower_ctx.get_value_as_source_or_const(val);

        if let Some(c) = inputs.constant {
            // Generate constants fresh at each use to minimize long-range
            // register pressure.
            let ty = self.value_type(val);
            return RegMem::reg(generated_code::constructor_imm(self, ty, c).unwrap());
        }

        if let Some((src_insn, 0)) = inputs.inst {
            if let Some((addr_input, offset)) = is_mergeable_load(self.lower_ctx, src_insn) {
                self.lower_ctx.sink_inst(src_insn);
                let amode = lower_to_amode(self.lower_ctx, addr_input, offset);
                return RegMem::mem(amode);
            }
        }

        RegMem::reg(self.put_in_reg(val))
    }

    #[inline]
    fn avx512vl_enabled(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_avx512vl_simd() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn avx512dq_enabled(&mut self, _: Type) -> Option<()> {
        if self.isa_flags.use_avx512dq_simd() {
            Some(())
        } else {
            None
        }
    }

    #[inline]
    fn imm8_from_value(&mut self, val: Value) -> Option<Imm8Reg> {
        let inst = self.lower_ctx.dfg().value_def(val).inst()?;
        let constant = self.lower_ctx.get_constant(inst)?;
        let imm = u8::try_from(constant).ok()?;
        Some(Imm8Reg::Imm8 { imm })
    }

    #[inline]
    fn simm32_from_value(&mut self, val: Value) -> Option<RegMemImm> {
        let inst = self.lower_ctx.dfg().value_def(val).inst()?;
        let constant: u64 = self.lower_ctx.get_constant(inst)?;
        let constant = constant as i64;
        to_simm32(constant)
    }

    #[inline]
    fn simm32_from_imm64(&mut self, imm: Imm64) -> Option<RegMemImm> {
        to_simm32(imm.bits())
    }

    fn sinkable_load(&mut self, val: Value) -> Option<SinkableLoad> {
        let input = self.lower_ctx.get_value_as_source_or_const(val);
        if let Some((inst, 0)) = input.inst {
            if let Some((addr_input, offset)) = is_mergeable_load(self.lower_ctx, inst) {
                return Some(SinkableLoad {
                    inst,
                    addr_input,
                    offset,
                });
            }
        }
        None
    }

    fn sink_load(&mut self, load: &SinkableLoad) -> RegMemImm {
        self.lower_ctx.sink_inst(load.inst);
        let addr = lower_to_amode(self.lower_ctx, load.addr_input, load.offset);
        RegMemImm::Mem {
            addr: SyntheticAmode::Real(addr),
        }
    }

    #[inline]
    fn ext_mode(&mut self, from_bits: u16, to_bits: u16) -> ExtMode {
        ExtMode::new(from_bits, to_bits).unwrap()
    }

    fn emit(&mut self, inst: &MInst) -> Unit {
        for inst in inst.clone().mov_mitosis() {
            self.emitted_insts.push(inst);
        }
    }

    #[inline]
    fn nonzero_u64_fits_in_u32(&mut self, x: u64) -> Option<u64> {
        if x != 0 && x < u64::from(u32::MAX) {
            Some(x)
        } else {
            None
        }
    }
}

#[inline]
fn to_simm32(constant: i64) -> Option<RegMemImm> {
    if constant == ((constant << 32) >> 32) {
        Some(RegMemImm::Imm {
            simm32: constant as u32,
        })
    } else {
        None
    }
}

#[inline(never)]
#[cold]
#[track_caller]
fn out_of_line_panic(msg: &str) -> ! {
    panic!("{}", msg);
}

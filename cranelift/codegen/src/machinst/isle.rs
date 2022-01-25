use crate::ir::{Inst, Value};
use crate::machinst::{get_output_reg, InsnOutput, LowerCtx, MachInst, RegRenamer};
use regalloc::{Reg, Writable};
use smallvec::SmallVec;

pub type Unit = ();
pub type ValueSlice<'a> = &'a [Value];
pub type ValueArray2 = [Value; 2];
pub type ValueArray3 = [Value; 3];
pub type WritableReg = Writable<Reg>;
pub type ValueRegs = crate::machinst::ValueRegs<Reg>;

/// Helper macro to define methods in `prelude.isle` within `impl Context for
/// ...` for each backend. These methods are shared amongst all backends.
#[macro_export]
#[doc(hidden)]
macro_rules! isle_prelude_methods {
    () => {
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
        fn value_regs_invalid(&mut self) -> ValueRegs {
            ValueRegs::invalid()
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
        fn ty_bits(&mut self, ty: Type) -> u8 {
            use std::convert::TryInto;
            ty.bits().try_into().unwrap()
        }

        #[inline]
        fn ty_bits_u16(&mut self, ty: Type) -> u16 {
            ty.bits()
        }

        #[inline]
        fn ty_bytes(&mut self, ty: Type) -> u16 {
            u16::try_from(ty.bytes()).unwrap()
        }

        fn fits_in_16(&mut self, ty: Type) -> Option<Type> {
            if ty.bits() <= 16 {
                Some(ty)
            } else {
                None
            }
        }

        #[inline]
        fn fits_in_32(&mut self, ty: Type) -> Option<Type> {
            if ty.bits() <= 32 {
                Some(ty)
            } else {
                None
            }
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
        fn ty_32_or_64(&mut self, ty: Type) -> Option<Type> {
            if ty.bits() == 32 || ty.bits() == 64 {
                Some(ty)
            } else {
                None
            }
        }

        #[inline]
        fn ty_8_or_16(&mut self, ty: Type) -> Option<Type> {
            if ty.bits() == 8 || ty.bits() == 16 {
                Some(ty)
            } else {
                None
            }
        }

        fn vec128(&mut self, ty: Type) -> Option<Type> {
            if ty.is_vector() && ty.bits() == 128 {
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
                _ => crate::machinst::isle::out_of_line_panic(
                    "`unwrap_head_value_list_1` on empty `ValueList`",
                ),
            }
        }

        #[inline]
        fn unwrap_head_value_list_2(&mut self, list: ValueList) -> (Value, Value, ValueSlice) {
            match self.value_list_slice(list) {
                [head1, head2, tail @ ..] => (*head1, *head2, tail),
                _ => crate::machinst::isle::out_of_line_panic(
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

        fn u64_from_ieee32(&mut self, val: Ieee32) -> u64 {
            val.bits().into()
        }

        fn u64_from_ieee64(&mut self, val: Ieee64) -> u64 {
            val.bits()
        }

        fn u8_from_uimm8(&mut self, val: Uimm8) -> u8 {
            val
        }

        fn not_i64x2(&mut self, ty: Type) -> Option<()> {
            if ty == I64X2 {
                None
            } else {
                Some(())
            }
        }

        fn trap_code_division_by_zero(&mut self) -> TrapCode {
            TrapCode::IntegerDivisionByZero
        }

        fn trap_code_integer_overflow(&mut self) -> TrapCode {
            TrapCode::IntegerOverflow
        }

        fn trap_code_bad_conversion_to_integer(&mut self) -> TrapCode {
            TrapCode::BadConversionToInteger
        }

        fn avoid_div_traps(&mut self, _: Type) -> Option<()> {
            if self.flags.avoid_div_traps() {
                Some(())
            } else {
                None
            }
        }

        fn nonzero_u64_from_imm64(&mut self, val: Imm64) -> Option<u64> {
            match val.bits() {
                0 => None,
                n => Some(n as u64),
            }
        }

        #[inline]
        fn u32_add(&mut self, a: u32, b: u32) -> u32 {
            a.wrapping_add(b)
        }

        #[inline]
        fn u8_and(&mut self, a: u8, b: u8) -> u8 {
            a & b
        }

        #[inline]
        fn lane_type(&mut self, ty: Type) -> Type {
            ty.lane_type()
        }
    };
}

/// This structure is used to implement the ISLE-generated `Context` trait and
/// internally has a temporary reference to a machinst `LowerCtx`.
pub(crate) struct IsleContext<'a, C: LowerCtx, F, I, const N: usize>
where
    [(C::I, bool); N]: smallvec::Array,
{
    pub lower_ctx: &'a mut C,
    pub flags: &'a F,
    pub isa_flags: &'a I,
    pub emitted_insts: SmallVec<[(C::I, bool); N]>,
}

/// Shared lowering code amongst all backends for doing ISLE-based lowering.
///
/// The `isle_lower` argument here is an ISLE-generated function for `lower` and
/// then this function otherwise handles register mapping and such around the
/// lowering.
pub(crate) fn lower_common<C, F, I, const N: usize>(
    lower_ctx: &mut C,
    flags: &F,
    isa_flags: &I,
    outputs: &[InsnOutput],
    inst: Inst,
    isle_lower: fn(&mut IsleContext<'_, C, F, I, N>, Inst) -> Option<ValueRegs>,
    map_regs: fn(&mut C::I, &RegRenamer),
) -> Result<(), ()>
where
    C: LowerCtx,
    [(C::I, bool); N]: smallvec::Array<Item = (C::I, bool)>,
{
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = IsleContext {
        lower_ctx,
        flags,
        isa_flags,
        emitted_insts: SmallVec::new(),
    };

    let temp_regs = isle_lower(&mut isle_ctx, inst).ok_or(())?;
    let mut temp_regs = temp_regs.regs().iter();

    #[cfg(debug_assertions)]
    {
        let all_dsts_len = outputs
            .iter()
            .map(|out| get_output_reg(isle_ctx.lower_ctx, *out).len())
            .sum();
        debug_assert_eq!(
            temp_regs.len(),
            all_dsts_len,
            "the number of temporary registers and destination registers do \
         not match ({} != {}); ensure the correct registers are being \
         returned.",
            temp_regs.len(),
            all_dsts_len,
        );
    }

    // The ISLE generated code emits its own registers to define the
    // instruction's lowered values in. We rename those registers to the
    // registers they were assigned when their value was used as an operand in
    // earlier lowerings.
    let mut renamer = RegRenamer::default();
    for output in outputs {
        let dsts = get_output_reg(isle_ctx.lower_ctx, *output);
        let ty = isle_ctx.lower_ctx.output_ty(output.insn, output.output);
        let (_, tys) = <C::I>::rc_for_type(ty).unwrap();
        for ((temp, dst), ty) in temp_regs.by_ref().zip(dsts.regs()).zip(tys) {
            renamer.add_rename(*temp, dst.to_reg(), *ty);
        }
    }
    for (inst, _) in isle_ctx.emitted_insts.iter_mut() {
        map_regs(inst, &renamer);
    }

    // If any renamed register wasn't actually defined in the ISLE-generated
    // instructions then what we're actually doing is "renaming" an input to a
    // new name which requires manually inserting a `mov` instruction. Note that
    // this typically doesn't happen and is only here for cases where the input
    // is sometimes passed through unmodified to the output, such as
    // zero-extending a 64-bit input to a 128-bit output which doesn't actually
    // change the input and simply produces another zero'd register.
    for (old, new, ty) in renamer.unmapped_defs() {
        isle_ctx
            .lower_ctx
            .emit(<C::I>::gen_move(Writable::from_reg(new), old, ty));
    }

    // Once everything is remapped we forward all emitted instructions to the
    // `lower_ctx`. Note that this happens after the synthetic mov's above in
    // case any of these instruction use those movs.
    for (inst, is_safepoint) in isle_ctx.emitted_insts {
        if is_safepoint {
            lower_ctx.emit_safepoint(inst);
        } else {
            lower_ctx.emit(inst);
        }
    }

    Ok(())
}

#[inline(never)]
#[cold]
pub fn out_of_line_panic(msg: &str) -> ! {
    panic!("{}", msg);
}

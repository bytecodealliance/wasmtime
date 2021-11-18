use crate::ir::Value;
use regalloc::{Reg, Writable};

pub type Unit = ();
pub type ValueSlice<'a> = &'a [Value];
pub type ValueArray2 = [Value; 2];
pub type ValueArray3 = [Value; 3];
pub type WritableReg = Writable<Reg>;
pub type ValueRegs = crate::machinst::ValueRegs<Reg>;

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
    };
}

#[inline(never)]
#[cold]
pub fn out_of_line_panic(msg: &str) -> ! {
    panic!("{}", msg);
}

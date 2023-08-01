//! ISLE integration glue code for riscv64 lowering.

// Pull in the ISLE generated code.
#[allow(unused)]
pub mod generated_code;
use generated_code::{Context, ExtendOp, MInst};

// Types that the generated ISLE code uses via `use super::*`.
use self::generated_code::{VecAluOpRR, VecLmul};
use super::{writable_zero_reg, zero_reg};
use crate::isa::riscv64::abi::Riscv64ABICallSite;
use crate::isa::riscv64::lower::args::{
    FReg, VReg, WritableFReg, WritableVReg, WritableXReg, XReg,
};
use crate::isa::riscv64::Riscv64Backend;
use crate::machinst::Reg;
use crate::machinst::{isle::*, MachInst, SmallInstVec};
use crate::machinst::{VCodeConstant, VCodeConstantData};
use crate::{
    ir::{
        immediates::*, types::*, AtomicRmwOp, BlockCall, ExternalName, Inst, InstructionData,
        MemFlags, StackSlot, TrapCode, Value, ValueList,
    },
    isa::riscv64::inst::*,
    machinst::{ArgPair, InstOutput, Lower},
};
use crate::{isa, isle_common_prelude_methods, isle_lower_prelude_methods};
use regalloc2::PReg;
use std::boxed::Box;
use std::convert::TryFrom;
use std::vec::Vec;

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;
type BoxReturnCallInfo = Box<ReturnCallInfo>;
type BoxExternalName = Box<ExternalName>;
type VecMachLabel = Vec<MachLabel>;
type VecArgPair = Vec<ArgPair>;
use crate::machinst::valueregs;

pub(crate) struct RV64IsleContext<'a, 'b, I, B>
where
    I: VCodeInst,
    B: LowerBackend,
{
    pub lower_ctx: &'a mut Lower<'b, I>,
    pub backend: &'a B,
    /// Precalucated value for the minimum vector register size. Will be 0 if
    /// vectors are not supported.
    min_vec_reg_size: u64,
}

impl<'a, 'b> RV64IsleContext<'a, 'b, MInst, Riscv64Backend> {
    isle_prelude_method_helpers!(Riscv64ABICallSite);

    fn new(lower_ctx: &'a mut Lower<'b, MInst>, backend: &'a Riscv64Backend) -> Self {
        Self {
            lower_ctx,
            backend,
            min_vec_reg_size: backend.isa_flags.min_vec_reg_size(),
        }
    }

    #[inline]
    fn emit_list(&mut self, list: &SmallInstVec<MInst>) {
        for i in list {
            self.lower_ctx.emit(i.clone());
        }
    }
}

impl generated_code::Context for RV64IsleContext<'_, '_, MInst, Riscv64Backend> {
    isle_lower_prelude_methods!();
    isle_prelude_caller_methods!(Riscv64MachineDeps, Riscv64ABICallSite);

    fn gen_return_call(
        &mut self,
        callee_sig: SigRef,
        callee: ExternalName,
        distance: RelocDistance,
        args: ValueSlice,
    ) -> InstOutput {
        let caller_conv = isa::CallConv::Tail;
        debug_assert_eq!(
            self.lower_ctx.abi().call_conv(self.lower_ctx.sigs()),
            caller_conv,
            "Can only do `return_call`s from within a `tail` calling convention function"
        );

        let call_site = Riscv64ABICallSite::from_func(
            self.lower_ctx.sigs(),
            callee_sig,
            &callee,
            distance,
            caller_conv,
            self.backend.flags().clone(),
        );
        call_site.emit_return_call(self.lower_ctx, args);

        InstOutput::new()
    }

    fn gen_return_call_indirect(
        &mut self,
        callee_sig: SigRef,
        callee: Value,
        args: ValueSlice,
    ) -> InstOutput {
        let caller_conv = isa::CallConv::Tail;
        debug_assert_eq!(
            self.lower_ctx.abi().call_conv(self.lower_ctx.sigs()),
            caller_conv,
            "Can only do `return_call`s from within a `tail` calling convention function"
        );

        let callee = self.put_in_reg(callee);

        let call_site = Riscv64ABICallSite::from_ptr(
            self.lower_ctx.sigs(),
            callee_sig,
            callee,
            Opcode::ReturnCallIndirect,
            caller_conv,
            self.backend.flags().clone(),
        );
        call_site.emit_return_call(self.lower_ctx, args);

        InstOutput::new()
    }

    fn vreg_new(&mut self, r: Reg) -> VReg {
        VReg::new(r).unwrap()
    }
    fn writable_vreg_new(&mut self, r: WritableReg) -> WritableVReg {
        r.map(|wr| VReg::new(wr).unwrap())
    }
    fn writable_vreg_to_vreg(&mut self, arg0: WritableVReg) -> VReg {
        arg0.to_reg()
    }
    fn writable_vreg_to_writable_reg(&mut self, arg0: WritableVReg) -> WritableReg {
        arg0.map(|vr| vr.to_reg())
    }
    fn vreg_to_reg(&mut self, arg0: VReg) -> Reg {
        *arg0
    }
    fn xreg_new(&mut self, r: Reg) -> XReg {
        XReg::new(r).unwrap()
    }
    fn writable_xreg_new(&mut self, r: WritableReg) -> WritableXReg {
        r.map(|wr| XReg::new(wr).unwrap())
    }
    fn writable_xreg_to_xreg(&mut self, arg0: WritableXReg) -> XReg {
        arg0.to_reg()
    }
    fn writable_xreg_to_writable_reg(&mut self, arg0: WritableXReg) -> WritableReg {
        arg0.map(|xr| xr.to_reg())
    }
    fn xreg_to_reg(&mut self, arg0: XReg) -> Reg {
        *arg0
    }
    fn freg_new(&mut self, r: Reg) -> FReg {
        FReg::new(r).unwrap()
    }
    fn writable_freg_new(&mut self, r: WritableReg) -> WritableFReg {
        r.map(|wr| FReg::new(wr).unwrap())
    }
    fn writable_freg_to_freg(&mut self, arg0: WritableFReg) -> FReg {
        arg0.to_reg()
    }
    fn writable_freg_to_writable_reg(&mut self, arg0: WritableFReg) -> WritableReg {
        arg0.map(|fr| fr.to_reg())
    }
    fn freg_to_reg(&mut self, arg0: FReg) -> Reg {
        *arg0
    }

    fn vec_writable_to_regs(&mut self, val: &VecWritableReg) -> ValueRegs {
        match val.len() {
            1 => ValueRegs::one(val[0].to_reg()),
            2 => ValueRegs::two(val[0].to_reg(), val[1].to_reg()),
            _ => unreachable!(),
        }
    }
    fn intcc_to_extend_op(&mut self, cc: &IntCC) -> ExtendOp {
        use IntCC::*;
        match *cc {
            Equal
            | NotEqual
            | UnsignedLessThan
            | UnsignedGreaterThanOrEqual
            | UnsignedGreaterThan
            | UnsignedLessThanOrEqual => ExtendOp::Zero,

            SignedLessThan
            | SignedGreaterThanOrEqual
            | SignedGreaterThan
            | SignedLessThanOrEqual => ExtendOp::Signed,
        }
    }
    fn lower_cond_br(
        &mut self,
        cc: &IntCC,
        a: ValueRegs,
        targets: &VecMachLabel,
        ty: Type,
    ) -> Unit {
        MInst::lower_br_icmp(
            *cc,
            a,
            self.int_zero_reg(ty),
            BranchTarget::Label(targets[0]),
            BranchTarget::Label(targets[1]),
            ty,
        )
        .iter()
        .for_each(|i| self.emit(i));
    }
    fn lower_br_icmp(
        &mut self,
        cc: &IntCC,
        a: ValueRegs,
        b: ValueRegs,
        targets: &VecMachLabel,
        ty: Type,
    ) -> Unit {
        let test = generated_code::constructor_lower_icmp(self, cc, a, b, ty);
        self.emit(&MInst::CondBr {
            taken: BranchTarget::Label(targets[0]),
            not_taken: BranchTarget::Label(targets[1]),
            kind: IntegerCompare {
                kind: IntCC::NotEqual,
                rs1: test,
                rs2: zero_reg(),
            },
        });
    }
    fn load_ra(&mut self) -> Reg {
        if self.backend.flags.preserve_frame_pointers() {
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::Load {
                rd: tmp,
                op: LoadOP::Ld,
                flags: MemFlags::trusted(),
                from: AMode::FPOffset(8, I64),
            });
            tmp.to_reg()
        } else {
            link_reg()
        }
    }
    fn int_zero_reg(&mut self, ty: Type) -> ValueRegs {
        assert!(ty.is_int(), "{:?}", ty);
        if ty.bits() == 128 {
            ValueRegs::two(self.zero_reg(), self.zero_reg())
        } else {
            ValueRegs::one(self.zero_reg())
        }
    }

    fn vec_label_get(&mut self, val: &VecMachLabel, x: u8) -> MachLabel {
        val[x as usize]
    }

    fn label_to_br_target(&mut self, label: MachLabel) -> BranchTarget {
        BranchTarget::Label(label)
    }

    fn vec_writable_clone(&mut self, v: &VecWritableReg) -> VecWritableReg {
        v.clone()
    }

    fn imm12_and(&mut self, imm: Imm12, x: u64) -> Imm12 {
        Imm12::from_bits(imm.as_i16() & (x as i16))
    }

    fn alloc_vec_writable(&mut self, ty: Type) -> VecWritableReg {
        if ty.is_int() || ty == R32 || ty == R64 {
            if ty.bits() <= 64 {
                vec![self.temp_writable_reg(I64)]
            } else {
                vec![self.temp_writable_reg(I64), self.temp_writable_reg(I64)]
            }
        } else if ty.is_float() || ty.is_vector() {
            vec![self.temp_writable_reg(ty)]
        } else {
            unimplemented!("ty:{:?}", ty)
        }
    }

    fn imm(&mut self, ty: Type, val: u64) -> Reg {
        let tmp = self.temp_writable_reg(ty);
        let alloc_tmp = &mut |ty| self.temp_writable_reg(ty);
        let insts = match ty {
            F32 => MInst::load_fp_constant32(tmp, val as u32, alloc_tmp),
            F64 => MInst::load_fp_constant64(tmp, val, alloc_tmp),
            _ => MInst::load_constant_u64(tmp, val, alloc_tmp),
        };
        self.emit_list(&insts);
        tmp.to_reg()
    }
    #[inline]
    fn emit(&mut self, arg0: &MInst) -> Unit {
        self.lower_ctx.emit(arg0.clone());
    }
    #[inline]
    fn imm12_from_u64(&mut self, arg0: u64) -> Option<Imm12> {
        Imm12::maybe_from_u64(arg0)
    }
    #[inline]
    fn imm5_from_u64(&mut self, arg0: u64) -> Option<Imm5> {
        Imm5::maybe_from_i8(i8::try_from(arg0 as i64).ok()?)
    }
    #[inline]
    fn imm5_from_i8(&mut self, arg0: i8) -> Option<Imm5> {
        Imm5::maybe_from_i8(arg0)
    }
    #[inline]
    fn uimm5_bitcast_to_imm5(&mut self, arg0: UImm5) -> Imm5 {
        Imm5::from_bits(arg0.bits() as u8)
    }
    #[inline]
    fn uimm5_from_u8(&mut self, arg0: u8) -> Option<UImm5> {
        UImm5::maybe_from_u8(arg0)
    }
    #[inline]
    fn uimm5_from_u64(&mut self, arg0: u64) -> Option<UImm5> {
        arg0.try_into().ok().and_then(UImm5::maybe_from_u8)
    }
    #[inline]
    fn writable_zero_reg(&mut self) -> WritableReg {
        writable_zero_reg()
    }
    #[inline]
    fn neg_imm12(&mut self, arg0: Imm12) -> Imm12 {
        -arg0
    }
    #[inline]
    fn zero_reg(&mut self) -> Reg {
        zero_reg()
    }
    #[inline]
    fn imm_from_bits(&mut self, val: u64) -> Imm12 {
        Imm12::maybe_from_u64(val).unwrap()
    }
    #[inline]
    fn imm_from_neg_bits(&mut self, val: i64) -> Imm12 {
        Imm12::maybe_from_u64(val as u64).unwrap()
    }

    fn gen_default_frm(&mut self) -> OptionFloatRoundingMode {
        None
    }
    fn gen_select_reg(&mut self, cc: &IntCC, a: XReg, b: XReg, rs1: Reg, rs2: Reg) -> Reg {
        let rd = self.temp_writable_reg(MInst::canonical_type_for_rc(rs1.class()));
        self.emit(&MInst::SelectReg {
            rd,
            rs1,
            rs2,
            condition: IntegerCompare {
                kind: *cc,
                rs1: a.to_reg(),
                rs2: b.to_reg(),
            },
        });
        rd.to_reg()
    }
    fn load_u64_constant(&mut self, val: u64) -> Reg {
        let rd = self.temp_writable_reg(I64);
        MInst::load_constant_u64(rd, val, &mut |ty| self.temp_writable_reg(ty))
            .iter()
            .for_each(|i| self.emit(i));
        rd.to_reg()
    }
    fn u8_as_i32(&mut self, x: u8) -> i32 {
        x as i32
    }

    fn imm12_const(&mut self, val: i32) -> Imm12 {
        if let Some(res) = Imm12::maybe_from_u64(val as u64) {
            res
        } else {
            panic!("Unable to make an Imm12 value from {}", val)
        }
    }
    fn imm12_const_add(&mut self, val: i32, add: i32) -> Imm12 {
        Imm12::maybe_from_u64((val + add) as u64).unwrap()
    }

    //
    fn gen_shamt(&mut self, ty: Type, shamt: XReg) -> ValueRegs {
        let ty_bits = if ty.bits() > 64 { 64 } else { ty.bits() };
        let shamt = {
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::AluRRImm12 {
                alu_op: AluOPRRI::Andi,
                rd: tmp,
                rs: shamt.to_reg(),
                imm12: Imm12::from_bits((ty_bits - 1) as i16),
            });
            tmp.to_reg()
        };
        let len_sub_shamt = {
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::load_imm12(tmp, Imm12::from_bits(ty_bits as i16)));
            let len_sub_shamt = self.temp_writable_reg(I64);
            self.emit(&MInst::AluRRR {
                alu_op: AluOPRRR::Sub,
                rd: len_sub_shamt,
                rs1: tmp.to_reg(),
                rs2: shamt,
            });
            len_sub_shamt.to_reg()
        };
        ValueRegs::two(shamt, len_sub_shamt)
    }

    fn has_v(&mut self) -> bool {
        self.backend.isa_flags.has_v()
    }

    fn has_zbkb(&mut self) -> bool {
        self.backend.isa_flags.has_zbkb()
    }

    fn has_zba(&mut self) -> bool {
        self.backend.isa_flags.has_zba()
    }

    fn has_zbb(&mut self) -> bool {
        self.backend.isa_flags.has_zbb()
    }

    fn has_zbc(&mut self) -> bool {
        self.backend.isa_flags.has_zbc()
    }

    fn has_zbs(&mut self) -> bool {
        self.backend.isa_flags.has_zbs()
    }

    fn offset32_imm(&mut self, offset: i32) -> Offset32 {
        Offset32::new(offset)
    }
    fn default_memflags(&mut self) -> MemFlags {
        MemFlags::new()
    }

    fn pack_float_rounding_mode(&mut self, f: &FRM) -> OptionFloatRoundingMode {
        Some(*f)
    }

    fn int_convert_2_float_op(&mut self, from: Type, is_signed: bool, to: Type) -> FpuOPRR {
        FpuOPRR::int_convert_2_float_op(from, is_signed, to)
    }

    fn gen_amode(&mut self, base: Reg, offset: Offset32, ty: Type) -> AMode {
        AMode::RegOffset(base, i64::from(offset), ty)
    }

    fn gen_const_amode(&mut self, c: VCodeConstant) -> AMode {
        AMode::Const(c)
    }

    fn valid_atomic_transaction(&mut self, ty: Type) -> Option<Type> {
        if ty.is_int() && ty.bits() <= 64 {
            Some(ty)
        } else {
            None
        }
    }
    fn is_atomic_rmw_max_etc(&mut self, op: &AtomicRmwOp) -> Option<(AtomicRmwOp, bool)> {
        let op = *op;
        match op {
            crate::ir::AtomicRmwOp::Umin => Some((op, false)),
            crate::ir::AtomicRmwOp::Umax => Some((op, false)),
            crate::ir::AtomicRmwOp::Smin => Some((op, true)),
            crate::ir::AtomicRmwOp::Smax => Some((op, true)),
            _ => None,
        }
    }
    fn load_op(&mut self, ty: Type) -> LoadOP {
        LoadOP::from_type(ty)
    }
    fn store_op(&mut self, ty: Type) -> StoreOP {
        StoreOP::from_type(ty)
    }
    fn load_ext_name(&mut self, name: ExternalName, offset: i64) -> Reg {
        let tmp = self.temp_writable_reg(I64);
        self.emit(&MInst::LoadExtName {
            rd: tmp,
            name: Box::new(name),
            offset,
        });
        tmp.to_reg()
    }

    fn offset32_add(&mut self, a: Offset32, adden: i64) -> Offset32 {
        a.try_add_i64(adden).expect("offset exceed range.")
    }

    fn gen_stack_addr(&mut self, slot: StackSlot, offset: Offset32) -> Reg {
        let result = self.temp_writable_reg(I64);
        let i = self
            .lower_ctx
            .abi()
            .sized_stackslot_addr(slot, i64::from(offset) as u32, result);
        self.emit(&i);
        result.to_reg()
    }
    fn atomic_amo(&mut self) -> AMO {
        AMO::SeqCst
    }

    fn lower_br_table(&mut self, index: Reg, targets: &VecMachLabel) -> Unit {
        let tmp1 = self.temp_writable_reg(I64);
        let tmp2 = self.temp_writable_reg(I64);
        let targets: Vec<BranchTarget> = targets
            .into_iter()
            .copied()
            .map(BranchTarget::Label)
            .collect();
        self.emit(&MInst::BrTable {
            index,
            tmp1,
            tmp2,
            targets,
        });
    }

    fn fp_reg(&mut self) -> PReg {
        px_reg(8)
    }

    fn sp_reg(&mut self) -> PReg {
        px_reg(2)
    }

    fn shift_int_to_most_significant(&mut self, v: XReg, ty: Type) -> XReg {
        assert!(ty.is_int() && ty.bits() <= 64);
        if ty == I64 {
            return v;
        }
        let tmp = self.temp_writable_reg(I64);
        self.emit(&MInst::AluRRImm12 {
            alu_op: AluOPRRI::Slli,
            rd: tmp,
            rs: v.to_reg(),
            imm12: Imm12::from_bits((64 - ty.bits()) as i16),
        });

        self.xreg_new(tmp.to_reg())
    }

    #[inline]
    fn int_compare(&mut self, kind: &IntCC, rs1: XReg, rs2: XReg) -> IntegerCompare {
        IntegerCompare {
            kind: *kind,
            rs1: rs1.to_reg(),
            rs2: rs2.to_reg(),
        }
    }

    #[inline]
    fn vstate_from_type(&mut self, ty: Type) -> VState {
        VState::from_type(ty)
    }

    #[inline]
    fn vstate_mf2(&mut self, vs: VState) -> VState {
        VState {
            vtype: VType {
                lmul: VecLmul::LmulF2,
                ..vs.vtype
            },
            ..vs
        }
    }

    fn min_vec_reg_size(&mut self) -> u64 {
        self.min_vec_reg_size
    }

    #[inline]
    fn ty_vec_fits_in_register(&mut self, ty: Type) -> Option<Type> {
        if ty.is_vector() && (ty.bits() as u64) <= self.min_vec_reg_size() {
            Some(ty)
        } else {
            None
        }
    }

    fn vec_alu_rr_dst_type(&mut self, op: &VecAluOpRR) -> Type {
        MInst::canonical_type_for_rc(op.dst_regclass())
    }
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &Riscv64Backend,
    inst: Inst,
) -> Option<InstOutput> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = RV64IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

/// The main entry point for branch lowering with ISLE.
pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &Riscv64Backend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = RV64IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower_branch(&mut isle_ctx, branch, &targets.to_vec())
}

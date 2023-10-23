//! ISLE integration glue code for zkasm lowering.

// Pull in the ISLE generated code.
#[allow(unused)]
pub mod generated_code;
use generated_code::{Context, ExtendOp, MInst};

// Types that the generated ISLE code uses via `use super::*`.
use super::{writable_zero_reg, zero_reg};
use crate::isa::zkasm::abi::ZkAsmABICallSite;
use crate::isa::zkasm::lower::args::{WritableXReg, XReg};
use crate::isa::zkasm::ZkAsmBackend;
use crate::machinst::Reg;
use crate::machinst::{isle::*, MachInst, SmallInstVec};
use crate::machinst::{VCodeConstant, VCodeConstantData};
use crate::{
    ir::{
        immediates::*, types::*, BlockCall, ExternalName, Inst, InstructionData, MemFlags,
        StackSlot, TrapCode, Value, ValueList,
    },
    isa::zkasm::inst::*,
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

pub(crate) struct ZkAsmIsleContext<'a, 'b, I, B>
where
    I: VCodeInst,
    B: LowerBackend,
{
    pub lower_ctx: &'a mut Lower<'b, I>,
    pub backend: &'a B,
}

impl<'a, 'b> ZkAsmIsleContext<'a, 'b, MInst, ZkAsmBackend> {
    isle_prelude_method_helpers!(ZkAsmABICallSite);

    fn new(lower_ctx: &'a mut Lower<'b, MInst>, backend: &'a ZkAsmBackend) -> Self {
        Self { lower_ctx, backend }
    }

    #[inline]
    fn emit_list(&mut self, list: &SmallInstVec<MInst>) {
        for i in list {
            self.lower_ctx.emit(i.clone());
        }
    }
}

impl generated_code::Context for ZkAsmIsleContext<'_, '_, MInst, ZkAsmBackend> {
    isle_lower_prelude_methods!();
    isle_prelude_caller_methods!(zkAsmMachineDeps, ZkAsmABICallSite);

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

        let call_site = ZkAsmABICallSite::from_func(
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

        let call_site = ZkAsmABICallSite::from_ptr(
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
    fn imm32_from_u64(&mut self, arg0: u64) -> Option<Imm32> {
        Imm32::maybe_from_u64(arg0)
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
    fn zero_reg(&mut self) -> Reg {
        zero_reg()
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

    //
    fn gen_shamt(&mut self, _ty: Type, _shamt: XReg) -> ValueRegs {
        todo!()
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

    fn shift_int_to_most_significant(&mut self, _v: XReg, _ty: Type) -> XReg {
        todo!()
    }

    #[inline]
    fn int_compare(&mut self, kind: &IntCC, rs1: XReg, rs2: XReg) -> IntegerCompare {
        IntegerCompare {
            kind: *kind,
            rs1: rs1.to_reg(),
            rs2: rs2.to_reg(),
        }
    }
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &ZkAsmBackend,
    inst: Inst,
) -> Option<InstOutput> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = ZkAsmIsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

/// The main entry point for branch lowering with ISLE.
pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &ZkAsmBackend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = ZkAsmIsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower_branch(&mut isle_ctx, branch, &targets.to_vec())
}

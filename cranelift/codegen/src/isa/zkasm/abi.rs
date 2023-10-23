//! Implementation of a standard zkASM ABI.

use std::sync::OnceLock;

use crate::ir;
use crate::ir::types::*;

use crate::ir::ExternalName;
use crate::ir::MemFlags;
use crate::isa;

use crate::isa::zkasm::{inst::EmitState, inst::*};
use crate::isa::CallConv;
use crate::machinst::*;

use crate::ir::LibCall;
use crate::ir::Signature;
use crate::isa::zkasm::settings::Flags;
use crate::settings;
use crate::CodegenError;
use crate::CodegenResult;
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc2::PRegSet;
use regs::{create_reg_environment, x_reg};

use smallvec::{smallvec, SmallVec};

/// Support for the zkASM ABI from the callee side (within a function body).
pub(crate) type ZkAsmCallee = Callee<ZkAsmMachineDeps>;

/// Support for the zkASM ABI from the caller side (at a callsite).
pub(crate) type ZkAsmABICallSite = CallSite<ZkAsmMachineDeps>;

/// This is the limit for the size of argument and return-value areas on the
/// stack. We place a reasonable limit here to avoid integer overflow issues
/// with 32-bit arithmetic: for now, 128 MB.
static STACK_ARG_RET_SIZE_LIMIT: u32 = 128 * 1024 * 1024;

/// zkASM-specific ABI behavior. This struct just serves as an implementation
/// point for the trait; it is never actually instantiated.
pub struct ZkAsmMachineDeps;

impl IsaFlags for Flags {}

impl ABIMachineSpec for ZkAsmMachineDeps {
    type I = Inst;
    type F = Flags;

    fn word_bits() -> u32 {
        64
    }

    /// Return required stack alignment in bytes.
    fn stack_align(_call_conv: isa::CallConv) -> u32 {
        8
    }

    fn compute_arg_locs<'a, I>(
        call_conv: isa::CallConv,
        _flags: &settings::Flags,
        params: I,
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        mut args: ArgsAccumulator<'_>,
    ) -> CodegenResult<(u32, Option<usize>)>
    where
        I: IntoIterator<Item = &'a ir::AbiParam>,
    {
        // FIXME(nagisa): this code needs to be rewritten for zkasm
        //
        // All registers that can be used as parameters or rets.
        // both start and end are included.
        let (x_start, x_end) = match (call_conv, args_or_rets) {
            (isa::CallConv::Tail, _) => (10, 11),
            (_, ArgsOrRets::Args) => (10, 11),
            (_, ArgsOrRets::Rets) => (10, 11),
        };
        let mut next_x_reg = x_start;
        // Stack space.
        let mut next_stack: u32 = 0;

        for param in params {
            if let ir::ArgumentPurpose::StructArgument(size) = param.purpose {
                let offset = next_stack;
                assert!(size % 8 == 0, "StructArgument size is not properly aligned");
                next_stack += size;
                args.push(ABIArg::StructArg {
                    pointer: None,
                    offset: offset as i64,
                    size: size as u64,
                    purpose: param.purpose,
                });
                continue;
            }

            // For now we pin VMContext register to `CTX` register of ZK ASM.
            if let ir::ArgumentPurpose::VMContext = param.purpose {
                let mut slots = ABIArgSlotVec::new();
                slots.push(ABIArgSlot::Reg {
                    reg: context_reg().to_real_reg().unwrap(),
                    ty: I32,
                    extension: param.extension,
                });
                args.push(ABIArg::Slots {
                    slots,
                    purpose: param.purpose,
                });
                continue;
            }

            // Find regclass(es) of the register(s) used to store a value of this type.
            let (rcs, reg_tys) = Inst::rc_for_type(param.value_type)?;
            let mut slots = ABIArgSlotVec::new();
            for (rc, reg_ty) in rcs.iter().zip(reg_tys.iter()) {
                let next_reg = if (next_x_reg <= x_end) && *rc == RegClass::Int {
                    let x = Some(x_reg(next_x_reg));
                    next_x_reg += 1;
                    x
                } else {
                    None
                };
                if let Some(reg) = next_reg {
                    slots.push(ABIArgSlot::Reg {
                        reg: reg.to_real_reg().unwrap(),
                        ty: *reg_ty,
                        extension: param.extension,
                    });
                } else {
                    // Compute size and 16-byte stack alignment happens
                    // separately after all args.
                    let size = reg_ty.bits() / 8;
                    let size = std::cmp::max(size, 8);
                    // Align.
                    debug_assert!(size.is_power_of_two());
                    next_stack = align_to(next_stack, size);
                    slots.push(ABIArgSlot::Stack {
                        offset: next_stack as i64,
                        ty: *reg_ty,
                        extension: param.extension,
                    });
                    next_stack += size;
                }
            }
            args.push(ABIArg::Slots {
                slots,
                purpose: param.purpose,
            });
        }
        let pos: Option<usize> = if add_ret_area_ptr {
            assert!(ArgsOrRets::Args == args_or_rets);
            if next_x_reg <= x_end {
                let arg = ABIArg::reg(
                    x_reg(next_x_reg).to_real_reg().unwrap(),
                    I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                );
                args.push(arg);
            } else {
                let arg = ABIArg::stack(
                    next_stack as i64,
                    I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                );
                args.push(arg);
                next_stack += 8;
            }
            Some(args.args().len() - 1)
        } else {
            None
        };

        next_stack = align_to(next_stack, Self::stack_align(call_conv));

        // To avoid overflow issues, limit the arg/return size to something
        // reasonable -- here, 128 MB.
        if next_stack > STACK_ARG_RET_SIZE_LIMIT {
            return Err(CodegenError::ImplLimitExceeded);
        }

        Ok((next_stack, pos))
    }

    fn fp_to_arg_offset(_call_conv: isa::CallConv, _flags: &settings::Flags) -> i64 {
        // lr fp.
        16
    }

    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: Type) -> Inst {
        Inst::gen_load(into_reg, mem.into(), ty, MemFlags::trusted())
    }

    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type) -> Inst {
        Inst::gen_store(mem.into(), from_reg, ty, MemFlags::trusted())
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Inst {
        Inst::gen_move(to_reg, from_reg, ty)
    }

    fn gen_extend(
        to_reg: Writable<Reg>,
        from_reg: Reg,
        signed: bool,
        from_bits: u8,
        to_bits: u8,
    ) -> Inst {
        assert!(from_bits < to_bits);
        Inst::Extend {
            rd: to_reg,
            rn: from_reg,
            signed,
            from_bits,
            to_bits,
        }
    }

    fn gen_args(args: Vec<ArgPair>) -> Self::I {
        Inst::Args { args }
    }

    fn gen_rets(rets: Vec<RetPair>) -> Self::I {
        Inst::Ret {
            rets,
            stack_bytes_to_pop: 0,
        }
    }

    fn gen_add_imm(
        _call_conv: isa::CallConv,
        into_reg: Writable<Reg>,
        from_reg: Reg,
        imm: u32,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        if let Some(_imm12) = Imm12::maybe_from_u64(imm as u64) {
            todo!();
        } else {
            insts.extend(Inst::load_constant_u32(
                writable_spilltmp_reg2(),
                imm as u64,
                &mut |_| writable_spilltmp_reg2(),
            ));
            insts.push(Inst::AluRRR {
                alu_op: AluOPRRR::Add,
                rd: into_reg,
                rs1: spilltmp_reg2(),
                rs2: from_reg,
            });
        }
        insts
    }

    fn gen_stack_lower_bound_trap(limit_reg: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        insts.push(Inst::TrapIfC {
            cc: IntCC::UnsignedLessThan,
            rs1: stack_reg(),
            rs2: limit_reg,
            trap_code: ir::TrapCode::StackOverflow,
        });
        insts
    }

    fn gen_get_stack_addr(mem: StackAMode, into_reg: Writable<Reg>, _ty: Type) -> Inst {
        Inst::LoadAddr {
            rd: into_reg,
            mem: mem.into(),
        }
    }

    fn get_stacklimit_reg(_call_conv: isa::CallConv) -> Reg {
        spilltmp_reg()
    }

    fn gen_load_base_offset(into_reg: Writable<Reg>, base: Reg, offset: i32, ty: Type) -> Inst {
        let mem = AMode::RegOffset(base, offset as i64, ty);
        Inst::gen_load(into_reg, mem, ty, MemFlags::trusted())
    }

    fn gen_store_base_offset(base: Reg, offset: i32, from_reg: Reg, ty: Type) -> Inst {
        let mem = AMode::RegOffset(base, offset as i64, ty);
        Inst::gen_store(mem, from_reg, ty, MemFlags::trusted())
    }

    fn gen_sp_reg_adjust(amount: i32) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        if amount == 0 {
            return insts;
        }
        // FIXME: is this right/sufficient for stack growing up
        insts.push(if amount < 0 {
            Inst::ReserveSp {
                amount: -amount as u64,
            }
        } else {
            Inst::ReleaseSp {
                amount: amount as u64,
            }
        });
        insts
    }

    fn gen_nominal_sp_adj(offset: i32) -> Inst {
        Inst::VirtualSPOffsetAdj {
            amount: offset as i64,
        }
    }

    fn compute_frame_layout(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        _sig: &Signature,
        regs: &[Writable<RealReg>],
        is_leaf: bool,
        stack_args_size: u32,
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> FrameLayout {
        let mut regs: Vec<Writable<RealReg>> = regs
            .iter()
            .cloned()
            .filter(|r| is_reg_saved_in_prologue(call_conv, r.to_reg()))
            .collect();

        regs.sort();

        // Compute clobber size.
        let clobber_size = compute_clobber_size(&regs);

        // Compute linkage frame size.
        let setup_area_size = if flags.preserve_frame_pointers()
            || !is_leaf
            // FIXME: we don’t currently maintain a frame pointer?
            // The function arguments that are passed on the stack are addressed
            // relative to the Frame Pointer.
            || stack_args_size > 0
            || clobber_size > 0
            || fixed_frame_storage_size > 0
        {
            8 // RR
        } else {
            0
        };

        // Return FrameLayout structure.
        debug_assert!(outgoing_args_size == 0);
        FrameLayout {
            stack_args_size,
            setup_area_size,
            clobber_size,
            fixed_frame_storage_size,
            outgoing_args_size,
            clobbered_callee_saves: regs,
        }
    }

    fn gen_prologue_frame_setup(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Self::F,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        let mut insts = SmallVec::new();

        if frame_layout.setup_area_size > 0 {
            insts.push(Inst::ReserveSp {
                amount: frame_layout.setup_area_size.into(),
            });
            insts.push(Self::gen_store_stack(
                StackAMode::SPOffset(-1, I64),
                link_reg(),
                I64,
            ));
        }

        insts
    }

    fn gen_epilogue_frame_restore(
        call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Self::F,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        let mut insts = SmallVec::new();

        if frame_layout.setup_area_size > 0 {
            insts.push(Self::gen_load_stack(
                StackAMode::SPOffset(-1, I64),
                writable_link_reg(),
                I64,
            ));
            insts.push(Inst::ReleaseSp {
                amount: frame_layout.setup_area_size.into(),
            });
        }

        if call_conv == isa::CallConv::Tail {
            todo!()
        }
        insts.push(Inst::Ret {
            rets: vec![],
            stack_bytes_to_pop: 0,
        });

        insts
    }

    fn gen_probestack(_insts: &mut SmallInstVec<Self::I>, _frame_size: u32) {
        todo!()
    }

    fn gen_inline_probestack(
        _insts: &mut SmallInstVec<Self::I>,
        _call_conv: isa::CallConv,
        _frame_size: u32,
        _guard_size: u32,
    ) {
        todo!()
    }

    fn gen_clobber_save(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        let mut insts = SmallVec::new();
        // Adjust the stack pointer upward for clobbers and the function fixed
        // frame (spillslots and storage slots).
        let stack_size = frame_layout.fixed_frame_storage_size + frame_layout.clobber_size;
        // Store each clobbered register in order at offsets from SP,
        // placing them above the fixed frame slots.
        if stack_size > 0 {
            insts.push(Inst::ReserveSp {
                amount: stack_size.into(),
            });
            let mut cur_offset = -1;
            for reg in &frame_layout.clobbered_callee_saves {
                let r_reg = reg.to_reg();
                let ty = match r_reg.class() {
                    RegClass::Int => I64,
                    RegClass::Float => F64,
                    RegClass::Vector => unimplemented!("Vector Clobber Saves"),
                };
                insts.push(Self::gen_store_stack(
                    StackAMode::SPOffset(cur_offset, ty),
                    real_reg_to_reg(reg.to_reg()),
                    ty,
                ));
                cur_offset -= 1
            }
        }
        insts
    }

    fn gen_clobber_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        let mut insts = SmallVec::new();
        let stack_size = frame_layout.fixed_frame_storage_size + frame_layout.clobber_size;
        if stack_size > 0 {
            let mut cur_offset = -1;
            for reg in &frame_layout.clobbered_callee_saves {
                let rreg = reg.to_reg();
                let ty = match rreg.class() {
                    RegClass::Int => I64,
                    RegClass::Float => F64,
                    RegClass::Vector => unimplemented!("Vector Clobber Restores"),
                };
                insts.push(Self::gen_load_stack(
                    StackAMode::SPOffset(cur_offset, ty),
                    Writable::from_reg(real_reg_to_reg(reg.to_reg())),
                    ty,
                ));
                cur_offset -= 1
            }
            insts.push(Inst::ReleaseSp {
                amount: stack_size.into(),
            });
        }
        insts
    }

    fn gen_call(
        dest: &CallDest,
        uses: CallArgList,
        defs: CallRetList,
        clobbers: PRegSet,
        opcode: ir::Opcode,
        _tmp: Writable<Reg>,
        callee_conv: isa::CallConv,
        caller_conv: isa::CallConv,
        callee_pop_size: u32,
    ) -> SmallVec<[Self::I; 2]> {
        let mut insts = SmallVec::new();
        match &dest {
            &CallDest::ExtName(ref name, _) => insts.push(Inst::Call {
                info: Box::new(CallInfo {
                    dest: name.clone(),
                    uses,
                    defs,
                    clobbers,
                    opcode,
                    caller_callconv: caller_conv,
                    callee_callconv: callee_conv,
                    callee_pop_size,
                }),
            }),
            &CallDest::Reg(reg) => insts.push(Inst::CallInd {
                info: Box::new(CallIndInfo {
                    rn: *reg,
                    uses,
                    defs,
                    clobbers,
                    opcode,
                    caller_callconv: caller_conv,
                    callee_callconv: callee_conv,
                    callee_pop_size,
                }),
            }),
        }
        insts
    }

    fn gen_memcpy<F: FnMut(Type) -> Writable<Reg>>(
        call_conv: isa::CallConv,
        dst: Reg,
        src: Reg,
        size: usize,
        mut alloc_tmp: F,
    ) -> SmallVec<[Self::I; 8]> {
        let mut insts = SmallVec::new();
        let arg0 = Writable::from_reg(x_reg(10));
        let arg1 = Writable::from_reg(x_reg(11));
        let arg2 = Writable::from_reg(x_reg(12));
        let tmp = alloc_tmp(Self::word_type());
        insts.extend(Inst::load_constant_u64(tmp, size as u64, &mut alloc_tmp).into_iter());
        insts.push(Inst::Call {
            info: Box::new(CallInfo {
                dest: ExternalName::LibCall(LibCall::Memcpy),
                uses: smallvec![
                    CallArgPair {
                        vreg: dst,
                        preg: arg0.to_reg()
                    },
                    CallArgPair {
                        vreg: src,
                        preg: arg1.to_reg()
                    },
                    CallArgPair {
                        vreg: tmp.to_reg(),
                        preg: arg2.to_reg()
                    }
                ],
                defs: smallvec![],
                clobbers: Self::get_regs_clobbered_by_call(call_conv),
                opcode: Opcode::Call,
                caller_callconv: call_conv,
                callee_callconv: call_conv,
                callee_pop_size: 0,
            }),
        });
        insts
    }

    fn get_number_of_spillslots_for_value(
        rc: RegClass,
        _target_vector_bytes: u32,
        _isa_flags: &Flags,
    ) -> u32 {
        // We allocate in terms of 8-byte slots.
        match rc {
            RegClass::Int => 1,
            RegClass::Float => 1,
            RegClass::Vector => todo!(),
        }
    }

    /// Get the current virtual-SP offset from an instruction-emission state.
    fn get_virtual_sp_offset_from_state(s: &EmitState) -> i64 {
        s.virtual_sp_offset
    }

    /// Get the nominal-SP-to-FP offset from an instruction-emission state.
    fn get_nominal_sp_to_fp(s: &EmitState) -> i64 {
        s.nominal_sp_to_fp
    }

    fn get_machine_env(
        _flags: &settings::Flags,
        _call_conv: isa::CallConv,
    ) -> &regalloc2::MachineEnv {
        static MACHINE_ENV: OnceLock<regalloc2::MachineEnv> = OnceLock::new();
        MACHINE_ENV.get_or_init(create_reg_environment)
    }

    fn get_regs_clobbered_by_call(_call_conv_of_callee: isa::CallConv) -> PRegSet {
        PRegSet::empty()
    }

    fn get_ext_mode(
        _call_conv: isa::CallConv,
        specified: ir::ArgumentExtension,
    ) -> ir::ArgumentExtension {
        specified
    }
}

impl ZkAsmABICallSite {
    pub fn emit_return_call(mut self, ctx: &mut Lower<Inst>, args: isle::ValueSlice) {
        let (new_stack_arg_size, old_stack_arg_size) =
            self.emit_temporary_tail_call_frame(ctx, args);

        let dest = self.dest().clone();
        let opcode = self.opcode();
        let uses = self.take_uses();
        let info = Box::new(ReturnCallInfo {
            uses,
            opcode,
            old_stack_arg_size,
            new_stack_arg_size,
        });

        match dest {
            CallDest::ExtName(name, RelocDistance::Near) => {
                ctx.emit(Inst::ReturnCall {
                    callee: Box::new(name),
                    info,
                });
            }
            CallDest::ExtName(name, RelocDistance::Far) => {
                let callee = ctx.alloc_tmp(ir::types::I64).only_reg().unwrap();
                ctx.emit(Inst::LoadExtName {
                    rd: callee,
                    name: Box::new(name),
                    offset: 0,
                });
                ctx.emit(Inst::ReturnCallInd {
                    callee: callee.to_reg(),
                    info,
                });
            }
            CallDest::Reg(callee) => ctx.emit(Inst::ReturnCallInd { callee, info }),
        }
    }
}

/// This should be the registers that must be saved by callee.
#[inline]
fn is_reg_saved_in_prologue(conv: CallConv, reg: RealReg) -> bool {
    if conv == CallConv::Tail {
        todo!()
    }
    // TODO(akashin): Figure out the correct calling convention.
    match reg.class() {
        // FIXME(#45): Register A for returns? Find where in the code is that defined.
        RegClass::Int if reg.hw_enc() == 10 => false,
        RegClass::Int => true,
        RegClass::Float => todo!(),
        RegClass::Vector => todo!(),
    }
}

fn compute_clobber_size(clobbers: &[Writable<RealReg>]) -> u32 {
    let mut clobbered_size = 0;
    for reg in clobbers {
        match reg.to_reg().class() {
            RegClass::Int => {
                clobbered_size += 8;
            }
            RegClass::Float => unimplemented!("floats are not supported"),
            RegClass::Vector => unimplemented!("vectors are not supported"),
        }
    }
    align_to(clobbered_size, 16)
}

//! Implementation of a standard AArch64 ABI.

use core::panic;

use crate::ir;

use crate::ir::types::*;
use crate::ir::MemFlags;
use crate::ir::Opcode;
use crate::ir::{ExternalName, LibCall};
use crate::isa;
use crate::isa::risc_v::inst::get_caller_save_register;
use crate::isa::risc_v::{inst::EmitState, inst::*};
use crate::isa::CallConv;
use crate::machinst::*;
use crate::machinst::*;
use crate::settings;
use crate::CodegenResult;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use smallvec::{smallvec, SmallVec};

// We use a generic implementation that factors out AArch64 and x64 ABI commonalities, because
// these ABIs are very similar.

/// Support for the AArch64 ABI from the callee side (within a function body).
pub(crate) type Riscv64Callee = ABICalleeImpl<Riscv64MachineDeps>;

/// Support for the AArch64 ABI from the caller side (at a callsite).
pub(crate) type Riscv64ABICaller = ABICallerImpl<Riscv64MachineDeps>;

//todo Spidermonkey specific ABI convention.

// /// This is SpiderMonkey's `WasmTableCallSigReg`.
// static BALdRDASh_SIG_REG: u8 = 10;

// /// This is SpiderMonkey's `WasmTlsReg`.
// static BALdRDASh_TLS_REG: u8 = 23;

// /// Offset in stack-arg area to callee-TLS slot in Baldrdash-2020 calling convention.
// static BALdRDASh_CALLEE_TLS_OFFSET: i64 = 0;
// /// Offset in stack-arg area to caller-TLS slot in Baldrdash-2020 calling convention.
// static BALdRDASh_CALLER_TLS_OFFSET: i64 = 8;

// These two lists represent the registers the JIT may *not* use at any point in generated code.
//
// So these are callee-preserved from the JIT's point of view, and every register not in this list
// has to be caller-preserved by definition.
//
// Keep these lists in sync with the NonAllocatableMask set in Spidermonkey's
// Architecture-arm64.cpp.

/// This is the limit for the size of argument and return-value areas on the
/// stack. We place a reasonable limit here to avoid integer overflow issues
/// with 32-bit arithmetic: for now, 128 MB.
static STACK_ARG_RET_SIZE_LIMIT: u64 = 128 * 1024 * 1024;

/// AArch64-specific ABI behavior. This struct just serves as an implementation
/// point for the trait; it is never actually instantiated.
pub(crate) struct Riscv64MachineDeps;

impl ABIMachineSpec for Riscv64MachineDeps {
    type I = Inst;

    fn word_bits() -> u32 {
        64
    }

    /// Return required stack alignment in bytes.
    fn stack_align(_call_conv: isa::CallConv) -> u32 {
        16
    }

    fn compute_arg_locs(
        call_conv: isa::CallConv,
        _flags: &settings::Flags,
        params: &[ir::AbiParam],
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
    ) -> CodegenResult<(Vec<ABIArg>, i64, Option<usize>)> {
        let x_registers = a0_t0_a7();
        if args_or_rets == ArgsOrRets::Rets {
            a0_t0_a7().resize(2, Writable::invalid_sentinel());
        }
        let f_registers = fa0_to_fa7();
        if args_or_rets == ArgsOrRets::Rets {
            fa0_to_fa7().resize(2, Writable::invalid_sentinel());
        }

        let mut next_stack: i64 = 0;
        let mut next_x_reg = 0;
        let mut next_f_reg = 0;

        let mut abi_args = vec![];
        for i in 0..params.len() {
            let param = params[i];
            // Validate "purpose".
            match &param.purpose {
                &ir::ArgumentPurpose::VMContext
                | &ir::ArgumentPurpose::Normal
                | &ir::ArgumentPurpose::StackLimit
                | &ir::ArgumentPurpose::SignatureId
                | &ir::ArgumentPurpose::CalleeTLS
                | &ir::ArgumentPurpose::CallerTLS
                | &ir::ArgumentPurpose::StructReturn
                | &ir::ArgumentPurpose::FramePointer
                | &ir::ArgumentPurpose::StructArgument(_) => {}
                _ => panic!(
                    "Unsupported argument purpose {:?} in signature: {:?}",
                    param.purpose, params
                ),
            }

            if let ir::ArgumentPurpose::StructArgument(size) = param.purpose {
                let offset = next_stack;
                assert!(size % 8 == 0, "StructArgument size is not properly aligned");
                next_stack += size as i64;
                abi_args.push(ABIArg::StructArg {
                    offset,
                    size: size as u64,
                    purpose: param.purpose,
                });
                continue;
            }
            match param.value_type {
                F32 | F64 => {
                    if next_f_reg < f_registers.len() {
                        let reg = f_registers[next_f_reg].clone();
                        let arg = ABIArg::reg(
                            reg.to_reg().to_real_reg().unwrap(),
                            param.value_type,
                            param.extension,
                            param.purpose,
                        );
                        abi_args.push(arg);
                        next_f_reg += 1;
                    } else {
                        let arg = ABIArg::stack(
                            next_stack,
                            param.value_type,
                            param.extension,
                            param.purpose,
                        );
                        abi_args.push(arg);
                        next_stack += 8
                    }
                }
                B1 | B8 | B16 | B32 | B64 | I8 | I16 | I32 | I64 | R32 | R64 => {
                    if next_x_reg < x_registers.len() {
                        let reg = x_registers[next_x_reg].clone();
                        let arg = ABIArg::reg(
                            reg.to_reg().to_real_reg().unwrap(),
                            param.value_type,
                            param.extension,
                            param.purpose,
                        );
                        abi_args.push(arg);
                        next_x_reg += 1;
                    } else {
                        let arg = ABIArg::stack(
                            next_stack,
                            param.value_type,
                            param.extension,
                            param.purpose,
                        );
                        abi_args.push(arg);
                        next_stack += 8
                    }
                }
                _ => todo!("type not supported {}", param.value_type),
            };
        }

        let pos: Option<usize> = if add_ret_area_ptr {
            assert!(ArgsOrRets::Rets == args_or_rets);
            if next_stack > 0 {
                let arg = ABIArg::stack(
                    next_stack,
                    I64,
                    ir::ArgumentExtension::Uext,
                    ir::ArgumentPurpose::Normal,
                );
                abi_args.push(arg);
                next_stack += 8;
                Some(abi_args.len() - 1)
            } else {
                None
            }
        } else {
            None
        };
        next_stack = align_to(next_stack, Self::stack_align(call_conv) as i64);
        CodegenResult::Ok((abi_args, next_stack, pos))
    }

    fn fp_to_arg_offset(_call_conv: isa::CallConv, _flags: &settings::Flags) -> i64 {
        8
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
        _to_reg: Writable<Reg>,
        _from_reg: Reg,
        _signed: bool,
        _from_bits: u8,
        _to_bits: u8,
    ) -> Inst {
        // assert!(from_bits < to_bits);
        // if let Some(x) = ExtendOp::from_extend_args(signed, from_bits, to_bits) {
        //     Inst::Extend {
        //         rd: to_reg,
        //         rn: from_reg,
        //         op: x,
        //     }
        // } else {
        //     Inst::Mov {
        //         rd: to_reg,
        //         rm: from_reg,
        //         ty: I64,
        //     }
        // }
        unreachable!("no need to extend integer")
    }

    fn get_ext_mode(
        _call_conv: isa::CallConv,
        _specified: ir::ArgumentExtension,
    ) -> ir::ArgumentExtension {
        /*
            todo:: right?????
            risc-v will sign for you anyway.

            if number is unsigned we should leave the bits no touch at all.
                like "lb"
                Format
                    lbu rd,offset(rs1)
                Description
                    Loads a 8-bit value from memory and zero-extends this to XLEN bits before storing it in register rd.
                Implementation
                    x[rd] = M[x[rs1] + sext(offset)][7:0]
            risc-v isa will sign the value for you always.

            if number is singed the value is always extend to 64-bit.
                lb

        */
        ir::ArgumentExtension::None
    }

    fn gen_ret(rets: Vec<Reg>) -> Inst {
        Inst::Ret
    }

    fn gen_add_imm(into_reg: Writable<Reg>, from_reg: Reg, imm: u32) -> SmallInstVec<Inst> {
        let mut insts = Inst::load_constant_u32(into_reg, imm);
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd: into_reg,
            rs1: into_reg.to_reg(),
            rs2: from_reg,
        });
        insts
    }

    fn gen_stack_lower_bound_trap(limit_reg: Reg) -> SmallInstVec<Inst> {
        // let mut insts = SmallVec::new();
        // insts.push(Inst::TrapIf {
        //     rs1: stacklimit_reg(),
        //     rs2: limit_reg,
        //     cond: IntCC::SignedLessThan,
        //     trap_code: ir::TrapCode::StackOverflow,
        // });
        // insts
        unimplemented!()
    }

    fn gen_epilogue_placeholder() -> Inst {
        Inst::EpiloguePlaceholder
    }

    fn gen_get_stack_addr(mem: StackAMode, into_reg: Writable<Reg>, _ty: Type) -> Inst {
        Inst::LoadAddr {
            rd: into_reg,
            mem: mem.into(),
        }
    }

    fn get_stacklimit_reg() -> Reg {
        stacklimit_reg()
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
        insts.push(Inst::AjustSp {
            amount: amount as i64,
        });
        insts
    }

    fn gen_nominal_sp_adj(offset: i32) -> Inst {
        Inst::VirtualSPOffsetAdj {
            amount: offset as i64,
        }
    }

    fn gen_debug_frame_info(
        flags: &settings::Flags,
        _isa_flags: &Vec<settings::Value>,
    ) -> SmallInstVec<Inst> {
        smallvec![]
    }

    // add  sp , sp-8   ;; alloc stack sapce for fp
    // st   fp , sp+0   ;; store old fp
    // move fp , sp    ;; set fp to sp
    fn gen_prologue_frame_setup(_flags: &settings::Flags) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        insts.push(Inst::AjustSp {
            amount: -(Self::word_bytes() as i64),
        });
        insts.push(Self::gen_store_stack(
            StackAMode::SPOffset(0, I64),
            fp_reg(),
            I64,
        ));
        insts.push(Inst::Mov {
            rd: writable_fp_reg(),
            rm: stack_reg(),
            ty: I64,
        });
        insts
    }

    //st fp , sp  ;; restore fp
    //add sp, sp+8  ;; desalloc stack sapce for fp
    fn gen_epilogue_frame_restore(_: &settings::Flags) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        insts.push(Self::gen_load_stack(
            StackAMode::SPOffset(0, I64),
            writable_fp_reg(),
            I64,
        ));
        insts.push(Inst::AjustSp {
            amount: Self::word_bytes() as i64,
        });
        insts
    }

    fn gen_probestack(_: u32) -> SmallInstVec<Self::I> {
        // TODO: I don't know this means.
        smallvec![]
    }

    // Returns stack bytes used as well as instructions. Does not adjust
    // nominal SP offset; abi_impl generic code will do that.
    fn gen_clobber_save(
        call_conv: isa::CallConv,
        setup_frame: bool,
        flags: &settings::Flags,
        clobbered_callee_saves: &[Writable<RealReg>],
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> (u64, SmallVec<[Inst; 16]>) {
        let mut insts = SmallVec::new();
        let clobbered_size = compute_clobber_size(&clobbered_callee_saves);
        // Adjust the stack pointer downward for clobbers and the function fixed
        // frame (spillslots and storage slots).
        let stack_size = fixed_frame_storage_size + clobbered_size;
        // Store each clobbered register in order at offsets from RSP,
        // placing them above the fixed frame slots.
        let mut cur_offset = fixed_frame_storage_size;
        for reg in clobbered_callee_saves {
            let r_reg = reg.to_reg();
            let off = cur_offset;
            let _type = if r_reg.class() == RegClass::Int {
                I64
            } else {
                F64
            };
            insts.push(Self::gen_store_stack(
                StackAMode::FPOffset(-(off as i64), _type),
                real_reg_to_reg(reg.to_reg()),
                _type,
            ));
            cur_offset += 8
        }
        if stack_size > 0 {
            insts.push(Inst::AjustSp {
                amount: -(stack_size as i64),
            });
        }
        (clobbered_size as u64, insts)
    }

    fn gen_clobber_restore(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        clobbers: &[Writable<RealReg>],
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> SmallVec<[Inst; 16]> {
        let mut insts = SmallVec::new();
        let clobbered_callee_saves = Self::get_clobbered_callee_saves(call_conv, clobbers);
        let stack_size = fixed_frame_storage_size + compute_clobber_size(&clobbered_callee_saves);
        let mut cur_offset = fixed_frame_storage_size;
        for reg in &clobbered_callee_saves {
            let rreg = reg.to_reg();
            let _type = if rreg.class() == RegClass::Int {
                I64
            } else {
                F64
            };
            insts.push(Self::gen_load_stack(
                StackAMode::FPOffset(cur_offset as i64, _type),
                Writable::from_reg(real_reg_to_reg(reg.to_reg())),
                _type,
            ));
            cur_offset += 8
        }
        if stack_size > 0 {
            insts.push(Inst::AjustSp {
                amount: stack_size as i64,
            });
        }
        insts
    }

    fn gen_call(
        dest: &CallDest,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        opcode: ir::Opcode,
        tmp: Writable<Reg>,
        callee_conv: isa::CallConv,
        caller_conv: isa::CallConv,
    ) -> SmallVec<[Self::I; 2]> {
        let mut insts = SmallVec::new();
        match &dest {
            &CallDest::ExtName(ref name, ..) => {
                insts.push(Inst::LoadExtName {
                    rd: tmp,
                    name: Box::new(name.clone()),
                    offset: 0,
                });
                insts.push(Inst::CallInd {
                    info: Box::new(CallIndInfo {
                        rn: tmp.to_reg(),
                        uses,
                        defs,
                        opcode,
                        caller_callconv: caller_conv,
                        callee_callconv: callee_conv,
                    }),
                });
            }
            &CallDest::Reg(reg) => insts.push(Inst::CallInd {
                info: Box::new(CallIndInfo {
                    rn: *reg,
                    uses,
                    defs,
                    opcode,
                    caller_callconv: caller_conv,
                    callee_callconv: callee_conv,
                }),
            }),
        }
        insts
    }

    fn gen_memcpy(
        call_conv: isa::CallConv,
        dst: Reg,
        src: Reg,
        size: usize,
    ) -> SmallVec<[Self::I; 8]> {
        let mut insts = SmallVec::new();
        let arg0 = writable_a0();
        let arg1 = writable_a1();
        let arg2 = writable_a2();
        insts.push(Inst::gen_move(arg0, dst, I64));
        insts.push(Inst::gen_move(arg1, src, I64));
        insts.extend(Inst::load_constant_u64(arg2, size as u64));
        insts.push(Inst::Call {
            info: Box::new(CallInfo {
                dest: ExternalName::LibCall(LibCall::Memcpy),
                uses: vec![arg0.to_reg(), arg1.to_reg(), arg2.to_reg()],
                defs: Self::get_regs_clobbered_by_call(call_conv),
                opcode: Opcode::Call,
                caller_callconv: call_conv,
                callee_callconv: call_conv,
            }),
        });
        insts
    }

    fn get_number_of_spillslots_for_value(rc: RegClass) -> u32 {
        // We allocate in terms of 8-byte slots.
        match rc {
            RegClass::Int => 1,
            RegClass::Float => 1,
            _ => panic!("Unexpected register class!"),
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

    fn get_regs_clobbered_by_call(call_conv_of_callee: isa::CallConv) -> Vec<Writable<Reg>> {
        get_caller_save_register(call_conv_of_callee)
    }

    fn get_clobbered_callee_saves(
        call_conv: isa::CallConv,
        regs: &[Writable<RealReg>],
    ) -> Vec<Writable<RealReg>> {
        let mut regs: Vec<Writable<RealReg>> = regs
            .iter()
            .cloned()
            .filter(|r| is_reg_saved_in_prologue(call_conv, r.to_reg()))
            .collect();

        // Sort registers for deterministic code output. We can do an unstable
        // sort because the registers will be unique (there are no dups).
        // regs.sort_unstable_by_key(|r| r.to_reg().get_index());
        regs.sort();
        regs
    }

    fn is_frame_setup_needed(
        is_leaf: bool,
        stack_args_size: u32,
        num_clobbered_callee_saves: usize,
        fixed_frame_storage_size: u32,
    ) -> bool {
        !is_leaf
            // The function arguments that are passed on the stack are addressed
            // relative to the Frame Pointer.
            || stack_args_size > 0
            || num_clobbered_callee_saves > 0
            || fixed_frame_storage_size > 0
    }
}

/*
    todo too slow.
*/
pub fn get_caller_save_x_gpr() -> [bool; 32] {
    let mut x: [bool; 32] = [false; 32];
    for (i, v) in get_callee_save_x_gpr().iter().enumerate() {
        if i == 0 || i == 3 || i == 4 || i == 30 || i == 31 {
            // there register caller and called not save at all , always been false.
            continue;
        }
        x[i] = !v;
    }
    x
}

pub fn get_caller_save_f_gpr() -> [bool; 32] {
    let mut x: [bool; 32] = [false; 32];
    for (i, v) in get_callee_save_f_gpr().iter().enumerate() {
        if i == 31 {
            continue;
        }
        x[i] = !v;
    }
    x
}

fn get_callee_save_x_gpr() -> [bool; 32] {
    let mut x = [false; 32];
    x[2] = true;
    for i in 8..=9 {
        x[i] = true
    }
    for i in 18..=27 {
        x[i] = true
    }
    x
}

fn get_callee_save_f_gpr() -> [bool; 32] {
    let mut x = [false; 32];
    for i in 8..9 {
        x[i] = true;
    }
    for i in 18..=27 {
        x[i] = true
    }
    x
}

// this should be the registers must be save by callee
fn is_reg_saved_in_prologue(_conv: CallConv, reg: RealReg) -> bool {
    if reg.class() == RegClass::Int {
        get_callee_save_x_gpr()[reg.hw_enc() as usize]
    } else {
        get_callee_save_f_gpr()[reg.hw_enc() as usize]
    }
}

fn compute_clobber_size(clobbers: &[Writable<RealReg>]) -> u32 {
    let mut clobbered_size = 0;
    for reg in clobbers {
        match reg.to_reg().class() {
            RegClass::Int => {
                clobbered_size += 8;
            }
            RegClass::Float => {
                clobbered_size += 8;
            }
            _ => unreachable!(),
        }
    }
    align_to(clobbered_size, 16)
}

#[cfg(test)]
mod test {
    #[test]
    fn compile_ok() {}
}

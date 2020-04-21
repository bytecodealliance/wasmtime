//! Implementation of the standard AArch64 ABI.

use crate::ir;
use crate::ir::types;
use crate::ir::types::*;
use crate::ir::{ArgumentExtension, StackSlot};
use crate::isa;
use crate::isa::aarch64::{self, inst::*};
use crate::machinst::*;
use crate::settings;

use alloc::vec::Vec;

use regalloc::{RealReg, Reg, RegClass, Set, SpillSlot, Writable};

use log::debug;

/// A location for an argument or return value.
#[derive(Clone, Copy, Debug)]
enum ABIArg {
    /// In a real register.
    Reg(RealReg, ir::Type),
    /// Arguments only: on stack, at given offset from SP at entry.
    Stack(i64, ir::Type),
}

/// AArch64 ABI information shared between body (callee) and caller.
struct ABISig {
    args: Vec<ABIArg>,
    rets: Vec<ABIArg>,
    stack_arg_space: i64,
    call_conv: isa::CallConv,
}

// Spidermonkey specific ABI convention.

/// This is SpiderMonkey's `WasmTableCallSigReg`.
static BALDRDASH_SIG_REG: u8 = 10;

/// This is SpiderMonkey's `WasmTlsReg`.
static BALDRDASH_TLS_REG: u8 = 23;

// These two lists represent the registers the JIT may *not* use at any point in generated code.
//
// So these are callee-preserved from the JIT's point of view, and every register not in this list
// has to be caller-preserved by definition.
//
// Keep these lists in sync with the NonAllocatableMask set in Spidermonkey's
// Architecture-arm64.cpp.

// Indexed by physical register number.
#[rustfmt::skip]
static BALDRDASH_JIT_CALLEE_SAVED_GPR: &[bool] = &[
    /* 0 = */ false, false, false, false, false, false, false, false,
    /* 8 = */ false, false, false, false, false, false, false, false,
    /* 16 = */ true /* x16 / ip1 */, true /* x17 / ip2 */, true /* x18 / TLS */, false,
    /* 20 = */ false, false, false, false,
    /* 24 = */ false, false, false, false,
    // There should be 28, the pseudo stack pointer in this list, however the wasm stubs trash it
    // gladly right now.
    /* 28 = */ false, false, true /* x30 = FP */, false /* x31 = SP */
];

#[rustfmt::skip]
static BALDRDASH_JIT_CALLEE_SAVED_FPU: &[bool] = &[
    /* 0 = */ false, false, false, false, false, false, false, false,
    /* 8 = */ false, false, false, false, false, false, false, false,
    /* 16 = */ false, false, false, false, false, false, false, false,
    /* 24 = */ false, false, false, false, false, false, false, true /* v31 / d31 */
];

/// Try to fill a Baldrdash register, returning it if it was found.
fn try_fill_baldrdash_reg(call_conv: isa::CallConv, param: &ir::AbiParam) -> Option<ABIArg> {
    if call_conv.extends_baldrdash() {
        match &param.purpose {
            &ir::ArgumentPurpose::VMContext => {
                // This is SpiderMonkey's `WasmTlsReg`.
                Some(ABIArg::Reg(
                    xreg(BALDRDASH_TLS_REG).to_real_reg(),
                    ir::types::I64,
                ))
            }
            &ir::ArgumentPurpose::SignatureId => {
                // This is SpiderMonkey's `WasmTableCallSigReg`.
                Some(ABIArg::Reg(
                    xreg(BALDRDASH_SIG_REG).to_real_reg(),
                    ir::types::I64,
                ))
            }
            _ => None,
        }
    } else {
        None
    }
}

/// Process a list of parameters or return values and allocate them to X-regs,
/// V-regs, and stack slots.
///
/// Returns the list of argument locations, and the stack-space used (rounded up
/// to a 16-byte-aligned boundary).
fn compute_arg_locs(call_conv: isa::CallConv, params: &[ir::AbiParam]) -> (Vec<ABIArg>, i64) {
    // See AArch64 ABI (https://c9x.me/compile/bib/abi-arm64.pdf), sections 5.4.
    let mut next_xreg = 0;
    let mut next_vreg = 0;
    let mut next_stack: u64 = 0;
    let mut ret = vec![];

    for param in params {
        // Validate "purpose".
        match &param.purpose {
            &ir::ArgumentPurpose::VMContext
            | &ir::ArgumentPurpose::Normal
            | &ir::ArgumentPurpose::StackLimit
            | &ir::ArgumentPurpose::SignatureId => {}
            _ => panic!(
                "Unsupported argument purpose {:?} in signature: {:?}",
                param.purpose, params
            ),
        }

        if in_int_reg(param.value_type) {
            if let Some(param) = try_fill_baldrdash_reg(call_conv, param) {
                ret.push(param);
            } else if next_xreg < 8 {
                ret.push(ABIArg::Reg(xreg(next_xreg).to_real_reg(), param.value_type));
                next_xreg += 1;
            } else {
                ret.push(ABIArg::Stack(next_stack as i64, param.value_type));
                next_stack += 8;
            }
        } else if in_vec_reg(param.value_type) {
            if next_vreg < 8 {
                ret.push(ABIArg::Reg(vreg(next_vreg).to_real_reg(), param.value_type));
                next_vreg += 1;
            } else {
                let size: u64 = match param.value_type {
                    F32 | F64 => 8,
                    _ => panic!("Unsupported vector-reg argument type"),
                };
                // Align.
                debug_assert!(size.is_power_of_two());
                next_stack = (next_stack + size - 1) & !(size - 1);
                ret.push(ABIArg::Stack(next_stack as i64, param.value_type));
                next_stack += size;
            }
        }
    }

    next_stack = (next_stack + 15) & !15;

    (ret, next_stack as i64)
}

impl ABISig {
    fn from_func_sig(sig: &ir::Signature) -> ABISig {
        // Compute args and retvals from signature.
        // TODO: pass in arg-mode or ret-mode. (Does not matter
        // for the types of arguments/return values that we support.)
        let (args, stack_arg_space) = compute_arg_locs(sig.call_conv, &sig.params);
        let (rets, _) = compute_arg_locs(sig.call_conv, &sig.returns);

        // Verify that there are no return values on the stack.
        debug_assert!(rets.iter().all(|a| match a {
            &ABIArg::Stack(..) => false,
            _ => true,
        }));

        ABISig {
            args,
            rets,
            stack_arg_space,
            call_conv: sig.call_conv,
        }
    }
}

/// AArch64 ABI object for a function body.
pub struct AArch64ABIBody {
    /// Signature: arg and retval regs.
    sig: ABISig,
    /// Offsets to each stackslot.
    stackslots: Vec<u32>,
    /// Total stack size of all stackslots.
    stackslots_size: u32,
    /// Clobbered registers, from regalloc.
    clobbered: Set<Writable<RealReg>>,
    /// Total number of spillslots, from regalloc.
    spillslots: Option<usize>,
    /// Total frame size.
    frame_size: Option<u32>,
    /// Calling convention this function expects.
    call_conv: isa::CallConv,
    /// The settings controlling this function's compilation.
    flags: settings::Flags,
    /// Whether or not this function is a "leaf", meaning it calls no other
    /// functions
    is_leaf: bool,
    /// If this function has a stack limit specified, then `Reg` is where the
    /// stack limit will be located after the instructions specified have been
    /// executed.
    ///
    /// Note that this is intended for insertion into the prologue, if
    /// present. Also note that because the instructions here execute in the
    /// prologue this happens after legalization/register allocation/etc so we
    /// need to be extremely careful with each instruction. The instructions are
    /// manually register-allocated and carefully only use caller-saved
    /// registers and keep nothing live after this sequence of instructions.
    stack_limit: Option<(Reg, Vec<Inst>)>,
}

fn in_int_reg(ty: ir::Type) -> bool {
    match ty {
        types::I8 | types::I16 | types::I32 | types::I64 => true,
        types::B1 | types::B8 | types::B16 | types::B32 | types::B64 => true,
        _ => false,
    }
}

fn in_vec_reg(ty: ir::Type) -> bool {
    match ty {
        types::F32 | types::F64 => true,
        _ => false,
    }
}

/// Generates the instructions necessary for the `gv` to be materialized into a
/// register.
///
/// This function will return a register that will contain the result of
/// evaluating `gv`. It will also return any instructions necessary to calculate
/// the value of the register.
///
/// Note that global values are typically lowered to instructions via the
/// standard legalization pass. Unfortunately though prologue generation happens
/// so late in the pipeline that we can't use these legalization passes to
/// generate the instructions for `gv`. As a result we duplicate some lowering
/// of `gv` here and support only some global values. This is similar to what
/// the x86 backend does for now, and hopefully this can be somewhat cleaned up
/// in the future too!
///
/// Also note that this function will make use of `writable_spilltmp_reg()` as a
/// temporary register to store values in if necessary. Currently after we write
/// to this register there's guaranteed to be no spilled values between where
/// it's used, because we're not participating in register allocation anyway!
fn gen_stack_limit(f: &ir::Function, abi: &ABISig, gv: ir::GlobalValue) -> (Reg, Vec<Inst>) {
    let mut insts = Vec::new();
    let reg = generate_gv(f, abi, gv, &mut insts);
    return (reg, insts);

    fn generate_gv(
        f: &ir::Function,
        abi: &ABISig,
        gv: ir::GlobalValue,
        insts: &mut Vec<Inst>,
    ) -> Reg {
        match f.global_values[gv] {
            // Return the direct register the vmcontext is in
            ir::GlobalValueData::VMContext => {
                get_special_purpose_param_register(f, abi, ir::ArgumentPurpose::VMContext)
                    .expect("no vmcontext parameter found")
            }
            // Load our base value into a register, then load from that register
            // in to a temporary register.
            ir::GlobalValueData::Load {
                base,
                offset,
                global_type: _,
                readonly: _,
            } => {
                let base = generate_gv(f, abi, base, insts);
                let into_reg = writable_spilltmp_reg();
                let mem = if let Some(offset) =
                    UImm12Scaled::maybe_from_i64(offset.into(), ir::types::I8)
                {
                    MemArg::UnsignedOffset(base, offset)
                } else {
                    let offset: i64 = offset.into();
                    insts.extend(Inst::load_constant(into_reg, offset as u64));
                    MemArg::RegReg(base, into_reg.to_reg())
                };
                insts.push(Inst::ULoad64 {
                    rd: into_reg,
                    mem,
                    srcloc: None,
                });
                return into_reg.to_reg();
            }
            ref other => panic!("global value for stack limit not supported: {}", other),
        }
    }
}

fn get_special_purpose_param_register(
    f: &ir::Function,
    abi: &ABISig,
    purpose: ir::ArgumentPurpose,
) -> Option<Reg> {
    let idx = f.signature.special_param_index(purpose)?;
    match abi.args[idx] {
        ABIArg::Reg(reg, _) => Some(reg.to_reg()),
        ABIArg::Stack(..) => None,
    }
}

impl AArch64ABIBody {
    /// Create a new body ABI instance.
    pub fn new(f: &ir::Function, flags: settings::Flags) -> Self {
        debug!("AArch64 ABI: func signature {:?}", f.signature);

        let sig = ABISig::from_func_sig(&f.signature);

        let call_conv = f.signature.call_conv;
        // Only these calling conventions are supported.
        debug_assert!(
            call_conv == isa::CallConv::SystemV
                || call_conv == isa::CallConv::Fast
                || call_conv == isa::CallConv::Cold
                || call_conv.extends_baldrdash(),
            "Unsupported calling convention: {:?}",
            call_conv
        );

        // Compute stackslot locations and total stackslot size.
        let mut stack_offset: u32 = 0;
        let mut stackslots = vec![];
        for (stackslot, data) in f.stack_slots.iter() {
            let off = stack_offset;
            stack_offset += data.size;
            stack_offset = (stack_offset + 7) & !7;
            debug_assert_eq!(stackslot.as_u32() as usize, stackslots.len());
            stackslots.push(off);
        }

        // Figure out what instructions, if any, will be needed to check the
        // stack limit. This can either be specified as a special-purpose
        // argument or as a global value which often calculates the stack limit
        // from the arguments.
        let stack_limit =
            get_special_purpose_param_register(f, &sig, ir::ArgumentPurpose::StackLimit)
                .map(|reg| (reg, Vec::new()))
                .or_else(|| f.stack_limit.map(|gv| gen_stack_limit(f, &sig, gv)));

        Self {
            sig,
            stackslots,
            stackslots_size: stack_offset,
            clobbered: Set::empty(),
            spillslots: None,
            frame_size: None,
            call_conv,
            flags,
            is_leaf: f.is_leaf(),
            stack_limit,
        }
    }

    /// Returns the size of a function call frame (including return address and FP) for this
    /// function's body.
    fn frame_size(&self) -> i64 {
        if self.call_conv.extends_baldrdash() {
            let num_words = self.flags.baldrdash_prologue_words() as i64;
            debug_assert!(num_words > 0, "baldrdash must set baldrdash_prologue_words");
            debug_assert_eq!(num_words % 2, 0, "stack must be 16-aligned");
            num_words * 8
        } else {
            16 // frame pointer + return address.
        }
    }

    /// Inserts instructions necessary for checking the stack limit into the
    /// prologue.
    ///
    /// This function will generate instructions necessary for perform a stack
    /// check at the header of a function. The stack check is intended to trap
    /// if the stack pointer goes below a particular threshold, preventing stack
    /// overflow in wasm or other code. The `stack_limit` argument here is the
    /// register which holds the threshold below which we're supposed to trap.
    /// This function is known to allocate `stack_size` bytes and we'll push
    /// instructions onto `insts`.
    ///
    /// Note that the instructions generated here are special because this is
    /// happening so late in the pipeline (e.g. after register allocation). This
    /// means that we need to do manual register allocation here and also be
    /// careful to not clobber any callee-saved or argument registers. For now
    /// this routine makes do with the `writable_spilltmp_reg` as one temporary
    /// register, and a second register of `x16` which is caller-saved. This
    /// should be fine for us since no spills should happen in this sequence of
    /// instructions, so our register won't get accidentally clobbered.
    ///
    /// No values can be live after the prologue, but in this case that's ok
    /// because we just need to perform a stack check before progressing with
    /// the rest of the function.
    fn insert_stack_check(&self, stack_limit: Reg, stack_size: u32, insts: &mut Vec<Inst>) {
        // With no explicit stack allocated we can just emit the simple check of
        // the stack registers against the stack limit register, and trap if
        // it's out of bounds.
        if stack_size == 0 {
            return push_check(stack_limit, insts);
        }

        // Note that the 32k stack size here is pretty special. See the
        // documentation in x86/abi.rs for why this is here. The general idea is
        // that we're protecting against overflow in the addition that happens
        // below.
        if stack_size >= 32 * 1024 {
            push_check(stack_limit, insts);
        }

        // Add the `stack_size` to `stack_limit`, placing the result in
        // `scratch`.
        //
        // Note though that `stack_limit`'s register may be the same as
        // `scratch`. If our stack size doesn't fit into an immediate this
        // means we need a second scratch register for loading the stack size
        // into a register. We use `x16` here since it's caller-saved and we're
        // in the function prologue and nothing else is allocated to it yet.
        let scratch = writable_spilltmp_reg();
        let stack_size = u64::from(stack_size);
        if let Some(imm12) = Imm12::maybe_from_u64(stack_size) {
            insts.push(Inst::AluRRImm12 {
                alu_op: ALUOp::Add64,
                rd: scratch,
                rn: stack_limit,
                imm12,
            });
        } else {
            let scratch2 = 16;
            insts.extend(Inst::load_constant(
                Writable::from_reg(xreg(scratch2)),
                stack_size.into(),
            ));
            insts.push(Inst::AluRRRExtend {
                alu_op: ALUOp::Add64,
                rd: scratch,
                rn: stack_limit,
                rm: xreg(scratch2),
                extendop: ExtendOp::UXTX,
            });
        }
        push_check(scratch.to_reg(), insts);

        fn push_check(stack_limit: Reg, insts: &mut Vec<Inst>) {
            insts.push(Inst::AluRRR {
                alu_op: ALUOp::SubS64XR,
                rd: writable_zero_reg(),
                rn: stack_reg(),
                rm: stack_limit,
            });
            insts.push(Inst::CondBrLowered {
                target: BranchTarget::ResolvedOffset(8),
                // Here `Hs` == "higher or same" when interpreting the two
                // operands as unsigned integers.
                kind: CondBrKind::Cond(Cond::Hs),
            });
            insts.push(Inst::Udf {
                trap_info: (ir::SourceLoc::default(), ir::TrapCode::StackOverflow),
            });
        }
    }
}

fn load_stack_from_fp(fp_offset: i64, into_reg: Writable<Reg>, ty: Type) -> Inst {
    let mem = MemArg::FPOffset(fp_offset);
    match ty {
        types::B1
        | types::B8
        | types::I8
        | types::B16
        | types::I16
        | types::B32
        | types::I32
        | types::B64
        | types::I64 => Inst::ULoad64 {
            rd: into_reg,
            mem,
            srcloc: None,
        },
        types::F32 => Inst::FpuLoad32 {
            rd: into_reg,
            mem,
            srcloc: None,
        },
        types::F64 => Inst::FpuLoad64 {
            rd: into_reg,
            mem,
            srcloc: None,
        },
        _ => unimplemented!("load_stack_from_fp({})", ty),
    }
}

fn store_stack(mem: MemArg, from_reg: Reg, ty: Type) -> Inst {
    debug_assert!(match &mem {
        MemArg::SPOffset(off) => SImm9::maybe_from_i64(*off).is_some(),
        _ => true,
    });
    match ty {
        types::B1
        | types::B8
        | types::I8
        | types::B16
        | types::I16
        | types::B32
        | types::I32
        | types::B64
        | types::I64 => Inst::Store64 {
            rd: from_reg,
            mem,
            srcloc: None,
        },
        types::F32 => Inst::FpuStore32 {
            rd: from_reg,
            mem,
            srcloc: None,
        },
        types::F64 => Inst::FpuStore64 {
            rd: from_reg,
            mem,
            srcloc: None,
        },
        _ => unimplemented!("store_stack({})", ty),
    }
}

fn store_stack_fp(fp_offset: i64, from_reg: Reg, ty: Type) -> Inst {
    store_stack(MemArg::FPOffset(fp_offset), from_reg, ty)
}

fn store_stack_sp<C: LowerCtx<I = Inst>>(
    ctx: &mut C,
    sp_offset: i64,
    from_reg: Reg,
    ty: Type,
) -> Vec<Inst> {
    if SImm9::maybe_from_i64(sp_offset).is_some() {
        vec![store_stack(MemArg::SPOffset(sp_offset), from_reg, ty)]
    } else {
        // mem_finalize will try to generate an add, but in an addition, x31 is the zero register,
        // not sp! So we have to synthesize the full add here.
        let tmp1 = ctx.tmp(RegClass::I64, I64);
        let tmp2 = ctx.tmp(RegClass::I64, I64);
        let mut result = Vec::new();
        // tmp1 := sp
        result.push(Inst::Mov {
            rd: tmp1,
            rm: stack_reg(),
        });
        // tmp2 := offset
        for inst in Inst::load_constant(tmp2, sp_offset as u64) {
            result.push(inst);
        }
        // tmp1 := add tmp1, tmp2
        result.push(Inst::AluRRR {
            alu_op: ALUOp::Add64,
            rd: tmp1,
            rn: tmp1.to_reg(),
            rm: tmp2.to_reg(),
        });
        // Actual store.
        result.push(store_stack(
            MemArg::Unscaled(tmp1.to_reg(), SImm9::maybe_from_i64(0).unwrap()),
            from_reg,
            ty,
        ));
        result
    }
}

fn is_callee_save(call_conv: isa::CallConv, r: RealReg) -> bool {
    if call_conv.extends_baldrdash() {
        match r.get_class() {
            RegClass::I64 => {
                let enc = r.get_hw_encoding();
                return BALDRDASH_JIT_CALLEE_SAVED_GPR[enc];
            }
            RegClass::V128 => {
                let enc = r.get_hw_encoding();
                return BALDRDASH_JIT_CALLEE_SAVED_FPU[enc];
            }
            _ => unimplemented!("baldrdash callee saved on non-i64 reg classes"),
        };
    }

    match r.get_class() {
        RegClass::I64 => {
            // x19 - x28 inclusive are callee-saves.
            r.get_hw_encoding() >= 19 && r.get_hw_encoding() <= 28
        }
        RegClass::V128 => {
            // v8 - v15 inclusive are callee-saves.
            r.get_hw_encoding() >= 8 && r.get_hw_encoding() <= 15
        }
        _ => panic!("Unexpected RegClass"),
    }
}

fn get_callee_saves(
    call_conv: isa::CallConv,
    regs: Vec<Writable<RealReg>>,
) -> (Vec<Writable<RealReg>>, Vec<Writable<RealReg>>) {
    let mut int_saves = vec![];
    let mut vec_saves = vec![];
    for reg in regs.into_iter() {
        if is_callee_save(call_conv, reg.to_reg()) {
            match reg.to_reg().get_class() {
                RegClass::I64 => int_saves.push(reg),
                RegClass::V128 => vec_saves.push(reg),
                _ => panic!("Unexpected RegClass"),
            }
        }
    }
    (int_saves, vec_saves)
}

fn is_caller_save(call_conv: isa::CallConv, r: RealReg) -> bool {
    if call_conv.extends_baldrdash() {
        match r.get_class() {
            RegClass::I64 => {
                let enc = r.get_hw_encoding();
                if !BALDRDASH_JIT_CALLEE_SAVED_GPR[enc] {
                    return true;
                }
                // Otherwise, fall through to preserve native's ABI caller-saved.
            }
            RegClass::V128 => {
                let enc = r.get_hw_encoding();
                if !BALDRDASH_JIT_CALLEE_SAVED_FPU[enc] {
                    return true;
                }
                // Otherwise, fall through to preserve native's ABI caller-saved.
            }
            _ => unimplemented!("baldrdash callee saved on non-i64 reg classes"),
        };
    }

    match r.get_class() {
        RegClass::I64 => {
            // x0 - x17 inclusive are caller-saves.
            r.get_hw_encoding() <= 17
        }
        RegClass::V128 => {
            // v0 - v7 inclusive and v16 - v31 inclusive are caller-saves.
            r.get_hw_encoding() <= 7 || (r.get_hw_encoding() >= 16 && r.get_hw_encoding() <= 31)
        }
        _ => panic!("Unexpected RegClass"),
    }
}

fn get_caller_saves_set(call_conv: isa::CallConv) -> Set<Writable<Reg>> {
    let mut set = Set::empty();
    for i in 0..29 {
        let x = writable_xreg(i);
        if is_caller_save(call_conv, x.to_reg().to_real_reg()) {
            set.insert(x);
        }
    }
    for i in 0..32 {
        let v = writable_vreg(i);
        if is_caller_save(call_conv, v.to_reg().to_real_reg()) {
            set.insert(v);
        }
    }
    set
}

impl ABIBody for AArch64ABIBody {
    type I = Inst;

    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn liveins(&self) -> Set<RealReg> {
        let mut set: Set<RealReg> = Set::empty();
        for &arg in &self.sig.args {
            if let ABIArg::Reg(r, _) = arg {
                set.insert(r);
            }
        }
        set
    }

    fn liveouts(&self) -> Set<RealReg> {
        let mut set: Set<RealReg> = Set::empty();
        for &ret in &self.sig.rets {
            if let ABIArg::Reg(r, _) = ret {
                set.insert(r);
            }
        }
        set
    }

    fn num_args(&self) -> usize {
        self.sig.args.len()
    }

    fn num_retvals(&self) -> usize {
        self.sig.rets.len()
    }

    fn num_stackslots(&self) -> usize {
        self.stackslots.len()
    }

    fn gen_copy_arg_to_reg(&self, idx: usize, into_reg: Writable<Reg>) -> Inst {
        match &self.sig.args[idx] {
            &ABIArg::Reg(r, ty) => Inst::gen_move(into_reg, r.to_reg(), ty),
            &ABIArg::Stack(off, ty) => load_stack_from_fp(off + self.frame_size(), into_reg, ty),
        }
    }

    fn gen_copy_reg_to_retval(
        &self,
        idx: usize,
        from_reg: Writable<Reg>,
        ext: ArgumentExtension,
    ) -> Vec<Inst> {
        let mut ret = Vec::new();
        match &self.sig.rets[idx] {
            &ABIArg::Reg(r, ty) => {
                let from_bits = aarch64::lower::ty_bits(ty) as u8;
                let dest_reg = Writable::from_reg(r.to_reg());
                match (ext, from_bits) {
                    (ArgumentExtension::Uext, n) if n < 64 => {
                        ret.push(Inst::Extend {
                            rd: dest_reg,
                            rn: from_reg.to_reg(),
                            signed: false,
                            from_bits,
                            to_bits: 64,
                        });
                    }
                    (ArgumentExtension::Sext, n) if n < 64 => {
                        ret.push(Inst::Extend {
                            rd: dest_reg,
                            rn: from_reg.to_reg(),
                            signed: true,
                            from_bits,
                            to_bits: 64,
                        });
                    }
                    _ => ret.push(Inst::gen_move(dest_reg, from_reg.to_reg(), ty)),
                };
            }
            &ABIArg::Stack(off, ty) => {
                let from_bits = aarch64::lower::ty_bits(ty) as u8;
                // Trash the from_reg; it should be its last use.
                match (ext, from_bits) {
                    (ArgumentExtension::Uext, n) if n < 64 => {
                        ret.push(Inst::Extend {
                            rd: from_reg,
                            rn: from_reg.to_reg(),
                            signed: false,
                            from_bits,
                            to_bits: 64,
                        });
                    }
                    (ArgumentExtension::Sext, n) if n < 64 => {
                        ret.push(Inst::Extend {
                            rd: from_reg,
                            rn: from_reg.to_reg(),
                            signed: true,
                            from_bits,
                            to_bits: 64,
                        });
                    }
                    _ => {}
                };
                ret.push(store_stack_fp(
                    off + self.frame_size(),
                    from_reg.to_reg(),
                    ty,
                ))
            }
        }
        ret
    }

    fn gen_ret(&self) -> Inst {
        Inst::Ret {}
    }

    fn gen_epilogue_placeholder(&self) -> Inst {
        Inst::EpiloguePlaceholder {}
    }

    fn set_num_spillslots(&mut self, slots: usize) {
        self.spillslots = Some(slots);
    }

    fn set_clobbered(&mut self, clobbered: Set<Writable<RealReg>>) {
        self.clobbered = clobbered;
    }

    fn load_stackslot(
        &self,
        slot: StackSlot,
        offset: u32,
        ty: Type,
        into_reg: Writable<Reg>,
    ) -> Inst {
        // Offset from beginning of stackslot area, which is at FP - stackslots_size.
        let stack_off = self.stackslots[slot.as_u32() as usize] as i64;
        let fp_off: i64 = -(self.stackslots_size as i64) + stack_off + (offset as i64);
        load_stack_from_fp(fp_off, into_reg, ty)
    }

    fn store_stackslot(&self, slot: StackSlot, offset: u32, ty: Type, from_reg: Reg) -> Inst {
        // Offset from beginning of stackslot area, which is at FP - stackslots_size.
        let stack_off = self.stackslots[slot.as_u32() as usize] as i64;
        let fp_off: i64 = -(self.stackslots_size as i64) + stack_off + (offset as i64);
        store_stack_fp(fp_off, from_reg, ty)
    }

    fn stackslot_addr(&self, slot: StackSlot, offset: u32, into_reg: Writable<Reg>) -> Inst {
        // Offset from beginning of stackslot area, which is at FP - stackslots_size.
        let stack_off = self.stackslots[slot.as_u32() as usize] as i64;
        let fp_off: i64 = -(self.stackslots_size as i64) + stack_off + (offset as i64);
        Inst::LoadAddr {
            rd: into_reg,
            mem: MemArg::FPOffset(fp_off),
        }
    }

    // Load from a spillslot.
    fn load_spillslot(&self, slot: SpillSlot, ty: Type, into_reg: Writable<Reg>) -> Inst {
        // Note that when spills/fills are generated, we don't yet know how many
        // spillslots there will be, so we allocate *downward* from the beginning
        // of the stackslot area. Hence: FP - stackslot_size - 8*spillslot -
        // sizeof(ty).
        let islot = slot.get() as i64;
        let ty_size = self.get_spillslot_size(into_reg.to_reg().get_class(), ty) * 8;
        let fp_off: i64 = -(self.stackslots_size as i64) - (8 * islot) - ty_size as i64;
        load_stack_from_fp(fp_off, into_reg, ty)
    }

    // Store to a spillslot.
    fn store_spillslot(&self, slot: SpillSlot, ty: Type, from_reg: Reg) -> Inst {
        let islot = slot.get() as i64;
        let ty_size = self.get_spillslot_size(from_reg.get_class(), ty) * 8;
        let fp_off: i64 = -(self.stackslots_size as i64) - (8 * islot) - ty_size as i64;
        store_stack_fp(fp_off, from_reg, ty)
    }

    fn gen_prologue(&mut self) -> Vec<Inst> {
        let mut insts = vec![];
        if !self.call_conv.extends_baldrdash() {
            // stp fp (x29), lr (x30), [sp, #-16]!
            insts.push(Inst::StoreP64 {
                rt: fp_reg(),
                rt2: link_reg(),
                mem: PairMemArg::PreIndexed(
                    writable_stack_reg(),
                    SImm7Scaled::maybe_from_i64(-16, types::I64).unwrap(),
                ),
            });
            // mov fp (x29), sp. This uses the ADDI rd, rs, 0 form of `MOV` because
            // the usual encoding (`ORR`) does not work with SP.
            insts.push(Inst::AluRRImm12 {
                alu_op: ALUOp::Add64,
                rd: writable_fp_reg(),
                rn: stack_reg(),
                imm12: Imm12 {
                    bits: 0,
                    shift12: false,
                },
            });
        }

        let mut total_stacksize = self.stackslots_size + 8 * self.spillslots.unwrap() as u32;
        if self.call_conv.extends_baldrdash() {
            debug_assert!(
                !self.flags.enable_probestack(),
                "baldrdash does not expect cranelift to emit stack probes"
            );
            total_stacksize += self.flags.baldrdash_prologue_words() as u32 * 8;
        }
        let total_stacksize = (total_stacksize + 15) & !15; // 16-align the stack.

        if !self.call_conv.extends_baldrdash() {
            // Leaf functions with zero stack don't need a stack check if one's
            // specified, otherwise always insert the stack check.
            if total_stacksize > 0 || !self.is_leaf {
                if let Some((reg, stack_limit_load)) = &self.stack_limit {
                    insts.extend_from_slice(stack_limit_load);
                    self.insert_stack_check(*reg, total_stacksize, &mut insts);
                }
            }
            if total_stacksize > 0 {
                // sub sp, sp, #total_stacksize
                if let Some(imm12) = Imm12::maybe_from_u64(total_stacksize as u64) {
                    let sub_inst = Inst::AluRRImm12 {
                        alu_op: ALUOp::Sub64,
                        rd: writable_stack_reg(),
                        rn: stack_reg(),
                        imm12,
                    };
                    insts.push(sub_inst);
                } else {
                    let tmp = writable_spilltmp_reg();
                    let const_inst = Inst::LoadConst64 {
                        rd: tmp,
                        const_data: total_stacksize as u64,
                    };
                    let sub_inst = Inst::AluRRRExtend {
                        alu_op: ALUOp::Sub64,
                        rd: writable_stack_reg(),
                        rn: stack_reg(),
                        rm: tmp.to_reg(),
                        extendop: ExtendOp::UXTX,
                    };
                    insts.push(const_inst);
                    insts.push(sub_inst);
                }
            }
        }

        // Save clobbered registers.
        let (clobbered_int, clobbered_vec) =
            get_callee_saves(self.call_conv, self.clobbered.to_vec());
        for reg_pair in clobbered_int.chunks(2) {
            let (r1, r2) = if reg_pair.len() == 2 {
                // .to_reg().to_reg(): Writable<RealReg> --> RealReg --> Reg
                (reg_pair[0].to_reg().to_reg(), reg_pair[1].to_reg().to_reg())
            } else {
                (reg_pair[0].to_reg().to_reg(), zero_reg())
            };

            debug_assert!(r1.get_class() == RegClass::I64);
            debug_assert!(r2.get_class() == RegClass::I64);

            // stp r1, r2, [sp, #-16]!
            insts.push(Inst::StoreP64 {
                rt: r1,
                rt2: r2,
                mem: PairMemArg::PreIndexed(
                    writable_stack_reg(),
                    SImm7Scaled::maybe_from_i64(-16, types::I64).unwrap(),
                ),
            });
        }
        let vec_save_bytes = clobbered_vec.len() * 16;
        if vec_save_bytes != 0 {
            insts.push(Inst::AluRRImm12 {
                alu_op: ALUOp::Sub64,
                rd: writable_stack_reg(),
                rn: stack_reg(),
                imm12: Imm12::maybe_from_u64(vec_save_bytes as u64).unwrap(),
            });
        }
        for (i, reg) in clobbered_vec.iter().enumerate() {
            insts.push(Inst::FpuStore128 {
                rd: reg.to_reg().to_reg(),
                mem: MemArg::Unscaled(stack_reg(), SImm9::maybe_from_i64((i * 16) as i64).unwrap()),
                srcloc: None,
            });
        }

        self.frame_size = Some(total_stacksize);
        insts
    }

    fn gen_epilogue(&self) -> Vec<Inst> {
        let mut insts = vec![];

        // Restore clobbered registers.
        let (clobbered_int, clobbered_vec) =
            get_callee_saves(self.call_conv, self.clobbered.to_vec());

        for (i, reg) in clobbered_vec.iter().enumerate() {
            insts.push(Inst::FpuLoad128 {
                rd: Writable::from_reg(reg.to_reg().to_reg()),
                mem: MemArg::Unscaled(stack_reg(), SImm9::maybe_from_i64((i * 16) as i64).unwrap()),
                srcloc: None,
            });
        }
        let vec_save_bytes = clobbered_vec.len() * 16;
        if vec_save_bytes != 0 {
            insts.push(Inst::AluRRImm12 {
                alu_op: ALUOp::Add64,
                rd: writable_stack_reg(),
                rn: stack_reg(),
                imm12: Imm12::maybe_from_u64(vec_save_bytes as u64).unwrap(),
            });
        }

        for reg_pair in clobbered_int.chunks(2).rev() {
            let (r1, r2) = if reg_pair.len() == 2 {
                (
                    reg_pair[0].map(|r| r.to_reg()),
                    reg_pair[1].map(|r| r.to_reg()),
                )
            } else {
                (reg_pair[0].map(|r| r.to_reg()), writable_zero_reg())
            };

            debug_assert!(r1.to_reg().get_class() == RegClass::I64);
            debug_assert!(r2.to_reg().get_class() == RegClass::I64);

            // ldp r1, r2, [sp], #16
            insts.push(Inst::LoadP64 {
                rt: r1,
                rt2: r2,
                mem: PairMemArg::PostIndexed(
                    writable_stack_reg(),
                    SImm7Scaled::maybe_from_i64(16, types::I64).unwrap(),
                ),
            });
        }

        if !self.call_conv.extends_baldrdash() {
            // The MOV (alias of ORR) interprets x31 as XZR, so use an ADD here.
            // MOV to SP is an alias of ADD.
            insts.push(Inst::AluRRImm12 {
                alu_op: ALUOp::Add64,
                rd: writable_stack_reg(),
                rn: fp_reg(),
                imm12: Imm12 {
                    bits: 0,
                    shift12: false,
                },
            });
            insts.push(Inst::LoadP64 {
                rt: writable_fp_reg(),
                rt2: writable_link_reg(),
                mem: PairMemArg::PostIndexed(
                    writable_stack_reg(),
                    SImm7Scaled::maybe_from_i64(16, types::I64).unwrap(),
                ),
            });
            insts.push(Inst::Ret {});
        }

        debug!("Epilogue: {:?}", insts);
        insts
    }

    fn frame_size(&self) -> u32 {
        self.frame_size
            .expect("frame size not computed before prologue generation")
    }

    fn get_spillslot_size(&self, rc: RegClass, ty: Type) -> u32 {
        // We allocate in terms of 8-byte slots.
        match (rc, ty) {
            (RegClass::I64, _) => 1,
            (RegClass::V128, F32) | (RegClass::V128, F64) => 1,
            (RegClass::V128, _) => 2,
            _ => panic!("Unexpected register class!"),
        }
    }

    fn gen_spill(&self, to_slot: SpillSlot, from_reg: RealReg, ty: Type) -> Inst {
        self.store_spillslot(to_slot, ty, from_reg.to_reg())
    }

    fn gen_reload(&self, to_reg: Writable<RealReg>, from_slot: SpillSlot, ty: Type) -> Inst {
        self.load_spillslot(from_slot, ty, to_reg.map(|r| r.to_reg()))
    }
}

enum CallDest {
    ExtName(ir::ExternalName, RelocDistance),
    Reg(Reg),
}

/// AArch64 ABI object for a function call.
pub struct AArch64ABICall {
    sig: ABISig,
    uses: Set<Reg>,
    defs: Set<Writable<Reg>>,
    dest: CallDest,
    loc: ir::SourceLoc,
    opcode: ir::Opcode,
}

fn abisig_to_uses_and_defs(sig: &ABISig) -> (Set<Reg>, Set<Writable<Reg>>) {
    // Compute uses: all arg regs.
    let mut uses = Set::empty();
    for arg in &sig.args {
        match arg {
            &ABIArg::Reg(reg, _) => uses.insert(reg.to_reg()),
            _ => {}
        }
    }

    // Compute defs: all retval regs, and all caller-save (clobbered) regs.
    let mut defs = get_caller_saves_set(sig.call_conv);
    for ret in &sig.rets {
        match ret {
            &ABIArg::Reg(reg, _) => defs.insert(Writable::from_reg(reg.to_reg())),
            _ => {}
        }
    }

    (uses, defs)
}

impl AArch64ABICall {
    /// Create a callsite ABI object for a call directly to the specified function.
    pub fn from_func(
        sig: &ir::Signature,
        extname: &ir::ExternalName,
        dist: RelocDistance,
        loc: ir::SourceLoc,
    ) -> AArch64ABICall {
        let sig = ABISig::from_func_sig(sig);
        let (uses, defs) = abisig_to_uses_and_defs(&sig);
        AArch64ABICall {
            sig,
            uses,
            defs,
            dest: CallDest::ExtName(extname.clone(), dist),
            loc,
            opcode: ir::Opcode::Call,
        }
    }

    /// Create a callsite ABI object for a call to a function pointer with the
    /// given signature.
    pub fn from_ptr(
        sig: &ir::Signature,
        ptr: Reg,
        loc: ir::SourceLoc,
        opcode: ir::Opcode,
    ) -> AArch64ABICall {
        let sig = ABISig::from_func_sig(sig);
        let (uses, defs) = abisig_to_uses_and_defs(&sig);
        AArch64ABICall {
            sig,
            uses,
            defs,
            dest: CallDest::Reg(ptr),
            loc,
            opcode,
        }
    }
}

fn adjust_stack(amt: u64, is_sub: bool) -> Vec<Inst> {
    if amt > 0 {
        let alu_op = if is_sub { ALUOp::Sub64 } else { ALUOp::Add64 };
        if let Some(imm12) = Imm12::maybe_from_u64(amt) {
            vec![Inst::AluRRImm12 {
                alu_op,
                rd: writable_stack_reg(),
                rn: stack_reg(),
                imm12,
            }]
        } else {
            let const_load = Inst::LoadConst64 {
                rd: writable_spilltmp_reg(),
                const_data: amt,
            };
            let adj = Inst::AluRRRExtend {
                alu_op,
                rd: writable_stack_reg(),
                rn: stack_reg(),
                rm: spilltmp_reg(),
                extendop: ExtendOp::UXTX,
            };
            vec![const_load, adj]
        }
    } else {
        vec![]
    }
}

impl ABICall for AArch64ABICall {
    type I = Inst;

    fn num_args(&self) -> usize {
        self.sig.args.len()
    }

    fn gen_stack_pre_adjust(&self) -> Vec<Inst> {
        adjust_stack(self.sig.stack_arg_space as u64, /* is_sub = */ true)
    }

    fn gen_stack_post_adjust(&self) -> Vec<Inst> {
        adjust_stack(self.sig.stack_arg_space as u64, /* is_sub = */ false)
    }

    fn gen_copy_reg_to_arg<C: LowerCtx<I = Self::I>>(
        &self,
        ctx: &mut C,
        idx: usize,
        from_reg: Reg,
    ) -> Vec<Inst> {
        match &self.sig.args[idx] {
            &ABIArg::Reg(reg, ty) => vec![Inst::gen_move(
                Writable::from_reg(reg.to_reg()),
                from_reg,
                ty,
            )],
            &ABIArg::Stack(off, ty) => store_stack_sp(ctx, off, from_reg, ty),
        }
    }

    fn gen_copy_retval_to_reg(&self, idx: usize, into_reg: Writable<Reg>) -> Inst {
        match &self.sig.rets[idx] {
            &ABIArg::Reg(reg, ty) => Inst::gen_move(into_reg, reg.to_reg(), ty),
            _ => unimplemented!(),
        }
    }

    fn gen_call(&self) -> Vec<Inst> {
        let (uses, defs) = (self.uses.clone(), self.defs.clone());
        match &self.dest {
            &CallDest::ExtName(ref name, RelocDistance::Near) => vec![Inst::Call {
                dest: name.clone(),
                uses,
                defs,
                loc: self.loc,
                opcode: self.opcode,
            }],
            &CallDest::ExtName(ref name, RelocDistance::Far) => vec![
                Inst::LoadExtName {
                    rd: writable_spilltmp_reg(),
                    name: name.clone(),
                    offset: 0,
                    srcloc: self.loc,
                },
                Inst::CallInd {
                    rn: spilltmp_reg(),
                    uses,
                    defs,
                    loc: self.loc,
                    opcode: self.opcode,
                },
            ],
            &CallDest::Reg(reg) => vec![Inst::CallInd {
                rn: reg,
                uses,
                defs,
                loc: self.loc,
                opcode: self.opcode,
            }],
        }
    }
}

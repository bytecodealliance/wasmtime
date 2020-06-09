//! Implementation of the standard AArch64 ABI.
//!
//! We implement the standard AArch64 ABI, as documented by ARM. This ABI
//! specifies how arguments are passed (in registers or on the stack, as
//! appropriate), which registers are caller- and callee-saved, and how a
//! particular part of the stack frame (the FP/LR pair) must be linked through
//! the active stack frames.
//!
//! Note, however, that the exact stack layout is up to us. We settled on the
//! below design based on several requirements. In particular, we need to be
//! able to generate instructions (or instruction sequences) to access
//! arguments, stack slots, and spill slots before we know how many spill slots
//! or clobber-saves there will be, because of our pass structure. We also
//! prefer positive offsets to negative offsets because of an asymmetry in
//! AArch64 addressing modes (positive offsets have a larger possible range
//! without a long-form sequence to synthesize an arbitrary offset). Finally, it
//! is not allowed to access memory below the current SP value.
//!
//! As a result, we keep the FP/LR pair just below stack args so that we can
//! access these args at known offsets from FP, and we access on-stack storage
//! using positive offsets from SP. In order to allow codegen for the latter
//! before knowing how many clobber-saves we have, and also allow it while SP is
//! being adjusted to set up a call, we implement a "nominal SP" tracking
//! feature by which a fixup (distance between actual SP and a "nominal" SP) is
//! known at each instruction. See the documentation for
//! [MemArg::NominalSPOffset] for more on this.
//!
//! The stack looks like:
//!
//! ```plain
//!   (high address)
//!
//!                              +---------------------------+
//!                              |          ...              |
//!                              | stack args                |
//!                              | (accessed via FP)         |
//!                              +---------------------------+
//! SP at function entry ----->  | LR (pushed by prologue)   |
//!                              +---------------------------+
//! FP after prologue -------->  | FP (pushed by prologue)   |
//!                              +---------------------------+
//!                              |          ...              |
//!                              | spill slots               |
//!                              | (accessed via nominal-SP) |
//!                              |          ...              |
//!                              | stack slots               |
//!                              | (accessed via nominal-SP) |
//! nominal SP --------------->  | (alloc'd by prologue)     |
//!                              +---------------------------+
//!                              |          ...              |
//!                              | clobbered callee-saves    |
//! SP at end of prologue ---->  | (pushed by prologue)      |
//!                              +---------------------------+
//!                              |          ...              |
//!                              | args for call             |
//! SP before making a call -->  | (pushed at callsite)      |
//!                              +---------------------------+
//!
//!   (low address)
//! ```
//!
//! # Multi-value Returns
//!
//! Note that we support multi-value returns by adopting the SpiderMonkey Wasm
//! ABI internally. Because we do not support linking with externally-compiled
//! multi-value-returning functions (yet), this choice is arbitrary and we are
//! free to make it as we please. Wasmtime generates trampolines to enter
//! toplevel multi-value-returning functions, so this does not concern the
//! Wasmtime embedding.
//!
//! For details of the multi-value return ABI, see:
//!
//! https://searchfox.org/mozilla-central/rev/bc3600def806859c31b2c7ac06e3d69271052a89/js/src/wasm/WasmStubs.h#134
//!
//! In brief:
//! - Return values are processed in *reverse* order.
//! - The first return value in this order (so the last return) goes into the
//!   ordinary return register, X0.
//! - Any further returns go in a struct-return area, allocated upwards (in
//!   address order) during the reverse traversal.
//! - This struct-return area is provided by the caller, and a pointer to its
//!   start is passed as an invisible last (extra) argument. Normally the caller
//!   will allocate this area on the stack. When we generate calls, we place it
//!   just above the on-stack argument area.
//! - So, for example, a function returning 4 i64's (v0, v1, v2, v3), with no
//!   formal arguments, would:
//!   - Accept a pointer P to the struct return area in x0 on entry.
//!   - Return v3 in x0.
//!   - Return v2 in memory at [P].
//!   - Return v1 in memory at [P+8].
//!   - Return v0 in memory at [P+16].

use crate::ir;
use crate::ir::types;
use crate::ir::types::*;
use crate::ir::{ArgumentExtension, StackSlot};
use crate::isa;
use crate::isa::aarch64::{inst::*, lower::ty_bits};
use crate::machinst::*;
use crate::settings;
use crate::{CodegenError, CodegenResult};

use alloc::boxed::Box;
use alloc::vec::Vec;

use regalloc::{RealReg, Reg, RegClass, Set, SpillSlot, Writable};

use core::mem;
use log::{debug, trace};

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
    /// Argument locations (regs or stack slots). Stack offsets are relative to
    /// SP on entry to function.
    args: Vec<ABIArg>,
    /// Return-value locations. Stack offsets are relative to the return-area
    /// pointer.
    rets: Vec<ABIArg>,
    /// Space on stack used to store arguments.
    stack_arg_space: i64,
    /// Space on stack used to store return values.
    stack_ret_space: i64,
    /// Index in `args` of the stack-return-value-area argument.
    stack_ret_arg: Option<usize>,
    /// Calling convention used.
    call_conv: isa::CallConv,
}

/// This is the limit for the size of argument and return-value areas on the
/// stack. We place a reasonable limit here to avoid integer overflow issues
/// with 32-bit arithmetic: for now, 128 MB.
static STACK_ARG_RET_SIZE_LIMIT: u64 = 128 * 1024 * 1024;

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

/// Are we computing information about arguments or return values? Much of the
/// handling is factored out into common routines; this enum allows us to
/// distinguish which case we're handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArgsOrRets {
    Args,
    Rets,
}

/// Process a list of parameters or return values and allocate them to X-regs,
/// V-regs, and stack slots.
///
/// Returns the list of argument locations, the stack-space used (rounded up
/// to a 16-byte-aligned boundary), and if `add_ret_area_ptr` was passed, the
/// index of the extra synthetic arg that was added.
fn compute_arg_locs(
    call_conv: isa::CallConv,
    params: &[ir::AbiParam],
    args_or_rets: ArgsOrRets,
    add_ret_area_ptr: bool,
) -> CodegenResult<(Vec<ABIArg>, i64, Option<usize>)> {
    let is_baldrdash = call_conv.extends_baldrdash();

    // See AArch64 ABI (https://c9x.me/compile/bib/abi-arm64.pdf), sections 5.4.
    let mut next_xreg = 0;
    let mut next_vreg = 0;
    let mut next_stack: u64 = 0;
    let mut ret = vec![];

    let max_reg_vals = match (args_or_rets, is_baldrdash) {
        (ArgsOrRets::Args, _) => 8,     // x0-x7, v0-v7
        (ArgsOrRets::Rets, false) => 8, // x0-x7, v0-v7
        (ArgsOrRets::Rets, true) => 1,  // x0 or v0
    };

    for i in 0..params.len() {
        // Process returns backward, according to the SpiderMonkey ABI (which we
        // adopt internally if `is_baldrdash` is set).
        let param = match (args_or_rets, is_baldrdash) {
            (ArgsOrRets::Args, _) => &params[i],
            (ArgsOrRets::Rets, false) => &params[i],
            (ArgsOrRets::Rets, true) => &params[params.len() - 1 - i],
        };

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

        let intreg = in_int_reg(param.value_type);
        let vecreg = in_vec_reg(param.value_type);
        debug_assert!(intreg || vecreg);
        debug_assert!(!(intreg && vecreg));

        let next_reg = if intreg {
            &mut next_xreg
        } else {
            &mut next_vreg
        };

        if let Some(param) = try_fill_baldrdash_reg(call_conv, param) {
            assert!(intreg);
            ret.push(param);
        } else if *next_reg < max_reg_vals {
            let reg = if intreg {
                xreg(*next_reg)
            } else {
                vreg(*next_reg)
            };
            ret.push(ABIArg::Reg(reg.to_real_reg(), param.value_type));
            *next_reg += 1;
        } else {
            // Compute size. Every arg takes a minimum slot of 8 bytes. (16-byte
            // stack alignment happens separately after all args.)
            let size = (ty_bits(param.value_type) / 8) as u64;
            let size = std::cmp::max(size, 8);
            // Align.
            debug_assert!(size.is_power_of_two());
            next_stack = (next_stack + size - 1) & !(size - 1);
            ret.push(ABIArg::Stack(next_stack as i64, param.value_type));
            next_stack += size;
        }
    }

    if args_or_rets == ArgsOrRets::Rets && is_baldrdash {
        ret.reverse();
    }

    let extra_arg = if add_ret_area_ptr {
        debug_assert!(args_or_rets == ArgsOrRets::Args);
        if next_xreg < max_reg_vals {
            ret.push(ABIArg::Reg(xreg(next_xreg).to_real_reg(), I64));
        } else {
            ret.push(ABIArg::Stack(next_stack as i64, I64));
            next_stack += 8;
        }
        Some(ret.len() - 1)
    } else {
        None
    };

    next_stack = (next_stack + 15) & !15;

    // To avoid overflow issues, limit the arg/return size to something
    // reasonable -- here, 128 MB.
    if next_stack > STACK_ARG_RET_SIZE_LIMIT {
        return Err(CodegenError::ImplLimitExceeded);
    }

    Ok((ret, next_stack as i64, extra_arg))
}

impl ABISig {
    fn from_func_sig(sig: &ir::Signature) -> CodegenResult<ABISig> {
        // Compute args and retvals from signature. Handle retvals first,
        // because we may need to add a return-area arg to the args.
        let (rets, stack_ret_space, _) = compute_arg_locs(
            sig.call_conv,
            &sig.returns,
            ArgsOrRets::Rets,
            /* extra ret-area ptr = */ false,
        )?;
        let need_stack_return_area = stack_ret_space > 0;
        let (args, stack_arg_space, stack_ret_arg) = compute_arg_locs(
            sig.call_conv,
            &sig.params,
            ArgsOrRets::Args,
            need_stack_return_area,
        )?;

        trace!(
            "ABISig: sig {:?} => args = {:?} rets = {:?} arg stack = {} ret stack = {} stack_ret_arg = {:?}",
            sig,
            args,
            rets,
            stack_arg_space,
            stack_ret_space,
            stack_ret_arg
        );

        Ok(ABISig {
            args,
            rets,
            stack_arg_space,
            stack_ret_space,
            stack_ret_arg,
            call_conv: sig.call_conv,
        })
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
    total_frame_size: Option<u32>,
    /// The register holding the return-area pointer, if needed.
    ret_area_ptr: Option<Writable<Reg>>,
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
        types::B8X16
        | types::I8X16
        | types::B16X8
        | types::I16X8
        | types::B32X4
        | types::I32X4
        | types::B64X2
        | types::I64X2 => true,
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
                let mem = MemArg::RegOffset(base, offset.into(), I64);
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
    pub fn new(f: &ir::Function, flags: settings::Flags) -> CodegenResult<Self> {
        debug!("AArch64 ABI: func signature {:?}", f.signature);

        let sig = ABISig::from_func_sig(&f.signature)?;

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

        Ok(Self {
            sig,
            stackslots,
            stackslots_size: stack_offset,
            clobbered: Set::empty(),
            spillslots: None,
            total_frame_size: None,
            ret_area_ptr: None,
            call_conv,
            flags,
            is_leaf: f.is_leaf(),
            stack_limit,
        })
    }

    /// Returns the offset from FP to the argument area, i.e., jumping over the saved FP, return
    /// address, and maybe other standard elements depending on ABI (e.g. Wasm TLS reg).
    fn fp_to_arg_offset(&self) -> i64 {
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
    /// this routine makes do with the `spilltmp_reg` as one temporary
    /// register, and a second register of `tmp2` which is caller-saved. This
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
        // into a register.
        let scratch = writable_spilltmp_reg();
        let scratch2 = writable_tmp2_reg();
        let stack_size = u64::from(stack_size);
        if let Some(imm12) = Imm12::maybe_from_u64(stack_size) {
            insts.push(Inst::AluRRImm12 {
                alu_op: ALUOp::Add64,
                rd: scratch,
                rn: stack_limit,
                imm12,
            });
        } else {
            insts.extend(Inst::load_constant(scratch2, stack_size.into()));
            insts.push(Inst::AluRRRExtend {
                alu_op: ALUOp::Add64,
                rd: scratch,
                rn: stack_limit,
                rm: scratch2.to_reg(),
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
            insts.push(Inst::OneWayCondBr {
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

fn load_stack(mem: MemArg, into_reg: Writable<Reg>, ty: Type) -> Inst {
    match ty {
        types::B1 | types::B8 | types::I8 => Inst::ULoad8 {
            rd: into_reg,
            mem,
            srcloc: None,
        },
        types::B16 | types::I16 => Inst::ULoad16 {
            rd: into_reg,
            mem,
            srcloc: None,
        },
        types::B32 | types::I32 => Inst::ULoad32 {
            rd: into_reg,
            mem,
            srcloc: None,
        },
        types::B64 | types::I64 => Inst::ULoad64 {
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
        _ => unimplemented!("load_stack({})", ty),
    }
}

fn store_stack(mem: MemArg, from_reg: Reg, ty: Type) -> Inst {
    match ty {
        types::B1 | types::B8 | types::I8 => Inst::Store8 {
            rd: from_reg,
            mem,
            srcloc: None,
        },
        types::B16 | types::I16 => Inst::Store16 {
            rd: from_reg,
            mem,
            srcloc: None,
        },
        types::B32 | types::I32 => Inst::Store32 {
            rd: from_reg,
            mem,
            srcloc: None,
        },
        types::B64 | types::I64 => Inst::Store64 {
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

fn get_caller_saves(call_conv: isa::CallConv) -> Vec<Writable<Reg>> {
    let mut caller_saved = Vec::new();
    for i in 0..29 {
        let x = writable_xreg(i);
        if is_caller_save(call_conv, x.to_reg().to_real_reg()) {
            caller_saved.push(x);
        }
    }
    for i in 0..32 {
        let v = writable_vreg(i);
        if is_caller_save(call_conv, v.to_reg().to_real_reg()) {
            caller_saved.push(v);
        }
    }
    caller_saved
}

impl ABIBody for AArch64ABIBody {
    type I = Inst;

    fn temp_needed(&self) -> bool {
        self.sig.stack_ret_arg.is_some()
    }

    fn init(&mut self, maybe_tmp: Option<Writable<Reg>>) {
        if self.sig.stack_ret_arg.is_some() {
            assert!(maybe_tmp.is_some());
            self.ret_area_ptr = maybe_tmp;
        }
    }

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
            &ABIArg::Stack(off, ty) => load_stack(
                MemArg::FPOffset(self.fp_to_arg_offset() + off, ty),
                into_reg,
                ty,
            ),
        }
    }

    fn gen_retval_area_setup(&self) -> Option<Inst> {
        if let Some(i) = self.sig.stack_ret_arg {
            let inst = self.gen_copy_arg_to_reg(i, self.ret_area_ptr.unwrap());
            trace!(
                "gen_retval_area_setup: inst {:?}; ptr reg is {:?}",
                inst,
                self.ret_area_ptr.unwrap().to_reg()
            );
            Some(inst)
        } else {
            trace!("gen_retval_area_setup: not needed");
            None
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
                let from_bits = ty_bits(ty) as u8;
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
                let from_bits = ty_bits(ty) as u8;
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
                let mem = MemArg::RegOffset(self.ret_area_ptr.unwrap().to_reg(), off, ty);
                ret.push(store_stack(mem, from_reg.to_reg(), ty))
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

    /// Load from a stackslot.
    fn load_stackslot(
        &self,
        slot: StackSlot,
        offset: u32,
        ty: Type,
        into_reg: Writable<Reg>,
    ) -> Inst {
        // Offset from beginning of stackslot area, which is at nominal-SP (see
        // [MemArg::NominalSPOffset] for more details on nominal-SP tracking).
        let stack_off = self.stackslots[slot.as_u32() as usize] as i64;
        let sp_off: i64 = stack_off + (offset as i64);
        trace!("load_stackslot: slot {} -> sp_off {}", slot, sp_off);
        load_stack(MemArg::NominalSPOffset(sp_off, ty), into_reg, ty)
    }

    /// Store to a stackslot.
    fn store_stackslot(&self, slot: StackSlot, offset: u32, ty: Type, from_reg: Reg) -> Inst {
        // Offset from beginning of stackslot area, which is at nominal-SP (see
        // [MemArg::NominalSPOffset] for more details on nominal-SP tracking).
        let stack_off = self.stackslots[slot.as_u32() as usize] as i64;
        let sp_off: i64 = stack_off + (offset as i64);
        trace!("store_stackslot: slot {} -> sp_off {}", slot, sp_off);
        store_stack(MemArg::NominalSPOffset(sp_off, ty), from_reg, ty)
    }

    /// Produce an instruction that computes a stackslot address.
    fn stackslot_addr(&self, slot: StackSlot, offset: u32, into_reg: Writable<Reg>) -> Inst {
        // Offset from beginning of stackslot area, which is at nominal-SP (see
        // [MemArg::NominalSPOffset] for more details on nominal-SP tracking).
        let stack_off = self.stackslots[slot.as_u32() as usize] as i64;
        let sp_off: i64 = stack_off + (offset as i64);
        Inst::LoadAddr {
            rd: into_reg,
            mem: MemArg::NominalSPOffset(sp_off, I8),
        }
    }

    /// Load from a spillslot.
    fn load_spillslot(&self, slot: SpillSlot, ty: Type, into_reg: Writable<Reg>) -> Inst {
        // Offset from beginning of spillslot area, which is at nominal-SP + stackslots_size.
        let islot = slot.get() as i64;
        let spill_off = islot * 8;
        let sp_off = self.stackslots_size as i64 + spill_off;
        trace!("load_spillslot: slot {:?} -> sp_off {}", slot, sp_off);
        load_stack(MemArg::NominalSPOffset(sp_off, ty), into_reg, ty)
    }

    /// Store to a spillslot.
    fn store_spillslot(&self, slot: SpillSlot, ty: Type, from_reg: Reg) -> Inst {
        // Offset from beginning of spillslot area, which is at nominal-SP + stackslots_size.
        let islot = slot.get() as i64;
        let spill_off = islot * 8;
        let sp_off = self.stackslots_size as i64 + spill_off;
        trace!("store_spillslot: slot {:?} -> sp_off {}", slot, sp_off);
        store_stack(MemArg::NominalSPOffset(sp_off, ty), from_reg, ty)
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

        // N.B.: "nominal SP", which we use to refer to stackslots
        // and spillslots, is *here* (the value of SP at this program point).
        // If we push any clobbers below, we emit a virtual-SP adjustment
        // meta-instruction so that the nominal-SP references behave as if SP
        // were still at this point. See documentation for
        // [crate::isa::aarch64::abi](this module) for more details on
        // stackframe layout and nominal-SP maintenance.

        // Save clobbered registers.
        let (clobbered_int, clobbered_vec) =
            get_callee_saves(self.call_conv, self.clobbered.to_vec());
        let mut clobber_size = 0;
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
            clobber_size += 16;
        }
        let vec_save_bytes = clobbered_vec.len() * 16;
        if vec_save_bytes != 0 {
            insts.push(Inst::AluRRImm12 {
                alu_op: ALUOp::Sub64,
                rd: writable_stack_reg(),
                rn: stack_reg(),
                imm12: Imm12::maybe_from_u64(vec_save_bytes as u64).unwrap(),
            });
            clobber_size += vec_save_bytes;
        }
        for (i, reg) in clobbered_vec.iter().enumerate() {
            insts.push(Inst::FpuStore128 {
                rd: reg.to_reg().to_reg(),
                mem: MemArg::Unscaled(stack_reg(), SImm9::maybe_from_i64((i * 16) as i64).unwrap()),
                srcloc: None,
            });
        }

        if clobber_size > 0 {
            insts.push(Inst::VirtualSPOffsetAdj {
                offset: clobber_size as i64,
            });
        }

        self.total_frame_size = Some(total_stacksize);
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

        // N.B.: we do *not* emit a nominal-SP adjustment here, because (i) there will be no
        // references to nominal-SP offsets before the return below, and (ii) the instruction
        // emission tracks running SP offset linearly (in straight-line order), not according to
        // the CFG, so early returns in the middle of function bodies would cause an incorrect
        // offset for the rest of the body.

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
        self.total_frame_size
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
    uses: Vec<Reg>,
    defs: Vec<Writable<Reg>>,
    dest: CallDest,
    loc: ir::SourceLoc,
    opcode: ir::Opcode,
}

fn abisig_to_uses_and_defs(sig: &ABISig) -> (Vec<Reg>, Vec<Writable<Reg>>) {
    // Compute uses: all arg regs.
    let mut uses = Vec::new();
    for arg in &sig.args {
        match arg {
            &ABIArg::Reg(reg, _) => uses.push(reg.to_reg()),
            _ => {}
        }
    }

    // Compute defs: all retval regs, and all caller-save (clobbered) regs.
    let mut defs = get_caller_saves(sig.call_conv);
    for ret in &sig.rets {
        match ret {
            &ABIArg::Reg(reg, _) => defs.push(Writable::from_reg(reg.to_reg())),
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
    ) -> CodegenResult<AArch64ABICall> {
        let sig = ABISig::from_func_sig(sig)?;
        let (uses, defs) = abisig_to_uses_and_defs(&sig);
        Ok(AArch64ABICall {
            sig,
            uses,
            defs,
            dest: CallDest::ExtName(extname.clone(), dist),
            loc,
            opcode: ir::Opcode::Call,
        })
    }

    /// Create a callsite ABI object for a call to a function pointer with the
    /// given signature.
    pub fn from_ptr(
        sig: &ir::Signature,
        ptr: Reg,
        loc: ir::SourceLoc,
        opcode: ir::Opcode,
    ) -> CodegenResult<AArch64ABICall> {
        let sig = ABISig::from_func_sig(sig)?;
        let (uses, defs) = abisig_to_uses_and_defs(&sig);
        Ok(AArch64ABICall {
            sig,
            uses,
            defs,
            dest: CallDest::Reg(ptr),
            loc,
            opcode,
        })
    }
}

fn adjust_stack<C: LowerCtx<I = Inst>>(ctx: &mut C, amount: u64, is_sub: bool) {
    if amount == 0 {
        return;
    }

    let sp_adjustment = if is_sub {
        amount as i64
    } else {
        -(amount as i64)
    };
    ctx.emit(Inst::VirtualSPOffsetAdj {
        offset: sp_adjustment,
    });

    let alu_op = if is_sub { ALUOp::Sub64 } else { ALUOp::Add64 };
    if let Some(imm12) = Imm12::maybe_from_u64(amount) {
        ctx.emit(Inst::AluRRImm12 {
            alu_op,
            rd: writable_stack_reg(),
            rn: stack_reg(),
            imm12,
        })
    } else {
        ctx.emit(Inst::LoadConst64 {
            rd: writable_spilltmp_reg(),
            const_data: amount,
        });
        ctx.emit(Inst::AluRRRExtend {
            alu_op,
            rd: writable_stack_reg(),
            rn: stack_reg(),
            rm: spilltmp_reg(),
            extendop: ExtendOp::UXTX,
        });
    }
}

impl ABICall for AArch64ABICall {
    type I = Inst;

    fn num_args(&self) -> usize {
        if self.sig.stack_ret_arg.is_some() {
            self.sig.args.len() - 1
        } else {
            self.sig.args.len()
        }
    }

    fn emit_stack_pre_adjust<C: LowerCtx<I = Self::I>>(&self, ctx: &mut C) {
        let off = self.sig.stack_arg_space + self.sig.stack_ret_space;
        adjust_stack(ctx, off as u64, /* is_sub = */ true)
    }

    fn emit_stack_post_adjust<C: LowerCtx<I = Self::I>>(&self, ctx: &mut C) {
        let off = self.sig.stack_arg_space + self.sig.stack_ret_space;
        adjust_stack(ctx, off as u64, /* is_sub = */ false)
    }

    fn emit_copy_reg_to_arg<C: LowerCtx<I = Self::I>>(
        &self,
        ctx: &mut C,
        idx: usize,
        from_reg: Reg,
    ) {
        match &self.sig.args[idx] {
            &ABIArg::Reg(reg, ty) => ctx.emit(Inst::gen_move(
                Writable::from_reg(reg.to_reg()),
                from_reg,
                ty,
            )),
            &ABIArg::Stack(off, ty) => {
                ctx.emit(store_stack(MemArg::SPOffset(off, ty), from_reg, ty))
            }
        }
    }

    fn emit_copy_retval_to_reg<C: LowerCtx<I = Self::I>>(
        &self,
        ctx: &mut C,
        idx: usize,
        into_reg: Writable<Reg>,
    ) {
        match &self.sig.rets[idx] {
            &ABIArg::Reg(reg, ty) => ctx.emit(Inst::gen_move(into_reg, reg.to_reg(), ty)),
            &ABIArg::Stack(off, ty) => {
                let ret_area_base = self.sig.stack_arg_space;
                ctx.emit(load_stack(
                    MemArg::SPOffset(off + ret_area_base, ty),
                    into_reg,
                    ty,
                ));
            }
        }
    }

    fn emit_call<C: LowerCtx<I = Self::I>>(&mut self, ctx: &mut C) {
        let (uses, defs) = (
            mem::replace(&mut self.uses, Default::default()),
            mem::replace(&mut self.defs, Default::default()),
        );
        if let Some(i) = self.sig.stack_ret_arg {
            let rd = ctx.alloc_tmp(RegClass::I64, I64);
            let ret_area_base = self.sig.stack_arg_space;
            ctx.emit(Inst::LoadAddr {
                rd,
                mem: MemArg::SPOffset(ret_area_base, I8),
            });
            self.emit_copy_reg_to_arg(ctx, i, rd.to_reg());
        }
        match &self.dest {
            &CallDest::ExtName(ref name, RelocDistance::Near) => ctx.emit(Inst::Call {
                info: Box::new(CallInfo {
                    dest: name.clone(),
                    uses,
                    defs,
                    loc: self.loc,
                    opcode: self.opcode,
                }),
            }),
            &CallDest::ExtName(ref name, RelocDistance::Far) => {
                ctx.emit(Inst::LoadExtName {
                    rd: writable_spilltmp_reg(),
                    name: Box::new(name.clone()),
                    offset: 0,
                    srcloc: self.loc,
                });
                ctx.emit(Inst::CallInd {
                    info: Box::new(CallIndInfo {
                        rn: spilltmp_reg(),
                        uses,
                        defs,
                        loc: self.loc,
                        opcode: self.opcode,
                    }),
                });
            }
            &CallDest::Reg(reg) => ctx.emit(Inst::CallInd {
                info: Box::new(CallIndInfo {
                    rn: reg,
                    uses,
                    defs,
                    loc: self.loc,
                    opcode: self.opcode,
                }),
            }),
        }
    }
}

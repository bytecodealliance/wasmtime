//! Lowering backend for Pulley.

pub mod isle;

use super::{PulleyBackend, PulleyTargetKind, inst::*};
use crate::{
    ir::{self, InstructionData, Opcode},
    isa::pulley_shared::inst::Inst,
    machinst::{lower::*, *},
};

impl<P> LowerBackend for PulleyBackend<P>
where
    P: PulleyTargetKind,
{
    type MInst = InstAndKind<P>;

    fn lower(&self, ctx: &mut Lower<Self::MInst>, ir_inst: ir::Inst) -> Option<InstOutput> {
        isle::lower(ctx, self, ir_inst)
    }

    fn lower_branch(
        &self,
        ctx: &mut Lower<Self::MInst>,
        ir_inst: ir::Inst,
        targets: &[MachLabel],
    ) -> Option<()> {
        // Phase-2/3 fuse band+brif+xload+xload across the brif and its
        // continuation block; phase-1 just band+brif. Both gated on the
        // eager-init predicate.
        if try_fuse_funcref_dispatch::<P>(ctx, ir_inst, targets) {
            return Some(());
        }
        if try_fuse_band_brif(ctx, ir_inst, targets) {
            return Some(());
        }
        isle::lower_branch(ctx, self, ir_inst, targets)
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        // Pulley does not support this feature right now.
        None
    }

    fn pre_lower(&self, ctx: &mut Lower<Self::MInst>) {
        // Block lowering runs in reverse layout order, so by the time
        // `lower_branch` sees the brif, the continuation block has already
        // been lowered. Marking the continuation's loads `absorbed_pure`
        // after the fact would create double-writes to their result vregs.
        // Run the recogniser once up front instead.
        pre_lower_pulley(ctx, P::pointer_width().bytes());
    }
}

/// Recognise `brif (band v -2) ...` at the call_indirect lazy-init site
/// and fuse it into `MInst::BandBrIf`. Returns true if fusion fired.
///
/// Soundness: testing `v_masked != 0` instead of `v != 0` is identical for
/// every reachable funcref-slot value under
/// `is_eagerly_initialized_funcref_table` — they differ only at the
/// tagged-null value `1`, which the predicate excludes.
fn try_fuse_band_brif<P>(
    ctx: &mut Lower<InstAndKind<P>>,
    ir_inst: ir::Inst,
    targets: &[MachLabel],
) -> bool
where
    P: PulleyTargetKind,
{
    if targets.len() != 2 {
        return false;
    }

    let dfg = ctx.dfg();
    let InstructionData::Brif {
        opcode: Opcode::Brif,
        arg: cond,
        ..
    } = dfg.insts[ir_inst]
    else {
        return false;
    };

    // The brif's cond must be `band(v, -2)` with a bit-exact `Imm64(-2)`.
    // The bit-exact match is load-bearing: it confines the fusion to
    // func_environ's `Imm64::from(-2_i64)` IR-rewrite site. The wat parser
    // encodes `(i32.const -2)` as `Imm64(0xFFFFFFFE)`, so user wasm can't
    // produce `Imm64(-2)` and slip into this code path.
    let band_inst = match dfg.value_def(cond).inst() {
        Some(inst) => inst,
        None => return false,
    };
    let (band_src, band_imm) = match dfg.insts[band_inst] {
        InstructionData::Binary {
            opcode: Opcode::Band,
            args: [a, b],
        } => match dfg.value_def(b).inst() {
            Some(b_inst) => match dfg.insts[b_inst] {
                InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } if imm.bits() == -2 => (a, -2_i8),
                _ => return false,
            },
            None => return false,
        },
        _ => return false,
    };

    // Both ops of the fusion must agree on size: the band's result is the
    // brif's cond, and its type drives the comparison width.
    let cond_ty = dfg.value_type(cond);
    let size = match cond_ty {
        ir::types::I32 => OperandSize::Size32,
        ir::types::I64 => OperandSize::Size64,
        _ => return false,
    };

    // Reuse the band-result vreg as the fused op's dst, so the block-arg
    // machinery downstream observes the correct masked value via the same
    // vreg (single def, single use — no SSA violation). The original band
    // CLIF inst is then marked as absorbed and skipped in lower_clif_block.
    let dst_vreg = ctx.put_value_in_regs(cond);
    let dst_reg = dst_vreg.only_reg().expect("scalar band result");
    let dst = WritableXReg::try_from(Writable::from_reg(dst_reg))
        .expect("band result is an x-class register");
    let src = XReg::new(ctx.put_value_in_regs(band_src).only_reg().expect("scalar"))
        .expect("band source is an x-class register");

    // Sink the band: the BandBrIf we emit below defines the same dst vreg,
    // so downstream uses of `cond` still find the value populated.
    ctx.sink_pure_inst(band_inst);

    ctx.emit(
        Inst::BandBrIf {
            dst,
            src,
            mask: band_imm,
            size,
            taken: targets[0],
            not_taken: targets[1],
        }
        .into(),
    );

    true
}

/// True iff `imm` encodes `-2` in `ty`'s width. The egraph canonicalises
/// `i32(-2)` as `Imm64(0xFFFFFFFE)`, not `Imm64(-2)`, so a width-aware
/// compare is needed for pulley32.
fn is_minus_two_for(imm: ir::immediates::Imm64, ty: ir::Type) -> bool {
    match ty {
        ir::types::I32 => (imm.bits() as u32) == (-2_i32 as u32),
        ir::types::I64 => imm.bits() == -2_i64,
        _ => false,
    }
}

/// `(wasm_call, vmctx)` byte offsets in `VMFuncRef`. Both fit in i8 (8/24
/// on 64-bit, 4/12 on 32-bit), matching the `xfuncref_dispatch_*` ops'
/// sign-extended-i8 offset operand.
fn vm_func_ref_offsets(pointer_bytes: u8) -> (i8, i8) {
    let size = pointer_bytes as i8;
    (size, size.checked_mul(3).expect("VMFuncRef offsets fit i8"))
}

/// Recognise the canonical funcref-dispatch shape:
///
/// ```text
/// predecessor:
///     value        = load .ptr (table_entry + 0)
///     value_masked = band value, -2
///     brif value_masked, continuation([value_masked]), null_block([])
/// continuation(funcref_ptr):
///     code  = load .ptr (funcref_ptr + offset_code)
///     vmctx = load .ptr (funcref_ptr + offset_vmctx)
/// ```
fn match_funcref_dispatch_pattern<P: PulleyTargetKind>(
    f: &ir::Function,
    brif_inst: ir::Inst,
    pointer_bytes: u8,
) -> Option<FuncrefDispatchPattern> {
    let dfg = &f.dfg;
    let InstructionData::Brif {
        opcode: Opcode::Brif,
        arg: cond,
        blocks,
        ..
    } = dfg.insts[brif_inst]
    else {
        return None;
    };
    // cond = band(v, -2)
    let band_inst = dfg.value_def(cond).inst()?;
    let (v, _imm) = match dfg.insts[band_inst] {
        InstructionData::Binary {
            opcode: Opcode::Band,
            args: [a, b],
        } => match dfg.value_def(b).inst() {
            Some(b_inst) => match dfg.insts[b_inst] {
                InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } if is_minus_two_for(imm, dfg.value_type(cond)) => (a, -2_i8),
                _ => return None,
            },
            None => return None,
        },
        _ => return None,
    };
    let cond_ty = dfg.value_type(cond);
    let size = match cond_ty {
        ir::types::I32 => OperandSize::Size32,
        ir::types::I64 => OperandSize::Size64,
        _ => return None,
    };
    // The 64-bit fused op handles I64 pointer types; the 32-bit fused op
    // handles I32. They line up with the target's pointer width.
    let expected_size = match pointer_bytes {
        4 => OperandSize::Size32,
        8 => OperandSize::Size64,
        _ => return None,
    };
    if size != expected_size {
        return None;
    }

    // Taken target = continuation block. Its first block param must equal
    // the brif's first block-call-arg (i.e. value_masked).
    let taken_call = blocks[0];
    let continuation = taken_call.block(&dfg.value_lists);
    let taken_args: smallvec::SmallVec<[ir::BlockArg; 4]> =
        taken_call.args(&dfg.value_lists).collect();
    if taken_args.len() < 1 {
        return None;
    }
    let first_arg_val = match taken_args[0] {
        ir::BlockArg::Value(v) => v,
        _ => return None,
    };
    if first_arg_val != cond {
        // The brif must pass value_masked as the first block-call-arg.
        return None;
    }
    let cont_params = dfg.block_params(continuation);
    if cont_params.is_empty() {
        return None;
    }
    let funcref_ptr = cont_params[0];

    // The first two instructions in the continuation block must be the
    // two field loads in either order.
    let (offset_code_expected, offset_vmctx_expected) = vm_func_ref_offsets(pointer_bytes);
    let mut iter = f.layout.block_insts(continuation);
    let load1 = iter.next()?;
    let load2 = iter.next()?;
    let (load_code_inst, load_vmctx_inst) = classify_funcref_loads(
        dfg,
        load1,
        load2,
        funcref_ptr,
        offset_code_expected,
        offset_vmctx_expected,
        cond_ty,
    )?;
    let code_val = dfg.inst_results(load_code_inst)[0];
    let vmctx_val = dfg.inst_results(load_vmctx_inst)[0];

    let _ = (band_inst, v); // captured for future variants of the pattern check
    Some(FuncrefDispatchPattern {
        load_code_inst,
        load_vmctx_inst,
        code_val,
        vmctx_val,
        offset_code: offset_code_expected,
        offset_vmctx: offset_vmctx_expected,
        size,
    })
}

struct FuncrefDispatchPattern {
    load_code_inst: ir::Inst,
    load_vmctx_inst: ir::Inst,
    code_val: ir::Value,
    vmctx_val: ir::Value,
    offset_code: i8,
    offset_vmctx: i8,
    size: OperandSize,
}

fn classify_funcref_loads(
    dfg: &ir::DataFlowGraph,
    a: ir::Inst,
    b: ir::Inst,
    funcref_ptr: ir::Value,
    offset_code: i8,
    offset_vmctx: i8,
    pointer_ty: ir::Type,
) -> Option<(ir::Inst, ir::Inst)> {
    let (a_off, a_base) = classify_load(dfg, a, pointer_ty)?;
    let (b_off, b_base) = classify_load(dfg, b, pointer_ty)?;
    if a_base != funcref_ptr || b_base != funcref_ptr {
        return None;
    }
    if a_off == offset_code && b_off == offset_vmctx {
        Some((a, b))
    } else if a_off == offset_vmctx && b_off == offset_code {
        Some((b, a))
    } else {
        None
    }
}

fn classify_load(
    dfg: &ir::DataFlowGraph,
    inst: ir::Inst,
    pointer_ty: ir::Type,
) -> Option<(i8, ir::Value)> {
    match dfg.insts[inst] {
        InstructionData::Load {
            opcode: Opcode::Load,
            arg,
            offset,
            ..
        } => {
            let result = *dfg.inst_results(inst).first()?;
            if dfg.value_type(result) != pointer_ty {
                return None;
            }
            let off_i32: i32 = offset.into();
            let off_i8 = i8::try_from(off_i32).ok()?;
            Some((off_i8, arg))
        }
        _ => None,
    }
}

/// Pulley-specific pre-lowering analysis. Walks every block looking for
/// the funcref-dispatch fusion shape (see
/// `match_funcref_dispatch_pattern`), and when it matches, sinks the band
/// inst and the two continuation-block loads via `sink_pure_inst`. The
/// brif's lowering (in `try_fuse_funcref_dispatch`) then emits one
/// `MInst::FuncrefDispatch` whose def vregs replace the absorbed loads'
/// def vregs.
fn pre_lower_pulley<P>(ctx: &mut Lower<InstAndKind<P>>, pointer_bytes: u8)
where
    P: PulleyTargetKind,
{
    // Collect candidates first so `&ctx.f` isn't held across the
    // `sink_pure_inst` calls below.
    let mut to_sink: smallvec::SmallVec<[(ir::Inst, ir::Inst); 8]> = smallvec::SmallVec::new();
    {
        let f = ctx.f;
        for block in f.layout.blocks() {
            let Some(term) = f.layout.last_inst(block) else {
                continue;
            };
            if !matches!(f.dfg.insts[term], InstructionData::Brif { .. }) {
                continue;
            }
            if let Some(pat) = match_funcref_dispatch_pattern::<P>(f, term, pointer_bytes) {
                to_sink.push((pat.load_code_inst, pat.load_vmctx_inst));
            }
        }
    }
    for (l_code, l_vmctx) in to_sink {
        ctx.sink_pure_inst(l_code);
        ctx.sink_pure_inst(l_vmctx);
    }
}

/// Phase-2 fusion: emit `MInst::FuncrefDispatch` when the brif matches the
/// canonical pattern. Relies on the pre-pass having marked the band + two
/// continuation-block loads as absorbed_pure; this routine just re-derives
/// the pattern, looks up the relevant vregs, and emits the single fused
/// MachInst. Returns `true` iff the fusion fired.
fn try_fuse_funcref_dispatch<P>(
    ctx: &mut Lower<InstAndKind<P>>,
    ir_inst: ir::Inst,
    targets: &[MachLabel],
) -> bool
where
    P: PulleyTargetKind,
{
    if targets.len() != 2 {
        return false;
    }
    let pointer_bytes = P::pointer_width().bytes();
    let Some(pat) = match_funcref_dispatch_pattern::<P>(ctx.f, ir_inst, pointer_bytes) else {
        return false;
    };

    let InstructionData::Brif { arg: cond, .. } = ctx.f.dfg.insts[ir_inst] else {
        return false;
    };

    // Try phase-3 (absorb the band into BandFuncrefDispatch). The fused
    // op defines `dst_masked` (= cond's vreg) so the brif's block-call
    // copy still has a producer, plus `dst_code` and `dst_vmctx`.
    let dfg = ctx.dfg();
    let band_inst = dfg.value_def(cond).inst();
    let v = band_inst.and_then(|bi| match dfg.insts[bi] {
        InstructionData::Binary {
            opcode: Opcode::Band,
            args: [a, b],
        } => match dfg.value_def(b).inst() {
            Some(b_inst) => match dfg.insts[b_inst] {
                InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } if is_minus_two_for(imm, dfg.value_type(cond)) => Some(a),
                _ => None,
            },
            None => None,
        },
        _ => None,
    });

    // The loads' result vregs become the fused op's defs. Their original
    // lowering was skipped via `sink_pure_inst` in `pre_lower_pulley`.
    let dst_code_reg = ctx
        .put_value_in_regs(pat.code_val)
        .only_reg()
        .expect("scalar funcref code result");
    let dst_vmctx_reg = ctx
        .put_value_in_regs(pat.vmctx_val)
        .only_reg()
        .expect("scalar funcref vmctx result");
    let dst_code = WritableXReg::try_from(Writable::from_reg(dst_code_reg))
        .expect("funcref code dst is an x-class register");
    let dst_vmctx = WritableXReg::try_from(Writable::from_reg(dst_vmctx_reg))
        .expect("funcref vmctx dst is an x-class register");

    if let (Some(band_inst), Some(v)) = (band_inst, v) {
        // Phase 3 fires: source is the unmasked `v`; the fused op masks
        // internally and writes `dst_masked = cond`.
        let dst_masked_regs = ctx.put_value_in_regs(cond);
        let dst_masked_reg = dst_masked_regs.only_reg().expect("scalar cond");
        let dst_masked = WritableXReg::try_from(Writable::from_reg(dst_masked_reg))
            .expect("cond is an x-class register");
        let src_reg = ctx
            .put_value_in_regs(v)
            .only_reg()
            .expect("scalar funcref source");
        let src = XReg::new(src_reg).expect("funcref source is an x-class register");
        ctx.sink_pure_inst(band_inst);
        ctx.emit(
            Inst::BandFuncrefDispatch {
                dst_masked,
                dst_code,
                dst_vmctx,
                src,
                offset_code: pat.offset_code,
                offset_vmctx: pat.offset_vmctx,
                size: pat.size,
                taken: targets[0],
                not_taken: targets[1],
            }
            .into(),
        );
        return true;
    }

    // Phase-2 fallback: band stays as a standalone op; FuncrefDispatch
    // consumes its masked result.
    let src_reg = ctx
        .put_value_in_regs(cond)
        .only_reg()
        .expect("scalar funcref source");
    let src = XReg::new(src_reg).expect("funcref source is an x-class register");

    ctx.emit(
        Inst::FuncrefDispatch {
            dst_code,
            dst_vmctx,
            src,
            offset_code: pat.offset_code,
            offset_vmctx: pat.offset_vmctx,
            size: pat.size,
            taken: targets[0],
            not_taken: targets[1],
        }
        .into(),
    );

    true
}

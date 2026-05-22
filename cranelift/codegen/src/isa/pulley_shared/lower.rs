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
        // Phase-2 first: try fusing band+brif+xload+xload across the brif's
        // predecessor block and its taken (continuation) target. The matching
        // continuation-block loads were marked `absorbed_pure` by the
        // `pre_lower` analysis hook below, so they have already been skipped
        // in `lower_clif_block` and the FuncrefDispatch MachInst here defs
        // their result vregs directly.
        if try_fuse_funcref_dispatch::<P>(ctx, ir_inst, targets) {
            return Some(());
        }
        // Phase-1 fallback: fuse just band+brif (no continuation loads).
        // Emits MInst::BandBrIf. See the doc-comment on the variant in
        // `pulley_shared::inst::Inst`.
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
        // Cross-block fusion analysis for phase-2 funcref dispatch.
        //
        // The main block-lowering loop runs in reverse layout order, so by
        // the time `lower_branch` fires for the predecessor's brif, its
        // taken target (the continuation block) has already had its
        // instructions emitted to VCode. Marking the continuation's loads
        // as `inst_absorbed_pure` AFTER that point is too late — the loads
        // have already been lowered into MachInsts that write to the
        // result vregs, and the FuncrefDispatch we'd emit at brif time
        // would double-write to those same vregs (SSA violation).
        //
        // This analysis runs once before any block is lowered. For each
        // brif whose cond is `band(v, -2)` AND whose taken target is a
        // block that starts with two loads from the brif's first
        // block-call-arg at the canonical VMFuncRef wasm_call / vmctx
        // offsets, mark band + the two loads as absorbed_pure. The brif
        // lowering then sees a clean slate (no double-writes) and emits
        // one FuncrefDispatch MachInst.
        pre_lower_pulley(ctx, P::pointer_width().bytes());
    }
}

/// Recognise the `brif (band v c) block(...) cold` shape emitted by
/// `func_environ::get_or_init_func_ref_table_elem` under the
/// `is_eagerly_initialized_funcref_table` predicate, and fuse it into a
/// single `MInst::BandBrIf`. Returns `true` if the fusion fired; the caller
/// then skips the generic ISLE rule.
///
/// Soundness: testing `v_masked != 0` instead of `v != 0` is identical on
/// every funcref-slot value REACHABLE in eagerly-initialized tables. The
/// only differing case is `v == 1` (the explicit tagged-null slot value),
/// which can only appear via runtime `table.fill(null)` and is therefore
/// excluded by the `tables_mutated == false` half of the predicate.
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

    // The brif's cond must be defined by a `band v -2`. We restrict the
    // mask to exactly `-2` (the init-bit strip used by the call_indirect
    // lazy-init brif site) because the fused op tests the UNMASKED `src`
    // for non-zero, not the masked `dst`. That equivalence holds iff
    // `(v & mask != 0) <=> (v != 0)`. For mask = -2 this holds for every
    // funcref-slot value reachable in eagerly-initialized tables (the
    // soundness argument from `is_eagerly_initialized_funcref_table`).
    // For other masks the equivalence is generally false, so fusing
    // would silently flip branch direction on user-code `band+brif`
    // sites. See pulley/PR for the design discussion.
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

    // `put_value_in_regs(cond)` bumped value_lowered_uses[cond] above zero,
    // which would normally force the band's standalone lowering. Sink the
    // band as a pure absorption: the BandBrIf MInst we emit below produces
    // exactly the same dst vreg, so any future use of `cond` (e.g. the
    // brif's block-call argument) finds the right value already populated.
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

/// VMFuncRef field offsets, parameterised on the Pulley pointer width.
///
/// Mirrors `crates/environ/src/vmoffsets.rs`'s `vm_func_ref_wasm_call` (=
/// 1 * size) and `vm_func_ref_vmctx` (= 3 * size). Both fit in i8 for both
/// pointer widths (8 + 24 on 64-bit, 4 + 12 on 32-bit), which is the
/// constraint imposed by the pulley `xfuncref_dispatch_*` ops (i8
/// sign-extended offsets).
fn vm_func_ref_offsets(pointer_bytes: u8) -> (i8, i8) {
    let size = pointer_bytes as i8;
    (size, size.checked_mul(3).expect("VMFuncRef offsets fit i8"))
}

/// Recognise the canonical funcref-dispatch shape produced by
/// `func_environ::get_or_init_func_ref_table_elem` followed by
/// `load_code_and_vmctx` under the eager-init predicate + statically-
/// elided sig check:
///
/// ```text
/// predecessor:
///     value        = load .ptr (table_entry + 0)
///     value_masked = band value, -2
///     brif value_masked, continuation([value_masked]), null_block([])
///
/// continuation(funcref_ptr):
///     code  = load .ptr (funcref_ptr + offset_code)
///     vmctx = load .ptr (funcref_ptr + offset_vmctx)
///     ...                                       <- other uses of code, vmctx
/// ```
///
/// If found, returns the brif inst, the band inst, the two load insts (in
/// continuation), the funcref source value `v` (band's first arg), the
/// CLIF result values `code` and `vmctx`, and the offsets. Otherwise None.
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
                } if imm.bits() == -2 => (a, -2_i8),
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

    // First two instructions in the continuation block must be the two
    // canonical loads. We tolerate the block-param ordering: load1 is
    // at offset_code, load2 at offset_vmctx (in either positional order).
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

    Some(FuncrefDispatchPattern {
        band_inst,
        load_code_inst,
        load_vmctx_inst,
        v,
        code_val,
        vmctx_val,
        offset_code: offset_code_expected,
        offset_vmctx: offset_vmctx_expected,
        size,
    })
}

struct FuncrefDispatchPattern {
    band_inst: ir::Inst,
    load_code_inst: ir::Inst,
    load_vmctx_inst: ir::Inst,
    v: ir::Value,
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
    // Collect candidates first so we don't hold &ctx.f while calling
    // sink_pure_inst (which takes &mut ctx).
    //
    // We only absorb the two field loads, NOT the band. The band stays
    // as a separate Pulley `xband_s8` op because `cond` (the band's
    // result) is the SOURCE vreg consumed by FuncrefDispatch — that
    // already-masked value gives us the branch test (`src != 0`) with
    // the same predictor-anchor semantics as the original brif. If we
    // also absorbed the band, FuncrefDispatch would have nothing
    // defining `cond`'s vreg, and the predecessor brif's block-call-arg
    // copy (which passes `cond` to the continuation block param) would
    // see an undefined vreg.
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

    // Source vreg: `cond` (the band's result — already-masked funcref
    // pointer). The band stays as a separate Pulley `xband_s8` op (we
    // do NOT sink it). Its result feeds both us and the brif's
    // block-call-arg in continuation, which is what makes the
    // predecessor brif's block-arg machinery well-defined here.
    //
    // Note we look up cond directly via the brif's cond arg — it's the
    // same value the matching pattern returned as `pat.code_val`'s base
    // (`funcref_ptr` after block-arg substitution).
    let InstructionData::Brif { arg: cond, .. } = ctx.f.dfg.insts[ir_inst] else {
        return false;
    };
    let src_reg = ctx
        .put_value_in_regs(cond)
        .only_reg()
        .expect("scalar funcref source");
    let src = XReg::new(src_reg).expect("funcref source is an x-class register");

    // Destination vregs: the loads' result values' canonical vregs.
    // pre_lower marked the loads as absorbed_pure, so their standalone
    // lowering (in the continuation block, processed earlier in reverse
    // iteration) was skipped — value_regs[code_val] and value_regs[vmctx_val]
    // are un-aliased, and our FuncrefDispatch's def of them is the sole
    // def each one has across the function.
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

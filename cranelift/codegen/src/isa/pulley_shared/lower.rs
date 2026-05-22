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
        // Peephole: fuse `brif (band v c) _ _` where the band's i8-fittable
        // immediate `c` is the only thing standing between the brif's cond
        // and the funcref load. Emitted by the call_indirect lazy-init
        // brif site when `is_eagerly_initialized_funcref_table` lets us
        // safely test the masked value. See the doc-comment on
        // `MInst::BandBrIf` for the bytecode-level shape.
        if try_fuse_band_brif(ctx, ir_inst, targets) {
            return Some(());
        }
        isle::lower_branch(ctx, self, ir_inst, targets)
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        // Pulley does not support this feature right now.
        None
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

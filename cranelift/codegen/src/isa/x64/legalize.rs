use crate::flowgraph;
use crate::ir;
use crate::isa::x64::X64Backend;
use crate::legalizer::isle;

// Used by ISLE
use crate::cursor::{Cursor, CursorPosition};
use crate::ir::condcodes::*;
use crate::ir::immediates::*;
use crate::ir::types::*;
use crate::ir::*;
use crate::machinst::isle::*;

#[allow(dead_code, unused_variables, unreachable_patterns)]
mod generated {
    include!(concat!(env!("ISLE_DIR"), "/legalize_x64.rs"));
}

pub(crate) fn run(
    isa: &X64Backend,
    func: &mut ir::Function,
    cfg: &mut flowgraph::ControlFlowGraph,
) {
    crate::legalizer::isle::run(isa, func, cfg, |cx, i| {
        generated::constructor_legalize(cx, i)
    })
}

impl generated::Context for isle::LegalizeContext<'_, X64Backend> {
    crate::isle_common_legalizer_methods!();

    /// On x64 there's no native instruction for converting an unsigned 64-bit
    /// number into a float, only a signed 64-bit number. To handle this
    /// `fcvt_from_uint` instructions are replaced with this sequence below.
    ///
    /// If the input value interpreted as a signed number is positive then
    /// `fcvt_from_sint` can be used because it will produce the same result.
    ///
    /// If the input value interpreted as a signed number is negative then
    /// the rough idea is to shift it to the right by one to halve it, use
    /// `fcvt_from_sint` on the result, and then double the result with an
    /// `fadd` afterwards.
    ///
    /// TODO: the previous incarnation of this code has a "see below for an
    /// explanation" comment for what it was doing and I don't know why the
    /// lower bit is preserved in the shifted-right temporary. I presume that
    /// has to do with something about how a u64 is so large than f64 doesn't
    /// have the precision to hold the lowest bit so this trick produces the
    /// right result "because of float precision weirdness". If someone knows
    /// better it'd be good to fill out this comment.
    fn x64_replace_fcvt_from_u64(&mut self, ty: Type, uint: Value) -> CursorPosition {
        let inst = self.replace.unwrap();
        let old_block = self
            .pos
            .func
            .layout
            .inst_block(inst)
            .expect("inst not in layout");
        let block_is_nonnegative = self.pos.func.dfg.make_block();
        let block_is_negative = self.pos.func.dfg.make_block();
        let block_resume = self.pos.func.dfg.make_block();

        // Test whether the input is negative when interpreted as a signed
        // number, and proceed from here.
        let zero = self.pos.ins().iconst(I64, 0);
        let is_negative = self.pos.ins().icmp(IntCC::SignedLessThan, uint, zero);
        self.pos.ins().brif(
            is_negative,
            block_is_negative,
            &[],
            block_is_nonnegative,
            &[],
        );

        // If the input value was negative "do the trick". More-or-less use a
        // signed conversion on a half-value and then double it afterwards.
        self.pos.insert_block(block_is_negative);
        let one = self.pos.ins().iconst(I64, 1);
        let ushr = self.pos.ins().ushr(uint, one);
        let band = self.pos.ins().band(uint, one);
        let bor = self.pos.ins().bor(ushr, band);
        let fcvt = self.pos.ins().fcvt_from_sint(ty, bor);
        let fadd = self.pos.ins().fadd(fcvt, fcvt);
        self.pos.ins().jump(block_resume, &[fadd]);

        // If the input value was positive then `fcvt_from_sint` can be used
        // since it'd produce the same result.
        self.pos.insert_block(block_is_nonnegative);
        let fcvt = self.pos.ins().fcvt_from_sint(ty, uint);
        self.pos.ins().jump(block_resume, &[fcvt]);

        // At the join point remove the previous `fcvt_from_uint` instruction
        // and change its result to alias our new result which is a parameter to
        // this block.
        self.pos.insert_block(block_resume);
        let result = self.pos.func.dfg.append_block_param(block_resume, ty);
        let prev_result = self.pos.func.dfg.first_result(inst);
        self.pos.func.dfg.clear_results(inst);
        self.pos.func.dfg.change_to_alias(prev_result, result);
        self.pos.func.layout.remove_inst(inst);

        // Finally update the CFG.
        self.cfg.recompute_block(self.pos.func, old_block);
        self.cfg
            .recompute_block(self.pos.func, block_is_nonnegative);
        self.cfg.recompute_block(self.pos.func, block_is_negative);
        self.cfg.recompute_block(self.pos.func, block_resume);

        CursorPosition::Before(block_resume)
    }
}

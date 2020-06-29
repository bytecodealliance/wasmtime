//! This module implements lowering (instruction selection) from Cranelift IR
//! to machine instructions with virtual registers. This is *almost* the final
//! machine code, except for register allocation.

use crate::entity::SecondaryMap;
use crate::fx::{FxHashMap, FxHashSet};
use crate::inst_predicates::is_safepoint;
use crate::inst_predicates::{has_side_effect_or_load, is_constant_64bit};
use crate::ir::instructions::BranchInfo;
use crate::ir::types::I64;
use crate::ir::{
    ArgumentExtension, Block, Constant, ConstantData, ExternalName, Function, GlobalValueData,
    Inst, InstructionData, MemFlags, Opcode, Signature, SourceLoc, Type, Value, ValueDef,
};
use crate::machinst::{
    ABIBody, BlockIndex, BlockLoweringOrder, LoweredBlock, MachLabel, VCode, VCodeBuilder,
    VCodeInst,
};
use crate::CodegenResult;

use regalloc::{Reg, RegClass, StackmapRequestInfo, VirtualReg, Writable};

use alloc::boxed::Box;
use alloc::vec::Vec;
use log::debug;
use smallvec::SmallVec;

/// An "instruction color" partitions CLIF instructions by side-effecting ops.
/// All instructions with the same "color" are guaranteed not to be separated by
/// any side-effecting op (for this purpose, loads are also considered
/// side-effecting, to avoid subtle questions w.r.t. the memory model), and
/// furthermore, it is guaranteed that for any two instructions A and B such
/// that color(A) == color(B), either A dominates B and B postdominates A, or
/// vice-versa. (For now, in practice, only ops in the same basic block can ever
/// have the same color, trivially providing the second condition.) Intuitively,
/// this means that the ops of the same color must always execute "together", as
/// part of one atomic contiguous section of the dynamic execution trace, and
/// they can be freely permuted (modulo true dataflow dependencies) without
/// affecting program behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InstColor(u32);
impl InstColor {
    fn new(n: u32) -> InstColor {
        InstColor(n)
    }

    /// Get an arbitrary index representing this color. The index is unique
    /// *within a single function compilation*, but indices may be reused across
    /// functions.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// A context that machine-specific lowering code can use to emit lowered
/// instructions. This is the view of the machine-independent per-function
/// lowering context that is seen by the machine backend.
pub trait LowerCtx {
    /// The instruction type for which this lowering framework is instantiated.
    type I: VCodeInst;

    // Function-level queries:

    /// Get the `ABIBody`.
    fn abi(&mut self) -> &dyn ABIBody<I = Self::I>;
    /// Get the (virtual) register that receives the return value. A return
    /// instruction should lower into a sequence that fills this register. (Why
    /// not allow the backend to specify its own result register for the return?
    /// Because there may be multiple return points.)
    fn retval(&self, idx: usize) -> Writable<Reg>;

    // General instruction queries:

    /// Get the instdata for a given IR instruction.
    fn data(&self, ir_inst: Inst) -> &InstructionData;
    /// Get the controlling type for a polymorphic IR instruction.
    fn ty(&self, ir_inst: Inst) -> Type;
    /// Get the target for a call instruction, as an `ExternalName`. Returns a tuple
    /// providing this name and the "relocation distance", i.e., whether the backend
    /// can assume the target will be "nearby" (within some small offset) or an
    /// arbitrary address. (This comes from the `colocated` bit in the CLIF.)
    fn call_target<'b>(&'b self, ir_inst: Inst) -> Option<(&'b ExternalName, RelocDistance)>;
    /// Get the signature for a call or call-indirect instruction.
    fn call_sig<'b>(&'b self, ir_inst: Inst) -> Option<&'b Signature>;
    /// Get the symbol name, relocation distance estimate, and offset for a
    /// symbol_value instruction.
    fn symbol_value<'b>(&'b self, ir_inst: Inst) -> Option<(&'b ExternalName, RelocDistance, i64)>;
    /// Returns the memory flags of a given memory access.
    fn memflags(&self, ir_inst: Inst) -> Option<MemFlags>;
    /// Get the source location for a given instruction.
    fn srcloc(&self, ir_inst: Inst) -> SourceLoc;
    /// Get the side-effect color of the given instruction (specifically, at the
    /// program point just prior to the instruction). The "color" changes at
    /// every side-effecting op; the backend should not try to merge across
    /// side-effect colors unless the op being merged is known to be pure.
    fn inst_color(&self, ir_inst: Inst) -> InstColor;
    /// Determine whether an instruction is a safepoint.
    fn is_safepoint(&self, ir_inst: Inst) -> bool;

    // Instruction input/output queries:

    /// Get the number of inputs to the given IR instruction.
    fn num_inputs(&self, ir_inst: Inst) -> usize;
    /// Get the number of outputs to the given IR instruction.
    fn num_outputs(&self, ir_inst: Inst) -> usize;
    /// Get the type for an instruction's input.
    fn input_ty(&self, ir_inst: Inst, idx: usize) -> Type;
    /// Get the type for an instruction's output.
    fn output_ty(&self, ir_inst: Inst, idx: usize) -> Type;
    /// Get the value of a constant instruction (`iconst`, etc.) as a 64-bit
    /// value, if possible.
    fn get_constant(&self, ir_inst: Inst) -> Option<u64>;
    /// Get the input in any combination of three forms:
    ///
    /// - An instruction, if the same color as this instruction or if the
    ///   producing instruction has no side effects (thus in both cases
    ///   mergeable);
    /// - A constant, if the value is a constant;
    /// - A register.
    ///
    /// The instruction input may be available in some or all of these
    /// forms. More than one is possible: e.g., it may be produced by an
    /// instruction in the same block, but may also have been forced into a
    /// register already by an earlier op. It will *always* be available
    /// in a register, at least.
    ///
    /// If the backend uses the register, rather than one of the other
    /// forms (constant or merging of the producing op), it must call
    /// `use_input_reg()` to ensure the producing inst is actually lowered
    /// as well. Failing to do so may result in the instruction that generates
    /// this value never being generated, thus resulting in incorrect execution.
    /// For correctness, backends should thus wrap `get_input()` and
    /// `use_input_regs()` with helpers that return a register only after
    /// ensuring it is marked as used.
    fn get_input(&self, ir_inst: Inst, idx: usize) -> LowerInput;
    /// Get the `idx`th output register of the given IR instruction. When
    /// `backend.lower_inst_to_regs(ctx, inst)` is called, it is expected that
    /// the backend will write results to these output register(s).  This
    /// register will always be "fresh"; it is guaranteed not to overlap with
    /// any of the inputs, and can be freely used as a scratch register within
    /// the lowered instruction sequence, as long as its final value is the
    /// result of the computation.
    fn get_output(&self, ir_inst: Inst, idx: usize) -> Writable<Reg>;

    // Codegen primitives: allocate temps, emit instructions, set result registers,
    // ask for an input to be gen'd into a register.

    /// Get a new temp.
    fn alloc_tmp(&mut self, rc: RegClass, ty: Type) -> Writable<Reg>;
    /// Emit a machine instruction.
    fn emit(&mut self, mach_inst: Self::I);
    /// Emit a machine instruction that is a safepoint.
    fn emit_safepoint(&mut self, mach_inst: Self::I);
    /// Indicate that the given input uses the register returned by
    /// `get_input()`. Codegen may not happen otherwise for the producing
    /// instruction if it has no side effects and no uses.
    fn use_input_reg(&mut self, input: LowerInput);
    /// Is the given register output needed after the given instruction? Allows
    /// instructions with multiple outputs to make fine-grained decisions on
    /// which outputs to actually generate.
    fn is_reg_needed(&self, ir_inst: Inst, reg: Reg) -> bool;
    /// Retrieve constant data given a handle.
    fn get_constant_data(&self, constant_handle: Constant) -> &ConstantData;
}

/// A representation of all of the ways in which an instruction input is
/// available: as a producing instruction (in the same color-partition), as a
/// constant, and/or in an existing register. See [LowerCtx::get_input] for more
/// details.
#[derive(Clone, Copy, Debug)]
pub struct LowerInput {
    /// The value is live in a register. This option is always available. Call
    /// [LowerCtx::use_input_reg()] if the register is used.
    pub reg: Reg,
    /// An instruction produces this value; the instruction's result index that
    /// produces this value is given.
    pub inst: Option<(Inst, usize)>,
    /// The value is a known constant.
    pub constant: Option<u64>,
}

/// A machine backend.
pub trait LowerBackend {
    /// The machine instruction type.
    type MInst: VCodeInst;

    /// Lower a single instruction.
    ///
    /// For a branch, this function should not generate the actual branch
    /// instruction. However, it must force any values it needs for the branch
    /// edge (block-param actuals) into registers, because the actual branch
    /// generation (`lower_branch_group()`) happens *after* any possible merged
    /// out-edge.
    fn lower<C: LowerCtx<I = Self::MInst>>(&self, ctx: &mut C, inst: Inst) -> CodegenResult<()>;

    /// Lower a block-terminating group of branches (which together can be seen
    /// as one N-way branch), given a vcode MachLabel for each target.
    fn lower_branch_group<C: LowerCtx<I = Self::MInst>>(
        &self,
        ctx: &mut C,
        insts: &[Inst],
        targets: &[MachLabel],
        fallthrough: Option<MachLabel>,
    ) -> CodegenResult<()>;

    /// A bit of a hack: give a fixed register that always holds the result of a
    /// `get_pinned_reg` instruction, if known.  This allows elision of moves
    /// into the associated vreg, instead using the real reg directly.
    fn maybe_pinned_reg(&self) -> Option<Reg> {
        None
    }
}

/// A pending instruction to insert and auxiliary information about it: its source location and
/// whether it is a safepoint.
struct InstTuple<I: VCodeInst> {
    loc: SourceLoc,
    is_safepoint: bool,
    inst: I,
}

/// Machine-independent lowering driver / machine-instruction container. Maintains a correspondence
/// from original Inst to MachInsts.
pub struct Lower<'func, I: VCodeInst> {
    /// The function to lower.
    f: &'func Function,

    /// Lowered machine instructions.
    vcode: VCodeBuilder<I>,

    /// Mapping from `Value` (SSA value in IR) to virtual register.
    value_regs: SecondaryMap<Value, Reg>,

    /// Return-value vregs.
    retval_regs: Vec<(Reg, ArgumentExtension)>,

    /// Instruction colors.
    inst_colors: SecondaryMap<Inst, InstColor>,

    /// Instruction constant values, if known.
    inst_constants: FxHashMap<Inst, u64>,

    /// Instruction has a side-effect and must be codegen'd.
    inst_needed: SecondaryMap<Inst, bool>,

    /// Value (vreg) is needed and producer must be codegen'd.
    vreg_needed: Vec<bool>,

    /// Next virtual register number to allocate.
    next_vreg: u32,

    /// Insts in reverse block order, before final copy to vcode.
    block_insts: Vec<InstTuple<I>>,

    /// Ranges in `block_insts` constituting BBs.
    block_ranges: Vec<(usize, usize)>,

    /// Instructions collected for the BB in progress, in reverse order, with
    /// source-locs attached.
    bb_insts: Vec<InstTuple<I>>,

    /// Instructions collected for the CLIF inst in progress, in forward order.
    ir_insts: Vec<InstTuple<I>>,

    /// The register to use for GetPinnedReg, if any, on this architecture.
    pinned_reg: Option<Reg>,
}

/// Notion of "relocation distance". This gives an estimate of how far away a symbol will be from a
/// reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocDistance {
    /// Target of relocation is "nearby". The threshold for this is fuzzy but should be interpreted
    /// as approximately "within the compiled output of one module"; e.g., within AArch64's +/-
    /// 128MB offset. If unsure, use `Far` instead.
    Near,
    /// Target of relocation could be anywhere in the address space.
    Far,
}

fn alloc_vreg(
    value_regs: &mut SecondaryMap<Value, Reg>,
    regclass: RegClass,
    value: Value,
    next_vreg: &mut u32,
) -> VirtualReg {
    if value_regs[value].is_invalid() {
        // default value in map.
        let v = *next_vreg;
        *next_vreg += 1;
        value_regs[value] = Reg::new_virtual(regclass, v);
        debug!("value {} gets vreg {:?}", value, v);
    }
    value_regs[value].as_virtual_reg().unwrap()
}

enum GenerateReturn {
    Yes,
    No,
}

impl<'func, I: VCodeInst> Lower<'func, I> {
    /// Prepare a new lowering context for the given IR function.
    pub fn new(
        f: &'func Function,
        abi: Box<dyn ABIBody<I = I>>,
        block_order: BlockLoweringOrder,
    ) -> CodegenResult<Lower<'func, I>> {
        let mut vcode = VCodeBuilder::new(abi, block_order);

        let mut next_vreg: u32 = 0;

        let mut value_regs = SecondaryMap::with_default(Reg::invalid());

        // Assign a vreg to each block param and each inst result.
        for bb in f.layout.blocks() {
            for &param in f.dfg.block_params(bb) {
                let ty = f.dfg.value_type(param);
                let vreg = alloc_vreg(&mut value_regs, I::rc_for_type(ty)?, param, &mut next_vreg);
                vcode.set_vreg_type(vreg, ty);
                debug!("bb {} param {}: vreg {:?}", bb, param, vreg);
            }
            for inst in f.layout.block_insts(bb) {
                for &result in f.dfg.inst_results(inst) {
                    let ty = f.dfg.value_type(result);
                    let vreg =
                        alloc_vreg(&mut value_regs, I::rc_for_type(ty)?, result, &mut next_vreg);
                    vcode.set_vreg_type(vreg, ty);
                    debug!(
                        "bb {} inst {} ({:?}): result vreg {:?}",
                        bb, inst, f.dfg[inst], vreg
                    );
                }
            }
        }

        // Assign a vreg to each return value.
        let mut retval_regs = vec![];
        for ret in &f.signature.returns {
            let v = next_vreg;
            next_vreg += 1;
            let regclass = I::rc_for_type(ret.value_type)?;
            let vreg = Reg::new_virtual(regclass, v);
            retval_regs.push((vreg, ret.extension));
            vcode.set_vreg_type(vreg.as_virtual_reg().unwrap(), ret.value_type);
        }

        // Compute instruction colors, find constant instructions, and find instructions with
        // side-effects, in one combined pass.
        let mut cur_color = 0;
        let mut inst_colors = SecondaryMap::with_default(InstColor::new(0));
        let mut inst_constants = FxHashMap::default();
        let mut inst_needed = SecondaryMap::with_default(false);
        for bb in f.layout.blocks() {
            cur_color += 1;
            for inst in f.layout.block_insts(bb) {
                let side_effect = has_side_effect_or_load(f, inst);

                // Assign colors. A new color is chosen *after* any side-effecting instruction.
                inst_colors[inst] = InstColor::new(cur_color);
                debug!("bb {} inst {} has color {}", bb, inst, cur_color);
                if side_effect {
                    debug!(" -> side-effecting");
                    inst_needed[inst] = true;
                    cur_color += 1;
                }

                // Determine if this is a constant; if so, add to the table.
                if let Some(c) = is_constant_64bit(f, inst) {
                    debug!(" -> constant: {}", c);
                    inst_constants.insert(inst, c);
                }
            }
        }

        let vreg_needed = std::iter::repeat(false).take(next_vreg as usize).collect();

        Ok(Lower {
            f,
            vcode,
            value_regs,
            retval_regs,
            inst_colors,
            inst_constants,
            inst_needed,
            vreg_needed,
            next_vreg,
            block_insts: vec![],
            block_ranges: vec![],
            bb_insts: vec![],
            ir_insts: vec![],
            pinned_reg: None,
        })
    }

    fn gen_arg_setup(&mut self) {
        if let Some(entry_bb) = self.f.layout.entry_block() {
            debug!(
                "gen_arg_setup: entry BB {} args are:\n{:?}",
                entry_bb,
                self.f.dfg.block_params(entry_bb)
            );
            for (i, param) in self.f.dfg.block_params(entry_bb).iter().enumerate() {
                let reg = Writable::from_reg(self.value_regs[*param]);
                let insn = self.vcode.abi().gen_copy_arg_to_reg(i, reg);
                self.emit(insn);
            }
            if let Some(insn) = self.vcode.abi().gen_retval_area_setup() {
                self.emit(insn);
            }
        }
    }

    fn gen_retval_setup(&mut self, gen_ret_inst: GenerateReturn) {
        let retval_regs = self.retval_regs.clone();
        for (i, (reg, ext)) in retval_regs.into_iter().enumerate() {
            let reg = Writable::from_reg(reg);
            let insns = self.vcode.abi().gen_copy_reg_to_retval(i, reg, ext);
            for insn in insns {
                self.emit(insn);
            }
        }
        let inst = match gen_ret_inst {
            GenerateReturn::Yes => self.vcode.abi().gen_ret(),
            GenerateReturn::No => self.vcode.abi().gen_epilogue_placeholder(),
        };
        self.emit(inst);
    }

    fn lower_edge(&mut self, pred: Block, inst: Inst, succ: Block) -> CodegenResult<()> {
        debug!("lower_edge: pred {} succ {}", pred, succ);

        let num_args = self.f.dfg.block_params(succ).len();
        debug_assert!(num_args == self.f.dfg.inst_variable_args(inst).len());

        // Most blocks have no params, so skip all the hoop-jumping below and make an early exit.
        if num_args == 0 {
            return Ok(());
        }

        // Make up two vectors of info:
        //
        // * one for dsts which are to be assigned constants.  We'll deal with those second, so
        //   as to minimise live ranges.
        //
        // * one for dsts whose sources are non-constants.

        let mut const_bundles = SmallVec::<[(Type, Writable<Reg>, u64); 16]>::new();
        let mut var_bundles = SmallVec::<[(Type, Writable<Reg>, Reg); 16]>::new();

        let mut i = 0;
        for (dst_val, src_val) in self
            .f
            .dfg
            .block_params(succ)
            .iter()
            .zip(self.f.dfg.inst_variable_args(inst).iter())
        {
            let src_val = self.f.dfg.resolve_aliases(*src_val);
            let ty = self.f.dfg.value_type(src_val);

            debug_assert!(ty == self.f.dfg.value_type(*dst_val));
            let dst_reg = self.value_regs[*dst_val];

            let input = self.get_input_for_val(inst, src_val);
            debug!("jump arg {} is {}, reg {:?}", i, src_val, input.reg);
            i += 1;

            if let Some(c) = input.constant {
                const_bundles.push((ty, Writable::from_reg(dst_reg), c));
            } else {
                self.use_input_reg(input);
                let src_reg = input.reg;
                // Skip self-assignments.  Not only are they pointless, they falsely trigger the
                // overlap-check below and hence can cause a lot of unnecessary copying through
                // temporaries.
                if dst_reg != src_reg {
                    var_bundles.push((ty, Writable::from_reg(dst_reg), src_reg));
                }
            }
        }

        // Deal first with the moves whose sources are variables.

        // FIXME: use regalloc.rs' SparseSetU here.  This would avoid all heap allocation
        // for cases of up to circa 16 args.  Currently not possible because regalloc.rs
        // does not export it.
        let mut src_reg_set = FxHashSet::<Reg>::default();
        for (_, _, src_reg) in &var_bundles {
            src_reg_set.insert(*src_reg);
        }
        let mut overlaps = false;
        for (_, dst_reg, _) in &var_bundles {
            if src_reg_set.contains(&dst_reg.to_reg()) {
                overlaps = true;
                break;
            }
        }

        // If, as is mostly the case, the source and destination register sets are non
        // overlapping, then we can copy directly, so as to save the register allocator work.
        if !overlaps {
            for (ty, dst_reg, src_reg) in &var_bundles {
                self.emit(I::gen_move(*dst_reg, *src_reg, *ty));
            }
        } else {
            // There's some overlap, so play safe and copy via temps.
            let mut tmp_regs = SmallVec::<[Writable<Reg>; 16]>::new();
            for (ty, _, _) in &var_bundles {
                tmp_regs.push(self.alloc_tmp(I::rc_for_type(*ty)?, *ty));
            }
            for ((ty, _, src_reg), tmp_reg) in var_bundles.iter().zip(tmp_regs.iter()) {
                self.emit(I::gen_move(*tmp_reg, *src_reg, *ty));
            }
            for ((ty, dst_reg, _), tmp_reg) in var_bundles.iter().zip(tmp_regs.iter()) {
                self.emit(I::gen_move(*dst_reg, (*tmp_reg).to_reg(), *ty));
            }
        }

        // Now, finally, deal with the moves whose sources are constants.
        for (ty, dst_reg, const_u64) in &const_bundles {
            for inst in I::gen_constant(*dst_reg, *const_u64, *ty).into_iter() {
                self.emit(inst);
            }
        }

        Ok(())
    }

    fn lower_clif_block<B: LowerBackend<MInst = I>>(
        &mut self,
        backend: &B,
        block: Block,
    ) -> CodegenResult<()> {
        // Lowering loop:
        // - For each non-branch instruction, in reverse order:
        //   - If side-effecting (load, store, branch/call/return, possible trap), or if
        //     used outside of this block, or if demanded by another inst, then lower.
        //
        // That's it! Lowering of side-effecting ops will force all *needed*
        // (live) non-side-effecting ops to be lowered at the right places, via
        // the `use_input_reg()` callback on the `LowerCtx` (that's us). That's
        // because `use_input_reg()` sets the eager/demand bit for any insts
        // whose result registers are used.
        //
        // We build up the BB in reverse instruction order in `bb_insts`.
        // Because the machine backend calls `ctx.emit()` in forward order, we
        // collect per-IR-inst lowered instructions in `ir_insts`, then reverse
        // these and append to `bb_insts` as we go backward through the block.
        // `bb_insts` are then reversed again and appended to the VCode at the
        // end of the BB (in the toplevel driver `lower()`).
        for inst in self.f.layout.block_insts(block).rev() {
            let data = &self.f.dfg[inst];
            let value_needed = self
                .f
                .dfg
                .inst_results(inst)
                .iter()
                .any(|&result| self.vreg_needed[self.value_regs[result].get_index()]);
            debug!(
                "lower_clif_block: block {} inst {} ({:?}) is_branch {} inst_needed {} value_needed {}",
                block,
                inst,
                data,
                data.opcode().is_branch(),
                self.inst_needed[inst],
                value_needed,
            );
            if self.f.dfg[inst].opcode().is_branch() {
                continue;
            }
            // Normal instruction: codegen if eager bit is set. (Other instructions may also be
            // codegened if not eager when they are used by another instruction.)
            if self.inst_needed[inst] || value_needed {
                debug!("lowering: inst {}: {:?}", inst, self.f.dfg[inst]);
                backend.lower(self, inst)?;
            }
            if data.opcode().is_return() {
                // Return: handle specially, using ABI-appropriate sequence.
                let gen_ret = if data.opcode() == Opcode::Return {
                    GenerateReturn::Yes
                } else {
                    debug_assert!(data.opcode() == Opcode::FallthroughReturn);
                    GenerateReturn::No
                };
                self.gen_retval_setup(gen_ret);
            }

            let loc = self.srcloc(inst);
            self.finish_ir_inst(loc);
        }
        Ok(())
    }

    fn finish_ir_inst(&mut self, loc: SourceLoc) {
        // `bb_insts` is kept in reverse order, so emit the instructions in
        // reverse order.
        for mut tuple in self.ir_insts.drain(..).rev() {
            tuple.loc = loc;
            self.bb_insts.push(tuple);
        }
    }

    fn finish_bb(&mut self) {
        let start = self.block_insts.len();
        for tuple in self.bb_insts.drain(..).rev() {
            self.block_insts.push(tuple);
        }
        let end = self.block_insts.len();
        self.block_ranges.push((start, end));
    }

    fn copy_bbs_to_vcode(&mut self) {
        for &(start, end) in self.block_ranges.iter().rev() {
            for &InstTuple {
                loc,
                is_safepoint,
                ref inst,
            } in &self.block_insts[start..end]
            {
                self.vcode.set_srcloc(loc);
                self.vcode.push(inst.clone(), is_safepoint);
            }
            self.vcode.end_bb();
        }
    }

    fn lower_clif_branches<B: LowerBackend<MInst = I>>(
        &mut self,
        backend: &B,
        block: Block,
        branches: &SmallVec<[Inst; 2]>,
        targets: &SmallVec<[MachLabel; 2]>,
        maybe_fallthrough: Option<MachLabel>,
    ) -> CodegenResult<()> {
        debug!(
            "lower_clif_branches: block {} branches {:?} targets {:?} maybe_fallthrough {:?}",
            block, branches, targets, maybe_fallthrough
        );
        backend.lower_branch_group(self, branches, targets, maybe_fallthrough)?;
        let loc = self.srcloc(branches[0]);
        self.finish_ir_inst(loc);
        Ok(())
    }

    fn collect_branches_and_targets(
        &self,
        bindex: BlockIndex,
        _bb: Block,
        branches: &mut SmallVec<[Inst; 2]>,
        targets: &mut SmallVec<[MachLabel; 2]>,
    ) {
        branches.clear();
        targets.clear();
        let mut last_inst = None;
        for &(inst, succ) in self.vcode.block_order().succ_indices(bindex) {
            // Avoid duplicates: this ensures a br_table is only inserted once.
            if last_inst != Some(inst) {
                branches.push(inst);
            } else {
                debug_assert!(self.f.dfg[inst].opcode() == Opcode::BrTable);
                debug_assert!(branches.len() == 1);
            }
            last_inst = Some(inst);
            targets.push(MachLabel::from_block(succ));
        }
    }

    /// Lower the function.
    pub fn lower<B: LowerBackend<MInst = I>>(
        mut self,
        backend: &B,
    ) -> CodegenResult<(VCode<I>, StackmapRequestInfo)> {
        debug!("about to lower function: {:?}", self.f);

        // Initialize the ABI object, giving it a temp if requested.
        let maybe_tmp = if self.vcode.abi().temp_needed() {
            Some(self.alloc_tmp(RegClass::I64, I64))
        } else {
            None
        };
        self.vcode.abi().init(maybe_tmp);

        // Get the pinned reg here (we only parameterize this function on `B`,
        // not the whole `Lower` impl).
        self.pinned_reg = backend.maybe_pinned_reg();

        self.vcode.set_entry(0);

        // Reused vectors for branch lowering.
        let mut branches: SmallVec<[Inst; 2]> = SmallVec::new();
        let mut targets: SmallVec<[MachLabel; 2]> = SmallVec::new();

        // get a copy of the lowered order; we hold this separately because we
        // need a mut ref to the vcode to mutate it below.
        let lowered_order: SmallVec<[LoweredBlock; 64]> = self
            .vcode
            .block_order()
            .lowered_order()
            .iter()
            .cloned()
            .collect();

        // Main lowering loop over lowered blocks.
        for (bindex, lb) in lowered_order.iter().enumerate().rev() {
            let bindex = bindex as BlockIndex;

            // Lower the block body in reverse order (see comment in
            // `lower_clif_block()` for rationale).

            // End branches.
            if let Some(bb) = lb.orig_block() {
                self.collect_branches_and_targets(bindex, bb, &mut branches, &mut targets);
                if branches.len() > 0 {
                    let maybe_fallthrough = if (bindex + 1) < (lowered_order.len() as BlockIndex) {
                        Some(MachLabel::from_block(bindex + 1))
                    } else {
                        None
                    };
                    self.lower_clif_branches(backend, bb, &branches, &targets, maybe_fallthrough)?;
                    self.finish_ir_inst(self.srcloc(branches[0]));
                }
            } else {
                // If no orig block, this must be a pure edge block; get the successor and
                // emit a jump.
                let (_, succ) = self.vcode.block_order().succ_indices(bindex)[0];
                self.emit(I::gen_jump(MachLabel::from_block(succ)));
                self.finish_ir_inst(SourceLoc::default());
            }

            // Out-edge phi moves.
            if let Some((pred, inst, succ)) = lb.out_edge() {
                self.lower_edge(pred, inst, succ)?;
                self.finish_ir_inst(SourceLoc::default());
            }
            // Original block body.
            if let Some(bb) = lb.orig_block() {
                self.lower_clif_block(backend, bb)?;
            }
            // In-edge phi moves.
            if let Some((pred, inst, succ)) = lb.in_edge() {
                self.lower_edge(pred, inst, succ)?;
                self.finish_ir_inst(SourceLoc::default());
            }

            if bindex == 0 {
                // Set up the function with arg vreg inits.
                self.gen_arg_setup();
                self.finish_ir_inst(SourceLoc::default());
            }

            self.finish_bb();
        }

        self.copy_bbs_to_vcode();

        // Now that we've emitted all instructions into the VCodeBuilder, let's build the VCode.
        let (vcode, stackmap_info) = self.vcode.build();
        debug!("built vcode: {:?}", vcode);

        Ok((vcode, stackmap_info))
    }

    /// Get the actual inputs for a value. This is the implementation for
    /// `get_input()` but starting from the SSA value, which is not exposed to
    /// the backend.
    fn get_input_for_val(&self, at_inst: Inst, val: Value) -> LowerInput {
        debug!("get_input_for_val: val {} at inst {}", val, at_inst);
        let mut reg = self.value_regs[val];
        debug!(" -> reg {:?}", reg);
        assert!(reg.is_valid());
        let mut inst = match self.f.dfg.value_def(val) {
            // OK to merge source instruction if (i) we have a source
            // instruction, and either (ii-a) it has no side effects, or (ii-b)
            // it has the same color as this instruction.
            ValueDef::Result(src_inst, result_idx) => {
                debug!(" -> src inst {}", src_inst);
                debug!(
                    " -> has side effect: {}",
                    has_side_effect_or_load(self.f, src_inst)
                );
                debug!(
                    " -> our color is {:?}, src inst is {:?}",
                    self.inst_color(at_inst),
                    self.inst_color(src_inst)
                );
                if !has_side_effect_or_load(self.f, src_inst)
                    || self.inst_color(at_inst) == self.inst_color(src_inst)
                {
                    Some((src_inst, result_idx))
                } else {
                    None
                }
            }
            _ => None,
        };
        let constant = inst.and_then(|(inst, _)| self.get_constant(inst));

        // Pinned-reg hack: if backend specifies a fixed pinned register, use it
        // directly when we encounter a GetPinnedReg op, rather than lowering
        // the actual op, and do not return the source inst to the caller; the
        // value comes "out of the ether" and we will not force generation of
        // the superfluous move.
        if let Some((i, _)) = inst {
            if self.f.dfg[i].opcode() == Opcode::GetPinnedReg {
                if let Some(pr) = self.pinned_reg {
                    reg = pr;
                }
                inst = None;
            }
        }

        LowerInput {
            reg,
            inst,
            constant,
        }
    }
}

impl<'func, I: VCodeInst> LowerCtx for Lower<'func, I> {
    type I = I;

    fn abi(&mut self) -> &dyn ABIBody<I = I> {
        self.vcode.abi()
    }

    fn retval(&self, idx: usize) -> Writable<Reg> {
        Writable::from_reg(self.retval_regs[idx].0)
    }

    fn data(&self, ir_inst: Inst) -> &InstructionData {
        &self.f.dfg[ir_inst]
    }

    fn ty(&self, ir_inst: Inst) -> Type {
        self.f.dfg.ctrl_typevar(ir_inst)
    }

    fn call_target<'b>(&'b self, ir_inst: Inst) -> Option<(&'b ExternalName, RelocDistance)> {
        match &self.f.dfg[ir_inst] {
            &InstructionData::Call { func_ref, .. }
            | &InstructionData::FuncAddr { func_ref, .. } => {
                let funcdata = &self.f.dfg.ext_funcs[func_ref];
                let dist = funcdata.reloc_distance();
                Some((&funcdata.name, dist))
            }
            _ => None,
        }
    }

    fn call_sig<'b>(&'b self, ir_inst: Inst) -> Option<&'b Signature> {
        match &self.f.dfg[ir_inst] {
            &InstructionData::Call { func_ref, .. } => {
                let funcdata = &self.f.dfg.ext_funcs[func_ref];
                Some(&self.f.dfg.signatures[funcdata.signature])
            }
            &InstructionData::CallIndirect { sig_ref, .. } => Some(&self.f.dfg.signatures[sig_ref]),
            _ => None,
        }
    }

    fn symbol_value<'b>(&'b self, ir_inst: Inst) -> Option<(&'b ExternalName, RelocDistance, i64)> {
        match &self.f.dfg[ir_inst] {
            &InstructionData::UnaryGlobalValue { global_value, .. } => {
                let gvdata = &self.f.global_values[global_value];
                match gvdata {
                    &GlobalValueData::Symbol {
                        ref name,
                        ref offset,
                        ..
                    } => {
                        let offset = offset.bits();
                        let dist = gvdata.maybe_reloc_distance().unwrap();
                        Some((name, dist, offset))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn memflags(&self, ir_inst: Inst) -> Option<MemFlags> {
        match &self.f.dfg[ir_inst] {
            &InstructionData::Load { flags, .. }
            | &InstructionData::LoadComplex { flags, .. }
            | &InstructionData::Store { flags, .. }
            | &InstructionData::StoreComplex { flags, .. } => Some(flags),
            _ => None,
        }
    }

    fn srcloc(&self, ir_inst: Inst) -> SourceLoc {
        self.f.srclocs[ir_inst]
    }

    fn inst_color(&self, ir_inst: Inst) -> InstColor {
        self.inst_colors[ir_inst]
    }

    fn is_safepoint(&self, ir_inst: Inst) -> bool {
        // There is no safepoint metadata at all if we have no reftyped values
        // in this function; lack of metadata implies "nothing to trace", and
        // avoids overhead.
        self.vcode.have_ref_values() && is_safepoint(self.f, ir_inst)
    }

    fn num_inputs(&self, ir_inst: Inst) -> usize {
        self.f.dfg.inst_args(ir_inst).len()
    }

    fn num_outputs(&self, ir_inst: Inst) -> usize {
        self.f.dfg.inst_results(ir_inst).len()
    }

    fn input_ty(&self, ir_inst: Inst, idx: usize) -> Type {
        let val = self.f.dfg.inst_args(ir_inst)[idx];
        let val = self.f.dfg.resolve_aliases(val);
        self.f.dfg.value_type(val)
    }

    fn output_ty(&self, ir_inst: Inst, idx: usize) -> Type {
        self.f.dfg.value_type(self.f.dfg.inst_results(ir_inst)[idx])
    }

    fn get_constant(&self, ir_inst: Inst) -> Option<u64> {
        self.inst_constants.get(&ir_inst).cloned()
    }

    fn get_input(&self, ir_inst: Inst, idx: usize) -> LowerInput {
        let val = self.f.dfg.inst_args(ir_inst)[idx];
        let val = self.f.dfg.resolve_aliases(val);
        self.get_input_for_val(ir_inst, val)
    }

    fn get_output(&self, ir_inst: Inst, idx: usize) -> Writable<Reg> {
        let val = self.f.dfg.inst_results(ir_inst)[idx];
        Writable::from_reg(self.value_regs[val])
    }

    fn alloc_tmp(&mut self, rc: RegClass, ty: Type) -> Writable<Reg> {
        let v = self.next_vreg;
        self.next_vreg += 1;
        let vreg = Reg::new_virtual(rc, v);
        self.vcode.set_vreg_type(vreg.as_virtual_reg().unwrap(), ty);
        Writable::from_reg(vreg)
    }

    fn emit(&mut self, mach_inst: I) {
        self.ir_insts.push(InstTuple {
            loc: SourceLoc::default(),
            is_safepoint: false,
            inst: mach_inst,
        });
    }

    fn emit_safepoint(&mut self, mach_inst: I) {
        self.ir_insts.push(InstTuple {
            loc: SourceLoc::default(),
            is_safepoint: true,
            inst: mach_inst,
        });
    }

    fn use_input_reg(&mut self, input: LowerInput) {
        debug!("use_input_reg: vreg {:?} is needed", input.reg);
        self.vreg_needed[input.reg.get_index()] = true;
    }

    fn is_reg_needed(&self, ir_inst: Inst, reg: Reg) -> bool {
        self.inst_needed[ir_inst] || self.vreg_needed[reg.get_index()]
    }

    fn get_constant_data(&self, constant_handle: Constant) -> &ConstantData {
        self.f.dfg.constants.get(constant_handle)
    }
}

/// Visit all successors of a block with a given visitor closure.
pub(crate) fn visit_block_succs<F: FnMut(Inst, Block)>(f: &Function, block: Block, mut visit: F) {
    for inst in f.layout.block_likely_branches(block) {
        if f.dfg[inst].opcode().is_branch() {
            visit_branch_targets(f, block, inst, &mut visit);
        }
    }
}

fn visit_branch_targets<F: FnMut(Inst, Block)>(
    f: &Function,
    block: Block,
    inst: Inst,
    visit: &mut F,
) {
    if f.dfg[inst].opcode() == Opcode::Fallthrough {
        visit(inst, f.layout.next_block(block).unwrap());
    } else {
        match f.dfg[inst].analyze_branch(&f.dfg.value_lists) {
            BranchInfo::NotABranch => {}
            BranchInfo::SingleDest(dest, _) => {
                visit(inst, dest);
            }
            BranchInfo::Table(table, maybe_dest) => {
                if let Some(dest) = maybe_dest {
                    visit(inst, dest);
                }
                for &dest in f.jump_tables[table].as_slice() {
                    visit(inst, dest);
                }
            }
        }
    }
}

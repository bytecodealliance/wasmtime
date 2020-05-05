//! This module implements lowering (instruction selection) from Cranelift IR
//! to machine instructions with virtual registers. This is *almost* the final
//! machine code, except for register allocation.

use crate::entity::SecondaryMap;
use crate::inst_predicates::has_side_effect;
use crate::ir::instructions::BranchInfo;
use crate::ir::{
    ArgumentExtension, Block, ExternalName, Function, GlobalValueData, Inst, InstructionData,
    MemFlags, Opcode, Signature, SourceLoc, Type, Value, ValueDef,
};
use crate::machinst::{ABIBody, BlockIndex, VCode, VCodeBuilder, VCodeInst};
use crate::{num_uses::NumUses, CodegenResult};

use regalloc::{Reg, RegClass, Set, VirtualReg, Writable};

use alloc::boxed::Box;
use alloc::vec::Vec;
use log::debug;
use smallvec::SmallVec;
use std::collections::VecDeque;

/// A context that machine-specific lowering code can use to emit lowered instructions. This is the
/// view of the machine-independent per-function lowering context that is seen by the machine
/// backend.
pub trait LowerCtx {
    /// The instruction type for which this lowering framework is instantiated.
    type I;

    /// Get the instdata for a given IR instruction.
    fn data(&self, ir_inst: Inst) -> &InstructionData;
    /// Get the controlling type for a polymorphic IR instruction.
    fn ty(&self, ir_inst: Inst) -> Type;
    /// Get the `ABIBody`.
    fn abi(&mut self) -> &dyn ABIBody<I = Self::I>;
    /// Emit a machine instruction.
    fn emit(&mut self, mach_inst: Self::I);
    /// Indicate that an IR instruction has been merged, and so one of its
    /// uses is gone (replaced by uses of the instruction's inputs). This
    /// helps the lowering algorithm to perform on-the-fly DCE, skipping over
    /// unused instructions (such as immediates incorporated directly).
    fn merged(&mut self, from_inst: Inst);
    /// Get the producing instruction, if any, and output number, for the `idx`th input to the
    /// given IR instruction
    fn input_inst(&self, ir_inst: Inst, idx: usize) -> Option<(Inst, usize)>;
    /// Map a Value to its associated writable (probably virtual) Reg.
    fn value_to_writable_reg(&self, val: Value) -> Writable<Reg>;
    /// Map a Value to its associated (probably virtual) Reg.
    fn value_to_reg(&self, val: Value) -> Reg;
    /// Get the `idx`th input to the given IR instruction as a virtual register.
    fn input(&self, ir_inst: Inst, idx: usize) -> Reg;
    /// Get the `idx`th output of the given IR instruction as a virtual register.
    fn output(&self, ir_inst: Inst, idx: usize) -> Writable<Reg>;
    /// Get the number of inputs to the given IR instruction.
    fn num_inputs(&self, ir_inst: Inst) -> usize;
    /// Get the number of outputs to the given IR instruction.
    fn num_outputs(&self, ir_inst: Inst) -> usize;
    /// Get the type for an instruction's input.
    fn input_ty(&self, ir_inst: Inst, idx: usize) -> Type;
    /// Get the type for an instruction's output.
    fn output_ty(&self, ir_inst: Inst, idx: usize) -> Type;
    /// Get a new temp.
    fn tmp(&mut self, rc: RegClass, ty: Type) -> Writable<Reg>;
    /// Get the number of block params.
    fn num_bb_params(&self, bb: Block) -> usize;
    /// Get the register for a block param.
    fn bb_param(&self, bb: Block, idx: usize) -> Reg;
    /// Get the register for a return value.
    fn retval(&self, idx: usize) -> Writable<Reg>;
    /// Get the target for a call instruction, as an `ExternalName`. Returns a tuple
    /// providing this name and the "relocation distance", i.e., whether the backend
    /// can assume the target will be "nearby" (within some small offset) or an
    /// arbitrary address. (This comes from the `colocated` bit in the CLIF.)
    fn call_target<'b>(&'b self, ir_inst: Inst) -> Option<(&'b ExternalName, RelocDistance)>;
    /// Get the signature for a call or call-indirect instruction.
    fn call_sig<'b>(&'b self, ir_inst: Inst) -> Option<&'b Signature>;
    /// Get the symbol name, relocation distance estimate, and offset for a symbol_value instruction.
    fn symbol_value<'b>(&'b self, ir_inst: Inst) -> Option<(&'b ExternalName, RelocDistance, i64)>;
    /// Returns the memory flags of a given memory access.
    fn memflags(&self, ir_inst: Inst) -> Option<MemFlags>;
    /// Get the source location for a given instruction.
    fn srcloc(&self, ir_inst: Inst) -> SourceLoc;
}

/// A machine backend.
pub trait LowerBackend {
    /// The machine instruction type.
    type MInst: VCodeInst;

    /// Lower a single instruction. Instructions are lowered in reverse order.
    /// This function need not handle branches; those are always passed to
    /// `lower_branch_group` below.
    fn lower<C: LowerCtx<I = Self::MInst>>(&self, ctx: &mut C, inst: Inst);

    /// Lower a block-terminating group of branches (which together can be seen as one
    /// N-way branch), given a vcode BlockIndex for each target.
    fn lower_branch_group<C: LowerCtx<I = Self::MInst>>(
        &self,
        ctx: &mut C,
        insts: &[Inst],
        targets: &[BlockIndex],
        fallthrough: Option<BlockIndex>,
    );
}

/// Machine-independent lowering driver / machine-instruction container. Maintains a correspondence
/// from original Inst to MachInsts.
pub struct Lower<'func, I: VCodeInst> {
    /// The function to lower.
    f: &'func Function,

    /// Lowered machine instructions.
    vcode: VCodeBuilder<I>,

    /// Number of active uses (minus `dec_use()` calls by backend) of each instruction.
    num_uses: SecondaryMap<Inst, u32>,

    /// Mapping from `Value` (SSA value in IR) to virtual register.
    value_regs: SecondaryMap<Value, Reg>,

    /// Return-value vregs.
    retval_regs: Vec<(Reg, ArgumentExtension)>,

    /// Next virtual register number to allocate.
    next_vreg: u32,
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
    if value_regs[value].get_index() == 0 {
        // default value in map.
        let v = *next_vreg;
        *next_vreg += 1;
        value_regs[value] = Reg::new_virtual(regclass, v);
    }
    value_regs[value].as_virtual_reg().unwrap()
}

enum GenerateReturn {
    Yes,
    No,
}

impl<'func, I: VCodeInst> Lower<'func, I> {
    /// Prepare a new lowering context for the given IR function.
    pub fn new(f: &'func Function, abi: Box<dyn ABIBody<I = I>>) -> CodegenResult<Lower<'func, I>> {
        let mut vcode = VCodeBuilder::new(abi);

        let num_uses = NumUses::compute(f).take_uses();

        let mut next_vreg: u32 = 1;

        // Default register should never be seen, but the `value_regs` map needs a default and we
        // don't want to push `Option` everywhere. All values will be assigned registers by the
        // loops over block parameters and instruction results below.
        //
        // We do not use vreg 0 so that we can detect any unassigned register that leaks through.
        let default_register = Reg::new_virtual(RegClass::I32, 0);
        let mut value_regs = SecondaryMap::with_default(default_register);

        // Assign a vreg to each value.
        for bb in f.layout.blocks() {
            for param in f.dfg.block_params(bb) {
                let vreg = alloc_vreg(
                    &mut value_regs,
                    I::rc_for_type(f.dfg.value_type(*param))?,
                    *param,
                    &mut next_vreg,
                );
                vcode.set_vreg_type(vreg, f.dfg.value_type(*param));
            }
            for inst in f.layout.block_insts(bb) {
                for result in f.dfg.inst_results(inst) {
                    let vreg = alloc_vreg(
                        &mut value_regs,
                        I::rc_for_type(f.dfg.value_type(*result))?,
                        *result,
                        &mut next_vreg,
                    );
                    vcode.set_vreg_type(vreg, f.dfg.value_type(*result));
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

        Ok(Lower {
            f,
            vcode,
            num_uses,
            value_regs,
            retval_regs,
            next_vreg,
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
                self.vcode.push(insn);
            }
        }
    }

    fn gen_retval_setup(&mut self, gen_ret_inst: GenerateReturn) {
        for (i, (reg, ext)) in self.retval_regs.iter().enumerate() {
            let reg = Writable::from_reg(*reg);
            let insns = self.vcode.abi().gen_copy_reg_to_retval(i, reg, *ext);
            for insn in insns {
                self.vcode.push(insn);
            }
        }
        let inst = match gen_ret_inst {
            GenerateReturn::Yes => self.vcode.abi().gen_ret(),
            GenerateReturn::No => self.vcode.abi().gen_epilogue_placeholder(),
        };
        self.vcode.push(inst);
    }

    fn find_reachable_bbs(&self) -> SmallVec<[Block; 16]> {
        if let Some(entry) = self.f.layout.entry_block() {
            let mut ret = SmallVec::new();
            let mut queue = VecDeque::new();
            let mut visited = SecondaryMap::with_default(false);
            queue.push_back(entry);
            visited[entry] = true;
            while !queue.is_empty() {
                let b = queue.pop_front().unwrap();
                ret.push(b);
                let mut succs: SmallVec<[Block; 16]> = SmallVec::new();
                for inst in self.f.layout.block_insts(b) {
                    if self.f.dfg[inst].opcode().is_branch() {
                        visit_branch_targets(self.f, b, inst, |succ| {
                            succs.push(succ);
                        });
                    }
                }
                for succ in succs.into_iter() {
                    if !visited[succ] {
                        queue.push_back(succ);
                        visited[succ] = true;
                    }
                }
            }

            ret
        } else {
            SmallVec::new()
        }
    }

    /// Lower the function.
    pub fn lower<B: LowerBackend<MInst = I>>(mut self, backend: &B) -> CodegenResult<VCode<I>> {
        // Find all reachable blocks.
        let bbs = self.find_reachable_bbs();

        // This records a Block-to-BlockIndex map so that branch targets can be resolved.
        let mut next_bindex = self.vcode.init_bb_map(&bbs[..]);

        // Allocate a separate BlockIndex for each control-flow instruction so that we can create
        // the edge blocks later. Each entry for a control-flow inst is the edge block; the list
        // has (control flow inst, edge block, orig block) tuples.
        let mut edge_blocks_by_inst: SecondaryMap<Inst, Vec<BlockIndex>> =
            SecondaryMap::with_default(vec![]);
        let mut edge_blocks: Vec<(Inst, BlockIndex, Block)> = vec![];

        debug!("about to lower function: {:?}", self.f);
        debug!("bb map: {:?}", self.vcode.blocks_by_bb());

        // Work backward (reverse block order, reverse through each block), skipping insns with zero
        // uses.
        for bb in bbs.iter().rev() {
            for inst in self.f.layout.block_insts(*bb) {
                let op = self.f.dfg[inst].opcode();
                if op.is_branch() {
                    // Find the original target.
                    let mut add_succ = |next_bb| {
                        let edge_block = next_bindex;
                        next_bindex += 1;
                        edge_blocks_by_inst[inst].push(edge_block);
                        edge_blocks.push((inst, edge_block, next_bb));
                    };
                    visit_branch_targets(self.f, *bb, inst, |succ| {
                        add_succ(succ);
                    });
                }
            }
        }

        for bb in bbs.iter() {
            debug!("lowering bb: {}", bb);

            // If this is a return block, produce the return value setup.  N.B.: this comes
            // *before* the below because it must occur *after* any other instructions, and
            // instructions are lowered in reverse order.
            let last_insn = self.f.layout.block_insts(*bb).last().unwrap();
            let last_insn_opcode = self.f.dfg[last_insn].opcode();
            if last_insn_opcode.is_return() {
                let gen_ret = if last_insn_opcode == Opcode::Return {
                    GenerateReturn::Yes
                } else {
                    debug_assert!(last_insn_opcode == Opcode::FallthroughReturn);
                    self.vcode.set_fallthrough_return_block(*bb);
                    GenerateReturn::No
                };
                self.gen_retval_setup(gen_ret);
                self.vcode.end_ir_inst();
            }

            // Find the branches at the end first, and process those, if any.
            let mut branches: SmallVec<[Inst; 2]> = SmallVec::new();
            let mut targets: SmallVec<[BlockIndex; 2]> = SmallVec::new();

            for inst in self.f.layout.block_insts(*bb).rev() {
                debug!("lower: inst {}", inst);
                if edge_blocks_by_inst[inst].len() > 0 {
                    branches.push(inst);
                    for target in edge_blocks_by_inst[inst].iter().rev().cloned() {
                        targets.push(target);
                    }
                } else {
                    // We've reached the end of the branches -- process all as a group, first.
                    if branches.len() > 0 {
                        let fallthrough = self.f.layout.next_block(*bb);
                        let fallthrough = fallthrough.map(|bb| self.vcode.bb_to_bindex(bb));
                        branches.reverse();
                        targets.reverse();
                        debug!(
                            "lower_branch_group: targets = {:?} branches = {:?}",
                            targets, branches
                        );
                        self.vcode.set_srcloc(self.srcloc(branches[0]));
                        backend.lower_branch_group(
                            &mut self,
                            &branches[..],
                            &targets[..],
                            fallthrough,
                        );
                        self.vcode.end_ir_inst();
                        branches.clear();
                        targets.clear();
                    }

                    // Only codegen an instruction if it either has a side
                    // effect, or has at least one use of one of its results.
                    let num_uses = self.num_uses[inst];
                    let side_effect = has_side_effect(self.f, inst);
                    if side_effect || num_uses > 0 {
                        self.vcode.set_srcloc(self.srcloc(inst));
                        backend.lower(&mut self, inst);
                        self.vcode.end_ir_inst();
                    } else {
                        // If we're skipping the instruction, we need to dec-ref
                        // its arguments.
                        for arg in self.f.dfg.inst_args(inst) {
                            let val = self.f.dfg.resolve_aliases(*arg);
                            match self.f.dfg.value_def(val) {
                                ValueDef::Result(src_inst, _) => {
                                    self.dec_use(src_inst);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // There are possibly some branches left if the block contained only branches.
            if branches.len() > 0 {
                let fallthrough = self.f.layout.next_block(*bb);
                let fallthrough = fallthrough.map(|bb| self.vcode.bb_to_bindex(bb));
                branches.reverse();
                targets.reverse();
                debug!(
                    "lower_branch_group: targets = {:?} branches = {:?}",
                    targets, branches
                );
                self.vcode.set_srcloc(self.srcloc(branches[0]));
                backend.lower_branch_group(&mut self, &branches[..], &targets[..], fallthrough);
                self.vcode.end_ir_inst();
                branches.clear();
                targets.clear();
            }

            // If this is the entry block, produce the argument setup.
            if Some(*bb) == self.f.layout.entry_block() {
                self.gen_arg_setup();
                self.vcode.end_ir_inst();
            }

            let vcode_bb = self.vcode.end_bb();
            debug!("finished building bb: BlockIndex {}", vcode_bb);
            debug!("bb_to_bindex map says: {}", self.vcode.bb_to_bindex(*bb));
            assert!(vcode_bb == self.vcode.bb_to_bindex(*bb));
            if Some(*bb) == self.f.layout.entry_block() {
                self.vcode.set_entry(vcode_bb);
            }
        }

        // Now create the edge blocks, with phi lowering (block parameter copies).
        for (inst, edge_block, orig_block) in edge_blocks.into_iter() {
            debug!(
                "creating edge block: inst {}, edge_block {}, orig_block {}",
                inst, edge_block, orig_block
            );

            // Create a temporary for each block parameter.
            let phi_classes: Vec<Type> = self
                .f
                .dfg
                .block_params(orig_block)
                .iter()
                .map(|p| self.f.dfg.value_type(*p))
                .collect();

            // FIXME sewardj 2020Feb29: use SmallVec
            let mut src_regs = vec![];
            let mut dst_regs = vec![];

            // Create all of the phi uses (reads) from jump args to temps.

            // Round up all the source and destination regs
            for (i, arg) in self.f.dfg.inst_variable_args(inst).iter().enumerate() {
                let arg = self.f.dfg.resolve_aliases(*arg);
                debug!("jump arg {} is {}", i, arg);
                src_regs.push(self.value_regs[arg]);
            }
            for (i, param) in self.f.dfg.block_params(orig_block).iter().enumerate() {
                debug!("bb arg {} is {}", i, param);
                dst_regs.push(Writable::from_reg(self.value_regs[*param]));
            }
            debug_assert!(src_regs.len() == dst_regs.len());
            debug_assert!(phi_classes.len() == dst_regs.len());

            // If, as is mostly the case, the source and destination register
            // sets are non overlapping, then we can copy directly, so as to
            // save the register allocator work.
            if !Set::<Reg>::from_vec(src_regs.clone()).intersects(&Set::<Reg>::from_vec(
                dst_regs.iter().map(|r| r.to_reg()).collect(),
            )) {
                for (dst_reg, (src_reg, ty)) in
                    dst_regs.iter().zip(src_regs.iter().zip(phi_classes))
                {
                    self.vcode.push(I::gen_move(*dst_reg, *src_reg, ty));
                }
            } else {
                // There's some overlap, so play safe and copy via temps.
                let mut tmp_regs = Vec::with_capacity(phi_classes.len());
                for &ty in &phi_classes {
                    tmp_regs.push(self.tmp(I::rc_for_type(ty)?, ty));
                }

                debug!("phi_temps = {:?}", tmp_regs);
                debug_assert!(tmp_regs.len() == src_regs.len());

                for (tmp_reg, (src_reg, &ty)) in
                    tmp_regs.iter().zip(src_regs.iter().zip(phi_classes.iter()))
                {
                    self.vcode.push(I::gen_move(*tmp_reg, *src_reg, ty));
                }
                for (dst_reg, (tmp_reg, &ty)) in
                    dst_regs.iter().zip(tmp_regs.iter().zip(phi_classes.iter()))
                {
                    self.vcode.push(I::gen_move(*dst_reg, tmp_reg.to_reg(), ty));
                }
            }

            // Create the unconditional jump to the original target block.
            self.vcode
                .push(I::gen_jump(self.vcode.bb_to_bindex(orig_block)));

            // End the IR inst and block. (We lower this as if it were one IR instruction so that
            // we can emit machine instructions in forward order.)
            self.vcode.end_ir_inst();
            let blocknum = self.vcode.end_bb();
            assert!(blocknum == edge_block);
        }

        // Now that we've emitted all instructions into the VCodeBuilder, let's build the VCode.
        Ok(self.vcode.build())
    }

    /// Reduce the use-count of an IR instruction. Use this when, e.g., isel incorporates the
    /// computation of an input instruction directly, so that input instruction has one
    /// fewer use.
    fn dec_use(&mut self, ir_inst: Inst) {
        assert!(self.num_uses[ir_inst] > 0);
        self.num_uses[ir_inst] -= 1;
        debug!(
            "incref: ir_inst {} now has {} uses",
            ir_inst, self.num_uses[ir_inst]
        );
    }

    /// Increase the use-count of an IR instruction. Use this when, e.g., isel incorporates
    /// the computation of an input instruction directly, so that input instruction's
    /// inputs are now used directly by the merged instruction.
    fn inc_use(&mut self, ir_inst: Inst) {
        self.num_uses[ir_inst] += 1;
        debug!(
            "decref: ir_inst {} now has {} uses",
            ir_inst, self.num_uses[ir_inst]
        );
    }
}

impl<'func, I: VCodeInst> LowerCtx for Lower<'func, I> {
    type I = I;

    /// Get the instdata for a given IR instruction.
    fn data(&self, ir_inst: Inst) -> &InstructionData {
        &self.f.dfg[ir_inst]
    }

    /// Get the controlling type for a polymorphic IR instruction.
    fn ty(&self, ir_inst: Inst) -> Type {
        self.f.dfg.ctrl_typevar(ir_inst)
    }

    fn abi(&mut self) -> &dyn ABIBody<I = I> {
        self.vcode.abi()
    }

    /// Emit a machine instruction.
    fn emit(&mut self, mach_inst: I) {
        self.vcode.push(mach_inst);
    }

    /// Indicate that a merge has occurred.
    fn merged(&mut self, from_inst: Inst) {
        debug!("merged: inst {}", from_inst);
        // First, inc-ref all inputs of `from_inst`, because they are now used
        // directly by `into_inst`.
        for arg in self.f.dfg.inst_args(from_inst) {
            let arg = self.f.dfg.resolve_aliases(*arg);
            match self.f.dfg.value_def(arg) {
                ValueDef::Result(src_inst, _) => {
                    debug!(" -> inc-reffing src inst {}", src_inst);
                    self.inc_use(src_inst);
                }
                _ => {}
            }
        }
        // Then, dec-ref the merged instruction itself. It still retains references
        // to its arguments (inc-ref'd above). If its refcount has reached zero,
        // it will be skipped during emission and its args will be dec-ref'd at that
        // time.
        self.dec_use(from_inst);
    }

    /// Get the producing instruction, if any, and output number, for the `idx`th input to the
    /// given IR instruction.
    fn input_inst(&self, ir_inst: Inst, idx: usize) -> Option<(Inst, usize)> {
        let val = self.f.dfg.inst_args(ir_inst)[idx];
        let val = self.f.dfg.resolve_aliases(val);
        match self.f.dfg.value_def(val) {
            ValueDef::Result(src_inst, result_idx) => Some((src_inst, result_idx)),
            _ => None,
        }
    }

    /// Map a Value to its associated writable (probably virtual) Reg.
    fn value_to_writable_reg(&self, val: Value) -> Writable<Reg> {
        let val = self.f.dfg.resolve_aliases(val);
        Writable::from_reg(self.value_regs[val])
    }

    /// Map a Value to its associated (probably virtual) Reg.
    fn value_to_reg(&self, val: Value) -> Reg {
        let val = self.f.dfg.resolve_aliases(val);
        self.value_regs[val]
    }

    /// Get the `idx`th input to the given IR instruction as a virtual register.
    fn input(&self, ir_inst: Inst, idx: usize) -> Reg {
        let val = self.f.dfg.inst_args(ir_inst)[idx];
        let val = self.f.dfg.resolve_aliases(val);
        self.value_to_reg(val)
    }

    /// Get the `idx`th output of the given IR instruction as a virtual register.
    fn output(&self, ir_inst: Inst, idx: usize) -> Writable<Reg> {
        let val = self.f.dfg.inst_results(ir_inst)[idx];
        self.value_to_writable_reg(val)
    }

    /// Get a new temp.
    fn tmp(&mut self, rc: RegClass, ty: Type) -> Writable<Reg> {
        let v = self.next_vreg;
        self.next_vreg += 1;
        let vreg = Reg::new_virtual(rc, v);
        self.vcode.set_vreg_type(vreg.as_virtual_reg().unwrap(), ty);
        Writable::from_reg(vreg)
    }

    /// Get the number of inputs for the given IR instruction.
    fn num_inputs(&self, ir_inst: Inst) -> usize {
        self.f.dfg.inst_args(ir_inst).len()
    }

    /// Get the number of outputs for the given IR instruction.
    fn num_outputs(&self, ir_inst: Inst) -> usize {
        self.f.dfg.inst_results(ir_inst).len()
    }

    /// Get the type for an instruction's input.
    fn input_ty(&self, ir_inst: Inst, idx: usize) -> Type {
        let val = self.f.dfg.inst_args(ir_inst)[idx];
        let val = self.f.dfg.resolve_aliases(val);
        self.f.dfg.value_type(val)
    }

    /// Get the type for an instruction's output.
    fn output_ty(&self, ir_inst: Inst, idx: usize) -> Type {
        self.f.dfg.value_type(self.f.dfg.inst_results(ir_inst)[idx])
    }

    /// Get the number of block params.
    fn num_bb_params(&self, bb: Block) -> usize {
        self.f.dfg.block_params(bb).len()
    }

    /// Get the register for a block param.
    fn bb_param(&self, bb: Block, idx: usize) -> Reg {
        let val = self.f.dfg.block_params(bb)[idx];
        self.value_regs[val]
    }

    /// Get the register for a return value.
    fn retval(&self, idx: usize) -> Writable<Reg> {
        Writable::from_reg(self.retval_regs[idx].0)
    }

    /// Get the target for a call instruction, as an `ExternalName`. Returns a tuple
    /// providing this name and the "relocation distance", i.e., whether the backend
    /// can assume the target will be "nearby" (within some small offset) or an
    /// arbitrary address. (This comes from the `colocated` bit in the CLIF.)
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
    /// Get the signature for a call or call-indirect instruction.
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

    /// Get the symbol name, relocation distance estimate, and offset for a symbol_value instruction.
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

    /// Returns the memory flags of a given memory access.
    fn memflags(&self, ir_inst: Inst) -> Option<MemFlags> {
        match &self.f.dfg[ir_inst] {
            &InstructionData::Load { flags, .. }
            | &InstructionData::LoadComplex { flags, .. }
            | &InstructionData::Store { flags, .. }
            | &InstructionData::StoreComplex { flags, .. } => Some(flags),
            _ => None,
        }
    }

    /// Get the source location for a given instruction.
    fn srcloc(&self, ir_inst: Inst) -> SourceLoc {
        self.f.srclocs[ir_inst]
    }
}

fn visit_branch_targets<F: FnMut(Block)>(f: &Function, block: Block, inst: Inst, mut visit: F) {
    if f.dfg[inst].opcode() == Opcode::Fallthrough {
        visit(f.layout.next_block(block).unwrap());
    } else {
        match f.dfg[inst].analyze_branch(&f.dfg.value_lists) {
            BranchInfo::NotABranch => {}
            BranchInfo::SingleDest(dest, _) => {
                visit(dest);
            }
            BranchInfo::Table(table, maybe_dest) => {
                if let Some(dest) = maybe_dest {
                    visit(dest);
                }
                for &dest in f.jump_tables[table].as_slice() {
                    visit(dest);
                }
            }
        }
    }
}

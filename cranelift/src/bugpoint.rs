//! CLI tool to reduce Cranelift IR files crashing during compilation.

use crate::utils::read_to_string;
use anyhow::{Context as _, Result};
use clap::Parser;
use cranelift::prelude::Value;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::types::{F32, F64, I128, I64};
use cranelift_codegen::ir::{
    self, Block, FuncRef, Function, GlobalValueData, Inst, InstBuilder, InstructionData,
    StackSlots, TrapCode,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::Context;
use cranelift_entity::PrimaryMap;
use cranelift_reader::{parse_sets_and_triple, parse_test, ParseOptions};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::collections::HashMap;
use std::path::PathBuf;

/// Reduce size of clif file causing panic during compilation.
#[derive(Parser)]
pub struct Options {
    /// Specify an input file to be used. Use '-' for stdin.
    file: PathBuf,

    /// Configure Cranelift settings
    #[arg(long = "set")]
    settings: Vec<String>,

    /// Specify the target architecture.
    target: String,

    /// Be more verbose
    #[arg(short, long)]
    verbose: bool,
}

pub fn run(options: &Options) -> Result<()> {
    let parsed = parse_sets_and_triple(&options.settings, &options.target)?;
    let fisa = parsed.as_fisa();

    let buffer = read_to_string(&options.file)?;
    let test_file = parse_test(&buffer, ParseOptions::default())
        .with_context(|| format!("failed to parse {}", options.file.display()))?;

    // If we have an isa from the command-line, use that. Otherwise if the
    // file contains a unique isa, use that.
    let isa = if let Some(isa) = fisa.isa {
        isa
    } else if let Some(isa) = test_file.isa_spec.unique_isa() {
        isa
    } else {
        anyhow::bail!("compilation requires a target isa");
    };

    std::env::set_var("RUST_BACKTRACE", "0"); // Disable backtraces to reduce verbosity

    for (func, _) in test_file.functions {
        let (orig_block_count, orig_inst_count) = (block_count(&func), inst_count(&func));

        match reduce(isa, func, options.verbose) {
            Ok((func, crash_msg)) => {
                println!("Crash message: {}", crash_msg);
                println!("\n{}", func);
                println!(
                    "{} blocks {} insts -> {} blocks {} insts",
                    orig_block_count,
                    orig_inst_count,
                    block_count(&func),
                    inst_count(&func)
                );
            }
            Err(err) => println!("Warning: {}", err),
        }
    }

    Ok(())
}

enum ProgressStatus {
    /// The mutation raised or reduced the amount of instructions or blocks.
    ExpandedOrShrinked,

    /// The mutation only changed an instruction. Performing another round of mutations may only
    /// reduce the test case if another mutation shrank the test case.
    Changed,

    /// No need to re-test if the program crashes, because the mutation had no effect, but we want
    /// to keep on iterating.
    Skip,
}

trait Mutator {
    fn name(&self) -> &'static str;
    fn mutation_count(&self, func: &Function) -> usize;
    fn mutate(&mut self, func: Function) -> Option<(Function, String, ProgressStatus)>;

    /// Gets called when the returned mutated function kept on causing the crash. This can be used
    /// to update position of the next item to look at. Does nothing by default.
    fn did_crash(&mut self) {}
}

/// Try to remove instructions.
struct RemoveInst {
    block: Block,
    inst: Inst,
}

impl RemoveInst {
    fn new(func: &Function) -> Self {
        let first_block = func.layout.entry_block().unwrap();
        let first_inst = func.layout.first_inst(first_block).unwrap();
        Self {
            block: first_block,
            inst: first_inst,
        }
    }
}

impl Mutator for RemoveInst {
    fn name(&self) -> &'static str {
        "remove inst"
    }

    fn mutation_count(&self, func: &Function) -> usize {
        inst_count(func)
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, ProgressStatus)> {
        next_inst_ret_prev(&func, &mut self.block, &mut self.inst).map(|(prev_block, prev_inst)| {
            func.layout.remove_inst(prev_inst);
            let msg = if func.layout.block_insts(prev_block).next().is_none() {
                // Make sure empty blocks are removed, as `next_inst_ret_prev` depends on non empty blocks
                func.layout.remove_block(prev_block);
                format!("Remove inst {} and empty block {}", prev_inst, prev_block)
            } else {
                format!("Remove inst {}", prev_inst)
            };
            (func, msg, ProgressStatus::ExpandedOrShrinked)
        })
    }
}

/// Try to replace instructions with `iconst` or `fconst`.
struct ReplaceInstWithConst {
    block: Block,
    inst: Inst,
}

impl ReplaceInstWithConst {
    fn new(func: &Function) -> Self {
        let first_block = func.layout.entry_block().unwrap();
        let first_inst = func.layout.first_inst(first_block).unwrap();
        Self {
            block: first_block,
            inst: first_inst,
        }
    }
}

impl Mutator for ReplaceInstWithConst {
    fn name(&self) -> &'static str {
        "replace inst with const"
    }

    fn mutation_count(&self, func: &Function) -> usize {
        inst_count(func)
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, ProgressStatus)> {
        next_inst_ret_prev(&func, &mut self.block, &mut self.inst).map(
            |(_prev_block, prev_inst)| {
                let num_results = func.dfg.inst_results(prev_inst).len();

                let opcode = func.dfg.insts[prev_inst].opcode();
                if num_results == 0
                    || opcode == ir::Opcode::Iconst
                    || opcode == ir::Opcode::F32const
                    || opcode == ir::Opcode::F64const
                {
                    return (func, format!(""), ProgressStatus::Skip);
                }

                // We replace a i128 const with a uextend+iconst, so we need to match that here
                // to avoid processing those multiple times
                if opcode == ir::Opcode::Uextend {
                    let ret_ty = func.dfg.value_type(func.dfg.first_result(prev_inst));
                    let is_uextend_i128 = ret_ty == I128;

                    let arg = func.dfg.inst_args(prev_inst)[0];
                    let arg_def = func.dfg.value_def(arg);
                    let arg_is_iconst = arg_def
                        .inst()
                        .map(|inst| func.dfg.insts[inst].opcode() == ir::Opcode::Iconst)
                        .unwrap_or(false);

                    if is_uextend_i128 && arg_is_iconst {
                        return (func, format!(""), ProgressStatus::Skip);
                    }
                }

                // At least 2 results. Replace each instruction with as many const instructions as
                // there are results.
                let mut pos = FuncCursor::new(&mut func).at_inst(prev_inst);

                // Copy result SSA names into our own vector; otherwise we couldn't mutably borrow pos
                // in the loop below.
                let results = pos.func.dfg.inst_results(prev_inst).to_vec();

                // Detach results from the previous instruction, since we're going to reuse them.
                pos.func.dfg.clear_results(prev_inst);

                let mut inst_names = Vec::new();
                for r in &results {
                    let new_inst_name = replace_with_const(&mut pos, *r);
                    inst_names.push(new_inst_name);
                }

                // Remove the instruction.
                assert_eq!(pos.remove_inst(), prev_inst);

                let progress = if results.len() == 1 {
                    ProgressStatus::Changed
                } else {
                    ProgressStatus::ExpandedOrShrinked
                };

                (
                    func,
                    format!("Replace inst {} with {}", prev_inst, inst_names.join(" / ")),
                    progress,
                )
            },
        )
    }
}

/// Try to replace instructions with `trap`.
struct ReplaceInstWithTrap {
    block: Block,
    inst: Inst,
}

impl ReplaceInstWithTrap {
    fn new(func: &Function) -> Self {
        let first_block = func.layout.entry_block().unwrap();
        let first_inst = func.layout.first_inst(first_block).unwrap();
        Self {
            block: first_block,
            inst: first_inst,
        }
    }
}

impl Mutator for ReplaceInstWithTrap {
    fn name(&self) -> &'static str {
        "replace inst with trap"
    }

    fn mutation_count(&self, func: &Function) -> usize {
        inst_count(func)
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, ProgressStatus)> {
        next_inst_ret_prev(&func, &mut self.block, &mut self.inst).map(
            |(_prev_block, prev_inst)| {
                let status = if func.dfg.insts[prev_inst].opcode() == ir::Opcode::Trap {
                    ProgressStatus::Skip
                } else {
                    func.dfg.replace(prev_inst).trap(TrapCode::User(0));
                    ProgressStatus::Changed
                };
                (
                    func,
                    format!("Replace inst {} with trap", prev_inst),
                    status,
                )
            },
        )
    }
}

/// Try to move instructions to entry block.
struct MoveInstToEntryBlock {
    block: Block,
    inst: Inst,
}

impl MoveInstToEntryBlock {
    fn new(func: &Function) -> Self {
        let first_block = func.layout.entry_block().unwrap();
        let first_inst = func.layout.first_inst(first_block).unwrap();
        Self {
            block: first_block,
            inst: first_inst,
        }
    }
}

impl Mutator for MoveInstToEntryBlock {
    fn name(&self) -> &'static str {
        "move inst to entry block"
    }

    fn mutation_count(&self, func: &Function) -> usize {
        inst_count(func)
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, ProgressStatus)> {
        next_inst_ret_prev(&func, &mut self.block, &mut self.inst).map(|(prev_block, prev_inst)| {
            // Don't move instructions that are already in entry block
            // and instructions that end blocks.
            let first_block = func.layout.entry_block().unwrap();
            if first_block == prev_block || self.block != prev_block {
                return (
                    func,
                    format!("did nothing for {}", prev_inst),
                    ProgressStatus::Skip,
                );
            }

            let last_inst_of_first_block = func.layout.last_inst(first_block).unwrap();
            func.layout.remove_inst(prev_inst);
            func.layout.insert_inst(prev_inst, last_inst_of_first_block);

            (
                func,
                format!("Move inst {} to entry block", prev_inst),
                ProgressStatus::ExpandedOrShrinked,
            )
        })
    }
}

/// Try to remove a block.
struct RemoveBlock {
    block: Block,
}

impl RemoveBlock {
    fn new(func: &Function) -> Self {
        Self {
            block: func.layout.entry_block().unwrap(),
        }
    }
}

impl Mutator for RemoveBlock {
    fn name(&self) -> &'static str {
        "remove block"
    }

    fn mutation_count(&self, func: &Function) -> usize {
        block_count(func)
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, ProgressStatus)> {
        func.layout.next_block(self.block).map(|next_block| {
            self.block = next_block;
            while let Some(inst) = func.layout.last_inst(self.block) {
                func.layout.remove_inst(inst);
            }
            func.layout.remove_block(self.block);
            (
                func,
                format!("Remove block {}", next_block),
                ProgressStatus::ExpandedOrShrinked,
            )
        })
    }
}

/// Try to replace the block params with constants.
struct ReplaceBlockParamWithConst {
    block: Block,
    params_remaining: usize,
}

impl ReplaceBlockParamWithConst {
    fn new(func: &Function) -> Self {
        let first_block = func.layout.entry_block().unwrap();
        Self {
            block: first_block,
            params_remaining: func.dfg.num_block_params(first_block),
        }
    }
}

impl Mutator for ReplaceBlockParamWithConst {
    fn name(&self) -> &'static str {
        "replace block parameter with const"
    }

    fn mutation_count(&self, func: &Function) -> usize {
        func.layout
            .blocks()
            .map(|block| func.dfg.num_block_params(block))
            .sum()
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, ProgressStatus)> {
        while self.params_remaining == 0 {
            self.block = func.layout.next_block(self.block)?;
            self.params_remaining = func.dfg.num_block_params(self.block);
        }

        self.params_remaining -= 1;
        let param_index = self.params_remaining;

        let param = func.dfg.block_params(self.block)[param_index];
        func.dfg.remove_block_param(param);

        let first_inst = func.layout.first_inst(self.block).unwrap();
        let mut pos = FuncCursor::new(&mut func).at_inst(first_inst);
        let new_inst_name = replace_with_const(&mut pos, param);

        let mut cfg = ControlFlowGraph::new();
        cfg.compute(&func);

        // Remove parameters in branching instructions that point to this block
        for pred in cfg.pred_iter(self.block) {
            let dfg = &mut func.dfg;
            for branch in dfg.insts[pred.inst].branch_destination_mut(&mut dfg.jump_tables) {
                if branch.block(&dfg.value_lists) == self.block {
                    branch.remove(param_index, &mut dfg.value_lists);
                }
            }
        }

        if Some(self.block) == func.layout.entry_block() {
            // Entry block params must match function params
            func.signature.params.remove(param_index);
        }

        Some((
            func,
            format!(
                "Replaced param {} of {} by {}",
                param, self.block, new_inst_name
            ),
            ProgressStatus::ExpandedOrShrinked,
        ))
    }
}

/// Try to remove unused entities.
struct RemoveUnusedEntities {
    kind: u32,
}

impl RemoveUnusedEntities {
    fn new() -> Self {
        Self { kind: 0 }
    }
}

impl Mutator for RemoveUnusedEntities {
    fn name(&self) -> &'static str {
        "remove unused entities"
    }

    fn mutation_count(&self, _func: &Function) -> usize {
        4
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, ProgressStatus)> {
        let name = match self.kind {
            0 => {
                let mut ext_func_usage_map = HashMap::new();
                for block in func.layout.blocks() {
                    for inst in func.layout.block_insts(block) {
                        match func.dfg.insts[inst] {
                            // Add new cases when there are new instruction formats taking a `FuncRef`.
                            InstructionData::Call { func_ref, .. }
                            | InstructionData::FuncAddr { func_ref, .. } => {
                                ext_func_usage_map
                                    .entry(func_ref)
                                    .or_insert_with(Vec::new)
                                    .push(inst);
                            }
                            _ => {}
                        }
                    }
                }

                let mut ext_funcs = PrimaryMap::new();

                for (func_ref, ext_func_data) in func.dfg.ext_funcs.clone().into_iter() {
                    if let Some(func_ref_usage) = ext_func_usage_map.get(&func_ref) {
                        let new_func_ref = ext_funcs.push(ext_func_data.clone());
                        for &inst in func_ref_usage {
                            match func.dfg.insts[inst] {
                                // Keep in sync with the above match.
                                InstructionData::Call {
                                    ref mut func_ref, ..
                                }
                                | InstructionData::FuncAddr {
                                    ref mut func_ref, ..
                                } => {
                                    *func_ref = new_func_ref;
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }

                func.dfg.ext_funcs = ext_funcs;

                "Remove unused ext funcs"
            }
            1 => {
                #[derive(Copy, Clone)]
                enum SigRefUser {
                    Instruction(Inst),
                    ExtFunc(FuncRef),
                }

                let mut signatures_usage_map = HashMap::new();
                for block in func.layout.blocks() {
                    for inst in func.layout.block_insts(block) {
                        // Add new cases when there are new instruction formats taking a `SigRef`.
                        if let InstructionData::CallIndirect { sig_ref, .. } = func.dfg.insts[inst]
                        {
                            signatures_usage_map
                                .entry(sig_ref)
                                .or_insert_with(Vec::new)
                                .push(SigRefUser::Instruction(inst));
                        }
                    }
                }
                for (func_ref, ext_func_data) in func.dfg.ext_funcs.iter() {
                    signatures_usage_map
                        .entry(ext_func_data.signature)
                        .or_insert_with(Vec::new)
                        .push(SigRefUser::ExtFunc(func_ref));
                }

                let mut signatures = PrimaryMap::new();

                for (sig_ref, sig_data) in func.dfg.signatures.clone().into_iter() {
                    if let Some(sig_ref_usage) = signatures_usage_map.get(&sig_ref) {
                        let new_sig_ref = signatures.push(sig_data.clone());
                        for &sig_ref_user in sig_ref_usage {
                            match sig_ref_user {
                                SigRefUser::Instruction(inst) => match func.dfg.insts[inst] {
                                    // Keep in sync with the above match.
                                    InstructionData::CallIndirect {
                                        ref mut sig_ref, ..
                                    } => {
                                        *sig_ref = new_sig_ref;
                                    }
                                    _ => unreachable!(),
                                },
                                SigRefUser::ExtFunc(func_ref) => {
                                    func.dfg.ext_funcs[func_ref].signature = new_sig_ref;
                                }
                            }
                        }
                    }
                }

                func.dfg.signatures = signatures;

                "Remove unused signatures"
            }
            2 => {
                let mut stack_slot_usage_map = HashMap::new();
                for block in func.layout.blocks() {
                    for inst in func.layout.block_insts(block) {
                        match func.dfg.insts[inst] {
                            // Add new cases when there are new instruction formats taking a `StackSlot`.
                            InstructionData::StackLoad { stack_slot, .. }
                            | InstructionData::StackStore { stack_slot, .. } => {
                                stack_slot_usage_map
                                    .entry(stack_slot)
                                    .or_insert_with(Vec::new)
                                    .push(inst);
                            }

                            _ => {}
                        }
                    }
                }

                let mut stack_slots = StackSlots::new();

                for (stack_slot, stack_slot_data) in func.sized_stack_slots.clone().iter() {
                    if let Some(stack_slot_usage) = stack_slot_usage_map.get(&stack_slot) {
                        let new_stack_slot = stack_slots.push(stack_slot_data.clone());
                        for &inst in stack_slot_usage {
                            match &mut func.dfg.insts[inst] {
                                // Keep in sync with the above match.
                                InstructionData::StackLoad { stack_slot, .. }
                                | InstructionData::StackStore { stack_slot, .. } => {
                                    *stack_slot = new_stack_slot;
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }

                func.sized_stack_slots = stack_slots;

                "Remove unused stack slots"
            }
            3 => {
                let mut global_value_usage_map = HashMap::new();
                for block in func.layout.blocks() {
                    for inst in func.layout.block_insts(block) {
                        // Add new cases when there are new instruction formats taking a `GlobalValue`.
                        if let InstructionData::UnaryGlobalValue { global_value, .. } =
                            func.dfg.insts[inst]
                        {
                            global_value_usage_map
                                .entry(global_value)
                                .or_insert_with(Vec::new)
                                .push(inst);
                        }
                    }
                }

                for (_global_value, global_value_data) in func.global_values.iter() {
                    match *global_value_data {
                        GlobalValueData::VMContext | GlobalValueData::Symbol { .. } => {}
                        // These can create cyclic references, which cause complications. Just skip
                        // the global value removal for now.
                        // FIXME Handle them in a better way.
                        GlobalValueData::Load { .. }
                        | GlobalValueData::IAddImm { .. }
                        | GlobalValueData::DynScaleTargetConst { .. } => return None,
                    }
                }

                let mut global_values = PrimaryMap::new();

                for (global_value, global_value_data) in func.global_values.clone().into_iter() {
                    if let Some(global_value_usage) = global_value_usage_map.get(&global_value) {
                        let new_global_value = global_values.push(global_value_data.clone());
                        for &inst in global_value_usage {
                            match &mut func.dfg.insts[inst] {
                                // Keep in sync with the above match.
                                InstructionData::UnaryGlobalValue { global_value, .. } => {
                                    *global_value = new_global_value;
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }

                func.global_values = global_values;

                "Remove unused global values"
            }
            _ => return None,
        };
        self.kind += 1;
        Some((func, name.to_owned(), ProgressStatus::Changed))
    }
}

struct MergeBlocks {
    block: Block,
    prev_block: Option<Block>,
}

impl MergeBlocks {
    fn new(func: &Function) -> Self {
        Self {
            block: func.layout.entry_block().unwrap(),
            prev_block: None,
        }
    }
}

impl Mutator for MergeBlocks {
    fn name(&self) -> &'static str {
        "merge blocks"
    }

    fn mutation_count(&self, func: &Function) -> usize {
        // N blocks may result in at most N-1 merges.
        block_count(func) - 1
    }

    fn mutate(&mut self, mut func: Function) -> Option<(Function, String, ProgressStatus)> {
        let block = match func.layout.next_block(self.block) {
            Some(block) => block,
            None => return None,
        };

        self.block = block;

        let mut cfg = ControlFlowGraph::new();
        cfg.compute(&func);

        if cfg.pred_iter(block).count() != 1 {
            return Some((
                func,
                format!("did nothing for {}", block),
                ProgressStatus::Skip,
            ));
        }

        let pred = cfg.pred_iter(block).next().unwrap();

        // If the branch instruction that lead us to this block wasn't an unconditional jump, then
        // we have a conditional jump sequence that we should not break.
        let branch_dests = func.dfg.insts[pred.inst].branch_destination(&func.dfg.jump_tables);
        if branch_dests.len() != 1 {
            return Some((
                func,
                format!("did nothing for {}", block),
                ProgressStatus::Skip,
            ));
        }

        let branch_args = branch_dests[0].args_slice(&func.dfg.value_lists).to_vec();

        // TODO: should we free the entity list associated with the block params?
        let block_params = func
            .dfg
            .detach_block_params(block)
            .as_slice(&func.dfg.value_lists)
            .to_vec();

        assert_eq!(block_params.len(), branch_args.len());

        // If there were any block parameters in block, then the last instruction in pred will
        // fill these parameters. Make the block params aliases of the terminator arguments.
        for (block_param, arg) in block_params.into_iter().zip(branch_args) {
            if block_param != arg {
                func.dfg.change_to_alias(block_param, arg);
            }
        }

        // Remove the terminator branch to the current block.
        func.layout.remove_inst(pred.inst);

        // Move all the instructions to the predecessor.
        while let Some(inst) = func.layout.first_inst(block) {
            func.layout.remove_inst(inst);
            func.layout.append_inst(inst, pred.block);
        }

        // Remove the predecessor block.
        func.layout.remove_block(block);

        // Record the previous block: if we caused a crash (as signaled by a call to did_crash), then
        // we'll start back to this block.
        self.prev_block = Some(pred.block);

        Some((
            func,
            format!("merged {} and {}", pred.block, block),
            ProgressStatus::ExpandedOrShrinked,
        ))
    }

    fn did_crash(&mut self) {
        self.block = self.prev_block.unwrap();
    }
}

fn replace_with_const(pos: &mut FuncCursor, param: Value) -> &'static str {
    let ty = pos.func.dfg.value_type(param);
    if ty == F32 {
        pos.ins().with_result(param).f32const(0.0);
        "f32const"
    } else if ty == F64 {
        pos.ins().with_result(param).f64const(0.0);
        "f64const"
    } else if ty.is_ref() {
        pos.ins().with_result(param).null(ty);
        "null"
    } else if ty.is_vector() {
        let zero_data = vec![0; ty.bytes() as usize].into();
        let zero_handle = pos.func.dfg.constants.insert(zero_data);
        pos.ins().with_result(param).vconst(ty, zero_handle);
        "vconst"
    } else if ty == I128 {
        let res = pos.ins().iconst(I64, 0);
        pos.ins().with_result(param).uextend(I128, res);
        "iconst+uextend"
    } else {
        // Default to an integer type and possibly create verifier error
        pos.ins().with_result(param).iconst(ty, 0);
        "iconst"
    }
}

fn next_inst_ret_prev(
    func: &Function,
    block: &mut Block,
    inst: &mut Inst,
) -> Option<(Block, Inst)> {
    let prev = (*block, *inst);
    if let Some(next_inst) = func.layout.next_inst(*inst) {
        *inst = next_inst;
        return Some(prev);
    }
    if let Some(next_block) = func.layout.next_block(*block) {
        *block = next_block;
        *inst = func.layout.first_inst(*block).expect("no inst");
        return Some(prev);
    }
    None
}

fn block_count(func: &Function) -> usize {
    func.layout.blocks().count()
}

fn inst_count(func: &Function) -> usize {
    func.layout
        .blocks()
        .map(|block| func.layout.block_insts(block).count())
        .sum()
}

/// Resolve aliases only if function still crashes after this.
fn try_resolve_aliases(context: &mut CrashCheckContext, func: &mut Function) {
    let mut func_with_resolved_aliases = func.clone();
    func_with_resolved_aliases.dfg.resolve_all_aliases();
    if let CheckResult::Crash(_) = context.check_for_crash(&func_with_resolved_aliases) {
        *func = func_with_resolved_aliases;
    }
}

fn reduce(isa: &dyn TargetIsa, mut func: Function, verbose: bool) -> Result<(Function, String)> {
    let mut context = CrashCheckContext::new(isa);

    if let CheckResult::Succeed = context.check_for_crash(&func) {
        anyhow::bail!("Given function compiled successfully or gave a verifier error.");
    }

    try_resolve_aliases(&mut context, &mut func);
    // Remove SourceLocs to make reduced clif IR easier to read
    func.srclocs.clear();

    let progress_bar = ProgressBar::with_draw_target(0, ProgressDrawTarget::stdout());
    progress_bar.set_style(
        ProgressStyle::default_bar().template("{bar:60} {prefix:40} {pos:>4}/{len:>4} {msg}"),
    );

    for pass_idx in 0..100 {
        let mut should_keep_reducing = false;
        let mut phase = 0;

        loop {
            let mut mutator: Box<dyn Mutator> = match phase {
                0 => Box::new(RemoveInst::new(&func)),
                1 => Box::new(ReplaceInstWithConst::new(&func)),
                2 => Box::new(ReplaceInstWithTrap::new(&func)),
                3 => Box::new(MoveInstToEntryBlock::new(&func)),
                4 => Box::new(RemoveBlock::new(&func)),
                5 => Box::new(ReplaceBlockParamWithConst::new(&func)),
                6 => Box::new(RemoveUnusedEntities::new()),
                7 => Box::new(MergeBlocks::new(&func)),
                _ => break,
            };

            progress_bar.set_prefix(&format!("pass {} phase {}", pass_idx, mutator.name()));
            progress_bar.set_length(mutator.mutation_count(&func) as u64);

            // Reset progress bar.
            progress_bar.set_position(0);
            progress_bar.set_draw_delta(0);

            for _ in 0..10000 {
                progress_bar.inc(1);

                let (mutated_func, msg, mutation_kind) = match mutator.mutate(func.clone()) {
                    Some(res) => res,
                    None => {
                        break;
                    }
                };

                if let ProgressStatus::Skip = mutation_kind {
                    // The mutator didn't change anything, but we want to try more mutator
                    // iterations.
                    continue;
                }

                progress_bar.set_message(&msg);

                match context.check_for_crash(&mutated_func) {
                    CheckResult::Succeed => {
                        // Mutating didn't hit the problem anymore, discard changes.
                        continue;
                    }
                    CheckResult::Crash(_) => {
                        // Panic remained while mutating, make changes definitive.
                        func = mutated_func;

                        // Notify the mutator that the mutation was successful.
                        mutator.did_crash();

                        let verb = match mutation_kind {
                            ProgressStatus::ExpandedOrShrinked => {
                                should_keep_reducing = true;
                                "shrink"
                            }
                            ProgressStatus::Changed => "changed",
                            ProgressStatus::Skip => unreachable!(),
                        };
                        if verbose {
                            progress_bar.println(format!("{}: {}", msg, verb));
                        }
                    }
                }
            }

            phase += 1;
        }

        progress_bar.println(format!(
            "After pass {}, remaining insts/blocks: {}/{} ({})",
            pass_idx,
            inst_count(&func),
            block_count(&func),
            if should_keep_reducing {
                "will keep reducing"
            } else {
                "stop reducing"
            }
        ));

        if !should_keep_reducing {
            // No new shrinking opportunities have been found this pass. This means none will ever
            // be found. Skip the rest of the passes over the function.
            break;
        }
    }

    try_resolve_aliases(&mut context, &mut func);
    progress_bar.finish();

    let crash_msg = match context.check_for_crash(&func) {
        CheckResult::Succeed => unreachable!("Used to crash, but doesn't anymore???"),
        CheckResult::Crash(crash_msg) => crash_msg,
    };

    Ok((func, crash_msg))
}

struct CrashCheckContext<'a> {
    /// Cached `Context`, to prevent repeated allocation.
    context: Context,

    /// Cached code memory, to prevent repeated allocation.
    code_memory: Vec<u8>,

    /// The target isa to compile for.
    isa: &'a dyn TargetIsa,
}

fn get_panic_string(panic: Box<dyn std::any::Any>) -> String {
    let panic = match panic.downcast::<&'static str>() {
        Ok(panic_msg) => {
            return panic_msg.to_string();
        }
        Err(panic) => panic,
    };
    match panic.downcast::<String>() {
        Ok(panic_msg) => *panic_msg,
        Err(_) => "Box<Any>".to_string(),
    }
}

enum CheckResult {
    /// The function compiled fine, or the verifier noticed an error.
    Succeed,

    /// The compilation of the function panicked.
    Crash(String),
}

impl<'a> CrashCheckContext<'a> {
    fn new(isa: &'a dyn TargetIsa) -> Self {
        CrashCheckContext {
            context: Context::new(),
            code_memory: Vec::new(),
            isa,
        }
    }

    #[cfg_attr(test, allow(unreachable_code))]
    fn check_for_crash(&mut self, func: &Function) -> CheckResult {
        self.context.clear();
        self.code_memory.clear();

        self.context.func = func.clone();

        use std::io::Write;
        std::io::stdout().flush().unwrap(); // Flush stdout to sync with panic messages on stderr

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cranelift_codegen::verifier::verify_function(&func, self.isa).err()
        })) {
            Ok(Some(_)) => return CheckResult::Succeed,
            Ok(None) => {}
            // The verifier panicked. Compiling it will probably give the same panic.
            // We treat it as succeeding to make it possible to reduce for the actual error.
            // FIXME prevent verifier panic on removing block0.
            Err(_) => return CheckResult::Succeed,
        }

        #[cfg(test)]
        {
            // For testing purposes we emulate a panic caused by the existence of
            // a `call` instruction.
            let contains_call = func.layout.blocks().any(|block| {
                func.layout
                    .block_insts(block)
                    .any(|inst| match func.dfg.insts[inst] {
                        InstructionData::Call { .. } => true,
                        _ => false,
                    })
            });
            if contains_call {
                return CheckResult::Crash("test crash".to_string());
            } else {
                return CheckResult::Succeed;
            }
        }

        let old_panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silence panics

        let res = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = self.context.compile_and_emit(
                self.isa,
                &mut self.code_memory,
                &mut Default::default(),
            );
        })) {
            Ok(()) => CheckResult::Succeed,
            Err(err) => CheckResult::Crash(get_panic_string(err)),
        };

        std::panic::set_hook(old_panic_hook);

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_test(test_str: &str, expected_str: &str) {
        let test_file = parse_test(test_str, ParseOptions::default()).unwrap();

        // If we have an isa from the command-line, use that. Otherwise if the
        // file contains a unique isa, use that.
        let isa = test_file.isa_spec.unique_isa().expect("Unknown isa");

        for (func, _) in test_file.functions {
            let (reduced_func, crash_msg) =
                reduce(isa, func, false).expect("Couldn't reduce test case");
            assert_eq!(crash_msg, "test crash");

            let (func_reduced_twice, crash_msg) =
                reduce(isa, reduced_func.clone(), false).expect("Couldn't re-reduce test case");
            assert_eq!(crash_msg, "test crash");

            assert_eq!(
                block_count(&func_reduced_twice),
                block_count(&reduced_func),
                "reduction wasn't maximal for blocks"
            );
            assert_eq!(
                inst_count(&func_reduced_twice),
                inst_count(&reduced_func),
                "reduction wasn't maximal for insts"
            );

            let actual_ir = format!("{}", reduced_func);
            let expected_ir = expected_str.replace("\r\n", "\n");
            assert!(
                expected_ir == actual_ir,
                "Expected:\n{}\nGot:\n{}",
                expected_ir,
                actual_ir,
            );
        }
    }

    #[test]
    fn test_reduce() {
        const TEST: &str = include_str!("../tests/bugpoint_test.clif");
        const EXPECTED: &str = include_str!("../tests/bugpoint_test_expected.clif");
        run_test(TEST, EXPECTED);
    }

    #[test]
    fn test_consts() {
        const TEST: &str = include_str!("../tests/bugpoint_consts.clif");
        const EXPECTED: &str = include_str!("../tests/bugpoint_consts_expected.clif");
        run_test(TEST, EXPECTED);
    }
}

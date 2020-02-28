//! Reload pass
//!
//! The reload pass runs between the spilling and coloring passes. Its primary responsibility is to
//! insert `spill` and `fill` instructions such that instruction operands expecting a register will
//! get a value with register affinity, and operands expecting a stack slot will get a value with
//! stack affinity.
//!
//! The secondary responsibility of the reload pass is to reuse values in registers as much as
//! possible to minimize the number of `fill` instructions needed. This must not cause the register
//! pressure limits to be exceeded.

use crate::cursor::{Cursor, EncCursor};
use crate::dominator_tree::DominatorTree;
use crate::entity::{SparseMap, SparseMapValue};
use crate::ir::{AbiParam, ArgumentLoc, InstBuilder};
use crate::ir::{Block, Function, Inst, InstructionData, Opcode, Value, ValueLoc};
use crate::isa::RegClass;
use crate::isa::{ConstraintKind, EncInfo, Encoding, RecipeConstraints, TargetIsa};
use crate::regalloc::affinity::Affinity;
use crate::regalloc::live_value_tracker::{LiveValue, LiveValueTracker};
use crate::regalloc::liveness::Liveness;
use crate::timing;
use crate::topo_order::TopoOrder;
use alloc::vec::Vec;
use log::debug;

/// Reusable data structures for the reload pass.
pub struct Reload {
    candidates: Vec<ReloadCandidate>,
    reloads: SparseMap<Value, ReloadedValue>,
}

/// Context data structure that gets instantiated once per pass.
struct Context<'a> {
    cur: EncCursor<'a>,

    // Cached ISA information.
    // We save it here to avoid frequent virtual function calls on the `TargetIsa` trait object.
    encinfo: EncInfo,

    // References to contextual data structures we need.
    domtree: &'a DominatorTree,
    liveness: &'a mut Liveness,
    topo: &'a mut TopoOrder,

    candidates: &'a mut Vec<ReloadCandidate>,
    reloads: &'a mut SparseMap<Value, ReloadedValue>,
}

impl Reload {
    /// Create a new blank reload pass.
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
            reloads: SparseMap::new(),
        }
    }

    /// Clear all data structures in this reload pass.
    pub fn clear(&mut self) {
        self.candidates.clear();
        self.reloads.clear();
    }

    /// Run the reload algorithm over `func`.
    pub fn run(
        &mut self,
        isa: &dyn TargetIsa,
        func: &mut Function,
        domtree: &DominatorTree,
        liveness: &mut Liveness,
        topo: &mut TopoOrder,
        tracker: &mut LiveValueTracker,
    ) {
        let _tt = timing::ra_reload();
        debug!("Reload for:\n{}", func.display(isa));
        let mut ctx = Context {
            cur: EncCursor::new(func, isa),
            encinfo: isa.encoding_info(),
            domtree,
            liveness,
            topo,
            candidates: &mut self.candidates,
            reloads: &mut self.reloads,
        };
        ctx.run(tracker)
    }
}

/// A reload candidate.
///
/// This represents a stack value that is used by the current instruction where a register is
/// needed.
struct ReloadCandidate {
    argidx: usize,
    value: Value,
    regclass: RegClass,
}

/// A Reloaded value.
///
/// This represents a value that has been reloaded into a register value from the stack.
struct ReloadedValue {
    stack: Value,
    reg: Value,
}

impl SparseMapValue<Value> for ReloadedValue {
    fn key(&self) -> Value {
        self.stack
    }
}

impl<'a> Context<'a> {
    fn run(&mut self, tracker: &mut LiveValueTracker) {
        self.topo.reset(self.cur.func.layout.blocks());
        while let Some(block) = self.topo.next(&self.cur.func.layout, self.domtree) {
            self.visit_block(block, tracker);
        }
    }

    fn visit_block(&mut self, block: Block, tracker: &mut LiveValueTracker) {
        debug!("Reloading {}:", block);
        self.visit_block_header(block, tracker);
        tracker.drop_dead_params();

        // visit_block_header() places us at the first interesting instruction in the block.
        while let Some(inst) = self.cur.current_inst() {
            if !self.cur.func.dfg[inst].opcode().is_ghost() {
                // This instruction either has an encoding or has ABI constraints, so visit it to
                // insert spills and fills as needed.
                let encoding = self.cur.func.encodings[inst];
                self.visit_inst(block, inst, encoding, tracker);
                tracker.drop_dead(inst);
            } else {
                // This is a ghost instruction with no encoding and no extra constraints, so we can
                // just skip over it.
                self.cur.next_inst();
            }
        }
    }

    /// Process the block parameters. Move to the next instruction in the block to be processed
    fn visit_block_header(&mut self, block: Block, tracker: &mut LiveValueTracker) {
        let (liveins, args) = tracker.block_top(
            block,
            &self.cur.func.dfg,
            self.liveness,
            &self.cur.func.layout,
            self.domtree,
        );

        if self.cur.func.layout.entry_block() == Some(block) {
            debug_assert_eq!(liveins.len(), 0);
            self.visit_entry_params(block, args);
        } else {
            self.visit_block_params(block, args);
        }
    }

    /// Visit the parameters on the entry block.
    /// These values have ABI constraints from the function signature.
    fn visit_entry_params(&mut self, block: Block, args: &[LiveValue]) {
        debug_assert_eq!(self.cur.func.signature.params.len(), args.len());
        self.cur.goto_first_inst(block);

        for (arg_idx, arg) in args.iter().enumerate() {
            let abi = self.cur.func.signature.params[arg_idx];
            match abi.location {
                ArgumentLoc::Reg(_) => {
                    if arg.affinity.is_stack() {
                        // An incoming register parameter was spilled. Replace the parameter value
                        // with a temporary register value that is immediately spilled.
                        let reg = self
                            .cur
                            .func
                            .dfg
                            .replace_block_param(arg.value, abi.value_type);
                        let affinity = Affinity::abi(&abi, self.cur.isa);
                        self.liveness.create_dead(reg, block, affinity);
                        self.insert_spill(block, arg.value, reg);
                    }
                }
                ArgumentLoc::Stack(_) => {
                    debug_assert!(arg.affinity.is_stack());
                }
                ArgumentLoc::Unassigned => panic!("Unexpected ABI location"),
            }
        }
    }

    fn visit_block_params(&mut self, block: Block, _args: &[LiveValue]) {
        self.cur.goto_first_inst(block);
    }

    /// Process the instruction pointed to by `pos`, and advance the cursor to the next instruction
    /// that needs processing.
    fn visit_inst(
        &mut self,
        block: Block,
        inst: Inst,
        encoding: Encoding,
        tracker: &mut LiveValueTracker,
    ) {
        self.cur.use_srcloc(inst);

        // Get the operand constraints for `inst` that we are trying to satisfy.
        let constraints = self.encinfo.operand_constraints(encoding);

        // Identify reload candidates.
        debug_assert!(self.candidates.is_empty());
        self.find_candidates(inst, constraints);

        // If we find a copy from a stack slot to the same stack slot, replace
        // it with a `copy_nop` but otherwise ignore it.  In particular, don't
        // generate a reload immediately followed by a spill.  The `copy_nop`
        // has a zero-length encoding, so will disappear at emission time.
        if let InstructionData::Unary {
            opcode: Opcode::Copy,
            arg,
        } = self.cur.func.dfg[inst]
        {
            let dst_vals = self.cur.func.dfg.inst_results(inst);
            if dst_vals.len() == 1 {
                let dst_val = dst_vals[0];
                let can_transform = match (
                    self.cur.func.locations[arg],
                    self.cur.func.locations[dst_val],
                ) {
                    (ValueLoc::Stack(src_slot), ValueLoc::Stack(dst_slot)) => {
                        src_slot == dst_slot && {
                            let src_ty = self.cur.func.dfg.value_type(arg);
                            let dst_ty = self.cur.func.dfg.value_type(dst_val);
                            debug_assert!(src_ty == dst_ty);
                            // This limits the transformation to copies of the
                            // types: I128 I64 I32 I16 I8 F64 and F32, since that's
                            // the set of `copy_nop` encodings available.
                            src_ty.is_int() || src_ty.is_float()
                        }
                    }
                    _ => false,
                };
                if can_transform {
                    // Convert the instruction into a `copy_nop`.
                    self.cur.func.dfg.replace(inst).copy_nop(arg);
                    let ok = self.cur.func.update_encoding(inst, self.cur.isa).is_ok();
                    debug_assert!(ok, "copy_nop encoding missing for this type");

                    // And move on to the next insn.
                    self.reloads.clear();
                    let _ = tracker.process_inst(inst, &self.cur.func.dfg, self.liveness);
                    self.cur.next_inst();
                    self.candidates.clear();
                    return;
                }
            }
        }

        // Deal with all instructions not special-cased by the immediately
        // preceding fragment.
        if let InstructionData::Unary {
            opcode: Opcode::Copy,
            ..
        } = self.cur.func.dfg[inst]
        {
            self.reload_copy_candidates(inst);
        } else {
            self.reload_inst_candidates(block, inst);
        }

        // TODO: Reuse reloads for future instructions.
        self.reloads.clear();

        let (_throughs, _kills, defs) =
            tracker.process_inst(inst, &self.cur.func.dfg, self.liveness);

        // Advance to the next instruction so we can insert any spills after the instruction.
        self.cur.next_inst();

        // Rewrite register defs that need to be spilled.
        //
        // Change:
        //
        // v2 = inst ...
        //
        // Into:
        //
        // v7 = inst ...
        // v2 = spill v7
        //
        // That way, we don't need to rewrite all future uses of v2.
        if let Some(constraints) = constraints {
            for (lv, op) in defs.iter().zip(constraints.outs) {
                if lv.affinity.is_stack() && op.kind != ConstraintKind::Stack {
                    if let InstructionData::Unary {
                        opcode: Opcode::Copy,
                        arg,
                    } = self.cur.func.dfg[inst]
                    {
                        self.cur.func.dfg.replace(inst).spill(arg);
                        let ok = self.cur.func.update_encoding(inst, self.cur.isa).is_ok();
                        debug_assert!(ok);
                    } else {
                        let value_type = self.cur.func.dfg.value_type(lv.value);
                        let reg = self.cur.func.dfg.replace_result(lv.value, value_type);
                        self.liveness.create_dead(reg, inst, Affinity::new(op));
                        self.insert_spill(block, lv.value, reg);
                    }
                }
            }
        }

        // Same thing for spilled call return values.
        let retvals = &defs[self.cur.func.dfg[inst]
            .opcode()
            .constraints()
            .num_fixed_results()..];
        if !retvals.is_empty() {
            let sig = self
                .cur
                .func
                .dfg
                .call_signature(inst)
                .expect("Extra results on non-call instruction");
            for (i, lv) in retvals.iter().enumerate() {
                let abi = self.cur.func.dfg.signatures[sig].returns[i];
                debug_assert!(
                    abi.location.is_reg(),
                    "expected reg; got {:?}",
                    abi.location
                );
                if lv.affinity.is_stack() {
                    let reg = self.cur.func.dfg.replace_result(lv.value, abi.value_type);
                    self.liveness
                        .create_dead(reg, inst, Affinity::abi(&abi, self.cur.isa));
                    self.insert_spill(block, lv.value, reg);
                }
            }
        }
    }

    // Reload the current candidates for the given `inst`.
    fn reload_inst_candidates(&mut self, block: Block, inst: Inst) {
        // Insert fill instructions before `inst` and replace `cand.value` with the filled value.
        for cand in self.candidates.iter_mut() {
            if let Some(reload) = self.reloads.get(cand.value) {
                cand.value = reload.reg;
                continue;
            }

            let reg = self.cur.ins().fill(cand.value);
            let fill = self.cur.built_inst();

            self.reloads.insert(ReloadedValue {
                stack: cand.value,
                reg,
            });
            cand.value = reg;

            // Create a live range for the new reload.
            let affinity = Affinity::Reg(cand.regclass.into());
            self.liveness.create_dead(reg, fill, affinity);
            self.liveness
                .extend_locally(reg, block, inst, &self.cur.func.layout);
        }

        // Rewrite instruction arguments.
        //
        // Only rewrite those arguments that were identified as candidates. This leaves block
        // arguments on branches as-is without rewriting them. A spilled block argument needs to stay
        // spilled because the matching block parameter is going to be in the same virtual register
        // and therefore the same stack slot as the block argument value.
        if !self.candidates.is_empty() {
            let args = self.cur.func.dfg.inst_args_mut(inst);
            while let Some(cand) = self.candidates.pop() {
                args[cand.argidx] = cand.value;
            }
        }
    }

    // Reload the current candidates for the given copy `inst`.
    //
    // As an optimization, replace a copy instruction where the argument has been spilled with
    // a fill instruction.
    fn reload_copy_candidates(&mut self, inst: Inst) {
        // Copy instructions can only have one argument.
        debug_assert!(self.candidates.is_empty() || self.candidates.len() == 1);

        if let Some(cand) = self.candidates.pop() {
            self.cur.func.dfg.replace(inst).fill(cand.value);
            let ok = self.cur.func.update_encoding(inst, self.cur.isa).is_ok();
            debug_assert!(ok);
        }
    }

    // Find reload candidates for `inst` and add them to `self.candidates`.
    //
    // These are uses of spilled values where the operand constraint requires a register.
    fn find_candidates(&mut self, inst: Inst, constraints: Option<&RecipeConstraints>) {
        let args = self.cur.func.dfg.inst_args(inst);

        if let Some(constraints) = constraints {
            for (argidx, (op, &arg)) in constraints.ins.iter().zip(args).enumerate() {
                if op.kind != ConstraintKind::Stack && self.liveness[arg].affinity.is_stack() {
                    self.candidates.push(ReloadCandidate {
                        argidx,
                        value: arg,
                        regclass: op.regclass,
                    })
                }
            }
        }

        // If we only have the fixed arguments, we're done now.
        let offset = self.cur.func.dfg[inst]
            .opcode()
            .constraints()
            .num_fixed_value_arguments();
        if args.len() == offset {
            return;
        }
        let var_args = &args[offset..];

        // Handle ABI arguments.
        if let Some(sig) = self.cur.func.dfg.call_signature(inst) {
            handle_abi_args(
                self.candidates,
                &self.cur.func.dfg.signatures[sig].params,
                var_args,
                offset,
                self.cur.isa,
                self.liveness,
            );
        } else if self.cur.func.dfg[inst].opcode().is_return() {
            handle_abi_args(
                self.candidates,
                &self.cur.func.signature.returns,
                var_args,
                offset,
                self.cur.isa,
                self.liveness,
            );
        }
    }

    /// Insert a spill at `pos` and update data structures.
    ///
    /// - Insert `stack = spill reg` at `pos`, and assign an encoding.
    /// - Move the `stack` live range starting point to the new instruction.
    /// - Extend the `reg` live range to reach the new instruction.
    fn insert_spill(&mut self, block: Block, stack: Value, reg: Value) {
        self.cur.ins().with_result(stack).spill(reg);
        let inst = self.cur.built_inst();

        // Update live ranges.
        self.liveness.move_def_locally(stack, inst);
        self.liveness
            .extend_locally(reg, block, inst, &self.cur.func.layout);
    }
}

/// Find reload candidates in the instruction's ABI variable arguments. This handles both
/// return values and call arguments.
fn handle_abi_args(
    candidates: &mut Vec<ReloadCandidate>,
    abi_types: &[AbiParam],
    var_args: &[Value],
    offset: usize,
    isa: &dyn TargetIsa,
    liveness: &Liveness,
) {
    debug_assert_eq!(abi_types.len(), var_args.len());
    for ((abi, &arg), argidx) in abi_types.iter().zip(var_args).zip(offset..) {
        if abi.location.is_reg() {
            let lv = liveness.get(arg).expect("Missing live range for ABI arg");
            if lv.affinity.is_stack() {
                candidates.push(ReloadCandidate {
                    argidx,
                    value: arg,
                    regclass: isa.regclass_for_abi_type(abi.value_type),
                });
            }
        }
    }
}

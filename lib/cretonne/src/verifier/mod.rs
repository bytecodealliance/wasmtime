//! A verifier for ensuring that functions are well formed.
//! It verifies:
//!
//!   EBB integrity
//!
//!    - All instructions reached from the `ebb_insts` iterator must belong to
//!      the EBB as reported by `inst_ebb()`.
//!    - Every EBB must end in a terminator instruction, and no other instruction
//!      can be a terminator.
//!    - Every value in the `ebb_params` iterator belongs to the EBB as reported by `value_ebb`.
//!
//!   Instruction integrity
//!
//!    - The instruction format must match the opcode.
//!    - All result values must be created for multi-valued instructions.
//!    - All referenced entities must exist. (Values, EBBs, stack slots, ...)
//!    - Instructions must not reference (eg. branch to) the entry block.
//!
//!   SSA form
//!
//!    - Values must be defined by an instruction that exists and that is inserted in
//!      an EBB, or be an argument of an existing EBB.
//!    - Values used by an instruction must dominate the instruction.
//!
//!   Control flow graph and dominator tree integrity:
//!
//!    - All predecessors in the CFG must be branches to the EBB.
//!    - All branches to an EBB must be present in the CFG.
//!    - A recomputed dominator tree is identical to the existing one.
//!
//!   Type checking
//!
//!    - Compare input and output values against the opcode's type constraints.
//!      For polymorphic opcodes, determine the controlling type variable first.
//!    - Branches and jumps must pass arguments to destination EBBs that match the
//!      expected types exactly. The number of arguments must match.
//!    - All EBBs in a jump_table must take no arguments.
//!    - Function calls are type checked against their signature.
//!    - The entry block must take arguments that match the signature of the current
//!      function.
//!    - All return instructions must have return value operands matching the current
//!      function signature.
//!
//!   Global variables
//!
//!   - Detect cycles in deref(base) declarations.
//!
//! TODO:
//!   Ad hoc checking
//!
//!    - Stack slot loads and stores must be in-bounds.
//!    - Immediate constraints for certain opcodes, like `udiv_imm v3, 0`.
//!    - `Insertlane` and `extractlane` instructions have immediate lane numbers that must be in
//!      range for their polymorphic type.
//!    - Swizzle and shuffle instructions take a variable number of lane arguments. The number
//!      of arguments must match the destination type, and the lane indexes must be in range.

use dbg::DisplayList;
use dominator_tree::DominatorTree;
use entity::SparseSet;
use flowgraph::ControlFlowGraph;
use ir::entities::AnyEntity;
use ir::instructions::{InstructionFormat, BranchInfo, ResolvedConstraint, CallInfo};
use ir::{types, Function, ValueDef, Ebb, Inst, SigRef, FuncRef, ValueList, JumpTable, StackSlot,
         StackSlotKind, GlobalVar, Value, Type, Opcode, ValueLoc, ArgumentLoc};
use ir;
use isa::TargetIsa;
use iterators::IteratorExtras;
use self::flags::verify_flags;
use settings::{Flags, FlagsOrIsa};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::error as std_error;
use std::fmt::{self, Display, Formatter, Write};
use std::result;

pub use self::cssa::verify_cssa;
pub use self::liveness::verify_liveness;
pub use self::locations::verify_locations;

// Create an `Err` variant of `Result<X>` from a location and `format!` arguments.
macro_rules! err {
    ( $loc:expr, $msg:expr ) => {
        Err(::verifier::Error {
            location: $loc.into(),
            message: String::from($msg),
        })
    };

    ( $loc:expr, $fmt:expr, $( $arg:expr ),+ ) => {
        Err(::verifier::Error {
            location: $loc.into(),
            message: format!( $fmt, $( $arg ),+ ),
        })
    };
}

mod cssa;
mod flags;
mod liveness;
mod locations;

/// A verifier error.
#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    /// The entity causing the verifier error.
    pub location: AnyEntity,
    /// Error message.
    pub message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

impl std_error::Error for Error {
    fn description(&self) -> &str {
        &self.message
    }
}

/// Verifier result.
pub type Result = result::Result<(), Error>;

/// Verify `func`.
pub fn verify_function<'a, FOI: Into<FlagsOrIsa<'a>>>(func: &Function, fisa: FOI) -> Result {
    Verifier::new(func, fisa.into()).run()
}

/// Verify `func` after checking the integrity of associated context data structures `cfg` and
/// `domtree`.
pub fn verify_context<'a, FOI: Into<FlagsOrIsa<'a>>>(
    func: &Function,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    fisa: FOI,
) -> Result {
    let verifier = Verifier::new(func, fisa.into());
    if cfg.is_valid() {
        verifier.cfg_integrity(cfg)?;
    }
    if domtree.is_valid() {
        verifier.domtree_integrity(domtree)?;
    }
    verifier.run()
}

struct Verifier<'a> {
    func: &'a Function,
    expected_cfg: ControlFlowGraph,
    expected_domtree: DominatorTree,
    flags: &'a Flags,
    isa: Option<&'a TargetIsa>,
}

impl<'a> Verifier<'a> {
    pub fn new(func: &'a Function, fisa: FlagsOrIsa<'a>) -> Verifier<'a> {
        let expected_cfg = ControlFlowGraph::with_function(func);
        let expected_domtree = DominatorTree::with_function(func, &expected_cfg);
        Verifier {
            func,
            expected_cfg,
            expected_domtree,
            flags: fisa.flags,
            isa: fisa.isa,
        }
    }

    // Check for cycles in the global variable declarations.
    fn verify_global_vars(&self) -> Result {
        let mut seen = SparseSet::new();

        for gv in self.func.global_vars.keys() {
            seen.clear();
            seen.insert(gv);

            let mut cur = gv;
            while let ir::GlobalVarData::Deref { base, .. } = self.func.global_vars[cur] {
                if seen.insert(base).is_some() {
                    return err!(gv, "deref cycle: {}", DisplayList(seen.as_slice()));
                }

                cur = base;
            }
        }

        Ok(())
    }

    fn ebb_integrity(&self, ebb: Ebb, inst: Inst) -> Result {

        let is_terminator = self.func.dfg[inst].opcode().is_terminator();
        let is_last_inst = self.func.layout.last_inst(ebb) == Some(inst);

        if is_terminator && !is_last_inst {
            // Terminating instructions only occur at the end of blocks.
            return err!(
                inst,
                "a terminator instruction was encountered before the end of {}",
                ebb
            );
        }
        if is_last_inst && !is_terminator {
            return err!(ebb, "block does not end in a terminator instruction!");
        }

        // Instructions belong to the correct ebb.
        let inst_ebb = self.func.layout.inst_ebb(inst);
        if inst_ebb != Some(ebb) {
            return err!(inst, "should belong to {} not {:?}", ebb, inst_ebb);
        }

        // Parameters belong to the correct ebb.
        for &arg in self.func.dfg.ebb_params(ebb) {
            match self.func.dfg.value_def(arg) {
                ValueDef::Param(arg_ebb, _) => {
                    if ebb != arg_ebb {
                        return err!(arg, "does not belong to {}", ebb);
                    }
                }
                _ => {
                    return err!(arg, "expected an argument, found a result");
                }
            }
        }

        Ok(())
    }

    fn instruction_integrity(&self, inst: Inst) -> Result {
        let inst_data = &self.func.dfg[inst];
        let dfg = &self.func.dfg;

        // The instruction format matches the opcode
        if inst_data.opcode().format() != InstructionFormat::from(inst_data) {
            return err!(inst, "instruction opcode doesn't match instruction format");
        }

        let fixed_results = inst_data.opcode().constraints().fixed_results();
        // var_results is 0 if we aren't a call instruction
        let var_results = dfg.call_signature(inst)
            .map(|sig| dfg.signatures[sig].returns.len())
            .unwrap_or(0);
        let total_results = fixed_results + var_results;

        // All result values for multi-valued instructions are created
        let got_results = dfg.inst_results(inst).len();
        if got_results != total_results {
            return err!(
                inst,
                "expected {} result values, found {}",
                total_results,
                got_results
            );
        }

        self.verify_entity_references(inst)
    }

    fn verify_entity_references(&self, inst: Inst) -> Result {
        use ir::instructions::InstructionData::*;

        for &arg in self.func.dfg.inst_args(inst) {
            self.verify_value(inst, arg)?;

            // All used values must be attached to something.
            let original = self.func.dfg.resolve_aliases(arg);
            if !self.func.dfg.value_is_attached(original) {
                return err!(inst, "argument {} -> {} is not attached", arg, original);
            }
        }

        for &res in self.func.dfg.inst_results(inst) {
            self.verify_value(inst, res)?;
        }

        match self.func.dfg[inst] {
            MultiAry { ref args, .. } => {
                self.verify_value_list(inst, args)?;
            }
            Jump {
                destination,
                ref args,
                ..
            } |
            Branch {
                destination,
                ref args,
                ..
            } |
            BranchInt {
                destination,
                ref args,
                ..
            } |
            BranchFloat {
                destination,
                ref args,
                ..
            } |
            BranchIcmp {
                destination,
                ref args,
                ..
            } => {
                self.verify_ebb(inst, destination)?;
                self.verify_value_list(inst, args)?;
            }
            BranchTable { table, .. } => {
                self.verify_jump_table(inst, table)?;
            }
            Call { func_ref, ref args, .. } => {
                self.verify_func_ref(inst, func_ref)?;
                self.verify_value_list(inst, args)?;
            }
            IndirectCall { sig_ref, ref args, .. } => {
                self.verify_sig_ref(inst, sig_ref)?;
                self.verify_value_list(inst, args)?;
            }
            FuncAddr { func_ref, .. } => {
                self.verify_func_ref(inst, func_ref)?;
            }
            StackLoad { stack_slot, .. } |
            StackStore { stack_slot, .. } => {
                self.verify_stack_slot(inst, stack_slot)?;
            }
            UnaryGlobalVar { global_var, .. } => {
                self.verify_global_var(inst, global_var)?;
            }
            HeapAddr { heap, .. } => {
                self.verify_heap(inst, heap)?;
            }
            RegSpill { dst, .. } => {
                self.verify_stack_slot(inst, dst)?;
            }
            RegFill { src, .. } => {
                self.verify_stack_slot(inst, src)?;
            }

            // Exhaustive list so we can't forget to add new formats
            Unary { .. } |
            UnaryImm { .. } |
            UnaryIeee32 { .. } |
            UnaryIeee64 { .. } |
            UnaryBool { .. } |
            Binary { .. } |
            BinaryImm { .. } |
            Ternary { .. } |
            InsertLane { .. } |
            ExtractLane { .. } |
            IntCompare { .. } |
            IntCompareImm { .. } |
            IntCond { .. } |
            FloatCompare { .. } |
            FloatCond { .. } |
            Load { .. } |
            Store { .. } |
            RegMove { .. } |
            CopySpecial { .. } |
            Trap { .. } |
            CondTrap { .. } |
            NullAry { .. } => {}
        }

        Ok(())
    }

    fn verify_ebb(&self, inst: Inst, e: Ebb) -> Result {
        if !self.func.dfg.ebb_is_valid(e) || !self.func.layout.is_ebb_inserted(e) {
            return err!(inst, "invalid ebb reference {}", e);
        }
        if let Some(entry_block) = self.func.layout.entry_block() {
            if e == entry_block {
                return err!(inst, "invalid reference to entry ebb {}", e);
            }
        }
        Ok(())
    }

    fn verify_sig_ref(&self, inst: Inst, s: SigRef) -> Result {
        if !self.func.dfg.signatures.is_valid(s) {
            err!(inst, "invalid signature reference {}", s)
        } else {
            Ok(())
        }
    }

    fn verify_func_ref(&self, inst: Inst, f: FuncRef) -> Result {
        if !self.func.dfg.ext_funcs.is_valid(f) {
            err!(inst, "invalid function reference {}", f)
        } else {
            Ok(())
        }
    }

    fn verify_stack_slot(&self, inst: Inst, ss: StackSlot) -> Result {
        if !self.func.stack_slots.is_valid(ss) {
            err!(inst, "invalid stack slot {}", ss)
        } else {
            Ok(())
        }
    }

    fn verify_global_var(&self, inst: Inst, gv: GlobalVar) -> Result {
        if !self.func.global_vars.is_valid(gv) {
            err!(inst, "invalid global variable {}", gv)
        } else {
            Ok(())
        }
    }

    fn verify_heap(&self, inst: Inst, heap: ir::Heap) -> Result {
        if !self.func.heaps.is_valid(heap) {
            err!(inst, "invalid heap {}", heap)
        } else {
            Ok(())
        }
    }

    fn verify_value_list(&self, inst: Inst, l: &ValueList) -> Result {
        if !l.is_valid(&self.func.dfg.value_lists) {
            err!(inst, "invalid value list reference {:?}", l)
        } else {
            Ok(())
        }
    }

    fn verify_jump_table(&self, inst: Inst, j: JumpTable) -> Result {
        if !self.func.jump_tables.is_valid(j) {
            err!(inst, "invalid jump table reference {}", j)
        } else {
            Ok(())
        }
    }

    fn verify_value(&self, loc_inst: Inst, v: Value) -> Result {
        let dfg = &self.func.dfg;
        if !dfg.value_is_valid(v) {
            return err!(loc_inst, "invalid value reference {}", v);
        }

        // SSA form
        match dfg.value_def(v) {
            ValueDef::Result(def_inst, _) => {
                // Value is defined by an instruction that exists.
                if !dfg.inst_is_valid(def_inst) {
                    return err!(
                        loc_inst,
                        "{} is defined by invalid instruction {}",
                        v,
                        def_inst
                    );
                }
                // Defining instruction is inserted in an EBB.
                if self.func.layout.inst_ebb(def_inst) == None {
                    return err!(
                        loc_inst,
                        "{} is defined by {} which has no EBB",
                        v,
                        def_inst
                    );
                }
                // Defining instruction dominates the instruction that uses the value.
                if self.expected_domtree.is_reachable(
                    self.func.layout.pp_ebb(loc_inst),
                ) &&
                    !self.expected_domtree.dominates(
                        def_inst,
                        loc_inst,
                        &self.func.layout,
                    )
                {
                    return err!(loc_inst, "uses value from non-dominating {}", def_inst);
                }
            }
            ValueDef::Param(ebb, _) => {
                // Value is defined by an existing EBB.
                if !dfg.ebb_is_valid(ebb) {
                    return err!(loc_inst, "{} is defined by invalid EBB {}", v, ebb);
                }
                // Defining EBB is inserted in the layout
                if !self.func.layout.is_ebb_inserted(ebb) {
                    return err!(
                        loc_inst,
                        "{} is defined by {} which is not in the layout",
                        v,
                        ebb
                    );
                }
                // The defining EBB dominates the instruction using this value.
                if self.expected_domtree.is_reachable(ebb) &&
                    !self.expected_domtree.dominates(
                        ebb,
                        loc_inst,
                        &self.func.layout,
                    )
                {
                    return err!(loc_inst, "uses value arg from non-dominating {}", ebb);
                }
            }
        }
        Ok(())
    }

    fn domtree_integrity(&self, domtree: &DominatorTree) -> Result {
        // We consider two `DominatorTree`s to be equal if they return the same immediate
        // dominator for each EBB. Therefore the current domtree is valid if it matches the freshly
        // computed one.
        for ebb in self.func.layout.ebbs() {
            let expected = self.expected_domtree.idom(ebb);
            let got = domtree.idom(ebb);
            if got != expected {
                return err!(
                    ebb,
                    "invalid domtree, expected idom({}) = {:?}, got {:?}",
                    ebb,
                    expected,
                    got
                );
            }
        }
        // We also verify if the postorder defined by `DominatorTree` is sane
        if domtree.cfg_postorder().len() != self.expected_domtree.cfg_postorder().len() {
            return err!(
                AnyEntity::Function,
                "incorrect number of Ebbs in postorder traversal"
            );
        }
        for (index, (&test_ebb, &true_ebb)) in
            domtree
                .cfg_postorder()
                .iter()
                .zip(self.expected_domtree.cfg_postorder().iter())
                .enumerate()
        {
            if test_ebb != true_ebb {
                return err!(
                    test_ebb,
                    "invalid domtree, postorder ebb number {} should be {}, got {}",
                    index,
                    true_ebb,
                    test_ebb
                );
            }
        }
        // We verify rpo_cmp on pairs of adjacent ebbs in the postorder
        for (&prev_ebb, &next_ebb) in domtree.cfg_postorder().iter().adjacent_pairs() {
            if self.expected_domtree.rpo_cmp(
                prev_ebb,
                next_ebb,
                &self.func.layout,
            ) != Ordering::Greater
            {
                return err!(
                    next_ebb,
                    "invalid domtree, rpo_cmp does not says {} is greater than {}",
                    prev_ebb,
                    next_ebb
                );
            }
        }
        Ok(())
    }

    fn typecheck_entry_block_params(&self) -> Result {
        if let Some(ebb) = self.func.layout.entry_block() {
            let expected_types = &self.func.signature.params;
            let ebb_param_count = self.func.dfg.num_ebb_params(ebb);

            if ebb_param_count != expected_types.len() {
                return err!(
                    ebb,
                    "entry block parameters ({}) must match function signature ({})",
                    ebb_param_count,
                    expected_types.len()
                );
            }

            for (i, &arg) in self.func.dfg.ebb_params(ebb).iter().enumerate() {
                let arg_type = self.func.dfg.value_type(arg);
                if arg_type != expected_types[i].value_type {
                    return err!(
                        ebb,
                        "entry block parameter {} expected to have type {}, got {}",
                        i,
                        expected_types[i],
                        arg_type
                    );
                }
            }
        }
        Ok(())
    }

    fn typecheck(&self, inst: Inst) -> Result {
        let inst_data = &self.func.dfg[inst];
        let constraints = inst_data.opcode().constraints();

        let ctrl_type = if let Some(value_typeset) = constraints.ctrl_typeset() {
            // For polymorphic opcodes, determine the controlling type variable first.
            let ctrl_type = self.func.dfg.ctrl_typevar(inst);

            if !value_typeset.contains(ctrl_type) {
                return err!(inst, "has an invalid controlling type {}", ctrl_type);
            }

            ctrl_type
        } else {
            // Non-polymorphic instructions don't check the controlling type variable, so `Option`
            // is unnecessary and we can just make it `VOID`.
            types::VOID
        };

        self.typecheck_results(inst, ctrl_type)?;
        self.typecheck_fixed_args(inst, ctrl_type)?;
        self.typecheck_variable_args(inst)?;
        self.typecheck_return(inst)?;
        self.typecheck_special(inst, ctrl_type)?;

        Ok(())
    }

    fn typecheck_results(&self, inst: Inst, ctrl_type: Type) -> Result {
        let mut i = 0;
        for &result in self.func.dfg.inst_results(inst) {
            let result_type = self.func.dfg.value_type(result);
            let expected_type = self.func.dfg.compute_result_type(inst, i, ctrl_type);
            if let Some(expected_type) = expected_type {
                if result_type != expected_type {
                    return err!(
                        inst,
                        "expected result {} ({}) to have type {}, found {}",
                        i,
                        result,
                        expected_type,
                        result_type
                    );
                }
            } else {
                return err!(inst, "has more result values than expected");
            }
            i += 1;
        }

        // There aren't any more result types left.
        if self.func.dfg.compute_result_type(inst, i, ctrl_type) != None {
            return err!(inst, "has fewer result values than expected");
        }
        Ok(())
    }

    fn typecheck_fixed_args(&self, inst: Inst, ctrl_type: Type) -> Result {
        let constraints = self.func.dfg[inst].opcode().constraints();

        for (i, &arg) in self.func.dfg.inst_fixed_args(inst).iter().enumerate() {
            let arg_type = self.func.dfg.value_type(arg);
            match constraints.value_argument_constraint(i, ctrl_type) {
                ResolvedConstraint::Bound(expected_type) => {
                    if arg_type != expected_type {
                        return err!(
                            inst,
                            "arg {} ({}) has type {}, expected {}",
                            i,
                            arg,
                            arg_type,
                            expected_type
                        );
                    }
                }
                ResolvedConstraint::Free(type_set) => {
                    if !type_set.contains(arg_type) {
                        return err!(
                            inst,
                            "arg {} ({}) with type {} failed to satisfy type set {:?}",
                            i,
                            arg,
                            arg_type,
                            type_set
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn typecheck_variable_args(&self, inst: Inst) -> Result {
        match self.func.dfg[inst].analyze_branch(&self.func.dfg.value_lists) {
            BranchInfo::SingleDest(ebb, _) => {
                let iter = self.func.dfg.ebb_params(ebb).iter().map(|&v| {
                    self.func.dfg.value_type(v)
                });
                self.typecheck_variable_args_iterator(inst, iter)?;
            }
            BranchInfo::Table(table) => {
                for (_, ebb) in self.func.jump_tables[table].entries() {
                    let arg_count = self.func.dfg.num_ebb_params(ebb);
                    if arg_count != 0 {
                        return err!(
                            inst,
                            "takes no arguments, but had target {} with {} arguments",
                            ebb,
                            arg_count
                        );
                    }
                }
            }
            BranchInfo::NotABranch => {}
        }

        match self.func.dfg[inst].analyze_call(&self.func.dfg.value_lists) {
            CallInfo::Direct(func_ref, _) => {
                let sig_ref = self.func.dfg.ext_funcs[func_ref].signature;
                let arg_types = self.func.dfg.signatures[sig_ref].params.iter().map(|a| {
                    a.value_type
                });
                self.typecheck_variable_args_iterator(inst, arg_types)?;
                self.check_outgoing_args(inst, sig_ref)?;
            }
            CallInfo::Indirect(sig_ref, _) => {
                let arg_types = self.func.dfg.signatures[sig_ref].params.iter().map(|a| {
                    a.value_type
                });
                self.typecheck_variable_args_iterator(inst, arg_types)?;
                self.check_outgoing_args(inst, sig_ref)?;
            }
            CallInfo::NotACall => {}
        }
        Ok(())
    }

    fn typecheck_variable_args_iterator<I: Iterator<Item = Type>>(
        &self,
        inst: Inst,
        iter: I,
    ) -> Result {
        let variable_args = self.func.dfg.inst_variable_args(inst);
        let mut i = 0;

        for expected_type in iter {
            if i >= variable_args.len() {
                // Result count mismatch handled below, we want the full argument count first though
                i += 1;
                continue;
            }
            let arg = variable_args[i];
            let arg_type = self.func.dfg.value_type(arg);
            if expected_type != arg_type {
                return err!(
                    inst,
                    "arg {} ({}) has type {}, expected {}",
                    i,
                    variable_args[i],
                    arg_type,
                    expected_type
                );
            }
            i += 1;
        }
        if i != variable_args.len() {
            return err!(
                inst,
                "mismatched argument count, got {}, expected {}",
                variable_args.len(),
                i
            );
        }
        Ok(())
    }

    /// Check the locations assigned to outgoing call arguments.
    ///
    /// When a signature has been legalized, all values passed as outgoing arguments on the stack
    /// must be assigned to a matching `OutgoingArg` stack slot.
    fn check_outgoing_args(&self, inst: Inst, sig_ref: SigRef) -> Result {
        let sig = &self.func.dfg.signatures[sig_ref];

        // Before legalization, there's nothing to check.
        if sig.argument_bytes.is_none() {
            return Ok(());
        }

        let args = self.func.dfg.inst_variable_args(inst);
        let expected_args = &sig.params[..];

        for (&arg, &abi) in args.iter().zip(expected_args) {
            // Value types have already been checked by `typecheck_variable_args_iterator()`.
            if let ArgumentLoc::Stack(offset) = abi.location {
                let arg_loc = self.func.locations[arg];
                if let ValueLoc::Stack(ss) = arg_loc {
                    // Argument value is assigned to a stack slot as expected.
                    self.verify_stack_slot(inst, ss)?;
                    let slot = &self.func.stack_slots[ss];
                    if slot.kind != StackSlotKind::OutgoingArg {
                        return err!(
                            inst,
                            "Outgoing stack argument {} in wrong stack slot: {} = {}",
                            arg,
                            ss,
                            slot
                        );
                    }
                    if slot.offset != offset {
                        return err!(
                            inst,
                            "Outgoing stack argument {} should have offset {}: {} = {}",
                            arg,
                            offset,
                            ss,
                            slot
                        );
                    }
                    if slot.size != abi.value_type.bytes() {
                        return err!(
                            inst,
                            "Outgoing stack argument {} wrong size for {}: {} = {}",
                            arg,
                            abi.value_type,
                            ss,
                            slot
                        );
                    }
                } else {
                    let reginfo = self.isa.map(|i| i.register_info());
                    return err!(
                        inst,
                        "Outgoing stack argument {} in wrong location: {}",
                        arg,
                        arg_loc.display(reginfo.as_ref())
                    );
                }
            }
        }
        Ok(())
    }

    fn typecheck_return(&self, inst: Inst) -> Result {
        if self.func.dfg[inst].opcode().is_return() {
            let args = self.func.dfg.inst_variable_args(inst);
            let expected_types = &self.func.signature.returns;
            if args.len() != expected_types.len() {
                return err!(inst, "arguments of return must match function signature");
            }
            for (i, (&arg, &expected_type)) in args.iter().zip(expected_types).enumerate() {
                let arg_type = self.func.dfg.value_type(arg);
                if arg_type != expected_type.value_type {
                    return err!(
                        inst,
                        "arg {} ({}) has type {}, must match function signature of {}",
                        i,
                        arg,
                        arg_type,
                        expected_type
                    );
                }
            }
        }
        Ok(())
    }

    // Check special-purpose type constraints that can't be expressed in the normal opcode
    // constraints.
    fn typecheck_special(&self, inst: Inst, ctrl_type: Type) -> Result {
        match self.func.dfg[inst] {
            ir::InstructionData::Unary { opcode, arg } => {
                let arg_type = self.func.dfg.value_type(arg);
                match opcode {
                    Opcode::Bextend | Opcode::Uextend | Opcode::Sextend | Opcode::Fpromote => {
                        if arg_type.lane_count() != ctrl_type.lane_count() {
                            return err!(
                                inst,
                                "input {} and output {} must have same number of lanes",
                                arg_type,
                                ctrl_type
                            );
                        }
                        if arg_type.lane_bits() >= ctrl_type.lane_bits() {
                            return err!(
                                inst,
                                "input {} must be smaller than output {}",
                                arg_type,
                                ctrl_type
                            );
                        }
                    }
                    Opcode::Breduce | Opcode::Ireduce | Opcode::Fdemote => {
                        if arg_type.lane_count() != ctrl_type.lane_count() {
                            return err!(
                                inst,
                                "input {} and output {} must have same number of lanes",
                                arg_type,
                                ctrl_type
                            );
                        }
                        if arg_type.lane_bits() <= ctrl_type.lane_bits() {
                            return err!(
                                inst,
                                "input {} must be larger than output {}",
                                arg_type,
                                ctrl_type
                            );
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn cfg_integrity(&self, cfg: &ControlFlowGraph) -> Result {
        let mut expected_succs = BTreeSet::<Ebb>::new();
        let mut got_succs = BTreeSet::<Ebb>::new();
        let mut expected_preds = BTreeSet::<Inst>::new();
        let mut got_preds = BTreeSet::<Inst>::new();

        for ebb in self.func.layout.ebbs() {
            expected_succs.extend(self.expected_cfg.succ_iter(ebb));
            got_succs.extend(cfg.succ_iter(ebb));

            let missing_succs: Vec<Ebb> = expected_succs.difference(&got_succs).cloned().collect();
            if !missing_succs.is_empty() {
                return err!(
                    ebb,
                    "cfg lacked the following successor(s) {:?}",
                    missing_succs
                );
            }

            let excess_succs: Vec<Ebb> = got_succs.difference(&expected_succs).cloned().collect();
            if !excess_succs.is_empty() {
                return err!(ebb, "cfg had unexpected successor(s) {:?}", excess_succs);
            }

            expected_preds.extend(self.expected_cfg.pred_iter(ebb).map(|(_, inst)| inst));
            got_preds.extend(cfg.pred_iter(ebb).map(|(_, inst)| inst));

            let missing_preds: Vec<Inst> = expected_preds.difference(&got_preds).cloned().collect();
            if !missing_preds.is_empty() {
                return err!(
                    ebb,
                    "cfg lacked the following predecessor(s) {:?}",
                    missing_preds
                );
            }

            let excess_preds: Vec<Inst> = got_preds.difference(&expected_preds).cloned().collect();
            if !excess_preds.is_empty() {
                return err!(ebb, "cfg had unexpected predecessor(s) {:?}", excess_preds);
            }

            expected_succs.clear();
            got_succs.clear();
            expected_preds.clear();
            got_preds.clear();
        }
        Ok(())
    }

    /// If the verifier has been set up with an ISA, make sure that the recorded encoding for the
    /// instruction (if any) matches how the ISA would encode it.
    fn verify_encoding(&self, inst: Inst) -> Result {
        // When the encodings table is empty, we don't require any instructions to be encoded.
        //
        // Once some instructions are encoded, we require all side-effecting instructions to have a
        // legal encoding.
        if self.func.encodings.is_empty() {
            return Ok(());
        }

        let isa = match self.isa {
            Some(isa) => isa,
            None => return Ok(()),
        };

        let encoding = self.func.encodings[inst];
        if encoding.is_legal() {
            let mut encodings = isa.legal_encodings(
                &self.func.dfg,
                &self.func.dfg[inst],
                self.func.dfg.ctrl_typevar(inst),
            ).peekable();

            if encodings.peek().is_none() {
                return err!(
                    inst,
                    "Instruction failed to re-encode {}",
                    isa.encoding_info().display(encoding)
                );
            }

            let has_valid_encoding = encodings.any(|possible_enc| encoding == possible_enc);

            if !has_valid_encoding {
                let mut possible_encodings = String::new();
                let mut multiple_encodings = false;

                for enc in isa.legal_encodings(
                    &self.func.dfg,
                    &self.func.dfg[inst],
                    self.func.dfg.ctrl_typevar(inst),
                )
                {
                    if !possible_encodings.is_empty() {
                        possible_encodings.push_str(", ");
                        multiple_encodings = true;
                    }
                    possible_encodings
                        .write_fmt(format_args!("{}", isa.encoding_info().display(enc)))
                        .unwrap();
                }

                return err!(
                    inst,
                    "encoding {} should be {}{}",
                    isa.encoding_info().display(encoding),
                    if multiple_encodings { "one of: " } else { "" },
                    possible_encodings
                );
            }
            return Ok(());
        }

        // Instruction is not encoded, so it is a ghost instruction.
        // Instructions with side effects are not allowed to be ghost instructions.
        let opcode = self.func.dfg[inst].opcode();

        // The `fallthrough` instruction is marked as a terminator and a branch, but it is not
        // required to have an encoding.
        if opcode == Opcode::Fallthrough {
            return Ok(());
        }

        // Check if this opcode must be encoded.
        let mut needs_enc = None;
        if opcode.is_branch() {
            needs_enc = Some("Branch");
        } else if opcode.is_call() {
            needs_enc = Some("Call");
        } else if opcode.is_return() {
            needs_enc = Some("Return");
        } else if opcode.can_store() {
            needs_enc = Some("Store");
        } else if opcode.can_trap() {
            needs_enc = Some("Trapping instruction");
        } else if opcode.other_side_effects() {
            needs_enc = Some("Instruction with side effects");
        }

        if let Some(text) = needs_enc {
            // This instruction needs an encoding, so generate an error.
            // Provide the ISA default encoding as a hint.
            match isa.encode(
                &self.func.dfg,
                &self.func.dfg[inst],
                self.func.dfg.ctrl_typevar(inst),
            ) {
                Ok(enc) => {
                    return err!(
                        inst,
                        "{} must have an encoding (e.g., {})",
                        text,
                        isa.encoding_info().display(enc)
                    )
                }
                Err(_) => return err!(inst, "{} must have an encoding", text),
            }
        }

        Ok(())
    }

    /// Verify the `return_at_end` property which requires that there are no internal return
    /// instructions.
    fn verify_return_at_end(&self) -> Result {
        for ebb in self.func.layout.ebbs() {
            let inst = self.func.layout.last_inst(ebb).unwrap();
            if self.func.dfg[inst].opcode().is_return() &&
                Some(ebb) != self.func.layout.last_ebb()
            {
                return err!(inst, "Internal return not allowed with return_at_end=1");
            }
        }

        Ok(())
    }

    pub fn run(&self) -> Result {
        self.verify_global_vars()?;
        self.typecheck_entry_block_params()?;
        for ebb in self.func.layout.ebbs() {
            for inst in self.func.layout.ebb_insts(ebb) {
                self.ebb_integrity(ebb, inst)?;
                self.instruction_integrity(inst)?;
                self.typecheck(inst)?;
                self.verify_encoding(inst)?;
            }
        }

        if self.flags.return_at_end() {
            self.verify_return_at_end()?;
        }

        verify_flags(self.func, &self.expected_cfg, self.isa)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Verifier, Error};
    use ir::Function;
    use ir::instructions::{InstructionData, Opcode};
    use entity::EntityList;
    use settings;

    macro_rules! assert_err_with_msg {
        ($e:expr, $msg:expr) => (
            match $e {
                Ok(_) => { panic!("Expected an error!") },
                Err(Error { message, .. } ) => {
                    if !message.contains($msg) {
                       panic!(format!("'{}' did not contain the substring '{}'", message, $msg));
                    }
                }
            }
        )
    }

    #[test]
    fn empty() {
        let func = Function::new();
        let flags = &settings::Flags::new(&settings::builder());
        let verifier = Verifier::new(&func, flags.into());
        assert_eq!(verifier.run(), Ok(()));
    }

    #[test]
    fn bad_instruction_format() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb0);
        let nullary_with_bad_opcode = func.dfg.make_inst(InstructionData::UnaryImm {
            opcode: Opcode::F32const,
            imm: 0.into(),
        });
        func.layout.append_inst(nullary_with_bad_opcode, ebb0);
        func.layout.append_inst(
            func.dfg.make_inst(InstructionData::Jump {
                opcode: Opcode::Jump,
                destination: ebb0,
                args: EntityList::default(),
            }),
            ebb0,
        );
        let flags = &settings::Flags::new(&settings::builder());
        let verifier = Verifier::new(&func, flags.into());
        assert_err_with_msg!(verifier.run(), "instruction format");
    }
}

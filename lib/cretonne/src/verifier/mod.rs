//! A verifier for ensuring that functions are well formed.
//! It verifies:
//!
//!   EBB integrity
//!
//!    - All instructions reached from the `ebb_insts` iterator must belong to
//!      the EBB as reported by `inst_ebb()`.
//!    - Every EBB must end in a terminator instruction, and no other instruction
//!      can be a terminator.
//!    - Every value in the `ebb_args` iterator belongs to the EBB as reported by `value_ebb`.
//!
//!   Instruction integrity
//!
//!    - The instruction format must match the opcode.
//!    - All result values must be created for multi-valued instructions.
//!    - All referenced entities must exist. (Values, EBBs, stack slots, ...)
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
//! TODO:
//!   Ad hoc checking
//!
//!    - Stack slot loads and stores must be in-bounds.
//!    - Immediate constraints for certain opcodes, like `udiv_imm v3, 0`.
//!    - Extend / truncate instructions have more type constraints: Source type can't be
//!      larger / smaller than result type.
//!    - `Insertlane` and `extractlane` instructions have immediate lane numbers that must be in
//!      range for their polymorphic type.
//!    - Swizzle and shuffle instructions take a variable number of lane arguments. The number
//!      of arguments must match the destination type, and the lane indexes must be in range.

use dominator_tree::DominatorTree;
use flowgraph::ControlFlowGraph;
use ir::entities::AnyEntity;
use ir::instructions::{InstructionFormat, BranchInfo, ResolvedConstraint, CallInfo};
use ir::{types, Function, ValueDef, Ebb, Inst, SigRef, FuncRef, ValueList, JumpTable, StackSlot,
         Value, Type, Opcode};
use isa::TargetIsa;
use std::error as std_error;
use std::fmt::{self, Display, Formatter};
use std::result;
use std::collections::BTreeSet;

pub use self::liveness::verify_liveness;
pub use self::cssa::verify_cssa;

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
mod liveness;

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
pub fn verify_function(func: &Function, isa: Option<&TargetIsa>) -> Result {
    Verifier::new(func, isa).run()
}

/// Verify `func` after checking the integrity of associated context data structures `cfg` and
/// `domtree`.
pub fn verify_context(func: &Function,
                      cfg: &ControlFlowGraph,
                      domtree: &DominatorTree,
                      isa: Option<&TargetIsa>)
                      -> Result {
    let verifier = Verifier::new(func, isa);
    verifier.cfg_integrity(cfg)?;
    verifier.domtree_integrity(domtree)?;
    verifier.run()
}

struct Verifier<'a> {
    func: &'a Function,
    cfg: ControlFlowGraph,
    domtree: DominatorTree,
    isa: Option<&'a TargetIsa>,
}

impl<'a> Verifier<'a> {
    pub fn new(func: &'a Function, isa: Option<&'a TargetIsa>) -> Verifier<'a> {
        let cfg = ControlFlowGraph::with_function(func);
        let domtree = DominatorTree::with_function(func, &cfg);
        Verifier {
            func,
            cfg,
            domtree,
            isa,
        }
    }

    fn ebb_integrity(&self, ebb: Ebb, inst: Inst) -> Result {

        let is_terminator = self.func.dfg[inst].opcode().is_terminator();
        let is_last_inst = self.func.layout.last_inst(ebb) == Some(inst);

        if is_terminator && !is_last_inst {
            // Terminating instructions only occur at the end of blocks.
            return err!(inst,
                        "a terminator instruction was encountered before the end of {}",
                        ebb);
        }
        if is_last_inst && !is_terminator {
            return err!(ebb, "block does not end in a terminator instruction!");
        }

        // Instructions belong to the correct ebb.
        let inst_ebb = self.func.layout.inst_ebb(inst);
        if inst_ebb != Some(ebb) {
            return err!(inst, "should belong to {} not {:?}", ebb, inst_ebb);
        }

        // Arguments belong to the correct ebb.
        for &arg in self.func.dfg.ebb_args(ebb) {
            match self.func.dfg.value_def(arg) {
                ValueDef::Arg(arg_ebb, _) => {
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
            .map(|sig| dfg.signatures[sig].return_types.len())
            .unwrap_or(0);
        let total_results = fixed_results + var_results;

        // All result values for multi-valued instructions are created
        let got_results = dfg.inst_results(inst).len();
        if got_results != total_results {
            return err!(inst,
                        "expected {} result values, found {}",
                        total_results,
                        got_results);
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
            StackLoad { stack_slot, .. } |
            StackStore { stack_slot, .. } => {
                self.verify_stack_slot(inst, stack_slot)?;
            }

            // Exhaustive list so we can't forget to add new formats
            Nullary { .. } |
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
            FloatCompare { .. } |
            HeapLoad { .. } |
            HeapStore { .. } |
            Load { .. } |
            Store { .. } |
            RegMove { .. } => {}
        }

        Ok(())
    }

    fn verify_ebb(&self, inst: Inst, e: Ebb) -> Result {
        if !self.func.dfg.ebb_is_valid(e) {
            err!(inst, "invalid ebb reference {}", e)
        } else {
            Ok(())
        }
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
            ValueDef::Res(def_inst, _) => {
                // Value is defined by an instruction that exists.
                if !dfg.inst_is_valid(def_inst) {
                    return err!(loc_inst,
                                "{} is defined by invalid instruction {}",
                                v,
                                def_inst);
                }
                // Defining instruction is inserted in an EBB.
                if self.func.layout.inst_ebb(def_inst) == None {
                    return err!(loc_inst,
                                "{} is defined by {} which has no EBB",
                                v,
                                def_inst);
                }
                // Defining instruction dominates the instruction that uses the value.
                if self.domtree.is_reachable(self.func.layout.pp_ebb(loc_inst)) &&
                   !self.domtree
                        .dominates(def_inst, loc_inst, &self.func.layout) {
                    return err!(loc_inst, "uses value from non-dominating {}", def_inst);
                }
            }
            ValueDef::Arg(ebb, _) => {
                // Value is defined by an existing EBB.
                if !dfg.ebb_is_valid(ebb) {
                    return err!(loc_inst, "{} is defined by invalid EBB {}", v, ebb);
                }
                // Defining EBB is inserted in the layout
                if !self.func.layout.is_ebb_inserted(ebb) {
                    return err!(loc_inst,
                                "{} is defined by {} which is not in the layout",
                                v,
                                ebb);
                }
                // The defining EBB dominates the instruction using this value.
                if self.domtree.is_reachable(ebb) &&
                   !self.domtree.dominates(ebb, loc_inst, &self.func.layout) {
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
            let expected = domtree.idom(ebb);
            let got = self.domtree.idom(ebb);
            if got != expected {
                return err!(ebb,
                            "invalid domtree, expected idom({}) = {:?}, got {:?}",
                            ebb,
                            expected,
                            got);
            }
        }
        Ok(())
    }

    fn typecheck_entry_block_arguments(&self) -> Result {
        if let Some(ebb) = self.func.layout.entry_block() {
            let expected_types = &self.func.signature.argument_types;
            let ebb_arg_count = self.func.dfg.num_ebb_args(ebb);

            if ebb_arg_count != expected_types.len() {
                return err!(ebb, "entry block arguments must match function signature");
            }

            for (i, &arg) in self.func.dfg.ebb_args(ebb).iter().enumerate() {
                let arg_type = self.func.dfg.value_type(arg);
                if arg_type != expected_types[i].value_type {
                    return err!(ebb,
                                "entry block argument {} expected to have type {}, got {}",
                                i,
                                expected_types[i],
                                arg_type);
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

        Ok(())
    }

    fn typecheck_results(&self, inst: Inst, ctrl_type: Type) -> Result {
        let mut i = 0;
        for &result in self.func.dfg.inst_results(inst) {
            let result_type = self.func.dfg.value_type(result);
            let expected_type = self.func.dfg.compute_result_type(inst, i, ctrl_type);
            if let Some(expected_type) = expected_type {
                if result_type != expected_type {
                    return err!(inst,
                                "expected result {} ({}) to have type {}, found {}",
                                i,
                                result,
                                expected_type,
                                result_type);
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
                        return err!(inst,
                                    "arg {} ({}) has type {}, expected {}",
                                    i,
                                    arg,
                                    arg_type,
                                    expected_type);
                    }
                }
                ResolvedConstraint::Free(type_set) => {
                    if !type_set.contains(arg_type) {
                        return err!(inst,
                                    "arg {} ({}) with type {} failed to satisfy type set {:?}",
                                    i,
                                    arg,
                                    arg_type,
                                    type_set);
                    }
                }
            }
        }
        Ok(())
    }

    fn typecheck_variable_args(&self, inst: Inst) -> Result {
        match self.func.dfg[inst].analyze_branch(&self.func.dfg.value_lists) {
            BranchInfo::SingleDest(ebb, _) => {
                let iter = self.func
                    .dfg
                    .ebb_args(ebb)
                    .iter()
                    .map(|&v| self.func.dfg.value_type(v));
                self.typecheck_variable_args_iterator(inst, iter)?;
            }
            BranchInfo::Table(table) => {
                for (_, ebb) in self.func.jump_tables[table].entries() {
                    let arg_count = self.func.dfg.num_ebb_args(ebb);
                    if arg_count != 0 {
                        return err!(inst,
                                    "takes no arguments, but had target {} with {} arguments",
                                    ebb,
                                    arg_count);
                    }
                }
            }
            BranchInfo::NotABranch => {}
        }

        match self.func.dfg[inst].analyze_call(&self.func.dfg.value_lists) {
            CallInfo::Direct(func_ref, _) => {
                let sig_ref = self.func.dfg.ext_funcs[func_ref].signature;
                let arg_types = self.func.dfg.signatures[sig_ref]
                    .argument_types
                    .iter()
                    .map(|a| a.value_type);
                self.typecheck_variable_args_iterator(inst, arg_types)?;
            }
            CallInfo::Indirect(sig_ref, _) => {
                let arg_types = self.func.dfg.signatures[sig_ref]
                    .argument_types
                    .iter()
                    .map(|a| a.value_type);
                self.typecheck_variable_args_iterator(inst, arg_types)?;
            }
            CallInfo::NotACall => {}
        }
        Ok(())
    }

    fn typecheck_variable_args_iterator<I: Iterator<Item = Type>>(&self,
                                                                  inst: Inst,
                                                                  iter: I)
                                                                  -> Result {
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
                return err!(inst,
                            "arg {} ({}) has type {}, expected {}",
                            i,
                            variable_args[i],
                            arg_type,
                            expected_type);
            }
            i += 1;
        }
        if i != variable_args.len() {
            return err!(inst,
                        "mismatched argument count, got {}, expected {}",
                        variable_args.len(),
                        i);
        }
        Ok(())
    }

    fn typecheck_return(&self, inst: Inst) -> Result {
        if self.func.dfg[inst].opcode().is_return() {
            let args = self.func.dfg.inst_variable_args(inst);
            let expected_types = &self.func.signature.return_types;
            if args.len() != expected_types.len() {
                return err!(inst, "arguments of return must match function signature");
            }
            for (i, (&arg, &expected_type)) in args.iter().zip(expected_types).enumerate() {
                let arg_type = self.func.dfg.value_type(arg);
                if arg_type != expected_type.value_type {
                    return err!(inst,
                                "arg {} ({}) has type {}, must match function signature of {}",
                                i,
                                arg,
                                arg_type,
                                expected_type);
                }
            }
        }
        Ok(())
    }

    fn cfg_integrity(&self, cfg: &ControlFlowGraph) -> Result {
        let mut expected_succs = BTreeSet::<Ebb>::new();
        let mut got_succs = BTreeSet::<Ebb>::new();
        let mut expected_preds = BTreeSet::<Inst>::new();
        let mut got_preds = BTreeSet::<Inst>::new();

        for ebb in self.func.layout.ebbs() {
            expected_succs.extend(self.cfg.get_successors(ebb));
            got_succs.extend(cfg.get_successors(ebb));

            let missing_succs: Vec<Ebb> = expected_succs.difference(&got_succs).cloned().collect();
            if !missing_succs.is_empty() {
                return err!(ebb,
                            "cfg lacked the following successor(s) {:?}",
                            missing_succs);
            }

            let excess_succs: Vec<Ebb> = got_succs.difference(&expected_succs).cloned().collect();
            if !excess_succs.is_empty() {
                return err!(ebb, "cfg had unexpected successor(s) {:?}", excess_succs);
            }

            expected_preds.extend(self.cfg.get_predecessors(ebb).iter().map(|&(_, inst)| inst));
            got_preds.extend(cfg.get_predecessors(ebb).iter().map(|&(_, inst)| inst));

            let missing_preds: Vec<Inst> = expected_preds.difference(&got_preds).cloned().collect();
            if !missing_preds.is_empty() {
                return err!(ebb,
                            "cfg lacked the following predecessor(s) {:?}",
                            missing_preds);
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

        let encoding = self.func.encodings.get_or_default(inst);
        if encoding.is_legal() {
            let verify_encoding =
                isa.encode(&self.func.dfg,
                           &self.func.dfg[inst],
                           self.func.dfg.ctrl_typevar(inst));
            match verify_encoding {
                Ok(verify_encoding) => {
                    if verify_encoding != encoding {
                        return err!(inst,
                                    "Instruction re-encoding {} doesn't match {}",
                                    isa.encoding_info().display(verify_encoding),
                                    isa.encoding_info().display(encoding));
                    }
                }
                Err(_) => {
                    return err!(inst,
                                "Instruction failed to re-encode {}",
                                isa.encoding_info().display(encoding))
                }
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

        if opcode.is_branch() {
            return err!(inst, "Branch must have an encoding");
        }

        if opcode.is_call() {
            return err!(inst, "Call must have an encoding");
        }

        if opcode.is_return() {
            return err!(inst, "Return must have an encoding");
        }

        if opcode.can_store() {
            return err!(inst, "Store must have an encoding");
        }

        if opcode.can_trap() {
            return err!(inst, "Trapping instruction must have an encoding");
        }

        if opcode.other_side_effects() {
            return err!(inst, "Instruction with side effects must have an encoding");
        }

        Ok(())
    }

    pub fn run(&self) -> Result {
        self.typecheck_entry_block_arguments()?;
        for ebb in self.func.layout.ebbs() {
            for inst in self.func.layout.ebb_insts(ebb) {
                self.ebb_integrity(ebb, inst)?;
                self.instruction_integrity(inst)?;
                self.typecheck(inst)?;
                self.verify_encoding(inst)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Verifier, Error};
    use ir::Function;
    use ir::instructions::{InstructionData, Opcode};

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
        let verifier = Verifier::new(&func, None);
        assert_eq!(verifier.run(), Ok(()));
    }

    #[test]
    fn bad_instruction_format() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb0);
        let nullary_with_bad_opcode =
            func.dfg
                .make_inst(InstructionData::Nullary { opcode: Opcode::Jump });
        func.layout.append_inst(nullary_with_bad_opcode, ebb0);
        let verifier = Verifier::new(&func, None);
        assert_err_with_msg!(verifier.run(), "instruction format");
    }
}

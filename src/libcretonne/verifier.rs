//! A verifier for ensuring that functions are well formed.
//! It verifies:
//!
//!   EBB integrity
//!
//!    - All instructions reached from the ebb_insts iterator must belong to
//!      the EBB as reported by inst_ebb().
//!    - Every EBB must end in a terminator instruction, and no other instruction
//!      can be a terminator.
//!    - Every value in the ebb_args iterator belongs to the EBB as reported by value_ebb.
//!
//!   Instruction integrity
//!
//!    - The instruction format must match the opcode.
//! TODO:
//!    - All result values must be created for multi-valued instructions.
//!    - Instructions with no results must have a VOID first_type().
//!    - All referenced entities must exist. (Values, EBBs, stack slots, ...)
//!
//!   SSA form
//!
//!    - Values must be defined by an instruction that exists and that is inserted in
//!      an EBB, or be an argument of an existing EBB.
//!    - Values used by an instruction must dominate the instruction.
//!     Control flow graph and dominator tree integrity:
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
//!      expected types excatly. The number of arguments must match.
//!    - All EBBs in a jump_table must take no arguments.
//!    - Function calls are type checked against their signature.
//!    - The entry block must take arguments that match the signature of the current
//!      function.
//!    - All return instructions must have return value operands matching the current
//!      function signature.
//!
//!   Ad hoc checking
//!
//!    - Stack slot loads and stores must be in-bounds.
//!    - Immediate constraints for certain opcodes, like udiv_imm v3, 0.
//!    - Extend / truncate instructions have more type constraints: Source type can't be
//!      larger / smaller than result type.
//!    - Insertlane and extractlane instructions have immediate lane numbers that must be in
//!      range for their polymorphic type.
//!    - Swizzle and shuffle instructions take a variable number of lane arguments. The number
//!      of arguments must match the destination type, and the lane indexes must be in range.

use ir::{Function, ValueDef, Ebb, Inst};
use ir::instructions::InstructionFormat;

pub fn verify_function(func: &Function) -> Result<(), String> {
    Verifier::new(func).run()
}

pub struct Verifier<'a> {
    func: &'a Function,
}

impl<'a> Verifier<'a> {
    pub fn new(func: &'a Function) -> Verifier {
        Verifier { func: func }
    }

    fn ebb_integrity(&self, ebb: Ebb, inst: Inst) -> Result<(), String> {

        let is_terminator = self.func.dfg[inst].is_terminating();
        let is_last_inst = self.func.layout.last_inst(ebb) == inst;

        if is_terminator && !is_last_inst {
            // Terminating instructions only occur at the end of blocks.
            return Err(format!("A terminating instruction was encountered before the \
                                end of ebb {:?}!",
                               ebb));
        }
        if is_last_inst && !is_terminator {
            return Err(format!("Block {:?} does not end in a terminating instruction!", ebb));
        }

        // Instructions belong to the correct ebb.
        let inst_ebb = self.func.layout.inst_ebb(inst);
        if inst_ebb != Some(ebb) {
            return Err(format!("{:?} should belong to {:?} not {:?}", inst, ebb, inst_ebb));
        }

        // Arguments belong to the correct ebb.
        for arg in self.func.dfg.ebb_args(ebb) {
            match self.func.dfg.value_def(arg) {
                ValueDef::Arg(arg_ebb, _) => {
                    if ebb != arg_ebb {
                        return Err(format!("{:?} does not belong to {:?}", arg, ebb));
                    }
                }
                _ => {
                    return Err("Expected an argument, found a result!".to_string());
                }
            }
        }

        Ok(())
    }

    fn instruction_integrity(&self, inst: Inst) -> Result<(), String> {
        let inst_data = &self.func.dfg[inst];

        // The instruction format matches the opcode
        if inst_data.opcode().format() != Some(InstructionFormat::from(inst_data)) {
            return Err("Instruction opcode doesn't match instruction format!".to_string());
        }

        Ok(())
    }

    pub fn run(&self) -> Result<(), String> {
        for ebb in self.func.layout.ebbs() {
            for inst in self.func.layout.ebb_insts(ebb) {
                try!(self.ebb_integrity(ebb, inst));
                try!(self.instruction_integrity(inst));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate regex;

    use super::*;
    use ir::Function;
    use ir::instructions::{InstructionData, Opcode};
    use ir::types;
    use self::regex::Regex;

    macro_rules! assert_err_with_msg {
        ($e:expr, $msg:expr) => (
            let err_re = Regex::new($msg).unwrap();
            match $e {
                Ok(_) => { panic!("Expected an error!") },
                Err(err_msg) => {
                    if !err_re.is_match(&err_msg) {
                       panic!(format!("'{}' did not contain the pattern '{}'", err_msg, $msg));
                    }
                }
            }
        )
    }

    #[test]
    fn empty() {
        let func = Function::new();
        let verifier = Verifier::new(&func);
        assert_eq!(verifier.run(), Ok(()));
    }

    #[test]
    fn bad_instruction_format() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb0);
        let nullary_with_bad_opcode = func.dfg.make_inst(InstructionData::Nullary {
            opcode: Opcode::Jump,
            ty: types::VOID,
        });
        func.layout.append_inst(nullary_with_bad_opcode, ebb0);
        let verifier = Verifier::new(&func);
        assert_err_with_msg!(verifier.run(), "instruction format");
    }
}

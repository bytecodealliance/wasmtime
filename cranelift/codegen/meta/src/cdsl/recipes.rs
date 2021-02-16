use std::rc::Rc;

use cranelift_entity::{entity_impl, PrimaryMap};

use crate::cdsl::formats::InstructionFormat;
use crate::cdsl::instructions::InstructionPredicate;
use crate::cdsl::regs::RegClassIndex;
use crate::cdsl::settings::SettingPredicateNumber;

/// A specific register in a register class.
///
/// A register is identified by the top-level register class it belongs to and
/// its first register unit.
///
/// Specific registers are used to describe constraints on instructions where
/// some operands must use a fixed register.
///
/// Register instances can be created with the constructor, or accessed as
/// attributes on the register class: `GPR.rcx`.
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) struct Register {
    pub regclass: RegClassIndex,
    pub unit: u8,
}

impl Register {
    pub fn new(regclass: RegClassIndex, unit: u8) -> Self {
        Self { regclass, unit }
    }
}

/// An operand that must be in a stack slot.
///
/// A `Stack` object can be used to indicate an operand constraint for a value
/// operand that must live in a stack slot.
#[derive(Copy, Clone, Hash, PartialEq)]
pub(crate) struct Stack {
    pub regclass: RegClassIndex,
}

impl Stack {
    pub fn new(regclass: RegClassIndex) -> Self {
        Self { regclass }
    }
    pub fn stack_base_mask(self) -> &'static str {
        // TODO: Make this configurable instead of just using the SP.
        "StackBaseMask(1)"
    }
}

#[derive(Clone, Hash, PartialEq)]
pub(crate) struct BranchRange {
    pub inst_size: u64,
    pub range: u64,
}

#[derive(Copy, Clone, Hash, PartialEq)]
pub(crate) enum OperandConstraint {
    RegClass(RegClassIndex),
    FixedReg(Register),
    TiedInput(usize),
    Stack(Stack),
}

impl Into<OperandConstraint> for RegClassIndex {
    fn into(self) -> OperandConstraint {
        OperandConstraint::RegClass(self)
    }
}

impl Into<OperandConstraint> for Register {
    fn into(self) -> OperandConstraint {
        OperandConstraint::FixedReg(self)
    }
}

impl Into<OperandConstraint> for usize {
    fn into(self) -> OperandConstraint {
        OperandConstraint::TiedInput(self)
    }
}

impl Into<OperandConstraint> for Stack {
    fn into(self) -> OperandConstraint {
        OperandConstraint::Stack(self)
    }
}

/// A recipe for encoding instructions with a given format.
///
/// Many different instructions can be encoded by the same recipe, but they
/// must all have the same instruction format.
///
/// The `operands_in` and `operands_out` arguments are tuples specifying the register
/// allocation constraints for the value operands and results respectively. The
/// possible constraints for an operand are:
///
/// - A `RegClass` specifying the set of allowed registers.
/// - A `Register` specifying a fixed-register operand.
/// - An integer indicating that this result is tied to a value operand, so
///   they must use the same register.
/// - A `Stack` specifying a value in a stack slot.
///
/// The `branch_range` argument must be provided for recipes that can encode
/// branch instructions. It is an `(origin, bits)` tuple describing the exact
/// range that can be encoded in a branch instruction.
#[derive(Clone)]
pub(crate) struct EncodingRecipe {
    /// Short mnemonic name for this recipe.
    pub name: String,

    /// Associated instruction format.
    pub format: Rc<InstructionFormat>,

    /// Base number of bytes in the binary encoded instruction.
    pub base_size: u64,

    /// Tuple of register constraints for value operands.
    pub operands_in: Vec<OperandConstraint>,

    /// Tuple of register constraints for results.
    pub operands_out: Vec<OperandConstraint>,

    /// Function name to use when computing actual size.
    pub compute_size: &'static str,

    /// `(origin, bits)` range for branches.
    pub branch_range: Option<BranchRange>,

    /// This instruction clobbers `iflags` and `fflags`; true by default.
    pub clobbers_flags: bool,

    /// Instruction predicate.
    pub inst_predicate: Option<InstructionPredicate>,

    /// ISA predicate.
    pub isa_predicate: Option<SettingPredicateNumber>,

    /// Rust code for binary emission.
    pub emit: Option<String>,
}

// Implement PartialEq ourselves: take all the fields into account but the name.
impl PartialEq for EncodingRecipe {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.format, &other.format)
            && self.base_size == other.base_size
            && self.operands_in == other.operands_in
            && self.operands_out == other.operands_out
            && self.compute_size == other.compute_size
            && self.branch_range == other.branch_range
            && self.clobbers_flags == other.clobbers_flags
            && self.inst_predicate == other.inst_predicate
            && self.isa_predicate == other.isa_predicate
            && self.emit == other.emit
    }
}

// To allow using it in a hashmap.
impl Eq for EncodingRecipe {}

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct EncodingRecipeNumber(u32);
entity_impl!(EncodingRecipeNumber);

pub(crate) type Recipes = PrimaryMap<EncodingRecipeNumber, EncodingRecipe>;

#[derive(Clone)]
pub(crate) struct EncodingRecipeBuilder {
    pub name: String,
    format: Rc<InstructionFormat>,
    pub base_size: u64,
    pub operands_in: Option<Vec<OperandConstraint>>,
    pub operands_out: Option<Vec<OperandConstraint>>,
    pub compute_size: Option<&'static str>,
    pub branch_range: Option<BranchRange>,
    pub emit: Option<String>,
    clobbers_flags: Option<bool>,
    inst_predicate: Option<InstructionPredicate>,
    isa_predicate: Option<SettingPredicateNumber>,
}

impl EncodingRecipeBuilder {
    pub fn new(name: impl Into<String>, format: &Rc<InstructionFormat>, base_size: u64) -> Self {
        Self {
            name: name.into(),
            format: format.clone(),
            base_size,
            operands_in: None,
            operands_out: None,
            compute_size: None,
            branch_range: None,
            emit: None,
            clobbers_flags: None,
            inst_predicate: None,
            isa_predicate: None,
        }
    }

    // Setters.
    pub fn operands_in(mut self, constraints: Vec<impl Into<OperandConstraint>>) -> Self {
        assert!(self.operands_in.is_none());
        self.operands_in = Some(
            constraints
                .into_iter()
                .map(|constr| constr.into())
                .collect(),
        );
        self
    }
    pub fn operands_out(mut self, constraints: Vec<impl Into<OperandConstraint>>) -> Self {
        assert!(self.operands_out.is_none());
        self.operands_out = Some(
            constraints
                .into_iter()
                .map(|constr| constr.into())
                .collect(),
        );
        self
    }
    pub fn clobbers_flags(mut self, flag: bool) -> Self {
        assert!(self.clobbers_flags.is_none());
        self.clobbers_flags = Some(flag);
        self
    }
    pub fn emit(mut self, code: impl Into<String>) -> Self {
        assert!(self.emit.is_none());
        self.emit = Some(code.into());
        self
    }
    pub fn branch_range(mut self, range: (u64, u64)) -> Self {
        assert!(self.branch_range.is_none());
        self.branch_range = Some(BranchRange {
            inst_size: range.0,
            range: range.1,
        });
        self
    }
    pub fn isa_predicate(mut self, pred: SettingPredicateNumber) -> Self {
        assert!(self.isa_predicate.is_none());
        self.isa_predicate = Some(pred);
        self
    }
    pub fn inst_predicate(mut self, inst_predicate: impl Into<InstructionPredicate>) -> Self {
        assert!(self.inst_predicate.is_none());
        self.inst_predicate = Some(inst_predicate.into());
        self
    }
    pub fn compute_size(mut self, compute_size: &'static str) -> Self {
        assert!(self.compute_size.is_none());
        self.compute_size = Some(compute_size);
        self
    }

    pub fn build(self) -> EncodingRecipe {
        let operands_in = self.operands_in.unwrap_or_default();
        let operands_out = self.operands_out.unwrap_or_default();

        // The number of input constraints must match the number of format input operands.
        if !self.format.has_value_list {
            assert!(
                operands_in.len() == self.format.num_value_operands,
                "missing operand constraints for recipe {} (format {})",
                self.name,
                self.format.name
            );
        }

        // Ensure tied inputs actually refer to existing inputs.
        for constraint in operands_in.iter().chain(operands_out.iter()) {
            if let OperandConstraint::TiedInput(n) = *constraint {
                assert!(n < operands_in.len());
            }
        }

        let compute_size = match self.compute_size {
            Some(compute_size) => compute_size,
            None => "base_size",
        };

        let clobbers_flags = self.clobbers_flags.unwrap_or(true);

        EncodingRecipe {
            name: self.name,
            format: self.format,
            base_size: self.base_size,
            operands_in,
            operands_out,
            compute_size,
            branch_range: self.branch_range,
            clobbers_flags,
            inst_predicate: self.inst_predicate,
            isa_predicate: self.isa_predicate,
            emit: self.emit,
        }
    }
}

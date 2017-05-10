//! Constraint solver for register coloring.
//!
//! The coloring phase of SSA-based register allocation is very simple in theory, but in practice
//! it is complicated by the various constraints imposed by individual instructions:
//!
//! - Call and return instructions have to satisfy ABI requirements for arguments and return
//!   values.
//! - Values live across a call must be in a callee-saved register.
//! - Some instructions have operand constraints such as register sub-classes, fixed registers, or
//!   tied operands.
//!
//! # The instruction register coloring problem
//!
//! The constraint solver addresses the problem of satisfying the constraints of a single
//! instruction. We have:
//!
//! - A set of values that are live in registers before the instruction, with current register
//!   assignments. Some are used by the instruction, some are not.
//! - A subset of the live register values that are killed by the instruction.
//! - A set of new register values that are defined by the instruction.
//! The constraint solver addresses the problem of satisfying the constraints of a single
//! instruction. We have:
//!
//! - A set of values that are live in registers before the instruction, with current register
//!   assignments. Some are used by the instruction, some are not.
//! - A subset of the live register values that are killed by the instruction.
//! - A set of new register values that are defined by the instruction.
//!
//! We are not concerned with stack values at all. The reload pass ensures that all values required
//! to be in a register by the instruction are already in a register.
//!
//! A solution to the register coloring problem consists of:
//!
//! - Register reassignment prescriptions for a subset of the live register values.
//! - Register assignments for the defined values.
//!
//! The solution ensures that when live registers are reassigned as prescribed before the
//! instruction, all its operand constraints are satisfied, and the definition assignments won't
//! conflict.
//!
//! # Register diversions and global interference
//!
//! We can divert register values temporarily to satisfy constraints, but we need to put the
//! values back into their originally assigned register locations before leaving the EBB.
//! Otherwise, values won't be in the right register at the entry point of other EBBs.
//!
//! Some values are *local*, and we don't need to worry about putting those values back since they
//! are not used in any other EBBs.
//!
//! When we assign register locations to defines, we are assigning both the register used locally
//! immediately after the instruction and the register used globally when the defined value is used
//! in a different EBB. We need to avoid interference both locally at the instruction and globally.
//!
//! We have multiple mappings of values to registers:
//!
//! 1. The initial local mapping before the instruction. This includes any diversions from previous
//!    instructions in the EBB, but not diversions for the current instruction.
//! 2. The local mapping after applying the additional reassignments required to satisfy the
//!    constraints of the current instruction.
//! 3. The local mapping after the instruction. This excludes values killed by the instruction and
//!    includes values defined by the instruction.
//! 4. The global mapping after the instruction. This mapping only contains values with global live
//!    ranges, and it does not include any diversions.
//!
//! All four mappings must be kept free of interference.
//!
//! # Problems handled by previous passes.
//!
//! The constraint solver can only reassign registers, it can't create spill code, so some
//! constraints are handled by earlier passes:
//!
//! - There will be enough free registers available for the defines. Ensuring this is the primary
//!   purpose of the spilling phase.
//! - When the same value is used for multiple operands, the intersection of operand constraints is
//!   non-empty. The spilling phase will insert copies to handle mutually incompatible constraints,
//!   such as when the same value is bound to two different function arguments.
//! - Values bound to tied operands must be killed by the instruction. Also enforced by the
//!   spiller.
//! - Values used by register operands are in registers, and values used by stack operands are in
//!   stack slots. This is enforced by the reload pass.
//!
//! # Solver algorithm
//!
//! The goal of the solver is to satisfy the instruction constraints with a minimal number of
//! register assignments before the instruction.
//!
//! 1. Compute the set of values used by operands with a fixed register constraint that isn't
//!    already satisfied. These are mandatory predetermined reassignments.
//! 2. Compute the set of values that don't satisfy their register class constraint. These are
//!    mandatory reassignments that we need to solve.
//! 3. Add the set of defines to the set of variables computed in 2. Exclude defines tied to an
//!    input operand since their value is pre-determined.
//!
//! The set of values computed in 2. and 3. are the *variables* for the solver. Given a set of
//! variables, we can also compute a set of allocatable registers by removing the variables from
//! the set of assigned registers before the instruction.
//!
//! 1. For each variable, compute its domain as the intersection of the allocatable registers and
//!    its register class constraint.
//! 2. Sort the variables in order of increasing domain size.
//! 3. Search for a solution that assigns each variable a register from its domain without
//!    interference between variables.
//!
//! If the search fails to find a solution, we may need to reassign more registers. Find an
//! appropriate candidate among the set of live register values, add it as a variable and start
//! over.

use ir::Value;
use isa::{RegInfo, RegClass, RegUnit};
use regalloc::allocatable_set::RegSetIter;
use sparse_map::{SparseMap, SparseMapValue};
use std::fmt;
use super::AllocatableSet;

/// A variable in the constraint problem.
///
/// Variables represent register values that can be assigned to any register unit within the
/// constraint register class. This includes live register values that can be reassigned to a new
/// register and values defined by the instruction which must be assigned to a register.
///
/// Besides satisfying the register class constraint, variables must also be mutually
/// non-interfering in up to three contexts:
///
/// 1. Input side live registers, after applying all the reassignments.
/// 2. Output side live registers, considering all the local register diversions.
/// 3. Global live register, not considering any local diversions.
///
pub struct Variable {
    /// The value whose register assignment we're looking for.
    pub value: Value,

    /// Original register unit holding this live value before the instruction, or `None` for a
    /// value that is defined by the instruction.
    from: Option<RegUnit>,

    /// Avoid interference on the input side.
    is_input: bool,

    /// Avoid interference on the output side.
    is_output: bool,

    /// Avoid interference with the global registers.
    is_global: bool,

    /// Number of registers available in the domain of this variable.
    domain: u16,

    /// The assigned register unit after a full solution was found.
    pub solution: RegUnit,

    /// Any solution must belong to the constraint register class.
    constraint: RegClass,
}

impl Variable {
    fn new_live(value: Value, constraint: RegClass, from: RegUnit) -> Variable {
        Variable {
            value,
            constraint,
            from: Some(from),
            is_input: true,
            is_output: true,
            is_global: false,
            domain: 0,
            solution: !0,
        }
    }

    fn new_def(value: Value, constraint: RegClass) -> Variable {
        Variable {
            value,
            constraint,
            from: None,
            is_input: false,
            is_output: true,
            is_global: false,
            domain: 0,
            solution: !0,
        }
    }

    /// Does this variable represent a value defined by the current instruction?
    pub fn is_define(&self) -> bool {
        self.from.is_none()
    }

    /// Get an iterator over possible register choices, given the available registers on the input
    /// and output sides respectively.
    fn iter(&self, iregs: &AllocatableSet, oregs: &AllocatableSet) -> RegSetIter {
        if self.is_input && self.is_output {
            let mut r = iregs.clone();
            r.intersect(oregs);
            r.iter(self.constraint)
        } else if self.is_input {
            iregs.iter(self.constraint)
        } else {
            oregs.iter(self.constraint)
        }
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({}", self.value, self.constraint)?;
        if self.is_input {
            write!(f, ", in")?;
        }
        if self.is_output {
            write!(f, ", out")?;
        }
        if self.is_global {
            write!(f, ", global")?;
        }
        if self.is_define() {
            write!(f, ", def")?;
        }
        if self.domain > 0 {
            write!(f, ", {}", self.domain)?;
        }
        write!(f, ")")
    }
}


struct Assignment {
    value: Value,
    from: RegUnit,
    to: RegUnit,
    rc: RegClass,
}

impl SparseMapValue<Value> for Assignment {
    fn key(&self) -> Value {
        self.value
    }
}

/// Constraint solver for register allocation around a single instruction.
///
/// Start by programming in the instruction constraints.
///
/// 1. Initialize the solver by calling `reset()` with the set of allocatable registers before the
///    instruction.
/// 2. Program the input side constraints: Call `reassign_in()` for all fixed register constraints,
///    and `add_var()` for any input operands whose constraints are not already satisfied.
/// 3. Check for conflicts between fixed input assignments and existing live values by calling
///    `has_fixed_input_conflicts()`. Resolve any conflicts by calling `add_var()` with the
///    conflicting values.
/// 4. Prepare for adding output side constraints by calling `inputs_done()`.
/// 5. Add any killed register values that no longer cause interference on the output side by
///    calling `add_kill()`.
/// 6. Program the output side constraints: Call `add_fixed_output()` for all fixed register
///    constraints and `add_def()` for free defines. Resolve fixed output conflicts by calling
///    `add_var()`.
///
pub struct Solver {
    /// Register reassignments that are required or decided as part of a full solution.
    assignments: SparseMap<Value, Assignment>,

    /// Variables are the values that should be reassigned as part of a solution.
    /// Values with a fixed register constraints are not considered variables. They are represented
    /// in the `assignments` vector if necessary.
    vars: Vec<Variable>,

    /// Are we finished adding input-side constraints? This changes the meaning of the `regs_in`
    /// and `regs_out` register sets.
    inputs_done: bool,

    /// Available registers on the input side of the instruction.
    ///
    /// While we're adding input constraints (`!inputs_done`):
    ///
    /// - Live values on the input side are marked as unavailable.
    /// - The 'from' registers of fixed input reassignments are marked as available as they are
    ///   added.
    /// - Input-side variables are marked as available.
    ///
    /// After finishing input constraints (`inputs_done`):
    ///
    /// - Live values on the input side are marked as unavailable.
    /// - The 'to' registers of fixed input reassignments are marked as unavailable.
    /// - Input-side variables are marked as available.
    ///
    regs_in: AllocatableSet,

    /// Available registers on the output side of the instruction / fixed input scratch space.
    ///
    /// While we're adding input constraints (`!inputs_done`):
    ///
    /// - The 'to' registers of fixed input reassignments are marked as unavailable.
    ///
    /// After finishing input constraints (`inputs_done`):
    ///
    /// - Live-through values are marked as unavailable.
    /// - Fixed output assignments are marked as unavailable.
    /// - Live-through variables are marked as available.
    ///
    regs_out: AllocatableSet,
}

/// Interface for programming the constraints into the solver.
impl Solver {
    /// Create a new empty solver.
    pub fn new() -> Solver {
        Solver {
            assignments: SparseMap::new(),
            vars: Vec::new(),
            inputs_done: false,
            regs_in: AllocatableSet::new(),
            regs_out: AllocatableSet::new(),
        }
    }

    /// Reset the solver state and prepare solving for a new instruction with an initial set of
    /// allocatable registers.
    ///
    /// The `regs` set is the allocatable registers before any reassignments are applied.
    pub fn reset(&mut self, regs: &AllocatableSet) {
        self.assignments.clear();
        self.vars.clear();
        self.inputs_done = false;
        self.regs_in = regs.clone();
        // Used for tracking fixed input assignments while `!inputs_done`:
        self.regs_out = AllocatableSet::new();
    }

    /// Add a fixed input reassignment of `value`.
    ///
    /// This means that `value` must be assigned to `to` and can't become a variable. Call with
    /// `from == to` to ensure that `value` is not reassigned from its existing register location.
    ///
    /// In either case, `to` will not be available for variables on the input side of the
    /// instruction.
    pub fn reassign_in(&mut self, value: Value, rc: RegClass, from: RegUnit, to: RegUnit) {
        debug_assert!(!self.inputs_done);
        if self.regs_in.is_avail(rc, from) {
            // It looks like `value` was already removed from the register set. It must have been
            // added as a variable previously. A fixed constraint beats a variable, so convert it.
            if let Some(idx) = self.vars.iter().position(|v| v.value == value) {
                let v = self.vars.remove(idx);
                dbg!("Converting variable {} to a fixed constraint", v);
                // The spiller is responsible for ensuring that all constraints on the uses of a
                // value are compatible.
                assert!(v.constraint.contains(to),
                        "Incompatible constraints for {}",
                        value);
            } else {
                panic!("Invalid from register for fixed {} constraint", value);
            }
        }
        self.regs_in.free(rc, from);
        self.regs_out.take(rc, to);
        self.assignments
            .insert(Assignment {
                        value,
                        rc,
                        from,
                        to,
                    });
    }

    /// Add a variable representing an input side value with an existing register assignment.
    ///
    /// A variable is a value that should be reassigned to something in the `constraint` register
    /// class.
    ///
    /// It is assumed initially that the value is also live on the output side of the instruction.
    /// This can be changed by calling to `add_kill()`.
    pub fn add_var(&mut self,
                   value: Value,
                   constraint: RegClass,
                   from: RegUnit,
                   reginfo: &RegInfo) {
        // Check for existing entries for this value.
        if self.regs_in.is_avail(constraint, from) {
            // There cold be an existing variable entry.
            if let Some(v) = self.vars.iter_mut().find(|v| v.value == value) {
                // We have an existing variable entry for `value`. Combine the constraints.
                if let Some(rci) = v.constraint.intersect(constraint) {
                    v.constraint = reginfo.rc(rci);
                    return;
                } else {
                    // The spiller should have made sure the same value is not used with disjoint
                    // constraints.
                    panic!("Incompatible constraints: {} + {}", constraint, *v)
                }
            }

            // No variable, then it must be a fixed reassignment.
            if let Some(a) = self.assignments.get(value) {
                assert!(constraint.contains(a.to),
                        "Incompatible constraints for {}",
                        value);
                return;
            }

            panic!("Wrong from register for {}", value);
        }
        self.regs_in.free(constraint, from);
        if self.inputs_done {
            self.regs_out.free(constraint, from);
        }
        self.vars
            .push(Variable::new_live(value, constraint, from));
    }

    /// Check for conflicts between fixed input assignments and existing live values.
    ///
    /// Returns true if one of the live values conflicts with a fixed input assignment. Such a
    /// conflicting value must be turned into a variable.
    pub fn has_fixed_input_conflicts(&self) -> bool {
        debug_assert!(!self.inputs_done);
        // The `from` side of the fixed input diversions are taken from `regs_out`.
        self.regs_out.interferes_with(&self.regs_in)
    }

    /// Check if `rc, reg` specifically conflicts with the fixed input assignments.
    pub fn is_fixed_input_conflict(&self, rc: RegClass, reg: RegUnit) -> bool {
        debug_assert!(!self.inputs_done);
        !self.regs_out.is_avail(rc, reg)
    }

    /// Finish adding input side constraints.
    ///
    /// Call this method to indicate that there will be no more fixed input reassignments added
    /// and prepare for the output side constraints.
    pub fn inputs_done(&mut self) {
        assert!(!self.has_fixed_input_conflicts());

        // At this point, `regs_out` contains the `to` side of the input reassignments, and the
        // `from` side has already been marked as available in `regs_in`.
        //
        // Remove the `to` assignments from `regs_in` so it now indicates the registers available
        // to variables at the input side.
        self.regs_in.intersect(&self.regs_out);

        // The meaning of `regs_out` now changes completely to indicate the registers available to
        // variables on the output side.
        // The initial mask will be modified by `add_kill()` and `add_fixed_output()`.
        self.regs_out = self.regs_in.clone();

        // Now we can't add more fixed input assignments, but `add_var()` is still allowed.
        self.inputs_done = true;
    }

    /// Record that an input register value is killed by the instruction.
    ///
    /// Even if a fixed reassignment has been added for the value, the `reg` argument should be the
    /// original location before the reassignments.
    ///
    /// This means that the register is available on the output side.
    pub fn add_kill(&mut self, value: Value, rc: RegClass, reg: RegUnit) {
        debug_assert!(self.inputs_done);

        // If a fixed assignment is killed, the `to` register becomes available on the output side.
        if let Some(a) = self.assignments.get(value) {
            debug_assert_eq!(a.from, reg);
            self.regs_out.free(a.rc, a.to);
            return;
        }

        // It's also possible that a variable is killed. That means it doesn't need to satisfy
        // interference constraints on the output side.
        // Variables representing tied operands will get their `is_output` flag set again later.
        if let Some(v) = self.vars.iter_mut().find(|v| v.value == value) {
            assert!(v.is_input);
            v.is_output = false;
            return;
        }

        // Alright, this is just a boring value being killed by the instruction. Just reclaim
        // the assigned register.
        self.regs_out.free(rc, reg);
    }

    /// Add a fixed output assignment.
    ///
    /// This means that `to` will not be available for variables on the output side of the
    /// instruction.
    ///
    /// Returns `false` if a live value conflicts with `to`, so it couldn't be added. Find the
    /// conflicting live-through value and turn it into a variable before calling this method
    /// again.
    #[allow(dead_code)]
    pub fn add_fixed_output(&mut self, rc: RegClass, reg: RegUnit) -> bool {
        debug_assert!(self.inputs_done);
        if self.regs_out.is_avail(rc, reg) {
            self.regs_out.take(rc, reg);
            true
        } else {
            false
        }
    }

    /// Add a defined output value.
    ///
    /// This is similar to `add_var`, except the value doesn't have a prior register assignment.
    pub fn add_def(&mut self, value: Value, constraint: RegClass) {
        debug_assert!(self.inputs_done);
        self.vars.push(Variable::new_def(value, constraint));
    }
}

/// Interface for searching for a solution.
impl Solver {
    /// Try a quick-and-dirty solution.
    ///
    /// This is expected to succeed for most instructions since the constraint problem is almost
    /// always trivial.
    ///
    /// Returns `true` is a solution was found.
    pub fn quick_solve(&mut self) -> Result<AllocatableSet, RegClass> {
        self.find_solution()
    }

    /// Search for a solution with the current list of variables.
    ///
    /// If a solution was found, returns `Ok(regs)` with the set of available registers on the
    /// output side after the solution. If no solution could be found, returns `Err(rc)` with the
    /// constraint register class that needs more available registers.
    fn find_solution(&mut self) -> Result<AllocatableSet, RegClass> {
        // Available registers on the input and output sides respectively.
        let mut iregs = self.regs_in.clone();
        let mut oregs = self.regs_out.clone();

        for v in &mut self.vars {
            let rc = v.constraint;
            let reg = match v.iter(&iregs, &oregs).next() {
                None => return Err(rc),
                Some(reg) => reg,
            };

            v.solution = reg;
            if v.is_input {
                iregs.take(rc, reg);
            }
            if v.is_output {
                oregs.take(rc, reg);
            }
        }

        Ok(oregs)
    }

    /// Get all the variables.
    pub fn vars(&self) -> &[Variable] {
        &self.vars
    }
}

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
//!
//! We are not concerned with stack values at all. The reload pass ensures that all values required
//! to be in a register by the instruction are already in a register.
//!
//! A solution to the register coloring problem consists of:
//!
//! - Register reassignment prescriptions for a subset of the live register values.
//! - Register assignments for the instruction's defined values.
//!
//! The solution ensures that when live registers are reassigned as prescribed before the
//! instruction, all its operand constraints are satisfied, and the definition assignments won't
//! conflict.
//!
//! # Register diversions and global interference
//!
//! We can divert register values temporarily to satisfy constraints, but we need to put the
//! values back into their originally assigned register locations before leaving the block.
//! Otherwise, values won't be in the right register at the entry point of other blocks.
//!
//! Some values are *local*, and we don't need to worry about putting those values back since they
//! are not used in any other blocks.
//!
//! When we assign register locations to defines, we are assigning both the register used locally
//! immediately after the instruction and the register used globally when the defined value is used
//! in a different block. We need to avoid interference both locally at the instruction and globally.
//!
//! We have multiple mappings of values to registers:
//!
//! 1. The initial local mapping before the instruction. This includes any diversions from previous
//!    instructions in the block, but not diversions for the current instruction.
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

use super::RegisterSet;
use crate::dbg::DisplayList;
use crate::entity::{SparseMap, SparseMapValue};
use crate::ir::Value;
use crate::isa::{RegClass, RegUnit};
use crate::regalloc::register_set::RegSetIter;
use alloc::vec::Vec;
use core::cmp;
use core::fmt;
use core::mem;
use core::u16;
use log::debug;

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
    fn new_live(value: Value, constraint: RegClass, from: RegUnit, is_output: bool) -> Self {
        Self {
            value,
            constraint,
            from: Some(from),
            is_input: true,
            is_output,
            is_global: false,
            domain: 0,
            solution: !0,
        }
    }

    fn new_def(value: Value, constraint: RegClass, is_global: bool) -> Self {
        Self {
            value,
            constraint,
            from: None,
            is_input: false,
            is_output: true,
            is_global,
            domain: 0,
            solution: !0,
        }
    }

    /// Does this variable represent a value defined by the current instruction?
    pub fn is_define(&self) -> bool {
        self.from.is_none()
    }

    /// Get an iterator over possible register choices, given the available registers on the input
    /// and output sides as well as the available global register set.
    fn iter(&self, iregs: &RegisterSet, oregs: &RegisterSet, gregs: &RegisterSet) -> RegSetIter {
        if !self.is_output {
            debug_assert!(!self.is_global, "Global implies output");
            debug_assert!(self.is_input, "Missing interference set");
            return iregs.iter(self.constraint);
        }

        let mut r = oregs.clone();
        if self.is_input {
            r.intersect(iregs);
        }
        if self.is_global {
            r.intersect(gregs);
        }
        r.iter(self.constraint)
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({}", self.value, self.constraint)?;
        if let Some(reg) = self.from {
            write!(f, ", from {}", self.constraint.info.display_regunit(reg))?;
        }
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

#[derive(Clone, Debug)]
pub struct Assignment {
    pub value: Value,
    pub from: RegUnit,
    pub to: RegUnit,
    pub rc: RegClass,
}

impl SparseMapValue<Value> for Assignment {
    fn key(&self) -> Value {
        self.value
    }
}

impl fmt::Display for Assignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ri = self.rc.info;
        write!(
            f,
            "{}:{}({} -> {})",
            self.value,
            self.rc,
            ri.display_regunit(self.from),
            ri.display_regunit(self.to)
        )
    }
}

/// A move operation between two registers or between a register and an emergency spill slot.
#[derive(Clone, PartialEq)]
pub enum Move {
    Reg {
        value: Value,
        rc: RegClass,
        from: RegUnit,
        to: RegUnit,
    },
    #[allow(dead_code)] // rustc doesn't see it isn't dead.
    Spill {
        value: Value,
        rc: RegClass,
        from: RegUnit,
        to_slot: usize,
    },
    Fill {
        value: Value,
        rc: RegClass,
        from_slot: usize,
        to: RegUnit,
    },
}

impl Move {
    /// Create a register move from an assignment, but not for identity assignments.
    fn with_assignment(a: &Assignment) -> Option<Self> {
        if a.from != a.to {
            Some(Self::Reg {
                value: a.value,
                from: a.from,
                to: a.to,
                rc: a.rc,
            })
        } else {
            None
        }
    }

    /// Get the "from" register and register class, if possible.
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::wrong_self_convention))]
    fn from_reg(&self) -> Option<(RegClass, RegUnit)> {
        match *self {
            Self::Reg { rc, from, .. } | Self::Spill { rc, from, .. } => Some((rc, from)),
            Self::Fill { .. } => None,
        }
    }

    /// Get the "to" register and register class, if possible.
    fn to_reg(&self) -> Option<(RegClass, RegUnit)> {
        match *self {
            Self::Reg { rc, to, .. } | Self::Fill { rc, to, .. } => Some((rc, to)),
            Self::Spill { .. } => None,
        }
    }

    /// Replace the "to" register with `new` and return the old value.
    fn replace_to_reg(&mut self, new: RegUnit) -> RegUnit {
        mem::replace(
            match *self {
                Self::Reg { ref mut to, .. } | Self::Fill { ref mut to, .. } => to,
                Self::Spill { .. } => panic!("No to register in a spill {}", self),
            },
            new,
        )
    }

    /// Convert this `Reg` move to a spill to `slot` and return the old "to" register.
    fn change_to_spill(&mut self, slot: usize) -> RegUnit {
        match self.clone() {
            Self::Reg {
                value,
                rc,
                from,
                to,
            } => {
                *self = Self::Spill {
                    value,
                    rc,
                    from,
                    to_slot: slot,
                };
                to
            }
            _ => panic!("Expected reg move: {}", self),
        }
    }

    /// Get the value being moved.
    fn value(&self) -> Value {
        match *self {
            Self::Reg { value, .. } | Self::Fill { value, .. } | Self::Spill { value, .. } => value,
        }
    }

    /// Get the associated register class.
    fn rc(&self) -> RegClass {
        match *self {
            Self::Reg { rc, .. } | Self::Fill { rc, .. } | Self::Spill { rc, .. } => rc,
        }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Reg {
                value,
                from,
                to,
                rc,
            } => write!(
                f,
                "{}:{}({} -> {})",
                value,
                rc,
                rc.info.display_regunit(from),
                rc.info.display_regunit(to)
            ),
            Self::Spill {
                value,
                from,
                to_slot,
                rc,
            } => write!(
                f,
                "{}:{}({} -> slot {})",
                value,
                rc,
                rc.info.display_regunit(from),
                to_slot
            ),
            Self::Fill {
                value,
                from_slot,
                to,
                rc,
            } => write!(
                f,
                "{}:{}(slot {} -> {})",
                value,
                rc,
                from_slot,
                rc.info.display_regunit(to)
            ),
        }
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let as_display: &dyn fmt::Display = self;
        as_display.fmt(f)
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
///    `add_through_var()`.
///
pub struct Solver {
    /// Register reassignments that are required or decided as part of a full solution.
    /// This includes identity assignments for values that are already in the correct fixed
    /// register.
    assignments: SparseMap<Value, Assignment>,

    /// Variables are the values that should be reassigned as part of a solution.
    /// Values with fixed register constraints are not considered variables. They are represented
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
    regs_in: RegisterSet,

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
    regs_out: RegisterSet,

    /// List of register moves scheduled to avoid conflicts.
    ///
    /// This is used as working space by the `schedule_moves()` function.
    moves: Vec<Move>,

    /// List of pending fill moves. This is only used during `schedule_moves()`.
    fills: Vec<Move>,
}

/// Interface for programming the constraints into the solver.
impl Solver {
    /// Create a new empty solver.
    pub fn new() -> Self {
        Self {
            assignments: SparseMap::new(),
            vars: Vec::new(),
            inputs_done: false,
            regs_in: RegisterSet::new(),
            regs_out: RegisterSet::new(),
            moves: Vec::new(),
            fills: Vec::new(),
        }
    }

    /// Clear all data structures in this coloring pass.
    pub fn clear(&mut self) {
        self.assignments.clear();
        self.vars.clear();
        self.inputs_done = false;
        self.regs_in = RegisterSet::new();
        self.regs_out = RegisterSet::new();
        self.moves.clear();
        self.fills.clear();
    }

    /// Reset the solver state and prepare solving for a new instruction with an initial set of
    /// allocatable registers.
    ///
    /// The `regs` set is the allocatable registers before any reassignments are applied.
    pub fn reset(&mut self, regs: &RegisterSet) {
        self.assignments.clear();
        self.vars.clear();
        self.inputs_done = false;
        self.regs_in = regs.clone();
        // Used for tracking fixed input assignments while `!inputs_done`:
        self.regs_out = RegisterSet::new();
        self.moves.clear();
        self.fills.clear();
    }

    /// Add a fixed input reassignment of `value`.
    ///
    /// This means that `value` must be assigned to `to` and can't become a variable. Call with
    /// `from == to` to ensure that `value` is not reassigned from its existing register location.
    ///
    /// In either case, `to` will not be available for variables on the input side of the
    /// instruction.
    pub fn reassign_in(&mut self, value: Value, rc: RegClass, from: RegUnit, to: RegUnit) {
        debug!(
            "reassign_in({}:{}, {} -> {})",
            value,
            rc,
            rc.info.display_regunit(from),
            rc.info.display_regunit(to)
        );
        debug_assert!(!self.inputs_done);
        if self.regs_in.is_avail(rc, from) {
            // It looks like `value` was already removed from the register set. It must have been
            // added as a variable previously. A fixed constraint beats a variable, so convert it.
            if let Some(idx) = self.vars.iter().position(|v| v.value == value) {
                let v = self.vars.remove(idx);
                debug!("-> converting variable {} to a fixed constraint", v);
                // The spiller is responsible for ensuring that all constraints on the uses of a
                // value are compatible.
                debug_assert!(
                    v.constraint.contains(to),
                    "Incompatible constraints for {}",
                    value
                );
            } else {
                panic!("Invalid from register for fixed {} constraint", value);
            }
        }
        self.regs_in.free(rc, from);
        self.regs_out.take(rc, to);
        self.assignments.insert(Assignment {
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
    ///
    /// This function can only be used before calling `inputs_done()`. Afterwards, more input-side
    /// variables can be added by calling `add_killed_var()` and `add_through_var()`
    pub fn add_var(&mut self, value: Value, constraint: RegClass, from: RegUnit) {
        debug!(
            "add_var({}:{}, from={})",
            value,
            constraint,
            constraint.info.display_regunit(from)
        );
        debug_assert!(!self.inputs_done);
        self.add_live_var(value, constraint, from, true);
    }

    /// Add an extra input-side variable representing a value that is killed by the current
    /// instruction.
    ///
    /// This function should be called after `inputs_done()` only. Use `add_var()` before.
    pub fn add_killed_var(&mut self, value: Value, rc: RegClass, from: RegUnit) {
        debug!(
            "add_killed_var({}:{}, from={})",
            value,
            rc,
            rc.info.display_regunit(from)
        );
        debug_assert!(self.inputs_done);
        self.add_live_var(value, rc, from, false);
    }

    /// Add an extra input-side variable representing a value that is live through the current
    /// instruction.
    ///
    /// This function should be called after `inputs_done()` only. Use `add_var()` before.
    pub fn add_through_var(&mut self, value: Value, rc: RegClass, from: RegUnit) {
        debug!(
            "add_through_var({}:{}, from={})",
            value,
            rc,
            rc.info.display_regunit(from)
        );
        debug_assert!(self.inputs_done);
        self.add_live_var(value, rc, from, true);
    }

    /// Shared code for `add_var`, `add_killed_var`, and `add_through_var`.
    ///
    /// Add a variable that is live before the instruction, and possibly live through. Merge
    /// constraints if the value has already been added as a variable or fixed assignment.
    fn add_live_var(&mut self, value: Value, rc: RegClass, from: RegUnit, live_through: bool) {
        // Check for existing entries for this value.
        if !self.can_add_var(rc, from) {
            // There could be an existing variable entry.
            if let Some(v) = self.vars.iter_mut().find(|v| v.value == value) {
                // We have an existing variable entry for `value`. Combine the constraints.
                if let Some(rc) = v.constraint.intersect(rc) {
                    debug!("-> combining constraint with {} yields {}", v, rc);
                    v.constraint = rc;
                    return;
                } else {
                    // The spiller should have made sure the same value is not used with disjoint
                    // constraints.
                    panic!("Incompatible constraints: {} + {}", rc, v)
                }
            }

            // No variable, then it must be a fixed reassignment.
            if let Some(a) = self.assignments.get(value) {
                debug!("-> already fixed assignment {}", a);
                debug_assert!(rc.contains(a.to), "Incompatible constraints for {}", value);
                return;
            }

            debug!("{}", self);
            panic!("Wrong from register for {}", value);
        }

        let new_var = Variable::new_live(value, rc, from, live_through);
        debug!("-> new var: {}", new_var);

        self.regs_in.free(rc, from);
        if self.inputs_done && live_through {
            self.regs_out.free(rc, from);
        }
        self.vars.push(new_var);
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
        debug_assert!(!self.has_fixed_input_conflicts());

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
            debug_assert!(v.is_input);
            v.is_output = false;
            return;
        }

        // Alright, this is just a boring value being killed by the instruction. Just reclaim
        // the assigned register.
        self.regs_out.free(rc, reg);
    }

    /// Record that an input register is tied to an output register.
    ///
    /// It is assumed that `add_kill` was called previously with the same arguments.
    ///
    /// The output value that must have the same register as the input value is not recorded in the
    /// solver.
    ///
    /// If the value has already been assigned to a fixed register, return that.
    pub fn add_tied_input(
        &mut self,
        value: Value,
        rc: RegClass,
        reg: RegUnit,
        is_global: bool,
    ) -> Option<RegUnit> {
        debug_assert!(self.inputs_done);

        // If a fixed assignment is tied, the `to` register is not available on the output side.
        if let Some(a) = self.assignments.get(value) {
            debug_assert_eq!(a.from, reg);
            self.regs_out.take(a.rc, a.to);
            return Some(a.to);
        }

        // Check if a variable was created.
        if let Some(v) = self.vars.iter_mut().find(|v| v.value == value) {
            debug_assert!(v.is_input);
            v.is_output = true;
            v.is_global = is_global;
            return None;
        }

        // No variable exists for `value` because its constraints are already satisfied.
        // However, if the tied output value has a global live range, we must create a variable to
        // avoid global interference too.
        if is_global {
            let mut new_var = Variable::new_live(value, rc, reg, true);
            new_var.is_global = true;
            debug!("add_tied_input: new tied-global value: {}", new_var);
            self.vars.push(new_var);
            self.regs_in.free(rc, reg);
        } else {
            self.regs_out.take(rc, reg);
        }

        None
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
    pub fn add_def(&mut self, value: Value, constraint: RegClass, is_global: bool) {
        debug_assert!(self.inputs_done);
        self.vars
            .push(Variable::new_def(value, constraint, is_global));
    }

    /// Clear the `is_global` flag on all solver variables.
    ///
    /// This is used when there are not enough global registers available, and global defines have
    /// to be replaced with local defines followed by a copy.
    pub fn clear_all_global_flags(&mut self) {
        for v in &mut self.vars {
            v.is_global = false;
        }
    }
}

/// Error reported when the solver fails to find a solution with the current constraints.
///
/// When no solution can be found, the error indicates how constraints could be loosened to help.
pub enum SolverError {
    /// There are not available registers in the given register class.
    ///
    /// This should be resolved by turning live-through values into variables so they can be moved
    /// out of the way.
    Divert(RegClass),

    /// There are insufficient available registers in the global set to assign an `is_global`
    /// variable with the given value.
    ///
    /// This should be resolved by converting the variable to a local one.
    Global(Value),
}

/// Interface for searching for a solution.
impl Solver {
    /// Try a quick-and-dirty solution.
    ///
    /// This is expected to succeed for most instructions since the constraint problem is almost
    /// always trivial.
    ///
    /// Returns `Ok(regs)` if a solution was found.
    pub fn quick_solve(
        &mut self,
        global_regs: &RegisterSet,
        is_reload: bool,
    ) -> Result<RegisterSet, SolverError> {
        self.find_solution(global_regs, is_reload)
    }

    /// Try harder to find a solution.
    ///
    /// Call this method after `quick_solve()` fails.
    ///
    /// This may return an error with a register class that has run out of registers. If registers
    /// can be freed up in the starving class, this method can be called again after adding
    /// variables for the freed registers.
    pub fn real_solve(
        &mut self,
        global_regs: &RegisterSet,
        is_reload: bool,
    ) -> Result<RegisterSet, SolverError> {
        // Compute domain sizes for all the variables given the current register sets.
        for v in &mut self.vars {
            let d = v.iter(&self.regs_in, &self.regs_out, global_regs).len();
            v.domain = cmp::min(d, u16::MAX as usize) as u16;
        }

        // Solve for vars with small domains first to increase the chance of finding a solution.
        //
        // Also consider this case:
        //
        // v0: out, global
        // v1: in
        // v2: in+out
        //
        // If only %r0 and %r1 are available, the global constraint may cause us to assign:
        //
        // v0 -> %r1
        // v1 -> %r0
        // v2 -> !
        //
        // Usually in+out variables will have a smaller domain, but in the above case the domain
        // size is the same, so we also prioritize in+out variables.
        //
        // Include the reversed previous solution for this variable partly as a stable tie breaker,
        // partly to shake things up on a second attempt.
        //
        // Use the `from` register and value number as a tie breaker to get a stable sort.
        self.vars.sort_unstable_by_key(|v| {
            (
                v.domain,
                !(v.is_input && v.is_output),
                !v.solution,
                v.from.unwrap_or(0),
                v.value,
            )
        });

        debug!("real_solve for {}", self);
        self.find_solution(global_regs, is_reload)
    }

    /// Search for a solution with the current list of variables.
    ///
    /// If a solution was found, returns `Ok(regs)` with the set of available registers on the
    /// output side after the solution. If no solution could be found, returns `Err(rc)` with the
    /// constraint register class that needs more available registers.
    fn find_solution(
        &mut self,
        global_regs: &RegisterSet,
        is_reload: bool,
    ) -> Result<RegisterSet, SolverError> {
        // Available registers on the input and output sides respectively.
        let mut iregs = self.regs_in.clone();
        let mut oregs = self.regs_out.clone();
        let mut gregs = global_regs.clone();

        for v in &mut self.vars {
            let rc = v.constraint;

            // Decide which register to assign.  In order to try and keep registers holding
            // reloaded values separate from all other registers to the extent possible, we choose
            // the first available register in the normal case, but the last available one in the
            // case of a reload.  See "A side note on register choice heuristics" in
            // src/redundant_reload_remover.rs for further details.
            let mut reg_set_iter = v.iter(&iregs, &oregs, &gregs);
            let maybe_reg = if is_reload {
                reg_set_iter.rnext()
            } else {
                reg_set_iter.next()
            };

            let reg = match maybe_reg {
                Some(reg) => reg,
                None => {
                    // If `v` must avoid global interference, there is not point in requesting
                    // live registers be diverted. We need to make it a non-global value.
                    if v.is_global && gregs.iter(rc).next().is_none() {
                        return Err(SolverError::Global(v.value));
                    }
                    return Err(SolverError::Divert(rc));
                }
            };

            v.solution = reg;
            if v.is_input {
                iregs.take(rc, reg);
            }
            if v.is_output {
                oregs.take(rc, reg);
            }
            if v.is_global {
                gregs.take(rc, reg);
            }
        }

        Ok(oregs)
    }

    /// Get all the variables.
    pub fn vars(&self) -> &[Variable] {
        &self.vars
    }

    /// Check if `value` can be added as a variable to help find a solution.
    pub fn can_add_var(&mut self, constraint: RegClass, from: RegUnit) -> bool {
        !self.regs_in.is_avail(constraint, from)
            && !self.vars.iter().any(|var| var.from == Some(from))
    }
}

/// Interface for working with parallel copies once a solution has been found.
impl Solver {
    /// Collect all the register moves we need to execute.
    fn collect_moves(&mut self) {
        self.moves.clear();

        // Collect moves from the chosen solution for all non-define variables.
        for v in &self.vars {
            if let Some(from) = v.from {
                // Omit variable solutions that don't require the value to be moved.
                if from != v.solution {
                    self.moves.push(Move::Reg {
                        value: v.value,
                        from,
                        to: v.solution,
                        rc: v.constraint,
                    });
                }
            }
        }

        // Convert all of the fixed register assignments into moves, but omit the ones that are
        // already in the right register.
        self.moves
            .extend(self.assignments.values().filter_map(Move::with_assignment));

        if !self.moves.is_empty() {
            debug!("collect_moves: {}", DisplayList(&self.moves));
        }
    }

    /// Try to schedule a sequence of `regmove` instructions that will shuffle registers into
    /// place.
    ///
    /// This may require the use of additional available registers, and it can fail if no
    /// additional registers are available.
    ///
    /// TODO: Handle failure by generating a sequence of register swaps, or by temporarily spilling
    /// a register.
    ///
    /// Returns the number of spills that had to be emitted.
    pub fn schedule_moves(&mut self, regs: &RegisterSet) -> usize {
        self.collect_moves();
        debug_assert!(self.fills.is_empty());

        let mut num_spill_slots = 0;
        let mut avail = regs.clone();
        let mut i = 0;
        while i < self.moves.len() + self.fills.len() {
            // Don't even look at the fills until we've spent all the moves. Deferring these lets
            // us potentially reuse the claimed registers to resolve multiple cycles.
            if i >= self.moves.len() {
                self.moves.append(&mut self.fills);
            }

            // Find the first move that can be executed now.
            if let Some(j) = self.moves[i..].iter().position(|m| match m.to_reg() {
                Some((rc, reg)) => avail.is_avail(rc, reg),
                None => true,
            }) {
                // This move can be executed now.
                self.moves.swap(i, i + j);
                let m = &self.moves[i];
                if let Some((rc, reg)) = m.to_reg() {
                    avail.take(rc, reg);
                }
                if let Some((rc, reg)) = m.from_reg() {
                    avail.free(rc, reg);
                }
                debug!("move #{}: {}", i, m);
                i += 1;
                continue;
            }

            // When we get here, none of the `moves[i..]` can be executed. This means there are
            // only cycles remaining. The cycles can be broken in a few ways:
            //
            // 1. Grab an available register and use it to break a cycle.
            // 2. Move a value temporarily into a stack slot instead of a register.
            // 3. Use swap instructions.
            //
            // TODO: So far we only implement 1 and 2.

            // Pick an assignment with the largest possible width. This is more likely to break up
            // a cycle than an assignment with fewer register units. For example, it may be
            // necessary to move two arm32 S-registers out of the way before a D-register can move
            // into place.
            //
            // We use `min_by_key` and `!` instead of `max_by_key` because it preserves the
            // existing order of moves with the same width.
            let j = self.moves[i..]
                .iter()
                .enumerate()
                .min_by_key(|&(_, m)| !m.rc().width)
                .unwrap()
                .0;
            self.moves.swap(i, i + j);

            // Check the top-level register class for an available register. It is an axiom of the
            // register allocator that we can move between all registers in the top-level RC.
            let m = self.moves[i].clone();
            let toprc = m.rc().toprc();
            if let Some(reg) = avail.iter(toprc).next() {
                debug!(
                    "breaking cycle at {} with available {} register {}",
                    m,
                    toprc,
                    toprc.info.display_regunit(reg)
                );

                // Alter the move so it is guaranteed to be picked up when we loop. It is important
                // that this move is scheduled immediately, otherwise we would have multiple moves
                // of the same value, and they would not be commutable.
                let old_to_reg = self.moves[i].replace_to_reg(reg);
                // Append a fixup move so we end up in the right place. This move will be scheduled
                // later. That's ok because it is the single remaining move of `m.value` after the
                // next iteration.
                self.moves.push(Move::Reg {
                    value: m.value(),
                    rc: toprc,
                    from: reg,
                    to: old_to_reg,
                });
                // TODO: What if allocating an extra register is not enough to break a cycle? This
                // can happen when there are registers of different widths in a cycle. For ARM, we
                // may have to move two S-registers out of the way before we can resolve a cycle
                // involving a D-register.
                continue;
            }

            // It was impossible to free up a register in toprc, so use an emergency spill slot as
            // a last resort.
            let slot = num_spill_slots;
            num_spill_slots += 1;
            debug!("breaking cycle at {} with slot {}", m, slot);
            let old_to_reg = self.moves[i].change_to_spill(slot);
            self.fills.push(Move::Fill {
                value: m.value(),
                rc: toprc,
                from_slot: slot,
                to: old_to_reg,
            });
        }

        num_spill_slots
    }

    /// Borrow the scheduled set of register moves that was computed by `schedule_moves()`.
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }
}

impl fmt::Display for Solver {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let reginfo = self.vars.first().map(|v| v.constraint.info);
        writeln!(f, "Solver {{ inputs_done: {},", self.inputs_done)?;
        writeln!(f, "  in:  {}", self.regs_in.display(reginfo))?;
        writeln!(f, "  out: {}", self.regs_out.display(reginfo))?;
        writeln!(
            f,
            "  assignments: {}",
            DisplayList(self.assignments.as_slice())
        )?;
        writeln!(f, "  vars: {}", DisplayList(&self.vars))?;
        writeln!(f, "  moves: {}", DisplayList(&self.moves))?;
        writeln!(f, "}}")
    }
}

#[cfg(test)]
#[cfg(feature = "arm32")]
mod tests {
    use super::{Move, Solver};
    use crate::entity::EntityRef;
    use crate::ir::Value;
    use crate::isa::registers::{RegBank, RegClassData};
    use crate::isa::{RegClass, RegInfo, RegUnit};
    use crate::regalloc::RegisterSet;
    use core::borrow::Borrow;

    // Arm32 `TargetIsa` is now `TargetIsaAdapter`, which does not hold any info
    // about registers, so we directly access `INFO` from registers-arm32.rs.
    include!(concat!(env!("OUT_DIR"), "/registers-arm32.rs"));

    // Get a register class by name.
    fn rc_by_name(reginfo: &RegInfo, name: &str) -> RegClass {
        reginfo
            .classes
            .iter()
            .find(|rc| rc.name == name)
            .expect("Can't find named register class.")
    }

    // Construct a register move.
    fn mov(value: Value, rc: RegClass, from: RegUnit, to: RegUnit) -> Move {
        Move::Reg {
            value,
            rc,
            from,
            to,
        }
    }

    fn spill(value: Value, rc: RegClass, from: RegUnit, to_slot: usize) -> Move {
        Move::Spill {
            value,
            rc,
            from,
            to_slot,
        }
    }

    fn fill(value: Value, rc: RegClass, from_slot: usize, to: RegUnit) -> Move {
        Move::Fill {
            value,
            rc,
            from_slot,
            to,
        }
    }

    #[test]
    fn simple_moves() {
        let reginfo = INFO.borrow();
        let gpr = rc_by_name(&reginfo, "GPR");
        let r0 = gpr.unit(0);
        let r1 = gpr.unit(1);
        let r2 = gpr.unit(2);
        let gregs = RegisterSet::new();
        let mut regs = RegisterSet::new();
        let mut solver = Solver::new();
        let v10 = Value::new(10);
        let v11 = Value::new(11);

        // As simple as it gets: Value is in r1, we want r0.
        regs.take(gpr, r1);
        solver.reset(&regs);
        solver.reassign_in(v10, gpr, r1, r0);
        solver.inputs_done();
        assert!(solver.quick_solve(&gregs, false).is_ok());
        assert_eq!(solver.schedule_moves(&regs), 0);
        assert_eq!(solver.moves(), &[mov(v10, gpr, r1, r0)]);

        // A bit harder: r0, r1 need to go in r1, r2.
        regs.take(gpr, r0);
        solver.reset(&regs);
        solver.reassign_in(v10, gpr, r0, r1);
        solver.reassign_in(v11, gpr, r1, r2);
        solver.inputs_done();
        assert!(solver.quick_solve(&gregs, false).is_ok());
        assert_eq!(solver.schedule_moves(&regs), 0);
        assert_eq!(
            solver.moves(),
            &[mov(v11, gpr, r1, r2), mov(v10, gpr, r0, r1)]
        );

        // Swap r0 and r1 in three moves using r2 as a scratch.
        solver.reset(&regs);
        solver.reassign_in(v10, gpr, r0, r1);
        solver.reassign_in(v11, gpr, r1, r0);
        solver.inputs_done();
        assert!(solver.quick_solve(&gregs, false).is_ok());
        assert_eq!(solver.schedule_moves(&regs), 0);
        assert_eq!(
            solver.moves(),
            &[
                mov(v10, gpr, r0, r2),
                mov(v11, gpr, r1, r0),
                mov(v10, gpr, r2, r1),
            ]
        );
    }

    #[test]
    fn harder_move_cycles() {
        let reginfo = INFO.borrow();
        let s = rc_by_name(&reginfo, "S");
        let d = rc_by_name(&reginfo, "D");
        let d0 = d.unit(0);
        let d1 = d.unit(1);
        let d2 = d.unit(2);
        let s0 = s.unit(0);
        let s1 = s.unit(1);
        let s2 = s.unit(2);
        let s3 = s.unit(3);
        let gregs = RegisterSet::new();
        let mut regs = RegisterSet::new();
        let mut solver = Solver::new();
        let v10 = Value::new(10);
        let v11 = Value::new(11);
        let v12 = Value::new(12);

        // Not a simple cycle: Swap d0 <-> (s2, s3)
        regs.take(d, d0);
        regs.take(d, d1);
        solver.reset(&regs);
        solver.reassign_in(v10, d, d0, d1);
        solver.reassign_in(v11, s, s2, s0);
        solver.reassign_in(v12, s, s3, s1);
        solver.inputs_done();
        assert!(solver.quick_solve(&gregs, false).is_ok());
        assert_eq!(solver.schedule_moves(&regs), 0);
        assert_eq!(
            solver.moves(),
            &[
                mov(v10, d, d0, d2),
                mov(v11, s, s2, s0),
                mov(v12, s, s3, s1),
                mov(v10, d, d2, d1),
            ]
        );

        // Same problem in the other direction: Swap (s0, s1) <-> d1.
        //
        // If we divert the moves in order, we will need to allocate *two* temporary S registers. A
        // trivial algorithm might assume that allocating a single temp is enough.
        solver.reset(&regs);
        solver.reassign_in(v11, s, s0, s2);
        solver.reassign_in(v12, s, s1, s3);
        solver.reassign_in(v10, d, d1, d0);
        solver.inputs_done();
        assert!(solver.quick_solve(&gregs, false).is_ok());
        assert_eq!(solver.schedule_moves(&regs), 0);
        assert_eq!(
            solver.moves(),
            &[
                mov(v10, d, d1, d2),
                mov(v12, s, s1, s3),
                mov(v11, s, s0, s2),
                mov(v10, d, d2, d0),
            ]
        );
    }

    #[test]
    fn emergency_spill() {
        let reginfo = INFO.borrow();
        let gpr = rc_by_name(&reginfo, "GPR");
        let r0 = gpr.unit(0);
        let r1 = gpr.unit(1);
        let r2 = gpr.unit(2);
        let r3 = gpr.unit(3);
        let r4 = gpr.unit(4);
        let r5 = gpr.unit(5);
        let gregs = RegisterSet::new();
        let mut regs = RegisterSet::new();
        let mut solver = Solver::new();
        let v10 = Value::new(10);
        let v11 = Value::new(11);
        let v12 = Value::new(12);
        let v13 = Value::new(13);
        let v14 = Value::new(14);
        let v15 = Value::new(15);

        // Claim r0--r2 and r3--r15 for other values.
        for i in 0..16 {
            regs.take(gpr, gpr.unit(i));
        }

        // Request a permutation cycle.
        solver.reset(&regs);
        solver.reassign_in(v10, gpr, r0, r1);
        solver.reassign_in(v11, gpr, r1, r2);
        solver.reassign_in(v12, gpr, r2, r0);
        solver.inputs_done();
        assert!(solver.quick_solve(&gregs, false).is_ok());
        assert_eq!(solver.schedule_moves(&regs), 1);
        assert_eq!(
            solver.moves(),
            &[
                spill(v10, gpr, r0, 0),
                mov(v12, gpr, r2, r0),
                mov(v11, gpr, r1, r2),
                fill(v10, gpr, 0, r1),
            ]
        );

        // Two cycles should only require a single spill.
        solver.reset(&regs);
        // Cycle 1.
        solver.reassign_in(v10, gpr, r0, r1);
        solver.reassign_in(v11, gpr, r1, r2);
        solver.reassign_in(v12, gpr, r2, r0);
        // Cycle 2.
        solver.reassign_in(v13, gpr, r3, r4);
        solver.reassign_in(v14, gpr, r4, r5);
        solver.reassign_in(v15, gpr, r5, r3);

        solver.inputs_done();
        assert!(solver.quick_solve(&gregs, false).is_ok());
        // We resolve two cycles with one spill.
        assert_eq!(solver.schedule_moves(&regs), 1);
        assert_eq!(
            solver.moves(),
            &[
                spill(v10, gpr, r0, 0),
                mov(v12, gpr, r2, r0),
                mov(v11, gpr, r1, r2),
                mov(v13, gpr, r3, r1), // Use available r1 to break cycle 2.
                mov(v15, gpr, r5, r3),
                mov(v14, gpr, r4, r5),
                mov(v13, gpr, r1, r4),
                fill(v10, gpr, 0, r1), // Finally complete cycle 1.
            ]
        );
    }
}

//! An optimizer for a set of peephole optimizations.

use crate::instruction_set::InstructionSet;
use crate::linear::{bool_to_match_result, Action, Else, MatchOp, MatchResult};
use crate::operator::UnquoteOperator;
use crate::optimizations::PeepholeOptimizations;
use crate::part::{Constant, Part};
use crate::r#type::{BitWidth, Type};
use peepmatic_automata::State;
use std::convert::TryFrom;
use std::fmt::{self, Debug};
use std::mem;
use std::num::NonZeroU32;

/// A peephole optimizer instance that can apply a set of peephole
/// optimizations to instructions.
///
/// These are created from a set of peephole optimizations with the
/// [`PeepholeOptimizer::instance`][crate::PeepholeOptimizer::instance] method.
///
/// Reusing an instance when applying peephole optimizations to different
/// instruction sequences means that you reuse internal allocations that are
/// used to match left-hand sides and build up right-hand sides.
pub struct PeepholeOptimizer<'peep, 'ctx, I>
where
    I: InstructionSet<'ctx>,
{
    pub(crate) peep_opt: &'peep PeepholeOptimizations,
    pub(crate) instr_set: I,
    pub(crate) left_hand_sides: Vec<Part<I::Instruction>>,
    pub(crate) right_hand_sides: Vec<Part<I::Instruction>>,
    pub(crate) actions: Vec<Action>,
    pub(crate) backtracking_states: Vec<(State, usize)>,
}

impl<'peep, 'ctx, I> Debug for PeepholeOptimizer<'peep, 'ctx, I>
where
    I: InstructionSet<'ctx>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let PeepholeOptimizer {
            peep_opt,
            instr_set: _,
            left_hand_sides,
            right_hand_sides,
            actions,
            backtracking_states,
        } = self;
        f.debug_struct("PeepholeOptimizer")
            .field("peep_opt", peep_opt)
            .field("instr_set", &"_")
            .field("left_hand_sides", left_hand_sides)
            .field("right_hand_sides", right_hand_sides)
            .field("actions", actions)
            .field("backtracking_states", backtracking_states)
            .finish()
    }
}

impl<'peep, 'ctx, I> PeepholeOptimizer<'peep, 'ctx, I>
where
    I: InstructionSet<'ctx>,
{
    fn eval_unquote_1(&self, operator: UnquoteOperator, a: Constant) -> Constant {
        use Constant::*;

        macro_rules! map_int {
            ( $c:expr , | $x:ident | $e:expr ) => {
                match $c {
                    Int($x, w) => Int($e, w),
                    Bool(..) => panic!("not an integer"),
                }
            };
        }

        match operator {
            UnquoteOperator::Log2 => map_int!(a, |x| x.trailing_zeros() as _),
            UnquoteOperator::Neg => map_int!(a, |x| x.wrapping_neg()),
            UnquoteOperator::Band
            | UnquoteOperator::Bor
            | UnquoteOperator::Bxor
            | UnquoteOperator::Iadd
            | UnquoteOperator::Imul => unreachable!("not a unary unquote operator: {:?}", operator),
        }
    }

    fn eval_unquote_2(&self, operator: UnquoteOperator, a: Constant, b: Constant) -> Constant {
        use Constant::*;

        macro_rules! fold_ints {
            ( $c1:expr , $c2:expr , | $x:ident , $y:ident | $e:expr ) => {
                match ($c1, $c2) {
                    (Int($x, w1), Int($y, w2)) if w1 == w2 => Int($e, w1),
                    _ => panic!("not two integers of the same width"),
                }
            };
        }

        match operator {
            UnquoteOperator::Band => fold_ints!(a, b, |x, y| x & y),
            UnquoteOperator::Bor => fold_ints!(a, b, |x, y| x | y),
            UnquoteOperator::Bxor => fold_ints!(a, b, |x, y| x ^ y),
            UnquoteOperator::Iadd => fold_ints!(a, b, |x, y| x.wrapping_add(y)),
            UnquoteOperator::Imul => fold_ints!(a, b, |x, y| x.wrapping_mul(y)),
            UnquoteOperator::Log2 | UnquoteOperator::Neg => {
                unreachable!("not a binary unquote operator: {:?}", operator)
            }
        }
    }

    fn eval_actions(&mut self, context: &mut I::Context, root: I::Instruction) {
        let mut actions = mem::replace(&mut self.actions, vec![]);

        for action in actions.drain(..) {
            log::trace!("Evaluating action: {:?}", action);
            match action {
                Action::GetLhs { path } => {
                    let path = self.peep_opt.paths.lookup(path);
                    let lhs = self
                        .instr_set
                        .get_part_at_path(context, root, path)
                        .expect("should always get part at path OK by the time it is bound");
                    self.right_hand_sides.push(lhs);
                }
                Action::UnaryUnquote { operator, operand } => {
                    let operand = self.right_hand_sides[operand.0 as usize];
                    let operand = match operand {
                        Part::Instruction(i) => self
                            .instr_set
                            .instruction_to_constant(context, i)
                            .expect("cannot convert instruction to constant for unquote operand"),
                        Part::Constant(c) => c,
                        Part::ConditionCode(_) => {
                            panic!("cannot use a condition code as an unquote operand")
                        }
                    };
                    let result = self.eval_unquote_1(operator, operand);
                    self.right_hand_sides.push(result.into());
                }
                Action::BinaryUnquote { operator, operands } => {
                    let a = self.right_hand_sides[operands[0].0 as usize];
                    let a = match a {
                        Part::Instruction(i) => self
                            .instr_set
                            .instruction_to_constant(context, i)
                            .expect("cannot convert instruction to constant for unquote operand"),
                        Part::Constant(c) => c,
                        Part::ConditionCode(_) => {
                            panic!("cannot use a condition code as an unquote operand")
                        }
                    };

                    let b = self.right_hand_sides[operands[1].0 as usize];
                    let b = match b {
                        Part::Instruction(i) => self
                            .instr_set
                            .instruction_to_constant(context, i)
                            .expect("cannot convert instruction to constant for unquote operand"),
                        Part::Constant(c) => c,
                        Part::ConditionCode(_) => {
                            panic!("cannot use a condition code as an unquote operand")
                        }
                    };

                    let result = self.eval_unquote_2(operator, a, b);
                    self.right_hand_sides.push(result.into());
                }
                Action::MakeIntegerConst {
                    value,
                    mut bit_width,
                } => {
                    let value = self.peep_opt.integers.lookup(value);
                    if bit_width.is_polymorphic() {
                        bit_width = BitWidth::try_from(
                            self.instr_set.instruction_result_bit_width(context, root),
                        )
                        .unwrap();
                    }
                    self.right_hand_sides
                        .push(Constant::Int(value, bit_width).into());
                }
                Action::MakeBooleanConst {
                    value,
                    mut bit_width,
                } => {
                    if bit_width.is_polymorphic() {
                        bit_width = BitWidth::try_from(
                            self.instr_set.instruction_result_bit_width(context, root),
                        )
                        .unwrap();
                    }
                    self.right_hand_sides
                        .push(Constant::Bool(value, bit_width).into());
                }
                Action::MakeConditionCode { cc } => {
                    self.right_hand_sides.push(Part::ConditionCode(cc));
                }
                Action::MakeUnaryInst {
                    operator,
                    r#type:
                        Type {
                            kind,
                            mut bit_width,
                        },
                    operand,
                } => {
                    if bit_width.is_polymorphic() {
                        bit_width = BitWidth::try_from(
                            self.instr_set.instruction_result_bit_width(context, root),
                        )
                        .unwrap();
                    }
                    let ty = Type { kind, bit_width };
                    let operand = self.right_hand_sides[operand.0 as usize];
                    let inst = self
                        .instr_set
                        .make_inst_1(context, root, operator, ty, operand);
                    self.right_hand_sides.push(Part::Instruction(inst));
                }
                Action::MakeBinaryInst {
                    operator,
                    r#type:
                        Type {
                            kind,
                            mut bit_width,
                        },
                    operands,
                } => {
                    if bit_width.is_polymorphic() {
                        bit_width = BitWidth::try_from(
                            self.instr_set.instruction_result_bit_width(context, root),
                        )
                        .unwrap();
                    }
                    let ty = Type { kind, bit_width };
                    let a = self.right_hand_sides[operands[0].0 as usize];
                    let b = self.right_hand_sides[operands[1].0 as usize];
                    let inst = self
                        .instr_set
                        .make_inst_2(context, root, operator, ty, a, b);
                    self.right_hand_sides.push(Part::Instruction(inst));
                }
                Action::MakeTernaryInst {
                    operator,
                    r#type:
                        Type {
                            kind,
                            mut bit_width,
                        },
                    operands,
                } => {
                    if bit_width.is_polymorphic() {
                        bit_width = BitWidth::try_from(
                            self.instr_set.instruction_result_bit_width(context, root),
                        )
                        .unwrap();
                    }
                    let ty = Type { kind, bit_width };
                    let a = self.right_hand_sides[operands[0].0 as usize];
                    let b = self.right_hand_sides[operands[1].0 as usize];
                    let c = self.right_hand_sides[operands[2].0 as usize];
                    let inst = self
                        .instr_set
                        .make_inst_3(context, root, operator, ty, a, b, c);
                    self.right_hand_sides.push(Part::Instruction(inst));
                }
            }
        }

        // Reuse the heap elements allocation.
        self.actions = actions;
    }

    fn eval_match_op(
        &mut self,
        context: &mut I::Context,
        root: I::Instruction,
        match_op: MatchOp,
    ) -> MatchResult {
        use crate::linear::MatchOp::*;

        log::trace!("Evaluating match operation: {:?}", match_op);
        let result: MatchResult = (|| match match_op {
            Opcode { path } => {
                let path = self.peep_opt.paths.lookup(path);
                let part = self
                    .instr_set
                    .get_part_at_path(context, root, path)
                    .ok_or(Else)?;
                let inst = part.as_instruction().ok_or(Else)?;
                let op = self.instr_set.operator(context, inst).ok_or(Else)?;
                let op = op as u32;
                debug_assert!(
                    op != 0,
                    "`Operator` doesn't have any variant represented
        with zero"
                );
                Ok(unsafe { NonZeroU32::new_unchecked(op as u32) })
            }
            IsConst { path } => {
                let path = self.peep_opt.paths.lookup(path);
                let part = self
                    .instr_set
                    .get_part_at_path(context, root, path)
                    .ok_or(Else)?;
                let is_const = match part {
                    Part::Instruction(i) => {
                        self.instr_set.instruction_to_constant(context, i).is_some()
                    }
                    Part::ConditionCode(_) | Part::Constant(_) => true,
                };
                bool_to_match_result(is_const)
            }
            IsPowerOfTwo { path } => {
                let path = self.peep_opt.paths.lookup(path);
                let part = self
                    .instr_set
                    .get_part_at_path(context, root, path)
                    .ok_or(Else)?;
                match part {
                    Part::Constant(c) => {
                        let is_pow2 = c.as_int().unwrap().is_power_of_two();
                        bool_to_match_result(is_pow2)
                    }
                    Part::Instruction(i) => {
                        let c = self
                            .instr_set
                            .instruction_to_constant(context, i)
                            .ok_or(Else)?;
                        let is_pow2 = c.as_int().unwrap().is_power_of_two();
                        bool_to_match_result(is_pow2)
                    }
                    Part::ConditionCode(_) => unreachable!(
                        "IsPowerOfTwo on a condition
        code"
                    ),
                }
            }
            BitWidth { path } => {
                let path = self.peep_opt.paths.lookup(path);
                let part = self
                    .instr_set
                    .get_part_at_path(context, root, path)
                    .ok_or(Else)?;
                let bit_width = match part {
                    Part::Instruction(i) => self.instr_set.instruction_result_bit_width(context, i),
                    Part::Constant(Constant::Int(_, w)) | Part::Constant(Constant::Bool(_, w)) => {
                        w.fixed_width().unwrap_or_else(|| {
                            self.instr_set.instruction_result_bit_width(context, root)
                        })
                    }
                    Part::ConditionCode(_) => panic!("BitWidth on condition code"),
                };
                debug_assert!(
                    bit_width != 0,
                    "`InstructionSet` implementors must uphold the contract that \
                     `instruction_result_bit_width` returns one of 1, 8, 16, 32, 64, or 128"
                );
                Ok(unsafe { NonZeroU32::new_unchecked(bit_width as u32) })
            }
            FitsInNativeWord { path } => {
                let native_word_size = self.instr_set.native_word_size_in_bits(context);
                debug_assert!(native_word_size.is_power_of_two());

                let path = self.peep_opt.paths.lookup(path);
                let part = self
                    .instr_set
                    .get_part_at_path(context, root, path)
                    .ok_or(Else)?;
                let fits = match part {
                    Part::Instruction(i) => {
                        let size = self.instr_set.instruction_result_bit_width(context, i);
                        size <= native_word_size
                    }
                    Part::Constant(c) => {
                        let root_width = self.instr_set.instruction_result_bit_width(context, root);
                        let size = c.bit_width(root_width);
                        size <= native_word_size
                    }
                    Part::ConditionCode(_) => panic!("FitsInNativeWord on condition code"),
                };
                bool_to_match_result(fits)
            }
            Eq { path_a, path_b } => {
                let path_a = self.peep_opt.paths.lookup(path_a);
                let part_a = self
                    .instr_set
                    .get_part_at_path(context, root, path_a)
                    .ok_or(Else)?;
                let path_b = self.peep_opt.paths.lookup(path_b);
                let part_b = self
                    .instr_set
                    .get_part_at_path(context, root, path_b)
                    .ok_or(Else)?;
                let eq = match (part_a, part_b) {
                    (Part::Instruction(inst), Part::Constant(c1))
                    | (Part::Constant(c1), Part::Instruction(inst)) => {
                        match self.instr_set.instruction_to_constant(context, inst) {
                            Some(c2) => c1 == c2,
                            None => false,
                        }
                    }
                    (a, b) => a == b,
                };
                bool_to_match_result(eq)
            }
            IntegerValue { path } => {
                let path = self.peep_opt.paths.lookup(path);
                let part = self
                    .instr_set
                    .get_part_at_path(context, root, path)
                    .ok_or(Else)?;
                match part {
                    Part::Constant(c) => {
                        let x = c.as_int().ok_or(Else)?;
                        let id = self.peep_opt.integers.already_interned(x).ok_or(Else)?;
                        Ok(id.into())
                    }
                    Part::Instruction(i) => {
                        let c = self
                            .instr_set
                            .instruction_to_constant(context, i)
                            .ok_or(Else)?;
                        let x = c.as_int().ok_or(Else)?;
                        let id = self.peep_opt.integers.already_interned(x).ok_or(Else)?;
                        Ok(id.into())
                    }
                    Part::ConditionCode(_) => unreachable!("IntegerValue on condition code"),
                }
            }
            BooleanValue { path } => {
                let path = self.peep_opt.paths.lookup(path);
                let part = self
                    .instr_set
                    .get_part_at_path(context, root, path)
                    .ok_or(Else)?;
                match part {
                    Part::Constant(c) => {
                        let b = c.as_bool().ok_or(Else)?;
                        bool_to_match_result(b)
                    }
                    Part::Instruction(i) => {
                        let c = self
                            .instr_set
                            .instruction_to_constant(context, i)
                            .ok_or(Else)?;
                        let b = c.as_bool().ok_or(Else)?;
                        bool_to_match_result(b)
                    }
                    Part::ConditionCode(_) => unreachable!("IntegerValue on condition code"),
                }
            }
            ConditionCode { path } => {
                let path = self.peep_opt.paths.lookup(path);
                let part = self
                    .instr_set
                    .get_part_at_path(context, root, path)
                    .ok_or(Else)?;
                let cc = part.as_condition_code().ok_or(Else)?;
                let cc = cc as u32;
                debug_assert!(cc != 0);
                Ok(unsafe { NonZeroU32::new_unchecked(cc) })
            }
            MatchOp::Nop => Err(Else),
        })();
        log::trace!("Evaluated match operation: {:?} = {:?}", match_op, result);
        result
    }

    /// Attempt to apply a single peephole optimization to the given root
    /// instruction.
    ///
    /// If an optimization is applied, then the `root` is replaced with the
    /// optimization's right-hand side, and the root of the right-hand side is
    /// returned as `Some`.
    ///
    /// If no optimization's left-hand side matches `root`, then `root` is left
    /// untouched and `None` is returned.
    pub fn apply_one(
        &mut self,
        context: &mut I::Context,
        root: I::Instruction,
    ) -> Option<I::Instruction> {
        log::trace!("PeepholeOptimizer::apply_one");

        self.backtracking_states.clear();
        self.actions.clear();
        self.left_hand_sides.clear();
        self.right_hand_sides.clear();

        let mut r#final = None;

        let mut query = self.peep_opt.automata.query();
        loop {
            log::trace!("Current state: {:?}", query.current_state());

            if query.is_in_final_state() {
                // If we're in a final state (which means an optimization is
                // applicable) then record that fact, but keep going. We don't
                // want to stop yet, because we might discover another,
                // more-specific optimization that is also applicable if we keep
                // going. And we always want to apply the most specific
                // optimization that matches.
                log::trace!("Found a match at state {:?}", query.current_state());
                r#final = Some((query.current_state(), self.actions.len()));
            }

            // Anything following a `Else` transition doesn't care about the
            // result of this match operation, so if we partially follow the
            // current non-`Else` path, but don't ultimately find a matching
            // optimization, we want to be able to backtrack to this state and
            // then try taking the `Else` transition.
            if query.has_transition_on(&Err(Else)) {
                self.backtracking_states
                    .push((query.current_state(), self.actions.len()));
            }

            let match_op = match query.current_state_data() {
                None => break,
                Some(op) => op,
            };

            let input = self.eval_match_op(context, root, *match_op);

            let actions = if let Some(actions) = query.next(&input) {
                actions
            } else if r#final.is_some() {
                break;
            } else if let Some((state, actions_len)) = self.backtracking_states.pop() {
                query.go_to_state(state);
                self.actions.truncate(actions_len);
                query
                    .next(&Err(Else))
                    .expect("backtracking states always have `Else` transitions")
            } else {
                break;
            };

            self.actions.extend(actions.iter().copied());
        }

        // If `final` is none, then we didn't encounter any final states, so
        // there are no applicable optimizations.
        let (final_state, actions_len) = match r#final {
            Some(f) => f,
            None => {
                log::trace!("No optimizations matched");
                return None;
            }
        };

        // Go to the last final state we saw, reset the LHS and RHS to how
        // they were at the time we saw the final state, and process the
        // final actions.
        self.actions.truncate(actions_len);
        query.go_to_state(final_state);
        let final_actions = query.finish().expect("should be in a final state");
        self.actions.extend(final_actions.iter().copied());
        self.eval_actions(context, root);

        // And finally, the root of the RHS for this optimization is the
        // last entry in `self.right_hand_sides`, so replace the old root
        // instruction with this one!
        let result = self.right_hand_sides.pop().unwrap();
        let new_root = self.instr_set.replace_instruction(context, root, result);
        Some(new_root)
    }

    /// Keep applying peephole optimizations to the given instruction until none
    /// can be applied anymore.
    pub fn apply_all(&mut self, context: &mut I::Context, mut inst: I::Instruction) {
        loop {
            if let Some(new_inst) = self.apply_one(context, inst) {
                inst = new_inst;
            } else {
                break;
            }
        }
    }
}

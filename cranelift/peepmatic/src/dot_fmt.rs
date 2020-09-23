//! Formatting a peephole optimizer's automata for GraphViz Dot.
//!
//! See also `crates/automata/src/dot.rs`.

use peepmatic_automata::dot::DotFmt;
use peepmatic_runtime::{
    cc::ConditionCode,
    integer_interner::{IntegerId, IntegerInterner},
    linear,
};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::io::{self, Write};
use std::num::{NonZeroU16, NonZeroU32};

#[derive(Debug)]
pub(crate) struct PeepholeDotFmt<'a>(pub(crate) &'a IntegerInterner);

impl<TOperator> DotFmt<linear::MatchResult, linear::MatchOp, Box<[linear::Action<TOperator>]>>
    for PeepholeDotFmt<'_>
where
    TOperator: Debug + TryFrom<NonZeroU32>,
{
    fn fmt_transition(
        &self,
        w: &mut impl Write,
        from: Option<&linear::MatchOp>,
        input: &linear::MatchResult,
        _to: Option<&linear::MatchOp>,
    ) -> io::Result<()> {
        let from = from.expect("we should have match op for every state");
        if let Some(x) = input.ok() {
            match from {
                linear::MatchOp::Opcode { .. } => {
                    let opcode = TOperator::try_from(x)
                        .map_err(|_| ())
                        .expect("we shouldn't generate non-opcode edges");
                    write!(w, "{:?}", opcode)
                }
                linear::MatchOp::ConditionCode { .. } => {
                    let cc = ConditionCode::try_from(x.get())
                        .expect("we shouldn't generate non-CC edges");
                    write!(w, "{}", cc)
                }
                linear::MatchOp::IntegerValue { .. } => {
                    let x = self.0.lookup(IntegerId(
                        NonZeroU16::new(x.get().try_into().unwrap()).unwrap(),
                    ));
                    write!(w, "{}", x)
                }
                _ => write!(w, "Ok({})", x),
            }
        } else {
            write!(w, "(else)")
        }
    }

    fn fmt_state(&self, w: &mut impl Write, op: &linear::MatchOp) -> io::Result<()> {
        use linear::MatchOp::*;

        write!(w, r#"<font face="monospace">"#)?;

        match op {
            Opcode(id) => write!(w, "opcode $lhs{}", id.0)?,
            IsConst(id) => write!(w, "is-const? $lhs{}", id.0)?,
            IsPowerOfTwo(id) => write!(w, "is-power-of-two? $lhs{}", id.0)?,
            BitWidth(id) => write!(w, "bit-width $lhs{}", id.0)?,
            FitsInNativeWord(id) => write!(w, "fits-in-native-word $lhs{}", id.0)?,
            Eq(a, b) => write!(w, "$lhs{} == $lhs{}", a.0, b.0)?,
            IntegerValue(id) => write!(w, "integer-value $lhs{}", id.0)?,
            BooleanValue(id) => write!(w, "boolean-value $lhs{}", id.0)?,
            ConditionCode(id) => write!(w, "condition-code $lhs{}", id.0)?,
            Nop => write!(w, "nop")?,
        }

        writeln!(w, "</font>")
    }

    fn fmt_output(
        &self,
        w: &mut impl Write,
        actions: &Box<[linear::Action<TOperator>]>,
    ) -> io::Result<()> {
        use linear::Action::*;

        if actions.is_empty() {
            return writeln!(w, "(no output)");
        }

        write!(w, r#"<font face="monospace">"#)?;

        for a in actions.iter() {
            match a {
                GetLhs { lhs } => write!(w, "get-lhs $lhs{}<br/>", lhs.0)?,
                UnaryUnquote { operator, operand } => {
                    write!(w, "eval {:?} $rhs{}<br/>", operator, operand.0)?
                }
                BinaryUnquote { operator, operands } => write!(
                    w,
                    "eval {:?} $rhs{}, $rhs{}<br/>",
                    operator, operands[0].0, operands[1].0,
                )?,
                MakeIntegerConst {
                    value,
                    bit_width: _,
                } => write!(w, "make {}<br/>", self.0.lookup(*value))?,
                MakeBooleanConst {
                    value,
                    bit_width: _,
                } => write!(w, "make {}<br/>", value)?,
                MakeConditionCode { cc } => write!(w, "{}<br/>", cc)?,
                MakeUnaryInst {
                    operand,
                    operator,
                    r#type: _,
                } => write!(w, "make {:?} $rhs{}<br/>", operator, operand.0,)?,
                MakeBinaryInst {
                    operator,
                    operands,
                    r#type: _,
                } => write!(
                    w,
                    "make {:?} $rhs{}, $rhs{}<br/>",
                    operator, operands[0].0, operands[1].0,
                )?,
                MakeTernaryInst {
                    operator,
                    operands,
                    r#type: _,
                } => write!(
                    w,
                    "make {:?} $rhs{}, $rhs{}, $rhs{}<br/>",
                    operator, operands[0].0, operands[1].0, operands[2].0,
                )?,
            }
        }

        writeln!(w, "</font>")
    }
}

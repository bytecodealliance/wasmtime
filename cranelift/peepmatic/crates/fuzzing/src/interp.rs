//! Interpreting compiled peephole optimizations against test instruction sequences.

use peepmatic::{
    Constraint, Dfs, DynAstRef, Optimizations, Pattern, Span, TraversalEvent, ValueLiteral,
    Variable,
};
use peepmatic_runtime::{
    cc::ConditionCode,
    part::Constant,
    r#type::BitWidth,
    r#type::{Kind, Type},
};
use peepmatic_test::{Program, TestIsa};
use peepmatic_test_operator::TestOperator;
use peepmatic_traits::{TypingContext as TypingContextTrait, TypingRules};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::str;

/// Compile the given source text, and if it is a valid set of optimizations,
/// then interpret the optimizations against test instruction sequences created
/// to reflect the optimizations.
pub fn interp(data: &[u8]) {
    let _ = env_logger::try_init();

    let source = match str::from_utf8(data) {
        Err(_) => return,
        Ok(s) => s,
    };

    let peep_opts = match peepmatic::compile_str(source, Path::new("fuzz")) {
        Err(_) => return,
        Ok(o) => o,
    };
    let mut optimizer = peep_opts.optimizer(TestIsa {
        native_word_size_in_bits: 32,
    });

    // Okay, we know it compiles and verifies alright, so (re)parse the AST.
    let buf = wast::parser::ParseBuffer::new(&source).unwrap();
    let ast = wast::parser::parse::<Optimizations<TestOperator>>(&buf).unwrap();

    // And we need access to the assigned types, so re-verify it as well.
    peepmatic::verify(&ast).unwrap();

    // Walk over each optimization and create an instruction sequence that
    // matches the optimization.
    let mut program = Program::default();
    for opt in &ast.optimizations {
        // The instruction sequence we generate must match an optimization (not
        // necessarily *this* optimization, if there is another that is more
        // specific but also matches) unless there is an `bit-width`
        // precondition or an implicit `bit-width` precondition via a type
        // ascription. When those things exist, we might have constructed
        // instructions with the wrong bit widths to match.
        let mut allow_no_match = false;

        // The last instruction we generated. After we've generated the full
        // instruction sequence, this will be its root.
        let mut last_inst = None;

        // Remember the instructions associated with variables and constants, so
        // that when they appear multiple times, we reuse the same instruction.
        let mut id_to_inst = HashMap::new();

        // Map from a pattern's span to the instruction we generated for
        // it. This allows parent operations to get the instructions for their
        // children.
        let mut span_to_inst = BTreeMap::new();

        for (te, lhs) in Dfs::new(&opt.lhs) {
            // NB: We use a post-order traversal because we want arguments to be
            // generated before they are used.
            if te != TraversalEvent::Exit {
                continue;
            }

            match lhs {
                DynAstRef::Precondition(p) => {
                    allow_no_match |= p.constraint == Constraint::BitWidth;
                }

                DynAstRef::Pattern(Pattern::Operation(op)) => {
                    allow_no_match |= op.r#type.get().is_some();

                    let num_imms = op.operator.immediates_arity() as usize;

                    // Generate this operation's immediates.
                    let mut imm_tys = vec![];
                    op.operator
                        .immediate_types((), &mut TypingContext, &mut imm_tys);
                    let imms: Vec<_> = op
                        .operands
                        .iter()
                        .take(num_imms)
                        .zip(imm_tys)
                        .map(|(pat, ty)| match pat {
                            Pattern::ValueLiteral(ValueLiteral::Integer(i)) => {
                                Constant::Int(i.value as _, BitWidth::ThirtyTwo).into()
                            }
                            Pattern::ValueLiteral(ValueLiteral::Boolean(b)) => {
                                Constant::Bool(b.value, BitWidth::One).into()
                            }
                            Pattern::ValueLiteral(ValueLiteral::ConditionCode(cc)) => cc.cc.into(),
                            Pattern::Constant(_) | Pattern::Variable(_) => match ty {
                                TypeOrConditionCode::ConditionCode => ConditionCode::Eq.into(),
                                TypeOrConditionCode::Type(ty) => match ty.kind {
                                    Kind::Int => Constant::Int(1, ty.bit_width).into(),
                                    Kind::Bool => Constant::Bool(false, ty.bit_width).into(),
                                    Kind::Void | Kind::CpuFlags => {
                                        unreachable!("void and cpu flags cannot be immediates")
                                    }
                                },
                            },
                            Pattern::Operation(_) => {
                                unreachable!("operations not allowed as immediates")
                            }
                        })
                        .collect();

                    // Generate (or collect already-generated) instructions for
                    // this operation's arguments.
                    let mut arg_tys = vec![];
                    op.operator
                        .parameter_types((), &mut TypingContext, &mut arg_tys);
                    let args: Vec<_> = op
                        .operands
                        .iter()
                        .skip(num_imms)
                        .zip(arg_tys)
                        .map(|(pat, ty)| match pat {
                            Pattern::Operation(op) => span_to_inst[&op.span()],
                            Pattern::ValueLiteral(ValueLiteral::Integer(i)) => program.r#const(
                                Constant::Int(i.value as _, BitWidth::ThirtyTwo),
                                BitWidth::ThirtyTwo,
                            ),
                            Pattern::ValueLiteral(ValueLiteral::Boolean(b)) => program.r#const(
                                Constant::Bool(b.value, BitWidth::One),
                                BitWidth::ThirtyTwo,
                            ),
                            Pattern::ValueLiteral(ValueLiteral::ConditionCode(_)) => {
                                unreachable!("condition codes cannot be arguments")
                            }
                            Pattern::Constant(peepmatic::Constant { id, .. })
                            | Pattern::Variable(Variable { id, .. }) => match ty {
                                TypeOrConditionCode::Type(ty) => {
                                    *id_to_inst.entry(id).or_insert_with(|| match ty.kind {
                                        Kind::Int => program.r#const(
                                            Constant::Int(1, ty.bit_width),
                                            BitWidth::ThirtyTwo,
                                        ),
                                        Kind::Bool => program.r#const(
                                            Constant::Bool(false, ty.bit_width),
                                            BitWidth::ThirtyTwo,
                                        ),
                                        Kind::CpuFlags => {
                                            unreachable!("cpu flags cannot be an argument")
                                        }
                                        Kind::Void => unreachable!("void cannot be an argument"),
                                    })
                                }
                                TypeOrConditionCode::ConditionCode => {
                                    unreachable!("condition codes cannot be arguments")
                                }
                            },
                        })
                        .collect();

                    let ty = match op.operator.result_type((), &mut TypingContext) {
                        TypeOrConditionCode::Type(ty) => ty,
                        TypeOrConditionCode::ConditionCode => {
                            unreachable!("condition codes cannot be operation results")
                        }
                    };
                    let inst = program.new_instruction(op.operator, ty, imms, args);
                    last_inst = Some(inst);
                    let old_inst = span_to_inst.insert(op.span(), inst);
                    assert!(old_inst.is_none());
                }
                _ => continue,
            }
        }

        // Run the optimizer on our newly generated instruction sequence.
        if let Some(inst) = last_inst {
            let replacement = optimizer.apply_one(&mut program, inst);
            assert!(
                replacement.is_some() || allow_no_match,
                "an optimization should match the generated instruction sequence"
            );
        }
    }

    // Finally, just try and run the optimizer on every instruction we
    // generated, just to potentially shake out some more bugs.
    let instructions: Vec<_> = program.instructions().map(|(k, _)| k).collect();
    for inst in instructions {
        let _ = optimizer.apply_one(&mut program, inst);
    }
}

enum TypeOrConditionCode {
    Type(Type),
    ConditionCode,
}

struct TypingContext;

impl<'a> TypingContextTrait<'a> for TypingContext {
    type Span = ();
    type TypeVariable = TypeOrConditionCode;

    fn cc(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::ConditionCode
    }

    fn bNN(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::Type(Type::b1())
    }

    fn iNN(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::Type(Type::i32())
    }

    fn iMM(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::Type(Type::i32())
    }

    fn cpu_flags(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::Type(Type::cpu_flags())
    }

    fn b1(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::Type(Type::b1())
    }

    fn void(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::Type(Type::void())
    }

    fn bool_or_int(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::Type(Type::b1())
    }

    fn any_t(&mut self, _: ()) -> Self::TypeVariable {
        TypeOrConditionCode::Type(Type::i32())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_interp() {
        crate::check(|s: Vec<u8>| interp(String::from_utf8_lossy(&s).as_bytes()));
    }

    #[test]
    fn regression_0() {
        interp(b"(=> (imul $x $x) $x)");
    }

    #[test]
    fn regression_1() {
        interp(b"(=> (when (imul $x $C) (is-power-of-two $C)) $x)");
    }

    #[test]
    fn regression_2() {
        interp(
            b"
            (=> (bor (bor $x $y) $x) (bor $x $y))
            (=> (bor (bor $x $C) 5) $x)
            ",
        );
    }

    #[test]
    fn regression_3() {
        interp(
            b"
            (=> (bor $y (bor $x 9)) $x)
            (=> (bor (bor $x $y) $x) $x)
            ",
        );
    }

    #[test]
    fn regression_4() {
        interp(
            b"
            (=> (bor $C 33) 0)
            (=> (bor $x 22) 1)
            (=> (bor $x 11) 2)
            ",
        );
    }

    #[test]
    fn regression_5() {
        interp(
            b"
            (=> (bor $y (bor $x $y)) (bor $x $y))
            (=> (bor (bor $x $y) $z) $x)
            (=> (bor (bor $x $y) $y) $x)
            ",
        );
    }

    #[test]
    fn regression_6() {
        interp(b"(=> (imul $x $f) of)");
    }

    #[test]
    fn regression_7() {
        interp(
            b"
            (=> (when (sdiv $x $C)
                      (fits-in-native-word $y))
                (sdiv $C $x))
            ",
        );
    }

    #[test]
    fn regression_8() {
        interp(
            b"
            (=> (adjust_sp_down $C) (adjust_sp_down_imm $C))
            ",
        );
    }

    #[test]
    fn regression_9() {
        interp(
            b"
            (=> (when $x) $x)
            (=> (trapnz $x) (trapnz $x))
            ",
        );
    }

    #[test]
    fn regression_10() {
        interp(b"(=> (sshr{i1} $x 0) $x)");
    }

    #[test]
    fn regression_11() {
        interp(
            b"
            (=> (when (ushr_imm $x (ishl 4 3))
                      (bit-width $x 64))
                (sextend{i64} (ireduce{i32} $x)))
            ",
        );
    }

    #[test]
    fn regression_12() {
        interp(b"(=> (band $C1 (band_imm $C1 1)) 1)");
    }

    #[test]
    fn regression_13() {
        interp(b"(=> (brz (icmp eq 0 $x)) (brz (ireduce{i32} $x)))");
    }

    #[test]
    fn regression_14() {
        interp(b"(=> (brz (icmp $E 0 $x)) (brz $x))");
    }
}

//! Converting Souper optimizations into Peepmatic DSL.
//!
//! Conversion from Souper into Peepmatic is implemented with a straightforward,
//! top-down recursive traversal of the optimization's left- and right-hand side
//! expression DAGs. Most Souper instructions have a corresponding Peepmatic
//! instruction. If we run into an instruction where that isn't the case, we
//! skip that Souper optimization and move on to the next one.
//!
//! Note that Souper fully supports DAGs, for example:
//!
//! ```text
//! %0 = var
//! %1 = add 1, %0
//! %2 = add %1, %1       ;; Two edges to `%1` makes this a DAG.
//! ```
//!
//! On the other hand, Peepmatic only currently supports trees, so shared
//! subexpressions are duplicated:
//!
//! ```text
//! (iadd (iadd 1 $x)
//!       (iadd 1 $x))    ;; The shared subexpression is duplicated.
//! ```
//!
//! This does not affect correctness.

#![deny(missing_docs)]

use anyhow::{Context, Result};
use souper_ir::ast;
use std::path::Path;

/// Maximum recursion depth, to avoid blowing the stack.
const MAX_DEPTH: u8 = 50;

/// Convert a file containing Souper optimizations into Peepmatic DSL.
pub fn convert_file(path: &Path) -> Result<String> {
    let souper = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read: {}", path.display()))?;
    convert_str(&souper, Some(path))
}

/// Convert a string of Souper optimizations into Peepmatic DSL.
///
/// The optional `filename` parameter is used for better error messages.
pub fn convert_str(souper: &str, filename: Option<&Path>) -> Result<String> {
    let mut peepmatic = String::new();

    let replacements = souper_ir::parse::parse_replacements_str(souper, filename)?;
    for replacement in replacements {
        let (statements, lhs, rhs) = match replacement {
            ast::Replacement::LhsRhs {
                statements,
                lhs,
                rhs,
            } => {
                if !lhs.attributes.is_empty() {
                    log::warn!("cannot translate Souper attributes to Peepmatic DSL");
                    continue;
                }
                (statements, lhs.value, rhs)
            }
            ast::Replacement::Cand { statements, cand } => {
                if !cand.attributes.is_empty() {
                    log::warn!("cannot translate Souper attributes to Peepmatic DSL");
                    continue;
                }
                let lhs = match cand.lhs {
                    ast::Operand::Value(v) => v,
                    ast::Operand::Constant(_) => {
                        log::warn!("optimization's LHS must not be a constant");
                        continue;
                    }
                };
                (statements, lhs, cand.rhs)
            }
        };

        if let Some(s) = convert_replacement(&statements, lhs, rhs) {
            peepmatic.push_str(&s);
            peepmatic.push('\n');
        }
    }

    Ok(peepmatic)
}

fn convert_replacement(
    statements: &ast::Arena<ast::Statement>,
    lhs: ast::ValueId,
    rhs: ast::Operand,
) -> Option<String> {
    let lhs = convert_lhs(statements, lhs)?;
    let rhs = convert_rhs(statements, rhs)?;
    Some(format!("(=> {}\n    {})\n", lhs, rhs))
}

fn convert_lhs(statements: &ast::Arena<ast::Statement>, lhs: ast::ValueId) -> Option<String> {
    let mut tys = vec![];
    let pattern = convert_operand(statements, lhs.into(), &mut tys, 0)?;

    Some(if tys.is_empty() {
        pattern
    } else {
        let mut lhs = format!("(when {}", pattern);
        for (name, width) in tys {
            lhs.push_str("\n        ");
            lhs.push_str(&format!("(bit-width ${} {})", name, width));
        }
        lhs.push(')');
        lhs
    })
}

fn convert_name(name: &str) -> String {
    debug_assert!(name.starts_with('%'));
    debug_assert!(name.len() >= 2);
    let c = name.chars().nth(1).unwrap();
    if 'a' <= c && c <= 'z' {
        name[1..].to_string()
    } else {
        format!("v{}", &name[1..])
    }
}

fn convert_operand(
    statements: &ast::Arena<ast::Statement>,
    value: ast::Operand,
    tys: &mut Vec<(String, u16)>,
    depth: u8,
) -> Option<String> {
    if depth > MAX_DEPTH {
        log::warn!("optimization too deep to translate recursively; skipping");
        return None;
    }

    let value = match value {
        ast::Operand::Value(v) => v,
        ast::Operand::Constant(c) => return Some(format!("{}", c.value)),
    };

    match &statements[value.into()] {
        ast::Statement::Pc(_) | ast::Statement::BlockPc(_) => {
            log::warn!("Peepmatic does not support path conditions yet");
            None
        }
        ast::Statement::Assignment(assn) => {
            debug_assert!(assn.name.starts_with('%'));

            if !assn.attributes.is_empty() {
                log::warn!("Peepmatic does not support attributes");
                return None;
            }

            match assn.value {
                ast::AssignmentRhs::Block(_)
                | ast::AssignmentRhs::Phi(_)
                | ast::AssignmentRhs::ReservedConst
                | ast::AssignmentRhs::ReservedInst => {
                    log::warn!("Peepmatic does not support {:?}", assn.value);
                    return None;
                }
                ast::AssignmentRhs::Var => {
                    if let Some(ast::Type { width }) = assn.r#type {
                        match width {
                            1 | 8 | 16 | 32 | 64 | 128 => {
                                tys.push((convert_name(&assn.name), width))
                            }
                            _ => {
                                log::warn!("unsupported bit width: {}", width);
                                return None;
                            }
                        }
                    }
                    Some(format!("${}", convert_name(&assn.name)))
                }
                ast::AssignmentRhs::Constant(c) => Some(format!("{}", c.value)),
                ast::AssignmentRhs::Instruction(inst) => match inst {
                    // Unsupported instructions.
                    ast::Instruction::Bswap { .. }
                    | ast::Instruction::SaddWithOverflow { .. }
                    | ast::Instruction::UaddWithOverflow { .. }
                    | ast::Instruction::SsubWithOverflow { .. }
                    | ast::Instruction::UsubWithOverflow { .. }
                    | ast::Instruction::SmulWithOverflow { .. }
                    | ast::Instruction::UmulWithOverflow { .. }
                    | ast::Instruction::Fshl { .. }
                    | ast::Instruction::Fshr { .. }
                    | ast::Instruction::ExtractValue { .. }
                    | ast::Instruction::Hole
                    | ast::Instruction::Freeze { .. } => {
                        log::warn!("Operation is not supported by Peepmatic: {:?}", inst);
                        return None;
                    }

                    // Comparison instructions return an `i1` in Souper but a
                    // `b1` in clif/Peepmatic. The `b1` needs to be extended
                    // into an `i{8,16,32,654,128}` for us to continue, so these
                    // instructions are special cased when handling `sext` and
                    // `zext` conversions.
                    ast::Instruction::Eq { .. }
                    | ast::Instruction::Ne { .. }
                    | ast::Instruction::Ult { .. }
                    | ast::Instruction::Slt { .. }
                    | ast::Instruction::Ule { .. }
                    | ast::Instruction::Sle { .. } => {
                        log::warn!("unsupported comparison");
                        return None;
                    }

                    // These instructions require type ascriptions.
                    ast::Instruction::Zext { a } => {
                        let width = require_width(assn.r#type)?;
                        let cmp = try_convert_comparison(statements, a, tys, depth)?;
                        if let Some(cmp) = cmp {
                            Some(format!("(bint {})", cmp))
                        } else {
                            convert_operation(statements, "uextend", &[a], Some(width), tys, depth)
                        }
                    }
                    ast::Instruction::Sext { a } => {
                        let width = require_width(assn.r#type)?;
                        let cmp = try_convert_comparison(statements, a, tys, depth)?;
                        if let Some(cmp) = cmp {
                            Some(format!("(bint {})", cmp))
                        } else {
                            convert_operation(statements, "sextend", &[a], Some(width), tys, depth)
                        }
                    }
                    ast::Instruction::Trunc { a } => {
                        let width = require_width(assn.r#type)?;
                        convert_operation(statements, "ireduce", &[a], Some(width), tys, depth)
                    }

                    ast::Instruction::Add { a, b }
                    | ast::Instruction::AddNsw { a, b }
                    | ast::Instruction::AddNuw { a, b }
                    | ast::Instruction::AddNw { a, b } => convert_commutative_operation(
                        statements, "iadd", "iadd_imm", a, b, tys, depth,
                    ),
                    ast::Instruction::Sub { a, b }
                    | ast::Instruction::SubNsw { a, b }
                    | ast::Instruction::SubNuw { a, b }
                    | ast::Instruction::SubNw { a, b } => {
                        if let ast::Operand::Constant(c) = b {
                            Some(format!(
                                "(iadd_imm -{} {})",
                                c.value,
                                convert_operand(statements, a, tys, depth + 1)?,
                            ))
                        } else if let ast::Operand::Constant(c) = a {
                            Some(format!(
                                "(irsub_imm {} {})",
                                c.value,
                                convert_operand(statements, b, tys, depth + 1)?,
                            ))
                        } else {
                            convert_operation(statements, "isub", &[a, b], None, tys, depth)
                        }
                    }
                    ast::Instruction::Mul { a, b }
                    | ast::Instruction::MulNsw { a, b }
                    | ast::Instruction::MulNuw { a, b }
                    | ast::Instruction::MulNw { a, b } => convert_commutative_operation(
                        statements, "imul", "imul_imm", a, b, tys, depth,
                    ),
                    ast::Instruction::Udiv { a, b } | ast::Instruction::UdivExact { a, b } => {
                        if let ast::Operand::Constant(c) = b {
                            Some(format!(
                                "(udiv_imm {} {})",
                                c.value,
                                convert_operand(statements, a, tys, depth + 1)?,
                            ))
                        } else {
                            convert_operation(statements, "udiv", &[a, b], None, tys, depth)
                        }
                    }
                    ast::Instruction::Sdiv { a, b } | ast::Instruction::SdivExact { a, b } => {
                        if let ast::Operand::Constant(c) = b {
                            Some(format!(
                                "(sdiv_imm {} {})",
                                c.value,
                                convert_operand(statements, a, tys, depth + 1)?,
                            ))
                        } else {
                            convert_operation(statements, "sdiv", &[a, b], None, tys, depth)
                        }
                    }
                    ast::Instruction::Urem { a, b } => {
                        if let ast::Operand::Constant(c) = b {
                            Some(format!(
                                "(urem_imm {} {})",
                                c.value,
                                convert_operand(statements, a, tys, depth + 1)?,
                            ))
                        } else {
                            convert_operation(statements, "urem", &[a, b], None, tys, depth)
                        }
                    }
                    ast::Instruction::Srem { a, b } => {
                        if let ast::Operand::Constant(c) = b {
                            Some(format!(
                                "(srem_imm {} {})",
                                c.value,
                                convert_operand(statements, a, tys, depth + 1)?,
                            ))
                        } else {
                            convert_operation(statements, "srem", &[a, b], None, tys, depth)
                        }
                    }
                    ast::Instruction::And { a, b } => convert_commutative_operation(
                        statements, "band", "band_imm", a, b, tys, depth,
                    ),
                    ast::Instruction::Or { a, b } => convert_commutative_operation(
                        statements, "bor", "bor_imm", a, b, tys, depth,
                    ),
                    ast::Instruction::Xor { a, b } => convert_commutative_operation(
                        statements, "bxor", "bxor_imm", a, b, tys, depth,
                    ),
                    ast::Instruction::Shl { a, b }
                    | ast::Instruction::ShlNsw { a, b }
                    | ast::Instruction::ShlNuw { a, b }
                    | ast::Instruction::ShlNw { a, b } => {
                        if let ast::Operand::Constant(c) = b {
                            Some(format!(
                                "(ishl_imm {} {})",
                                c.value,
                                convert_operand(statements, a, tys, depth + 1)?,
                            ))
                        } else {
                            convert_operation(statements, "ishl", &[a, b], None, tys, depth)
                        }
                    }
                    ast::Instruction::Lshr { a, b } | ast::Instruction::LshrExact { a, b } => {
                        if let ast::Operand::Constant(c) = b {
                            Some(format!(
                                "(ushr_imm {} {})",
                                c.value,
                                convert_operand(statements, a, tys, depth + 1)?,
                            ))
                        } else {
                            convert_operation(statements, "ushr", &[a, b], None, tys, depth)
                        }
                    }
                    ast::Instruction::Ashr { a, b } | ast::Instruction::AshrExact { a, b } => {
                        if let ast::Operand::Constant(c) = b {
                            Some(format!(
                                "(sshr_imm {} {})",
                                c.value,
                                convert_operand(statements, a, tys, depth + 1)?,
                            ))
                        } else {
                            convert_operation(statements, "sshr", &[a, b], None, tys, depth)
                        }
                    }
                    ast::Instruction::Select { a, b, c } => {
                        convert_operation(statements, "select", &[a, b, c], None, tys, depth)
                    }
                    ast::Instruction::Ctpop { a } => {
                        convert_operation(statements, "popcnt", &[a], None, tys, depth)
                    }
                    ast::Instruction::BitReverse { a } => {
                        convert_operation(statements, "bitrev", &[a], None, tys, depth)
                    }
                    ast::Instruction::Cttz { a } => {
                        convert_operation(statements, "ctz", &[a], None, tys, depth)
                    }
                    ast::Instruction::Ctlz { a } => {
                        convert_operation(statements, "clz", &[a], None, tys, depth)
                    }
                    ast::Instruction::SaddSat { a, b } => {
                        convert_operation(statements, "sadd_sat", &[a, b], None, tys, depth)
                    }
                    ast::Instruction::UaddSat { a, b } => {
                        convert_operation(statements, "uadd_sat", &[a, b], None, tys, depth)
                    }
                    ast::Instruction::SsubSat { a, b } => {
                        convert_operation(statements, "ssub_sat", &[a, b], None, tys, depth)
                    }
                    ast::Instruction::UsubSat { a, b } => {
                        convert_operation(statements, "usub_sat", &[a, b], None, tys, depth)
                    }
                },
            }
        }
    }
}

/// Try and convert `value` into an `icmp` comparison.
///
/// Returns `Some(Some(icmp))` if the conversion is successful.
///
/// Returns `Some(None)` if `value` is not a comparison.
///
/// Returns `None` in the case where `value` cannot be converted to Peepmatic
/// DSL at all.
fn try_convert_comparison(
    statements: &ast::Arena<ast::Statement>,
    value: ast::Operand,
    tys: &mut Vec<(String, u16)>,
    depth: u8,
) -> Option<Option<String>> {
    let value = match value {
        ast::Operand::Value(v) => v,
        ast::Operand::Constant(_) => return None,
    };
    let assn = match &statements[value.into()] {
        ast::Statement::Assignment(a) => a,
        _ => return None,
    };
    Some(match assn.value {
        ast::AssignmentRhs::Instruction(ast::Instruction::Eq { a, b }) => Some(convert_operation(
            statements,
            "icmp eq",
            &[a, b],
            None,
            tys,
            depth,
        )?),
        ast::AssignmentRhs::Instruction(ast::Instruction::Ne { a, b }) => Some(convert_operation(
            statements,
            "icmp ne",
            &[a, b],
            None,
            tys,
            depth,
        )?),
        ast::AssignmentRhs::Instruction(ast::Instruction::Ult { a, b }) => Some(convert_operation(
            statements,
            "icmp ult",
            &[a, b],
            None,
            tys,
            depth,
        )?),
        ast::AssignmentRhs::Instruction(ast::Instruction::Slt { a, b }) => Some(convert_operation(
            statements,
            "icmp slt",
            &[a, b],
            None,
            tys,
            depth,
        )?),
        ast::AssignmentRhs::Instruction(ast::Instruction::Ule { a, b }) => Some(convert_operation(
            statements,
            "icmp ule",
            &[a, b],
            None,
            tys,
            depth,
        )?),
        ast::AssignmentRhs::Instruction(ast::Instruction::Sle { a, b }) => Some(convert_operation(
            statements,
            "icmp sle",
            &[a, b],
            None,
            tys,
            depth,
        )?),
        _ => None,
    })
}

fn require_width(ty: Option<ast::Type>) -> Option<u16> {
    match ty {
        Some(ast::Type { width: w @ 8 })
        | Some(ast::Type { width: w @ 16 })
        | Some(ast::Type { width: w @ 32 })
        | Some(ast::Type { width: w @ 64 })
        | Some(ast::Type { width: w @ 128 }) => Some(w),
        Some(ty) => {
            log::warn!("unsupported bit width: {}", ty.width);
            None
        }
        None => {
            log::warn!("required bit width is missing");
            None
        }
    }
}

fn convert_operation(
    statements: &ast::Arena<ast::Statement>,
    operator: &str,
    operands: &[ast::Operand],
    ty: Option<u16>,
    tys: &mut Vec<(String, u16)>,
    depth: u8,
) -> Option<String> {
    let mut op = format!("({}", operator);

    if let Some(width) = ty {
        op.push_str(&format!(" {{i{}}}", width));
    }

    for operand in operands {
        op.push(' ');
        let operand = convert_operand(statements, *operand, tys, depth + 1)?;
        op.push_str(&operand);
    }
    op.push(')');
    Some(op)
}

/// Convert a commutative operation, using the `_imm` form if any of its
/// operands is a constant.
fn convert_commutative_operation(
    statements: &ast::Arena<ast::Statement>,
    operator: &str,
    operator_imm: &str,
    a: ast::Operand,
    b: ast::Operand,
    tys: &mut Vec<(String, u16)>,
    depth: u8,
) -> Option<String> {
    Some(match (a, b) {
        (ast::Operand::Constant(c), _) => format!(
            "({} {} {})",
            operator_imm,
            c.value,
            convert_operand(statements, b, tys, depth + 1)?,
        ),
        (_, ast::Operand::Constant(c)) => format!(
            "({} {} {})",
            operator_imm,
            c.value,
            convert_operand(statements, a, tys, depth + 1)?,
        ),
        _ => format!(
            "({} {} {})",
            operator,
            convert_operand(statements, a, tys, depth + 1)?,
            convert_operand(statements, b, tys, depth + 1)?,
        ),
    })
}

fn convert_rhs(statements: &ast::Arena<ast::Statement>, rhs: ast::Operand) -> Option<String> {
    let mut tys = vec![];
    convert_operand(statements, rhs, &mut tys, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use peepmatic_test_operator::TestOperator;

    fn assert_converts(name: &str, souper: &str, expected: &str) {
        let expected = expected.trim();
        eprintln!("expected:\n{}", expected);

        let actual = convert_str(souper, Some(Path::new(name))).unwrap();
        let actual = actual.trim();
        eprintln!("actual:\n{}", actual);

        assert_eq!(expected, actual);

        // Assert that the generated Peepmatic DSL parses and verifies.
        let buf = wast::parser::ParseBuffer::new(actual).expect("peepmatic DSL should lex OK");
        let opts = match wast::parser::parse::<peepmatic::Optimizations<TestOperator>>(&buf) {
            Ok(opts) => opts,
            Err(mut e) => {
                e.set_path(Path::new(name));
                e.set_text(actual);
                eprintln!("{}", e);
                panic!("peepmatic DSL should parse OK")
            }
        };
        if let Err(mut e) = peepmatic::verify(&opts) {
            e.set_path(Path::new(name));
            e.set_text(actual);
            eprintln!("{}", e);
            panic!("peepmatic DSL should verify OK")
        }
    }

    macro_rules! test {
        ( $(
            $name:ident => converts($souper:expr, $peepmatic:expr $(,)? );
        )* ) => {
            $(
                #[test]
                fn $name() {
                    assert_converts(stringify!($name), $souper, $peepmatic);
                }
            )*
        };
    }

    test! {
        simple_lhs_rhs => converts(
            "
                %0 = var
                %1 = mul %0, 2
                infer %1
                %2 = shl %0, 1
                result %2
            ",
            "\
(=> (imul_imm 2 $v0)
    (ishl_imm 1 $v0))",
        );

        simple_cand => converts(
            "
                %0 = var
                %1 = mul %0, 2
                %2 = shl %0, 1
                cand %1 %2
            ",
            "\
(=> (imul_imm 2 $v0)
    (ishl_imm 1 $v0))",
        );

        // These instructions require type ascriptions, so our conversion better
        // have them.
        trunc => converts(
            "
                %0:i64 = var
                %1:i32 = trunc %0
                %2:i32 = 0
                cand %1 %2
            ",
            "\
(=> (when (ireduce {i32} $v0)
        (bit-width $v0 64))
    0)",
        );
        sext => converts(
            "
                %0:i32 = var
                %1:i64 = sext %0
                %2:i64 = 0
                cand %1 %2
            ",
            "\
(=> (when (sextend {i64} $v0)
        (bit-width $v0 32))
    0)",
        );
        zext => converts(
            "
                %0:i32 = var
                %1:i64 = zext %0
                %2:i64 = 0
                cand %1 %2
            ",
            "\
(=> (when (uextend {i64} $v0)
        (bit-width $v0 32))
    0)",
        );

        // Type annotations on intermediate values (e.g. on %1, %2, and %3) do
        // not turn into type ascriptions in the Peepmatic DSL.
        unnecessary_types => converts(
            "
                %0:i32 = var
                %1:i32 = add 1, %0
                %2:i32 = add 1, %1
                %3:i32 = add 1, %2
                %4:i32 = add 3, %0
                cand %3 %4
            ",
            "\
(=> (when (iadd_imm 1 (iadd_imm 1 (iadd_imm 1 $v0)))
        (bit-width $v0 32))
    (iadd_imm 3 $v0))",
        );

        // Comparisons need to add a `bint` instruction in Peepmatic, since clif
        // has a separate `b1` type that needs to be extended into an integer.
        comparison_has_bint => converts(
            "
                %0:i32 = var
                %1:i32 = var
                %2:i1 = eq %0, %1
                %3:i32 = zext %2
                %4:i32 = 0
                cand %3 %4
            ",
            "\
(=> (when (bint (icmp eq $v0 $v1))
        (bit-width $v0 32)
        (bit-width $v1 32))
    0)",
        );

        // We correctly introduce `_imm` variants of instructions, regardless of
        // which side the constant is on for commutative instructions.
        iadd_imm_right => converts(
            "
                %0:i32 = var
                %1:i32 = add %0, 1
                %2:i32 = 0
                cand %1 %2
            ",
            "\
(=> (when (iadd_imm 1 $v0)
        (bit-width $v0 32))
    0)"
        );
        iadd_imm_left => converts(
            "
                %0:i32 = var
                %1:i32 = add 1, %0
                %2:i32 = 0
                cand %1 %2
            ",
            "\
(=> (when (iadd_imm 1 $v0)
        (bit-width $v0 32))
    0)"
        );
    }
}

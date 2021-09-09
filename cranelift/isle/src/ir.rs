//! Lowered matching IR.

use crate::declare_id;
use crate::lexer::Pos;
use crate::sema::*;
use std::collections::HashMap;

declare_id!(InstId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// A value produced by an instruction in the Pattern (LHS).
    Pattern { inst: InstId, output: usize },
    /// A value produced by an instruction in the Expr (RHS).
    Expr { inst: InstId, output: usize },
}

/// A single Pattern instruction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PatternInst {
    /// Get the Nth input argument, which corresponds to the Nth field
    /// of the root term.
    Arg { index: usize, ty: TypeId },

    /// Match a value as equal to another value. Produces no values.
    MatchEqual { a: Value, b: Value, ty: TypeId },

    /// Try matching the given value as the given integer. Produces no values.
    MatchInt {
        input: Value,
        ty: TypeId,
        int_val: i64,
    },

    /// Try matching the given value as the given variant, producing
    /// `|arg_tys|` values as output.
    MatchVariant {
        input: Value,
        input_ty: TypeId,
        arg_tys: Vec<TypeId>,
        variant: VariantId,
    },

    /// Invoke an extractor, taking the given values as input (the
    /// first is the value to extract, the other are the
    /// `Input`-polarity extractor args) and producing an output valu
    /// efor each `Output`-polarity extractor arg.
    Extract {
        inputs: Vec<Value>,
        input_tys: Vec<TypeId>,
        output_tys: Vec<TypeId>,
        term: TermId,
        infallible: bool,
    },

    /// Evaluate an expression and provide the given value as the
    /// result of this match instruction. The expression has access to
    /// the pattern-values up to this point in the sequence.
    Expr {
        seq: ExprSequence,
        output: Value,
        output_ty: TypeId,
    },
}

/// A single Expr instruction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExprInst {
    /// Produce a constant integer.
    ConstInt { ty: TypeId, val: i64 },

    /// Create a variant.
    CreateVariant {
        inputs: Vec<(Value, TypeId)>,
        ty: TypeId,
        variant: VariantId,
    },

    /// Invoke a constructor.
    Construct {
        inputs: Vec<(Value, TypeId)>,
        ty: TypeId,
        term: TermId,
        infallible: bool,
    },

    /// Set the Nth return value. Produces no values.
    Return {
        index: usize,
        ty: TypeId,
        value: Value,
    },
}

impl ExprInst {
    pub fn visit_values<F: FnMut(Value)>(&self, mut f: F) {
        match self {
            &ExprInst::ConstInt { .. } => {}
            &ExprInst::Construct { ref inputs, .. }
            | &ExprInst::CreateVariant { ref inputs, .. } => {
                for (input, _ty) in inputs {
                    f(*input);
                }
            }
            &ExprInst::Return { value, .. } => {
                f(value);
            }
        }
    }
}

/// A linear sequence of instructions that match on and destructure an
/// argument. A pattern is fallible (may not match). If it does not
/// fail, its result consists of the values produced by the
/// `PatternInst`s, which may be used by a subsequent `Expr`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PatternSequence {
    /// Instruction sequence for pattern. InstId indexes into this
    /// sequence for `Value::Pattern` values.
    pub insts: Vec<PatternInst>,
}

/// A linear sequence of instructions that produce a new value from
/// the right-hand side of a rule, given bindings that come from a
/// `Pattern` derived from the left-hand side.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct ExprSequence {
    /// Instruction sequence for expression. InstId indexes into this
    /// sequence for `Value::Expr` values.
    pub insts: Vec<ExprInst>,
    /// Position at which the rule producing this sequence was located.
    pub pos: Pos,
}

impl ExprSequence {
    pub fn is_const_int(&self) -> Option<(TypeId, i64)> {
        if self.insts.len() == 2 && matches!(&self.insts[1], &ExprInst::Return { .. }) {
            match &self.insts[0] {
                &ExprInst::ConstInt { ty, val } => Some((ty, val)),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn is_const_variant(&self) -> Option<(TypeId, VariantId)> {
        if self.insts.len() == 2 && matches!(&self.insts[1], &ExprInst::Return { .. }) {
            match &self.insts[0] {
                &ExprInst::CreateVariant {
                    ref inputs,
                    ty,
                    variant,
                } if inputs.len() == 0 => Some((ty, variant)),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ValueOrArgs {
    Value(Value),
    ImplicitTermFromArgs(TermId),
}

impl ValueOrArgs {
    fn to_value(&self) -> Option<Value> {
        match self {
            &ValueOrArgs::Value(v) => Some(v),
            _ => None,
        }
    }
}

impl PatternSequence {
    fn add_inst(&mut self, inst: PatternInst) -> InstId {
        let id = InstId(self.insts.len());
        self.insts.push(inst);
        id
    }

    fn add_arg(&mut self, index: usize, ty: TypeId) -> Value {
        let inst = InstId(self.insts.len());
        self.add_inst(PatternInst::Arg { index, ty });
        Value::Pattern { inst, output: 0 }
    }

    fn add_match_equal(&mut self, a: Value, b: Value, ty: TypeId) {
        self.add_inst(PatternInst::MatchEqual { a, b, ty });
    }

    fn add_match_int(&mut self, input: Value, ty: TypeId, int_val: i64) {
        self.add_inst(PatternInst::MatchInt { input, ty, int_val });
    }

    fn add_match_variant(
        &mut self,
        input: Value,
        input_ty: TypeId,
        arg_tys: &[TypeId],
        variant: VariantId,
    ) -> Vec<Value> {
        let inst = InstId(self.insts.len());
        let mut outs = vec![];
        for (i, _arg_ty) in arg_tys.iter().enumerate() {
            let val = Value::Pattern { inst, output: i };
            outs.push(val);
        }
        let arg_tys = arg_tys.iter().cloned().collect();
        self.add_inst(PatternInst::MatchVariant {
            input,
            input_ty,
            arg_tys,
            variant,
        });
        outs
    }

    fn add_extract(
        &mut self,
        inputs: Vec<Value>,
        input_tys: Vec<TypeId>,
        output_tys: Vec<TypeId>,
        term: TermId,
        infallible: bool,
    ) -> Vec<Value> {
        let inst = InstId(self.insts.len());
        let mut outs = vec![];
        for i in 0..output_tys.len() {
            let val = Value::Pattern { inst, output: i };
            outs.push(val);
        }
        let output_tys = output_tys.iter().cloned().collect();
        self.add_inst(PatternInst::Extract {
            inputs,
            input_tys,
            output_tys,
            term,
            infallible,
        });
        outs
    }

    fn add_expr_seq(&mut self, seq: ExprSequence, output: Value, output_ty: TypeId) -> Value {
        let inst = self.add_inst(PatternInst::Expr {
            seq,
            output,
            output_ty,
        });

        // Create values for all outputs.
        Value::Pattern { inst, output: 0 }
    }

    /// Generate PatternInsts to match the given (sub)pattern. Works
    /// recursively down the AST.
    fn gen_pattern(
        &mut self,
        input: ValueOrArgs,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        pat: &Pattern,
        vars: &mut HashMap<VarId, Value>,
    ) {
        match pat {
            &Pattern::BindPattern(_ty, var, ref subpat) => {
                // Bind the appropriate variable and recurse.
                assert!(!vars.contains_key(&var));
                if let Some(v) = input.to_value() {
                    vars.insert(var, v);
                }
                let root_term = self.gen_pattern(input, typeenv, termenv, &*subpat, vars);
                root_term
            }
            &Pattern::Var(ty, var) => {
                // Assert that the value matches the existing bound var.
                let var_val = vars
                    .get(&var)
                    .cloned()
                    .expect("Variable should already be bound");
                let input_val = input
                    .to_value()
                    .expect("Cannot match an =var pattern against root term");
                self.add_match_equal(input_val, var_val, ty);
            }
            &Pattern::ConstInt(ty, value) => {
                // Assert that the value matches the constant integer.
                let input_val = input
                    .to_value()
                    .expect("Cannot match an =var pattern against root term");
                self.add_match_int(input_val, ty, value);
            }
            &Pattern::Term(ty, term, ref args) => {
                match input {
                    ValueOrArgs::ImplicitTermFromArgs(termid) => {
                        assert_eq!(
                            termid, term,
                            "Cannot match a different term against root pattern"
                        );
                        let termdata = &termenv.terms[term.index()];
                        let arg_tys = &termdata.arg_tys[..];
                        for (i, subpat) in args.iter().enumerate() {
                            let value = self.add_arg(i, arg_tys[i]);
                            let subpat = match subpat {
                                &TermArgPattern::Expr(..) => {
                                    panic!("Should have been caught in typechecking")
                                }
                                &TermArgPattern::Pattern(ref pat) => pat,
                            };
                            self.gen_pattern(
                                ValueOrArgs::Value(value),
                                typeenv,
                                termenv,
                                subpat,
                                vars,
                            );
                        }
                    }
                    ValueOrArgs::Value(input) => {
                        // Determine whether the term has an external extractor or not.
                        let termdata = &termenv.terms[term.index()];
                        let arg_tys = &termdata.arg_tys[..];
                        match &termdata.kind {
                            &TermKind::Declared => {
                                panic!("Pattern invocation of undefined term body");
                            }
                            &TermKind::EnumVariant { variant } => {
                                let arg_values =
                                    self.add_match_variant(input, ty, arg_tys, variant);
                                for (subpat, value) in args.iter().zip(arg_values.into_iter()) {
                                    let subpat = match subpat {
                                        &TermArgPattern::Pattern(ref pat) => pat,
                                        _ => unreachable!("Should have been caught by sema"),
                                    };
                                    self.gen_pattern(
                                        ValueOrArgs::Value(value),
                                        typeenv,
                                        termenv,
                                        subpat,
                                        vars,
                                    );
                                }
                            }
                            &TermKind::InternalConstructor
                            | &TermKind::ExternalConstructor { .. } => {
                                panic!("Should not invoke constructor in pattern");
                            }
                            &TermKind::InternalExtractor { .. } => {
                                panic!("Should have been expanded away");
                            }
                            &TermKind::ExternalExtractor {
                                ref arg_polarity,
                                infallible,
                                ..
                            } => {
                                // Evaluate all `input` args.
                                let mut inputs = vec![];
                                let mut input_tys = vec![];
                                let mut output_tys = vec![];
                                let mut output_pats = vec![];
                                inputs.push(input);
                                input_tys.push(termdata.ret_ty);
                                for (arg, pol) in args.iter().zip(arg_polarity.iter()) {
                                    match pol {
                                        &ArgPolarity::Input => {
                                            let expr = match arg {
                                                &TermArgPattern::Expr(ref expr) => expr,
                                                _ => panic!(
                                                    "Should have been caught by typechecking"
                                                ),
                                            };
                                            let mut seq = ExprSequence::default();
                                            let value = seq.gen_expr(typeenv, termenv, expr, vars);
                                            seq.add_return(expr.ty(), value);
                                            let value = self.add_expr_seq(seq, value, expr.ty());
                                            inputs.push(value);
                                            input_tys.push(expr.ty());
                                        }
                                        &ArgPolarity::Output => {
                                            let pat = match arg {
                                                &TermArgPattern::Pattern(ref pat) => pat,
                                                _ => panic!(
                                                    "Should have been caught by typechecking"
                                                ),
                                            };
                                            output_tys.push(pat.ty());
                                            output_pats.push(pat);
                                        }
                                    }
                                }

                                // Invoke the extractor.
                                let arg_values = self
                                    .add_extract(inputs, input_tys, output_tys, term, infallible);

                                for (pat, &val) in output_pats.iter().zip(arg_values.iter()) {
                                    self.gen_pattern(
                                        ValueOrArgs::Value(val),
                                        typeenv,
                                        termenv,
                                        pat,
                                        vars,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            &Pattern::And(_ty, ref children) => {
                for child in children {
                    self.gen_pattern(input, typeenv, termenv, child, vars);
                }
            }
            &Pattern::Wildcard(_ty) => {
                // Nothing!
            }
        }
    }
}

impl ExprSequence {
    fn add_inst(&mut self, inst: ExprInst) -> InstId {
        let id = InstId(self.insts.len());
        self.insts.push(inst);
        id
    }

    fn add_const_int(&mut self, ty: TypeId, val: i64) -> Value {
        let inst = InstId(self.insts.len());
        self.add_inst(ExprInst::ConstInt { ty, val });
        Value::Expr { inst, output: 0 }
    }

    fn add_create_variant(
        &mut self,
        inputs: &[(Value, TypeId)],
        ty: TypeId,
        variant: VariantId,
    ) -> Value {
        let inst = InstId(self.insts.len());
        let inputs = inputs.iter().cloned().collect();
        self.add_inst(ExprInst::CreateVariant {
            inputs,
            ty,
            variant,
        });
        Value::Expr { inst, output: 0 }
    }

    fn add_construct(
        &mut self,
        inputs: &[(Value, TypeId)],
        ty: TypeId,
        term: TermId,
        infallible: bool,
    ) -> Value {
        let inst = InstId(self.insts.len());
        let inputs = inputs.iter().cloned().collect();
        self.add_inst(ExprInst::Construct {
            inputs,
            ty,
            term,
            infallible,
        });
        Value::Expr { inst, output: 0 }
    }

    fn add_return(&mut self, ty: TypeId, value: Value) {
        self.add_inst(ExprInst::Return {
            index: 0,
            ty,
            value,
        });
    }

    fn add_multi_return(&mut self, index: usize, ty: TypeId, value: Value) {
        self.add_inst(ExprInst::Return { index, ty, value });
    }

    /// Creates a sequence of ExprInsts to generate the given
    /// expression value. Returns the value ID as well as the root
    /// term ID, if any.
    fn gen_expr(
        &mut self,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        expr: &Expr,
        vars: &HashMap<VarId, Value>,
    ) -> Value {
        log::trace!("gen_expr: expr {:?}", expr);
        match expr {
            &Expr::ConstInt(ty, val) => self.add_const_int(ty, val),
            &Expr::Let(_ty, ref bindings, ref subexpr) => {
                let mut vars = vars.clone();
                for &(var, _var_ty, ref var_expr) in bindings {
                    let var_value = self.gen_expr(typeenv, termenv, &*var_expr, &vars);
                    vars.insert(var, var_value);
                }
                self.gen_expr(typeenv, termenv, &*subexpr, &vars)
            }
            &Expr::Var(_ty, var_id) => vars.get(&var_id).cloned().unwrap(),
            &Expr::Term(ty, term, ref arg_exprs) => {
                let termdata = &termenv.terms[term.index()];
                let mut arg_values_tys = vec![];
                for (arg_ty, arg_expr) in termdata.arg_tys.iter().cloned().zip(arg_exprs.iter()) {
                    arg_values_tys
                        .push((self.gen_expr(typeenv, termenv, &*arg_expr, &vars), arg_ty));
                }
                match &termdata.kind {
                    &TermKind::EnumVariant { variant } => {
                        self.add_create_variant(&arg_values_tys[..], ty, variant)
                    }
                    &TermKind::InternalConstructor => {
                        self.add_construct(
                            &arg_values_tys[..],
                            ty,
                            term,
                            /* infallible = */ true,
                        )
                    }
                    &TermKind::ExternalConstructor { .. } => {
                        self.add_construct(
                            &arg_values_tys[..],
                            ty,
                            term,
                            /* infallible = */ false,
                        )
                    }
                    _ => panic!("Should have been caught by typechecking"),
                }
            }
        }
    }
}

/// Build a sequence from a rule.
pub fn lower_rule(
    tyenv: &TypeEnv,
    termenv: &TermEnv,
    rule: RuleId,
) -> (PatternSequence, ExprSequence) {
    let mut pattern_seq: PatternSequence = Default::default();
    let mut expr_seq: ExprSequence = Default::default();
    expr_seq.pos = termenv.rules[rule.index()].pos;

    let ruledata = &termenv.rules[rule.index()];
    let mut vars = HashMap::new();
    let root_term = ruledata
        .lhs
        .root_term()
        .expect("Pattern must have a term at the root");

    log::trace!("lower_rule: ruledata {:?}", ruledata,);

    // Lower the pattern, starting from the root input value.
    pattern_seq.gen_pattern(
        ValueOrArgs::ImplicitTermFromArgs(root_term),
        tyenv,
        termenv,
        &ruledata.lhs,
        &mut vars,
    );

    // Lower the expression, making use of the bound variables
    // from the pattern.
    let rhs_root_val = expr_seq.gen_expr(tyenv, termenv, &ruledata.rhs, &vars);
    // Return the root RHS value.
    let output_ty = ruledata.rhs.ty();
    expr_seq.add_return(output_ty, rhs_root_val);
    (pattern_seq, expr_seq)
}

//! Lowered matching IR.

use crate::declare_id;
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
    /// Get the input root-term value.
    Arg { ty: TypeId },

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

    /// Invoke an extractor, taking the given value as input and
    /// producing `|arg_tys|` values as output.
    Extract {
        input: Value,
        input_ty: TypeId,
        arg_tys: Vec<TypeId>,
        term: TermId,
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
    },

    /// Set the return value. Produces no values.
    Return { ty: TypeId, value: Value },
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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ExprSequence {
    /// Instruction sequence for expression. InstId indexes into this
    /// sequence for `Value::Expr` values.
    pub insts: Vec<ExprInst>,
}

impl PatternSequence {
    fn add_inst(&mut self, inst: PatternInst) -> InstId {
        let id = InstId(self.insts.len());
        self.insts.push(inst);
        id
    }

    fn add_arg(&mut self, ty: TypeId) -> Value {
        let inst = InstId(self.insts.len());
        self.add_inst(PatternInst::Arg { ty });
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
        input: Value,
        input_ty: TypeId,
        arg_tys: &[TypeId],
        term: TermId,
    ) -> Vec<Value> {
        let inst = InstId(self.insts.len());
        let mut outs = vec![];
        for (i, _arg_ty) in arg_tys.iter().enumerate() {
            let val = Value::Pattern { inst, output: i };
            outs.push(val);
        }
        let arg_tys = arg_tys.iter().cloned().collect();
        self.add_inst(PatternInst::Extract {
            input,
            input_ty,
            arg_tys,
            term,
        });
        outs
    }

    /// Generate PatternInsts to match the given (sub)pattern. Works
    /// recursively down the AST. Returns the root term matched by
    /// this pattern, if any.
    fn gen_pattern(
        &mut self,
        input: Value,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        pat: &Pattern,
        vars: &mut HashMap<VarId, (Option<TermId>, Value)>,
    ) -> Option<TermId> {
        match pat {
            &Pattern::BindPattern(_ty, var, ref subpat) => {
                // Bind the appropriate variable and recurse.
                assert!(!vars.contains_key(&var));
                vars.insert(var, (None, input)); // bind first, so subpat can use it
                let root_term = self.gen_pattern(input, typeenv, termenv, &*subpat, vars);
                vars.get_mut(&var).unwrap().0 = root_term;
                root_term
            }
            &Pattern::Var(ty, var) => {
                // Assert that the value matches the existing bound var.
                let (var_val_term, var_val) = vars
                    .get(&var)
                    .cloned()
                    .expect("Variable should already be bound");
                self.add_match_equal(input, var_val, ty);
                var_val_term
            }
            &Pattern::ConstInt(ty, value) => {
                // Assert that the value matches the constant integer.
                self.add_match_int(input, ty, value);
                None
            }
            &Pattern::Term(ty, term, ref args) => {
                // Determine whether the term has an external extractor or not.
                let termdata = &termenv.terms[term.index()];
                let arg_tys = &termdata.arg_tys[..];
                match &termdata.kind {
                    &TermKind::EnumVariant { variant } => {
                        let arg_values = self.add_match_variant(input, ty, arg_tys, variant);
                        for (subpat, value) in args.iter().zip(arg_values.into_iter()) {
                            self.gen_pattern(value, typeenv, termenv, subpat, vars);
                        }
                        None
                    }
                    &TermKind::Regular { .. } => {
                        let arg_values = self.add_extract(input, ty, arg_tys, term);
                        for (subpat, value) in args.iter().zip(arg_values.into_iter()) {
                            self.gen_pattern(value, typeenv, termenv, subpat, vars);
                        }
                        Some(term)
                    }
                }
            }
            &Pattern::Wildcard(_ty) => {
                // Nothing!
                None
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

    fn add_construct(&mut self, inputs: &[(Value, TypeId)], ty: TypeId, term: TermId) -> Value {
        let inst = InstId(self.insts.len());
        let inputs = inputs.iter().cloned().collect();
        self.add_inst(ExprInst::Construct { inputs, ty, term });
        Value::Expr { inst, output: 0 }
    }

    fn add_return(&mut self, ty: TypeId, value: Value) {
        self.add_inst(ExprInst::Return { ty, value });
    }

    /// Creates a sequence of ExprInsts to generate the given
    /// expression value. Returns the value ID as well as the root
    /// term ID, if any.
    fn gen_expr(
        &mut self,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        expr: &Expr,
        vars: &HashMap<VarId, (Option<TermId>, Value)>,
    ) -> (Option<TermId>, Value) {
        match expr {
            &Expr::ConstInt(ty, val) => (None, self.add_const_int(ty, val)),
            &Expr::Let(_ty, ref bindings, ref subexpr) => {
                let mut vars = vars.clone();
                for &(var, _var_ty, ref var_expr) in bindings {
                    let (var_value_term, var_value) =
                        self.gen_expr(typeenv, termenv, &*var_expr, &vars);
                    vars.insert(var, (var_value_term, var_value));
                }
                self.gen_expr(typeenv, termenv, &*subexpr, &vars)
            }
            &Expr::Var(_ty, var_id) => vars.get(&var_id).cloned().unwrap(),
            &Expr::Term(ty, term, ref arg_exprs) => {
                let termdata = &termenv.terms[term.index()];
                let mut arg_values_tys = vec![];
                for (arg_ty, arg_expr) in termdata.arg_tys.iter().cloned().zip(arg_exprs.iter()) {
                    arg_values_tys
                        .push((self.gen_expr(typeenv, termenv, &*arg_expr, &vars).1, arg_ty));
                }
                match &termdata.kind {
                    &TermKind::EnumVariant { variant } => (
                        None,
                        self.add_create_variant(&arg_values_tys[..], ty, variant),
                    ),
                    &TermKind::Regular { .. } => (
                        Some(termdata.id),
                        self.add_construct(&arg_values_tys[..], ty, term),
                    ),
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
) -> (
    Option<TermId>,
    PatternSequence,
    Option<TermId>,
    ExprSequence,
) {
    let mut pattern_seq: PatternSequence = Default::default();
    let mut expr_seq: ExprSequence = Default::default();

    // Lower the pattern, starting from the root input value.
    let ruledata = &termenv.rules[rule.index()];
    let input_ty = ruledata.lhs.ty();
    let input = pattern_seq.add_arg(input_ty);
    let mut vars = HashMap::new();
    let lhs_root_term = pattern_seq.gen_pattern(input, tyenv, termenv, &ruledata.lhs, &mut vars);

    // Lower the expression, making use of the bound variables
    // from the pattern.
    let (rhs_root_term, rhs_root) = expr_seq.gen_expr(tyenv, termenv, &ruledata.rhs, &vars);
    // Return the root RHS value.
    let output_ty = ruledata.rhs.ty();
    expr_seq.add_return(output_ty, rhs_root);

    (lhs_root_term, pattern_seq, rhs_root_term, expr_seq)
}

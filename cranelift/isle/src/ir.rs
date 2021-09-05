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

    /// Set the Nth return value. Produces no values.
    Return { index: usize, ty: TypeId, value: Value },
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
    /// Position at which the rule producing this sequence was located.
    pub pos: Pos,
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
        // If `input` is `None`, then this is the root pattern, and is
        // implicitly an extraction with the N args as results.
        input: Option<Value>,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        pat: &Pattern,
        vars: &mut HashMap<VarId, (Option<TermId>, Value)>,
    ) -> Option<TermId> {
        match pat {
            &Pattern::BindPattern(_ty, var, ref subpat) => {
                // Bind the appropriate variable and recurse.
                assert!(!vars.contains_key(&var));
                vars.insert(var, (None, input.unwrap())); // bind first, so subpat can use it
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
                self.add_match_equal(input.unwrap(), var_val, ty);
                var_val_term
            }
            &Pattern::ConstInt(ty, value) => {
                // Assert that the value matches the constant integer.
                self.add_match_int(input.unwrap(), ty, value);
                None
            }
            &Pattern::Term(_, term, ref args) if input.is_none() => {
                let termdata = &termenv.terms[term.index()];
                let arg_tys = &termdata.arg_tys[..];
                for (i, subpat) in args.iter().enumerate() {
                    let value = self.add_arg(i, arg_tys[i]);
                    self.gen_pattern(Some(value), typeenv, termenv, subpat, vars);
                }
                Some(term)
            }
            &Pattern::Term(ty, term, ref args) => {
                // Determine whether the term has an external extractor or not.
                let termdata = &termenv.terms[term.index()];
                let arg_tys = &termdata.arg_tys[..];
                match &termdata.kind {
                    &TermKind::EnumVariant { variant } => {
                        let arg_values =
                            self.add_match_variant(input.unwrap(), ty, arg_tys, variant);
                        for (subpat, value) in args.iter().zip(arg_values.into_iter()) {
                            self.gen_pattern(Some(value), typeenv, termenv, subpat, vars);
                        }
                        None
                    }
                    &TermKind::Regular { .. } => {
                        let arg_values = self.add_extract(input.unwrap(), ty, arg_tys, term);
                        for (subpat, value) in args.iter().zip(arg_values.into_iter()) {
                            self.gen_pattern(Some(value), typeenv, termenv, subpat, vars);
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
        self.add_inst(ExprInst::Return { index: 0, ty, value });
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
    expr_seq.pos = termenv.rules[rule.index()].pos;

    // Lower the pattern, starting from the root input value.
    let ruledata = &termenv.rules[rule.index()];
    let mut vars = HashMap::new();
    let lhs_root_term = pattern_seq.gen_pattern(None, tyenv, termenv, &ruledata.lhs, &mut vars);

    // Lower the expression, making use of the bound variables
    // from the pattern.
    let (rhs_root_term, rhs_root) = expr_seq.gen_expr(tyenv, termenv, &ruledata.rhs, &vars);
    // Return the root RHS value.
    let output_ty = ruledata.rhs.ty();
    expr_seq.add_return(output_ty, rhs_root);

    (lhs_root_term, pattern_seq, rhs_root_term, expr_seq)
}

/// Reverse a sequence to form an extractor from a constructor.
pub fn reverse_rule(
    orig_pat: &PatternSequence,
    orig_expr: &ExprSequence,
) -> Option<(
    PatternSequence,
    ExprSequence,
    )>
{
    let mut pattern_seq = PatternSequence::default();
    let mut expr_seq = ExprSequence::default();
    expr_seq.pos = orig_expr.pos;

    let mut value_map = HashMap::new();

    for (id, inst) in orig_expr.insts.iter().enumerate().rev() {
        let id = InstId(id);
        match inst {
            &ExprInst::Return { index, ty, value } => {
                let new_value = pattern_seq.add_arg(index, ty);
                value_map.insert(value, new_value);
            }
            &ExprInst::Construct { ref inputs, ty, term } => {
                let arg_tys = inputs.iter().map(|(_, ty)| *ty).collect::<Vec<_>>();
                let input_ty = ty;
                // Input to the Extract is the output of the Construct.
                let input = value_map.get(&Value::Expr { inst: id, output: 0 })?.clone();
                let outputs = pattern_seq.add_extract(input, input_ty, &arg_tys[..], term);
                for (input, output) in inputs.iter().map(|(val, _)| val).zip(outputs.into_iter()) {
                    value_map.insert(*input, output);
                }
            }
            &ExprInst::CreateVariant { ref inputs, ty, variant } => {
                let arg_tys = inputs.iter().map(|(_, ty)| *ty).collect::<Vec<_>>();
                let input_ty = ty;
                // Input to the MatchVariant is the output of the CreateVariant.
                let input = value_map.get(&Value::Expr { inst: id, output: 0 })?.clone();
                let outputs = pattern_seq.add_match_variant(input, input_ty, &arg_tys[..], variant);
                for (input, output) in inputs.iter().map(|(val, _)| val).zip(outputs.into_iter()) {
                    value_map.insert(*input, output);
                }
            }
            &ExprInst::ConstInt { ty, val } => {
                let input = value_map.get(&Value::Expr { inst: id, output: 0 })?.clone();
                pattern_seq.add_match_int(input, ty, val);
            }
        }
    }

    for (id, inst) in orig_pat.insts.iter().enumerate().rev() {
        let id = InstId(id);
        match inst {
            &PatternInst::Extract { input, input_ty, ref arg_tys, term } => {
                let mut inputs = vec![];
                for i in 0..arg_tys.len() {
                    let value = Value::Pattern { inst: id, output: i };
                    let new_value = value_map.get(&value)?.clone();
                    inputs.push((new_value, arg_tys[i]));
                }
                let output = expr_seq.add_construct(&inputs[..], input_ty, term);
                value_map.insert(input, output);
                
            }
            &PatternInst::MatchVariant { input, input_ty, ref arg_tys, variant } => {
                let mut inputs = vec![];
                for i in 0..arg_tys.len() {
                    let value = Value::Pattern { inst: id, output: i };
                    let new_value = value_map.get(&value)?.clone();
                    inputs.push((new_value, arg_tys[i]));
                }
                let output = expr_seq.add_create_variant(&inputs[..], input_ty, variant);
                value_map.insert(input, output);
            }
            &PatternInst::MatchEqual { a, b, .. } => {
                if let Some(new_a) = value_map.get(&a).cloned() {
                    if !value_map.contains_key(&b) {
                        value_map.insert(b, new_a);
                    }
                } else if let Some(new_b) = value_map.get(&b).cloned() {
                    if !value_map.contains_key(&a) {
                        value_map.insert(a, new_b);
                    }
                }
            }
            &PatternInst::MatchInt { input, ty, int_val } => {
                let output = expr_seq.add_const_int(ty, int_val);
                value_map.insert(input, output);
            }
            &PatternInst::Arg { index, ty } => {
                let value = Value::Pattern { inst: id, output: 0 };
                let new_value = value_map.get(&value)?.clone();
                expr_seq.add_multi_return(index, ty, new_value);
            }
        }
    }

    Some((pattern_seq, expr_seq))
}

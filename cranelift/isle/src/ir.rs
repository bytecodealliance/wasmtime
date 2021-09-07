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
            &Pattern::And(_ty, ref children) => {
                let input = input.unwrap();
                for child in children {
                    self.gen_pattern(Some(input), typeenv, termenv, child, vars);
                }
                None
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
    ///
    /// If `gen_final_construct` is false and the value is a
    /// constructor call, this returns the arguments instead. This is
    /// used when codegen'ing extractors for internal terms.
    fn gen_expr(
        &mut self,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        expr: &Expr,
        vars: &HashMap<VarId, (Option<TermId>, Value)>,
        gen_final_construct: bool,
    ) -> (Option<TermId>, Vec<Value>) {
        log::trace!(
            "gen_expr: expr {:?} gen_final_construct {}",
            expr,
            gen_final_construct
        );
        match expr {
            &Expr::ConstInt(ty, val) => (None, vec![self.add_const_int(ty, val)]),
            &Expr::Let(_ty, ref bindings, ref subexpr) => {
                let mut vars = vars.clone();
                for &(var, _var_ty, ref var_expr) in bindings {
                    let (var_value_term, var_value) =
                        self.gen_expr(typeenv, termenv, &*var_expr, &vars, true);
                    let var_value = var_value[0];
                    vars.insert(var, (var_value_term, var_value));
                }
                self.gen_expr(typeenv, termenv, &*subexpr, &vars, gen_final_construct)
            }
            &Expr::Var(_ty, var_id) => {
                let (root_term, value) = vars.get(&var_id).cloned().unwrap();
                (root_term, vec![value])
            }
            &Expr::Term(ty, term, ref arg_exprs) => {
                let termdata = &termenv.terms[term.index()];
                let mut arg_values_tys = vec![];
                log::trace!("Term gen_expr term {}", term.index());
                for (arg_ty, arg_expr) in termdata.arg_tys.iter().cloned().zip(arg_exprs.iter()) {
                    log::trace!("generating for arg_expr {:?}", arg_expr);
                    arg_values_tys.push((
                        self.gen_expr(typeenv, termenv, &*arg_expr, &vars, true).1[0],
                        arg_ty,
                    ));
                }
                match &termdata.kind {
                    &TermKind::EnumVariant { variant } => (
                        None,
                        vec![self.add_create_variant(&arg_values_tys[..], ty, variant)],
                    ),
                    &TermKind::Regular { .. } if !gen_final_construct => (
                        Some(termdata.id),
                        arg_values_tys.into_iter().map(|(val, _ty)| val).collect(),
                    ),
                    &TermKind::Regular { .. } => (
                        Some(termdata.id),
                        vec![self.add_construct(&arg_values_tys[..], ty, term)],
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
    is_forward_dir: bool,
) -> Option<(PatternSequence, ExprSequence, TermId)> {
    let mut pattern_seq: PatternSequence = Default::default();
    let mut expr_seq: ExprSequence = Default::default();
    expr_seq.pos = termenv.rules[rule.index()].pos;

    // Lower the pattern, starting from the root input value.
    let ruledata = &termenv.rules[rule.index()];
    let mut vars = HashMap::new();

    log::trace!(
        "lower_rule: ruledata {:?} forward {}",
        ruledata,
        is_forward_dir
    );

    if is_forward_dir {
        let can_do_forward = match &ruledata.lhs {
            &Pattern::Term(..) => true,
            _ => false,
        };
        if !can_do_forward {
            return None;
        }

        let lhs_root_term = pattern_seq.gen_pattern(None, tyenv, termenv, &ruledata.lhs, &mut vars);
        let root_term = match lhs_root_term {
            Some(t) => t,
            None => {
                return None;
            }
        };

        // Lower the expression, making use of the bound variables
        // from the pattern.
        let (_, rhs_root_vals) = expr_seq.gen_expr(
            tyenv,
            termenv,
            &ruledata.rhs,
            &vars,
            /* final_construct = */ true,
        );
        // Return the root RHS value.
        let output_ty = ruledata.rhs.ty();
        assert_eq!(rhs_root_vals.len(), 1);
        expr_seq.add_return(output_ty, rhs_root_vals[0]);
        Some((pattern_seq, expr_seq, root_term))
    } else {
        let can_reverse = match &ruledata.rhs {
            &Expr::Term(..) => true,
            _ => false,
        };
        if !can_reverse {
            return None;
        }

        let arg = pattern_seq.add_arg(0, ruledata.lhs.ty());
        let _ = pattern_seq.gen_pattern(Some(arg), tyenv, termenv, &ruledata.lhs, &mut vars);
        let (rhs_root_term, rhs_root_vals) = expr_seq.gen_expr(
            tyenv,
            termenv,
            &ruledata.rhs,
            &vars,
            /* final_construct = */ false,
        );

        let root_term = match rhs_root_term {
            Some(t) => t,
            None => {
                return None;
            }
        };
        let termdata = &termenv.terms[root_term.index()];
        for (i, (val, ty)) in rhs_root_vals
            .into_iter()
            .zip(termdata.arg_tys.iter())
            .enumerate()
        {
            expr_seq.add_multi_return(i, *ty, val);
        }

        Some((pattern_seq, expr_seq, root_term))
    }
}

/// Trim the final Construct and Return ops in an ExprSequence in
/// order to allow the extractor to be codegen'd.
pub fn trim_expr_for_extractor(mut expr: ExprSequence) -> ExprSequence {
    let ret_inst = expr.insts.pop().unwrap();
    let retval = match ret_inst {
        ExprInst::Return { value, .. } => value,
        _ => panic!("Last instruction is not a return"),
    };
    assert_eq!(
        retval,
        Value::Expr {
            inst: InstId(expr.insts.len() - 1),
            output: 0
        }
    );
    let construct_inst = expr.insts.pop().unwrap();
    let inputs = match construct_inst {
        ExprInst::Construct { inputs, .. } => inputs,
        _ => panic!("Returned value is not a construct call"),
    };
    for (i, (value, ty)) in inputs.into_iter().enumerate() {
        expr.add_multi_return(i, ty, value);
    }

    expr
}

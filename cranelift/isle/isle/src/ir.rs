//! Lowered matching IR.

use crate::lexer::Pos;
use crate::sema::*;
use std::collections::BTreeMap;

declare_id!(
    /// The id of an instruction in a `PatternSequence`.
    InstId
);

/// A value produced by a LHS or RHS instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// A value produced by an instruction in the Pattern (LHS).
    Pattern {
        /// The instruction that produces this value.
        inst: InstId,
        /// This value is the `output`th value produced by this pattern.
        output: usize,
    },
    /// A value produced by an instruction in the Expr (RHS).
    Expr {
        /// The instruction that produces this value.
        inst: InstId,
        /// This value is the `output`th value produced by this expression.
        output: usize,
    },
}

/// A single Pattern instruction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PatternInst {
    /// Match a value as equal to another value. Produces no values.
    MatchEqual {
        /// The first value.
        a: Value,
        /// The second value.
        b: Value,
        /// The type of the values.
        ty: TypeId,
    },

    /// Try matching the given value as the given integer. Produces no values.
    MatchInt {
        /// The value to match on.
        input: Value,
        /// The value's type.
        ty: TypeId,
        /// The integer to match against the value.
        int_val: i64,
    },

    /// Try matching the given value as the given constant. Produces no values.
    MatchPrim {
        /// The value to match on.
        input: Value,
        /// The type of the value.
        ty: TypeId,
        /// The primitive to match against the value.
        val: Sym,
    },

    /// Try matching the given value as the given variant, producing `|arg_tys|`
    /// values as output.
    MatchVariant {
        /// The value to match on.
        input: Value,
        /// The type of the value.
        input_ty: TypeId,
        /// The types of values produced upon a successful match.
        arg_tys: Vec<TypeId>,
        /// The value type's variant that we are matching against.
        variant: VariantId,
    },

    /// Evaluate an expression and provide the given value as the result of this
    /// match instruction. The expression has access to the pattern-values up to
    /// this point in the sequence.
    Expr {
        /// The expression to evaluate.
        seq: ExprSequence,
        /// The value produced by the expression.
        output: Value,
        /// The type of the output value.
        output_ty: TypeId,
    },

    // NB: this has to come second-to-last, because it might be infallible, for
    // the same reasons that `Arg` has to be last.
    //
    /// Invoke an extractor, taking the given values as input (the first is the
    /// value to extract, the other are the `Input`-polarity extractor args) and
    /// producing an output value for each `Output`-polarity extractor arg.
    Extract {
        /// The value to extract, followed by polarity extractor args.
        inputs: Vec<Value>,
        /// The types of the inputs.
        input_tys: Vec<TypeId>,
        /// The types of the output values produced upon a successful match.
        output_tys: Vec<TypeId>,
        /// This extractor's term.
        term: TermId,
        /// Whether this extraction is infallible or not.
        infallible: bool,
    },

    // NB: This has to go last, since it is infallible, so that when we sort
    // edges in the trie, we visit infallible edges after first having tried the
    // more-specific fallible options.
    //
    /// Get the Nth input argument, which corresponds to the Nth field
    /// of the root term.
    Arg {
        /// The index of the argument to get.
        index: usize,
        /// The type of the argument.
        ty: TypeId,
    },
}

/// A single Expr instruction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExprInst {
    /// Produce a constant integer.
    ConstInt {
        /// This integer type.
        ty: TypeId,
        /// The integer value. Must fit within the type.
        val: i64,
    },

    /// Produce a constant extern value.
    ConstPrim {
        /// The primitive type.
        ty: TypeId,
        /// The primitive value.
        val: Sym,
    },

    /// Create a variant.
    CreateVariant {
        /// The input arguments that will make up this variant's fields.
        ///
        /// These must be in the same order as the variant's fields.
        inputs: Vec<(Value, TypeId)>,
        /// The enum type.
        ty: TypeId,
        /// The variant within the enum that we are contructing.
        variant: VariantId,
    },

    /// Invoke a constructor.
    Construct {
        /// The arguments to the constructor.
        inputs: Vec<(Value, TypeId)>,
        /// The type of the constructor.
        ty: TypeId,
        /// The constructor term.
        term: TermId,
        /// Whether this constructor is infallible or not.
        infallible: bool,
    },

    /// Set the Nth return value. Produces no values.
    Return {
        /// The index of the return value to set.
        index: usize,
        /// The type of the return value.
        ty: TypeId,
        /// The value to set as the `index`th return value.
        value: Value,
    },
}

impl ExprInst {
    /// Invoke `f` for each value in this expression.
    pub fn visit_values<F: FnMut(Value)>(&self, mut f: F) {
        match self {
            &ExprInst::ConstInt { .. } => {}
            &ExprInst::ConstPrim { .. } => {}
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
/// argument. A pattern is fallible (may not match). If it does not fail, its
/// result consists of the values produced by the `PatternInst`s, which may be
/// used by a subsequent `Expr`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PatternSequence {
    /// Instruction sequence for pattern.
    ///
    /// `InstId` indexes into this sequence for `Value::Pattern` values.
    pub insts: Vec<PatternInst>,
}

/// A linear sequence of instructions that produce a new value from the
/// right-hand side of a rule, given bindings that come from a `Pattern` derived
/// from the left-hand side.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct ExprSequence {
    /// Instruction sequence for expression.
    ///
    /// `InstId` indexes into this sequence for `Value::Expr` values.
    pub insts: Vec<ExprInst>,
    /// Position at which the rule producing this sequence was located.
    pub pos: Pos,
}

impl ExprSequence {
    /// Is this expression sequence producing a constant integer?
    ///
    /// If so, return the integer type and the constant.
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

    fn add_match_prim(&mut self, input: Value, ty: TypeId, val: Sym) {
        self.add_inst(PatternInst::MatchPrim { input, ty, val });
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
        vars: &mut BTreeMap<VarId, Value>,
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
                    .expect("Cannot match an integer pattern against root term");
                self.add_match_int(input_val, ty, value);
            }
            &Pattern::ConstPrim(ty, value) => {
                let input_val = input
                    .to_value()
                    .expect("Cannot match a constant-primitive pattern against root term");
                self.add_match_prim(input_val, ty, value);
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
                            TermKind::EnumVariant { variant } => {
                                let arg_values =
                                    self.add_match_variant(input, ty, arg_tys, *variant);
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
                            TermKind::Decl {
                                extractor_kind: None,
                                ..
                            } => {
                                panic!("Pattern invocation of undefined term body")
                            }
                            TermKind::Decl {
                                extractor_kind: Some(ExtractorKind::InternalExtractor { .. }),
                                ..
                            } => {
                                panic!("Should have been expanded away")
                            }
                            TermKind::Decl {
                                extractor_kind:
                                    Some(ExtractorKind::ExternalExtractor {
                                        ref arg_polarity,
                                        infallible,
                                        ..
                                    }),
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
                                let arg_values = self.add_extract(
                                    inputs,
                                    input_tys,
                                    output_tys,
                                    term,
                                    *infallible,
                                );

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

    fn add_const_prim(&mut self, ty: TypeId, val: Sym) -> Value {
        let inst = InstId(self.insts.len());
        self.add_inst(ExprInst::ConstPrim { ty, val });
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

    /// Creates a sequence of ExprInsts to generate the given
    /// expression value. Returns the value ID as well as the root
    /// term ID, if any.
    fn gen_expr(
        &mut self,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        expr: &Expr,
        vars: &BTreeMap<VarId, Value>,
    ) -> Value {
        log::trace!("gen_expr: expr {:?}", expr);
        match expr {
            &Expr::ConstInt(ty, val) => self.add_const_int(ty, val),
            &Expr::ConstPrim(ty, val) => self.add_const_prim(ty, val),
            &Expr::Let {
                ty: _ty,
                ref bindings,
                ref body,
            } => {
                let mut vars = vars.clone();
                for &(var, _var_ty, ref var_expr) in bindings {
                    let var_value = self.gen_expr(typeenv, termenv, &*var_expr, &vars);
                    vars.insert(var, var_value);
                }
                self.gen_expr(typeenv, termenv, body, &vars)
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
                    TermKind::EnumVariant { variant } => {
                        self.add_create_variant(&arg_values_tys[..], ty, *variant)
                    }
                    TermKind::Decl {
                        constructor_kind: Some(ConstructorKind::InternalConstructor),
                        ..
                    } => {
                        self.add_construct(
                            &arg_values_tys[..],
                            ty,
                            term,
                            /* infallible = */ false,
                        )
                    }
                    TermKind::Decl {
                        constructor_kind: Some(ConstructorKind::ExternalConstructor { .. }),
                        ..
                    } => {
                        self.add_construct(
                            &arg_values_tys[..],
                            ty,
                            term,
                            /* infallible = */ true,
                        )
                    }
                    TermKind::Decl {
                        constructor_kind: None,
                        ..
                    } => panic!("Should have been caught by typechecking"),
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
    let mut vars = BTreeMap::new();
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

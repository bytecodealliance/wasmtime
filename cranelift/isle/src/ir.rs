//! Lowered matching IR.

use crate::declare_id;
use crate::sema::*;
use std::collections::hash_map::Entry as HashEntry;
use std::collections::HashMap;

declare_id!(InstId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(InstId, usize);

/// A single node in the sea-of-nodes. Each node produces one value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Inst {
    /// Get the input root-term value.
    Arg { ty: TypeId },

    /// Set the return value. Produces no values.
    Return { ty: TypeId, value: Value },

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

    /// Copy a value. Used mainly when rewriting/inlining.
    Copy { ty: TypeId, val: Value },

    /// A non-operation (nop). Used to "nop out" unused instructions
    /// without renumbering all values.
    Nop,
}

impl Inst {
    fn map_values<F: Fn(Value) -> Value>(&self, f: F) -> Self {
        match self {
            &Inst::Arg { ty } => Inst::Arg { ty },
            &Inst::Return { ty, value } => Inst::Return {
                ty,
                value: f(value),
            },
            &Inst::MatchEqual { a, b, ty } => Inst::MatchEqual {
                a: f(a),
                b: f(b),
                ty,
            },
            &Inst::MatchInt { input, ty, int_val } => Inst::MatchInt {
                input: f(input),
                ty,
                int_val,
            },
            &Inst::MatchVariant {
                input,
                input_ty,
                ref arg_tys,
                variant,
            } => Inst::MatchVariant {
                input: f(input),
                input_ty,
                arg_tys: arg_tys.clone(),
                variant,
            },
            &Inst::Extract {
                input,
                input_ty,
                ref arg_tys,
                term,
            } => Inst::Extract {
                input: f(input),
                input_ty,
                arg_tys: arg_tys.clone(),
                term,
            },
            &Inst::ConstInt { ty, val } => Inst::ConstInt { ty, val },
            &Inst::CreateVariant {
                ref inputs,
                ty,
                variant,
            } => Inst::CreateVariant {
                inputs: inputs
                    .iter()
                    .map(|(i, ty)| (f(*i), *ty))
                    .collect::<Vec<_>>(),
                ty,
                variant,
            },
            &Inst::Construct {
                ref inputs,
                ty,
                term,
            } => Inst::Construct {
                inputs: inputs
                    .iter()
                    .map(|(i, ty)| (f(*i), *ty))
                    .collect::<Vec<_>>(),
                ty,
                term,
            },
            &Inst::Copy { ty, val } => Inst::Copy { ty, val: f(val) },
            &Inst::Nop => Inst::Nop,
        }
    }

    fn map_insts<F: Fn(InstId) -> InstId>(&self, f: F) -> Self {
        self.map_values(|val| Value(f(val.0), val.1))
    }

    fn num_results(&self) -> usize {
        match self {
            &Inst::Arg { .. }
            | &Inst::ConstInt { .. }
            | &Inst::Construct { .. }
            | &Inst::CreateVariant { .. }
            | &Inst::Copy { .. } => 1,
            &Inst::Return { .. } | &Inst::MatchEqual { .. } | &Inst::MatchInt { .. } => 0,
            &Inst::Extract { ref arg_tys, .. } | &Inst::MatchVariant { ref arg_tys, .. } => {
                arg_tys.len()
            }
            &Inst::Nop => 0,
        }
    }
}

impl Value {
    fn map_inst<F: Fn(InstId) -> InstId>(&self, f: F) -> Self {
        Value(f(self.0), self.1)
    }
}

/// A linear sequence of instructions that either convert an input
/// value to an output value, or fail.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Sequence {
    /// Instruction sequence. InstId indexes into this sequence.
    insts: Vec<Inst>,
}

impl Sequence {
    fn add_inst(&mut self, inst: Inst) -> InstId {
        let id = InstId(self.insts.len());
        self.insts.push(inst);
        id
    }

    fn add_arg(&mut self, ty: TypeId) -> Value {
        let inst = InstId(self.insts.len());
        self.add_inst(Inst::Arg { ty });
        Value(inst, 0)
    }

    fn add_return(&mut self, ty: TypeId, value: Value) {
        self.add_inst(Inst::Return { ty, value });
    }

    fn add_match_equal(&mut self, a: Value, b: Value, ty: TypeId) {
        self.add_inst(Inst::MatchEqual { a, b, ty });
    }

    fn add_match_int(&mut self, input: Value, ty: TypeId, int_val: i64) {
        self.add_inst(Inst::MatchInt { input, ty, int_val });
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
            let val = Value(inst, i);
            outs.push(val);
        }
        let arg_tys = arg_tys.iter().cloned().collect();
        self.add_inst(Inst::MatchVariant {
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
            let val = Value(inst, i);
            outs.push(val);
        }
        let arg_tys = arg_tys.iter().cloned().collect();
        self.add_inst(Inst::Extract {
            input,
            input_ty,
            arg_tys,
            term,
        });
        outs
    }

    fn add_const_int(&mut self, ty: TypeId, val: i64) -> Value {
        let inst = InstId(self.insts.len());
        self.add_inst(Inst::ConstInt { ty, val });
        Value(inst, 0)
    }

    fn add_create_variant(
        &mut self,
        inputs: &[(Value, TypeId)],
        ty: TypeId,
        variant: VariantId,
    ) -> Value {
        let inst = InstId(self.insts.len());
        let inputs = inputs.iter().cloned().collect();
        self.add_inst(Inst::CreateVariant {
            inputs,
            ty,
            variant,
        });
        Value(inst, 0)
    }

    fn add_construct(&mut self, inputs: &[(Value, TypeId)], ty: TypeId, term: TermId) -> Value {
        let inst = InstId(self.insts.len());
        let inputs = inputs.iter().cloned().collect();
        self.add_inst(Inst::Construct { inputs, ty, term });
        Value(inst, 0)
    }

    fn gen_pattern(
        &mut self,
        input: Value,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        pat: &Pattern,
        vars: &mut HashMap<VarId, Value>,
    ) {
        match pat {
            &Pattern::BindPattern(_ty, var, ref subpat) => {
                // Bind the appropriate variable and recurse.
                assert!(!vars.contains_key(&var));
                vars.insert(var, input);
                self.gen_pattern(input, typeenv, termenv, &*subpat, vars);
            }
            &Pattern::Var(ty, var) => {
                // Assert that the value matches the existing bound var.
                let var_val = vars
                    .get(&var)
                    .cloned()
                    .expect("Variable should already be bound");
                self.add_match_equal(input, var_val, ty);
            }
            &Pattern::ConstInt(ty, value) => {
                // Assert that the value matches the constant integer.
                self.add_match_int(input, ty, value);
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
                    }
                    &TermKind::Regular { .. } => {
                        let arg_values = self.add_extract(input, ty, arg_tys, term);
                        for (subpat, value) in args.iter().zip(arg_values.into_iter()) {
                            self.gen_pattern(value, typeenv, termenv, subpat, vars);
                        }
                    }
                }
            }
            &Pattern::Wildcard(_ty) => {
                // Nothing!
            }
        }
    }

    fn gen_expr(
        &mut self,
        typeenv: &TypeEnv,
        termenv: &TermEnv,
        expr: &Expr,
        vars: &HashMap<VarId, Value>,
    ) -> Value {
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
                    &TermKind::Regular { .. } => self.add_construct(&arg_values_tys[..], ty, term),
                }
            }
        }
    }
}

impl Sequence {
    /// Build a sequence from a rule.
    pub fn from_rule(tyenv: &TypeEnv, termenv: &TermEnv, rule: RuleId) -> Sequence {
        let mut seq: Sequence = Default::default();

        // Lower the pattern, starting from the root input value.
        let ruledata = &termenv.rules[rule.index()];
        let input_ty = ruledata.lhs.ty();
        let input = seq.add_arg(input_ty);
        let mut vars = HashMap::new();
        seq.gen_pattern(input, tyenv, termenv, &ruledata.lhs, &mut vars);

        // Lower the expression, making use of the bound variables
        // from the pattern.
        let rhs_root = seq.gen_expr(tyenv, termenv, &ruledata.rhs, &vars);
        // Return the root RHS value.
        let output_ty = ruledata.rhs.ty();
        seq.add_return(output_ty, rhs_root);

        seq
    }

    /// Inline sequence(s) in place of given instructions.
    pub fn inline(&self, inlines: Vec<(InstId, &'_ Sequence)>) -> Sequence {
        let mut seq: Sequence = Default::default();
        // Map from inst ID in this seq to inst ID in final seq.
        let mut inst_map: HashMap<InstId, InstId> = HashMap::new();

        let mut next_inline = 0;
        for (id, inst) in self.insts.iter().enumerate() {
            let orig_inst_id = InstId(id);

            // If this is an inlining point, do the inlining. The
            // inlining point must be at a Construct or Extract call.
            //
            // - For a Construct inlining, we emit the Construct
            //   *first*, taking its output value as the arg for the
            //   invoked sequence. The returned value will in turn be
            //   substituted for that value at the end of inlining.
            //
            // - For an Extract inlining, we emit the sequence first,
            //   taking the input of the Extract as the arg for the
            //   invoked sequence. The returned value will then be the
            //   new input to the Extract.

            if next_inline < inlines.len() && inlines[next_inline].0 == orig_inst_id {
                let inlined_seq = &inlines[next_inline].1;
                next_inline += 1;

                let (arg, arg_ty) = match inst {
                    &Inst::Construct { ty, .. } => {
                        // Emit the Construct, mapping its input
                        // values across the mapping, and saving its
                        // output as the arg for the inlined sequence.
                        let inst = inst.map_insts(|id| {
                            inst_map
                                .get(&id)
                                .cloned()
                                .expect("Should have inst mapping")
                        });
                        let new_inst_id = seq.add_inst(inst);
                        (Value(new_inst_id, 0), ty)
                    }
                    &Inst::Extract {
                        input, input_ty, ..
                    } => {
                        // Map the input and save it as the arg, but
                        // don't emit the Extract yet.
                        (
                            input.map_inst(|id| {
                                inst_map
                                    .get(&id)
                                    .cloned()
                                    .expect("Should have inst mapping")
                            }),
                            input_ty,
                        )
                    }
                    _ => panic!("Unexpected instruction {:?} at inlining point", inst),
                };

                // Copy the inlined insts into the output sequence. We
                // map `Arg` to the input, and save the `Ret`, which
                // must come last.
                let mut inlined_inst_map: HashMap<InstId, InstId> = HashMap::new();
                let mut ret: Option<(InstId, TypeId)> = None;
                for (i, inst) in inlined_seq.insts.iter().enumerate() {
                    let inlined_orig_inst_id = InstId(i);
                    let new_inst_id = InstId(seq.insts.len());
                    let inst = match inst {
                        &Inst::Return { ty, value } => {
                            let value =
                                value.map_inst(|id| inlined_inst_map.get(&id).cloned().unwrap());
                            ret = Some((new_inst_id, ty));
                            Inst::Copy { ty, val: value }
                        }
                        &Inst::Arg { ty } => {
                            assert_eq!(ty, arg_ty);
                            Inst::Copy { ty, val: arg }
                        }
                        _ => inst.map_insts(|id| inlined_inst_map.get(&id).cloned().unwrap()),
                    };
                    let new_id = seq.add_inst(inst);
                    inlined_inst_map.insert(inlined_orig_inst_id, new_id);
                }

                // Now, emit the Extract if appropriate (it comes
                // after the inlined sequence, while Construct goes
                // before), and map the old inst ID to the resulting
                // output of either the Extract or the return above.
                let final_inst_id = match inst {
                    &Inst::Extract {
                        input_ty,
                        ref arg_tys,
                        term,
                        ..
                    } => {
                        let input = Value(ret.unwrap().0, 0);
                        seq.add_inst(Inst::Extract {
                            input,
                            input_ty,
                            arg_tys: arg_tys.clone(),
                            term,
                        })
                    }
                    &Inst::Construct { .. } => ret.unwrap().0,
                    _ => unreachable!(),
                };

                inst_map.insert(orig_inst_id, final_inst_id);
            } else {
                // Non-inlining-point instruction. Just copy over,
                // mapping values as appropriate.
                let inst = inst.map_insts(|id| {
                    inst_map
                        .get(&id)
                        .cloned()
                        .expect("inst ID should be present")
                });
                let new_id = seq.add_inst(inst);
                inst_map.insert(orig_inst_id, new_id);
            }
        }

        seq
    }

    /// Perform constant-propagation / simplification across
    /// construct/extract pairs, variants and integer values, and
    /// copies.
    pub fn simplify(&self) -> Option<Sequence> {
        #[derive(Clone, Debug)]
        enum SymbolicValue {
            Value(Value),
            ConstInt(Value, i64),
            Variant(Value, VariantId, Vec<SymbolicValue>),
            Term(Value, TermId, Vec<SymbolicValue>),
        }
        impl SymbolicValue {
            fn to_value(&self) -> Value {
                match self {
                    &SymbolicValue::Value(v) => v,
                    &SymbolicValue::ConstInt(v, ..) => v,
                    &SymbolicValue::Variant(v, ..) => v,
                    &SymbolicValue::Term(v, ..) => v,
                }
            }
        }
        let mut value_map: HashMap<Value, SymbolicValue> = HashMap::new();
        let mut seq: Sequence = Default::default();

        for (i, inst) in self.insts.iter().enumerate() {
            let orig_inst_id = InstId(i);
            match inst {
                &Inst::Arg { .. } => {
                    let new_inst = seq.add_inst(inst.clone());
                    value_map.insert(
                        Value(orig_inst_id, 0),
                        SymbolicValue::Value(Value(new_inst, 0)),
                    );
                }
                &Inst::Return { ty, value } => {
                    let inst = Inst::Return {
                        ty,
                        value: value_map.get(&value).unwrap().to_value(),
                    };
                    seq.add_inst(inst);
                }
                &Inst::MatchEqual { a, b, ty } => {
                    let sym_a = value_map.get(&a).unwrap();
                    let sym_b = value_map.get(&b).unwrap();
                    match (sym_a, sym_b) {
                        (
                            &SymbolicValue::ConstInt(_, int_a),
                            &SymbolicValue::ConstInt(_, int_b),
                        ) => {
                            if int_a == int_b {
                                // No-op -- we can skip it.
                                continue;
                            } else {
                                // We can't possibly match!
                                return None;
                            }
                        }
                        (
                            &SymbolicValue::Term(_, term_a, _),
                            &SymbolicValue::Term(_, term_b, _),
                        ) => {
                            if term_a != term_b {
                                return None;
                            }
                        }
                        (
                            &SymbolicValue::Variant(_, var_a, _),
                            &SymbolicValue::Variant(_, var_b, _),
                        ) => {
                            if var_a != var_b {
                                return None;
                            }
                        }
                        _ => {}
                    }
                    let val_a = sym_a.to_value();
                    let val_b = sym_b.to_value();
                    seq.add_inst(Inst::MatchEqual {
                        a: val_a,
                        b: val_b,
                        ty,
                    });
                }
                &Inst::MatchInt { input, int_val, ty } => {
                    let sym_input = value_map.get(&input).unwrap();
                    match sym_input {
                        &SymbolicValue::ConstInt(_, const_val) => {
                            if int_val == const_val {
                                // No runtime check needed -- we can continue.
                                continue;
                            } else {
                                // Static mismatch, so we can remove this
                                // whole Sequence.
                                return None;
                            }
                        }
                        _ => {}
                    }
                    let val_input = sym_input.to_value();
                    seq.add_inst(Inst::MatchInt {
                        input: val_input,
                        int_val,
                        ty,
                    });
                }
                &Inst::MatchVariant {
                    input,
                    input_ty,
                    variant,
                    ref arg_tys,
                } => {
                    let sym_input = value_map.get(&input).unwrap();
                    match sym_input {
                        &SymbolicValue::Variant(_, val_variant, ref args) => {
                            if val_variant != variant {
                                return None;
                            }
                            // Variant matches: unpack args' symbolic values into results.
                            let args = args.clone();
                            for (i, arg) in args.iter().enumerate() {
                                let val = Value(orig_inst_id, i);
                                value_map.insert(val, arg.clone());
                            }
                        }
                        _ => {
                            let val_input = sym_input.to_value();
                            let new_inst = seq.add_inst(Inst::MatchVariant {
                                input: val_input,
                                input_ty,
                                variant,
                                arg_tys: arg_tys.clone(),
                            });
                            for i in 0..arg_tys.len() {
                                let val = Value(orig_inst_id, i);
                                let sym = SymbolicValue::Value(Value(new_inst, i));
                                value_map.insert(val, sym);
                            }
                        }
                    }
                }
                &Inst::Extract {
                    input,
                    input_ty,
                    term,
                    ref arg_tys,
                } => {
                    let sym_input = value_map.get(&input).unwrap();
                    match sym_input {
                        &SymbolicValue::Term(_, val_term, ref args) => {
                            if val_term != term {
                                return None;
                            }
                            // Term matches: unpack args' symbolic values into results.
                            let args = args.clone();
                            for (i, arg) in args.iter().enumerate() {
                                let val = Value(orig_inst_id, i);
                                value_map.insert(val, arg.clone());
                            }
                        }
                        _ => {
                            let val_input = sym_input.to_value();
                            let new_inst = seq.add_inst(Inst::Extract {
                                input: val_input,
                                input_ty,
                                term,
                                arg_tys: arg_tys.clone(),
                            });
                            for i in 0..arg_tys.len() {
                                let val = Value(orig_inst_id, i);
                                let sym = SymbolicValue::Value(Value(new_inst, i));
                                value_map.insert(val, sym);
                            }
                        }
                    }
                }
                &Inst::ConstInt { ty, val } => {
                    let new_inst = seq.add_inst(Inst::ConstInt { ty, val });
                    value_map.insert(
                        Value(orig_inst_id, 0),
                        SymbolicValue::ConstInt(Value(new_inst, 0), val),
                    );
                }
                &Inst::CreateVariant {
                    ref inputs,
                    variant,
                    ty,
                } => {
                    let sym_inputs = inputs
                        .iter()
                        .map(|input| value_map.get(&input.0).cloned().unwrap())
                        .collect::<Vec<_>>();
                    let inputs = sym_inputs
                        .iter()
                        .zip(inputs.iter())
                        .map(|(si, (_, ty))| (si.to_value(), *ty))
                        .collect::<Vec<_>>();
                    let new_inst = seq.add_inst(Inst::CreateVariant {
                        inputs,
                        variant,
                        ty,
                    });
                    value_map.insert(
                        Value(orig_inst_id, 0),
                        SymbolicValue::Variant(Value(new_inst, 0), variant, sym_inputs),
                    );
                }
                &Inst::Construct {
                    ref inputs,
                    term,
                    ty,
                } => {
                    let sym_inputs = inputs
                        .iter()
                        .map(|input| value_map.get(&input.0).cloned().unwrap())
                        .collect::<Vec<_>>();
                    let inputs = sym_inputs
                        .iter()
                        .zip(inputs.iter())
                        .map(|(si, (_, ty))| (si.to_value(), *ty))
                        .collect::<Vec<_>>();
                    let new_inst = seq.add_inst(Inst::Construct { inputs, term, ty });
                    value_map.insert(
                        Value(orig_inst_id, 0),
                        SymbolicValue::Term(Value(new_inst, 0), term, sym_inputs),
                    );
                }
                &Inst::Copy { val, .. } => {
                    let sym_value = value_map.get(&val).cloned().unwrap();
                    value_map.insert(Value(orig_inst_id, 0), sym_value);
                }
                &Inst::Nop => {}
            };
        }

        // Now do a pass backward to track which instructions are used.
        let mut used = vec![false; seq.insts.len()];
        for (id, inst) in seq.insts.iter().enumerate().rev() {
            // Mark roots as used unconditionally: Return, MatchEqual,
            // MatchInt, MatchVariant.
            match inst {
                &Inst::Return { .. }
                | &Inst::MatchEqual { .. }
                | &Inst::MatchInt { .. }
                | &Inst::MatchVariant { .. } => used[id] = true,
                _ => {}
            }
            // If this instruction is not used, continue.
            if !used[id] {
                continue;
            }
            // Otherwise, mark all inputs as used as well.
            match inst {
                &Inst::Return { value, .. } => used[value.0.index()] = true,
                &Inst::MatchEqual { a, b, .. } => {
                    used[a.0.index()] = true;
                    used[b.0.index()] = true;
                }
                &Inst::MatchInt { input, .. }
                | &Inst::MatchVariant { input, .. }
                | &Inst::Extract { input, .. } => {
                    used[input.0.index()] = true;
                }
                &Inst::CreateVariant { ref inputs, .. } | Inst::Construct { ref inputs, .. } => {
                    for input in inputs {
                        used[input.0 .0.index()] = true;
                    }
                }
                &Inst::Copy { val, .. } => {
                    used[val.0.index()] = true;
                }
                &Inst::Arg { .. } | &Inst::ConstInt { .. } => {}
                &Inst::Nop => {}
            }
        }

        // Now, remove any non-used instructions.
        for id in 0..seq.insts.len() {
            if !used[id] {
                seq.insts[id] = Inst::Nop;
            }
        }

        Some(seq)
    }

    /// Build a tree summary of the output produced by a sequence.
    pub fn output_tree_summary(&self) -> TreeSummary {
        // Scan forward, building a TreeSummary for what is known
        // about each value (a "lower bound" on its shape).
        let mut value_summaries: HashMap<Value, TreeSummary> = HashMap::new();
        for (id, inst) in self.insts.iter().enumerate() {
            let inst_id = InstId(id);
            match inst {
                &Inst::Arg { .. } => {
                    value_summaries.insert(Value(inst_id, 0), TreeSummary::Other);
                }
                &Inst::Return { value, .. } => {
                    return value_summaries
                        .get(&value)
                        .cloned()
                        .unwrap_or(TreeSummary::Other);
                }
                &Inst::MatchEqual { .. }
                | &Inst::MatchInt { .. }
                | &Inst::MatchVariant { .. }
                | &Inst::Extract { .. } => {}
                &Inst::ConstInt { val, .. } => {
                    value_summaries.insert(Value(inst_id, 0), TreeSummary::ConstInt(val));
                }
                &Inst::CreateVariant {
                    ref inputs,
                    variant,
                    ..
                } => {
                    let args = inputs
                        .iter()
                        .map(|(val, _)| {
                            value_summaries
                                .get(&val)
                                .cloned()
                                .unwrap_or(TreeSummary::Other)
                        })
                        .collect::<Vec<_>>();
                    value_summaries.insert(Value(inst_id, 0), TreeSummary::Variant(variant, args));
                }
                &Inst::Construct {
                    ref inputs, term, ..
                } => {
                    let args = inputs
                        .iter()
                        .map(|(val, _)| {
                            value_summaries
                                .get(&val)
                                .cloned()
                                .unwrap_or(TreeSummary::Other)
                        })
                        .collect::<Vec<_>>();
                    value_summaries.insert(Value(inst_id, 0), TreeSummary::Term(term, args));
                }
                &Inst::Copy { val, .. } => {
                    // Copy summary from input to output.
                    let input_value = value_summaries
                        .get(&val)
                        .cloned()
                        .unwrap_or(TreeSummary::Other);
                    value_summaries.insert(Value(inst_id, 0), input_value);
                }
                &Inst::Nop => {}
            }
        }

        panic!("Sequence did not end in Return")
    }

    /// Build a tree summary of the input expected by a sequence.
    pub fn input_tree_summary(&self) -> TreeSummary {
        // Scan backward, building a TreeSummary for each value (a
        // "lower bound" on what it must be to satisfy the sequence's
        // conditions).
        let mut value_summaries: HashMap<Value, TreeSummary> = HashMap::new();
        for (id, inst) in self.insts.iter().enumerate().rev() {
            let inst_id = InstId(id);
            match inst {
                &Inst::Arg { .. } => {
                    // Must *start* with Arg; otherwise we might have missed some condition.
                    assert_eq!(id, 0);
                    return value_summaries
                        .get(&Value(inst_id, 0))
                        .cloned()
                        .unwrap_or(TreeSummary::Other);
                }
                &Inst::Return { .. } => {}

                &Inst::MatchEqual { a, b, .. } => {
                    if value_summaries.contains_key(&a) && !value_summaries.contains_key(&b) {
                        let val = value_summaries.get(&a).cloned().unwrap();
                        value_summaries.insert(b, val);
                    } else if value_summaries.contains_key(&b) && !value_summaries.contains_key(&a)
                    {
                        let val = value_summaries.get(&b).cloned().unwrap();
                        value_summaries.insert(a, val);
                    } else if value_summaries.contains_key(&a) && value_summaries.contains_key(&b) {
                        let val_a = value_summaries.get(&a).cloned().unwrap();
                        let val_b = value_summaries.get(&b).cloned().unwrap();
                        let combined = TreeSummary::Conjunction(vec![val_a, val_b]);
                        value_summaries.insert(a, combined.clone());
                        value_summaries.insert(b, combined);
                    }
                }
                &Inst::MatchInt { input, int_val, .. } => {
                    value_summaries.insert(input, TreeSummary::ConstInt(int_val));
                }
                &Inst::MatchVariant {
                    input,
                    variant,
                    ref arg_tys,
                    ..
                } => {
                    let args = (0..arg_tys.len())
                        .map(|i| Value(inst_id, i))
                        .map(|val| {
                            value_summaries
                                .get(&val)
                                .cloned()
                                .unwrap_or(TreeSummary::Other)
                        })
                        .collect::<Vec<_>>();
                    let summary = TreeSummary::Variant(variant, args);
                    match value_summaries.entry(input) {
                        HashEntry::Vacant(v) => {
                            v.insert(summary);
                        }
                        HashEntry::Occupied(mut o) => {
                            let combined = TreeSummary::Conjunction(vec![
                                summary,
                                std::mem::replace(o.get_mut(), TreeSummary::Other),
                            ]);
                            *o.get_mut() = combined;
                        }
                    }
                }

                &Inst::Extract {
                    input,
                    term,
                    ref arg_tys,
                    ..
                } => {
                    let args = (0..arg_tys.len())
                        .map(|i| Value(inst_id, i))
                        .map(|val| {
                            value_summaries
                                .get(&val)
                                .cloned()
                                .unwrap_or(TreeSummary::Other)
                        })
                        .collect::<Vec<_>>();
                    let summary = TreeSummary::Term(term, args);
                    match value_summaries.entry(input) {
                        HashEntry::Vacant(v) => {
                            v.insert(summary);
                        }
                        HashEntry::Occupied(mut o) => {
                            let combined = TreeSummary::Conjunction(vec![
                                summary,
                                std::mem::replace(o.get_mut(), TreeSummary::Other),
                            ]);
                            *o.get_mut() = combined;
                        }
                    }
                }

                &Inst::ConstInt { .. } | &Inst::CreateVariant { .. } | &Inst::Construct { .. } => {}

                &Inst::Copy { val, .. } => {
                    // Copy summary from output to input.
                    let output_value = value_summaries
                        .get(&Value(inst_id, 0))
                        .cloned()
                        .unwrap_or(TreeSummary::Other);
                    value_summaries.insert(val, output_value);
                }

                &Inst::Nop => {}
            }
        }

        panic!("Sequence did not start with Arg")
    }
}

/// A "summary" of a tree shape -- a template that describes a tree of
/// terms and constant integer values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeSummary {
    /// A known term, with given subtrees.
    Term(TermId, Vec<TreeSummary>),
    /// A known enum variant, with given subtrees.
    Variant(VariantId, Vec<TreeSummary>),
    /// A known constant integer value.
    ConstInt(i64),
    /// All of a list of summaries: represents a combined list of
    /// requirements. The "provides" relation is satisfied if the
    /// provider provides *all* of the providee's summaries in the
    /// conjunction. A conjunction on the provider side (i.e., as an
    /// "output summary") is illegal.
    Conjunction(Vec<TreeSummary>),
    /// Something else.
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeSummaryOverlap {
    Never,
    Sometimes,
    Always,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TermOrVariant {
    Term(TermId),
    Variant(VariantId),
}

impl TreeSummary {
    /// Does a term tree matching this summary "provide" the shape
    /// described/expected by another summary? Answer can be "always",
    /// "possibly", or "no".
    pub fn provides(&self, other: &TreeSummary) -> TreeSummaryOverlap {
        match (self, other) {
            (_, &TreeSummary::Other) => TreeSummaryOverlap::Always,
            (&TreeSummary::Other, _) => TreeSummaryOverlap::Sometimes,

            (&TreeSummary::Conjunction(..), _) => {
                panic!("Conjunction on LHS of `provides` relation")
            }
            (this, &TreeSummary::Conjunction(ref args)) => args
                .iter()
                .map(|arg| this.provides(arg))
                .min()
                .unwrap_or(TreeSummaryOverlap::Always),

            (
                &TreeSummary::Term(self_term, ref self_args),
                &TreeSummary::Term(other_term, ref other_args),
            ) => {
                if self_term != other_term {
                    TreeSummaryOverlap::Never
                } else {
                    assert_eq!(self_args.len(), other_args.len());
                    self_args
                        .iter()
                        .zip(other_args.iter())
                        .map(|(self_arg, other_arg)| self_arg.provides(other_arg))
                        .min()
                        .unwrap_or(TreeSummaryOverlap::Always)
                }
            }

            (
                &TreeSummary::Variant(self_var, ref self_args),
                &TreeSummary::Variant(other_var, ref other_args),
            ) => {
                if self_var != other_var {
                    TreeSummaryOverlap::Never
                } else {
                    assert_eq!(self_args.len(), other_args.len());
                    self_args
                        .iter()
                        .zip(other_args.iter())
                        .map(|(self_arg, other_arg)| self_arg.provides(other_arg))
                        .min()
                        .unwrap_or(TreeSummaryOverlap::Always)
                }
            }

            (&TreeSummary::ConstInt(i1), &TreeSummary::ConstInt(i2)) => {
                if i1 != i2 {
                    TreeSummaryOverlap::Never
                } else {
                    TreeSummaryOverlap::Always
                }
            }

            _ => TreeSummaryOverlap::Never,
        }
    }

    pub fn root(&self) -> Option<TermOrVariant> {
        match self {
            &TreeSummary::Term(term, ..) => Some(TermOrVariant::Term(term)),
            &TreeSummary::Variant(variant, ..) => Some(TermOrVariant::Variant(variant)),
            _ => None,
        }
    }
}

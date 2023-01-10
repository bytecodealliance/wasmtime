use itertools::Itertools;
/// Convert our internal Verification IR to an external SMT AST and pass
/// queries to that solver.
///
/// Right now, this uses the rsmt2 crate.
use rsmt2::Solver;
use std::collections::{HashMap, HashSet};
use veri_ir::{
    BinaryOp, Counterexample, Expr, RulePath, RuleSemantics, Terminal, Type, TypeContext, UnaryOp,
    VerificationResult,
};

use crate::REG_WIDTH;

struct SolverCtx {
    tyctx: TypeContext,
    bitwidth: usize,
    var_map: HashMap<String, String>,
    width_vars: HashMap<u32, String>,
    width_assumptions: Vec<String>,
    additional_decls: Vec<(String, String)>,
    additional_assumptions: Vec<String>,
    fresh_bits_idx: usize,
}

impl SolverCtx {
    fn new_fresh_bits(&mut self, width: usize) -> String {
        let name = format!("fresh{}", self.fresh_bits_idx);
        self.fresh_bits_idx += 1;
        self.additional_decls
            .push((name.clone(), format!("(_ BitVec {})", width)));
        name
    }

    fn new_fresh_int(&mut self) -> String {
        let name = format!("fresh{}", self.fresh_bits_idx);
        self.fresh_bits_idx += 1;
        self.additional_decls
            .push((name.clone(), "Int".to_string()));
        name
    }

    fn new_fresh_bool(&mut self) -> String {
        let name = format!("fresh{}", self.fresh_bits_idx);
        self.fresh_bits_idx += 1;
        self.additional_decls
            .push((name.clone(), "Bool".to_string()));
        name
    }

    // Extend with concrete source and destination sizes. Includes extracting relevant bits.
    fn extend_concrete(
        &mut self,
        dest_width: usize,
        source: &String,
        source_width: usize,
        op: &str,
    ) -> String {
        if dest_width < source_width {
            self.additional_assumptions.push("false".to_string());
            return source.to_string();
        }

        let delta = dest_width - source_width;
        // Extract the relevant bits of the source (which is modeled with a wider,
        // register-width bitvector).
        let extract = format!(
            "((_ extract {} 0) {})",
            source_width.wrapping_sub(1),
            source
        );

        // Do the extend itself.
        let extend = format!("((_ {} {}) {})", op, delta, extract);

        // Pad the extended result back to the full register bitwidth. Use the bits
        // that were already in the source register. That is, given:
        //                       reg - source width              source width
        //                                |                           |
        // SOURCE: [               don't care bits           |   care bits    ]
        //
        //                             dest width
        //                                |
        // OUT:    [ same don't care bits |  defined extend  |   care bits     ]
        let unconstrained_bits = self
            .bitwidth
            .checked_sub(delta)
            .unwrap()
            .checked_sub(source_width)
            .unwrap();

        // If we are extending to the full register width, no padding needed
        if unconstrained_bits == 0 {
            extend
        } else {
            let padding = format!(
                "((_ extract {} {}) {})",
                self.bitwidth.checked_sub(1).unwrap(),
                self.bitwidth.checked_sub(unconstrained_bits).unwrap(),
                source
            );
            format!("(concat {} {})", padding, extend)
        }
    }

    // SMTLIB only supports extends (zero or sign) by concrete amounts, but we
    // need symbolic ones. This method essentially does if-conversion over possible
    // concrete forms, outputting nested ITE blocks. We consider both the starting
    // width and the destination width to be potentially symbolic.
    // For safety, we add an assertion that some arm of this ITE must match.
    fn extend_symbolic(
        &mut self,
        dest_width: &String,
        source: &String,
        source_width: &String,
        op: &str,
    ) -> String {
        // Symbolic expression for amount to shift
        let shift = format!("(- {} {})", dest_width, source_width);

        let mut some_match = vec![];
        let mut ite_str = source.clone();

        // Special case: if we are asked to extend by 0, just return the source
        let matching = format!("(and (= 0 {}))", shift);
        some_match.push(matching.clone());
        ite_str = format!("(ite {} {} {})", matching, source, ite_str);

        // Possible amounts to extend by
        for possible_delta in 1..self.bitwidth + 1 {
            // Possible starting widths
            for possible_source in 1..self.bitwidth + 1 {
                // For now, ignore extends beyond the bitwidth. This is safe because
                // we will fail the rule feasibility check if this is violated.
                if possible_source + possible_delta > self.bitwidth {
                    continue;
                }

                // Statement meaning the symbolic case matches this concrete case
                let matching = format!(
                    "(and (= {} {}) (= {} {}))",
                    possible_delta, shift, possible_source, source_width
                );
                some_match.push(matching.clone());
                let extend = self.extend_concrete(
                    possible_source + possible_delta,
                    source,
                    possible_source,
                    op,
                );
                ite_str = format!("(ite {} {} {})", matching, extend, ite_str);
            }
        }
        let some_shift_matches = format!("(or {})", some_match.join(" "));
        self.width_assumptions.push(some_shift_matches);
        ite_str
    }

    // SMTLIB only supports rotates by concrete amounts, but we
    // need symbolic ones. This method essentially does if-conversion over possible
    // concrete forms, outputting nested ITE blocks. We consider both the starting
    // width and the rotate amount to be potentially symbolic.
    // For safety, we add an assertion that some arm of this ITE must match.
    fn rotate_symbolic(
        &mut self,
        source: &String,
        source_width: &String,
        amount: &String,
        op: &str,
    ) -> String {
        let mut some_match = vec![];
        let mut ite_str = source.clone();

        // Special case: if we are asked to rotate by 0, just return the source
        let matching = format!("(and (= (_ bv0 {}) {}))", self.bitwidth, amount);
        some_match.push(matching.clone());
        ite_str = format!("(ite {} {} {})", matching, source, ite_str);

        // Possible starting widths
        for possible_source in 1..self.bitwidth + 1 {
            // For now, ignore rotates beyond the source width. This is safe because
            // we will fail the rule feasibility check if this is violated.
            // Possible amounts to rotate by
            for possible_rotate in 1..possible_source {
                // Statement meaning the symbolic case matches this concrete case
                let matching = format!(
                    "(and (= (_ bv{} {}) {}) (= {} {}))",
                    possible_rotate, self.bitwidth, amount, possible_source, source_width
                );
                some_match.push(matching.clone());

                // Extract the relevant bits of the source (which is modeled with a wider,
                // register-width bitvector).
                let extract = format!(
                    "((_ extract {} 0) {})",
                    possible_source.checked_sub(1).unwrap(),
                    source
                );

                // Do the rotate itself.
                let rotate = format!("((_ {} {}) {})", op, possible_rotate, extract);

                // Pad the extended result back to the full register bitwidth. Use the bits
                // that were already in the source register. That is, given:
                //                       reg - source width              source width
                //                                |                           |
                // SOURCE: [               don't care bits           |   care bits    ]
                //
                //                             dest width
                //                                |
                // OUT:    [ same don't care bits                   |   care bits     ]
                let unconstrained_bits = self.bitwidth.checked_sub(possible_source).unwrap();

                // If we are extending to the full register width, no padding needed
                let after_padding = if unconstrained_bits == 0 {
                    rotate
                } else {
                    let padding = format!(
                        "((_ extract {} {}) {})",
                        self.bitwidth.checked_sub(1).unwrap(),
                        self.bitwidth.checked_sub(unconstrained_bits).unwrap(),
                        source
                    );
                    format!("(concat {} {})", padding, rotate)
                };
                ite_str = format!("(ite {} {} {})", matching, after_padding, ite_str);
            }
        }
        let some_shift_matches = format!("(or {})", some_match.join(" "));
        self.width_assumptions.push(some_shift_matches);
        ite_str
    }

    pub fn widen_to_query_width(
        &mut self,
        tyvar: u32,
        narrow_width: usize,
        narrow_decl: String,
        name: Option<String>,
    ) -> String {
        let width = self.bitwidth.checked_sub(narrow_width).unwrap();
        if width > 0 {
            let mut narrow_name = format!("narrow__{}", tyvar);
            let mut wide_name = format!("wide__{}", tyvar);
            if let Some(s) = name {
                narrow_name = format!("{}_{}", s, narrow_name);
                wide_name = format!("{}_{}", s, wide_name);
            }
            self.additional_assumptions
                .push(format!("(= {} {})", narrow_name, narrow_decl));
            self.additional_decls
                .push((narrow_name.clone(), format!("(_ BitVec {})", narrow_width)));
            self.additional_decls
                .push((wide_name.clone(), format!("(_ BitVec {})", self.bitwidth)));
            let padding = self.new_fresh_bits(width);
            self.additional_assumptions.push(format!(
                "(= {} (concat {} {}))",
                wide_name, padding, narrow_name,
            ));
            wide_name
        } else {
            if let Some(s) = name {
                // self.additional_decls
                //     .push((s.clone(), format!("(_ BitVec {})", self.bitwidth)));
                self.additional_assumptions
                    .push(format!("(= {} {})", s, narrow_decl));
                s
            } else {
                narrow_decl
            }
        }
    }

    pub fn get_expr_width_var(&self, e: &Expr) -> Option<&String> {
        if let Some(tyvar) = self.tyctx.tyvars.get(e) {
            self.width_vars.get(tyvar)
        } else {
            None
        }
    }

    pub fn vir_to_rsmt2_constant_ty(&self, ty: &Type) -> String {
        match ty {
            Type::BitVector(w) => format!("(_ BitVec {})", w.unwrap_or(self.bitwidth)),
            Type::Int => "Int".to_string(),
            Type::Bool => "Bool".to_string(),
        }
    }

    pub fn get_type(&self, x: &Expr) -> Option<&Type> {
        self.tyctx.tymap.get(self.tyctx.tyvars.get(x)?)
    }

    pub fn get_expr_value(&self, e: &Expr) -> Option<i128> {
        if let Some(tyvar) = self.tyctx.tyvars.get(e) {
            if let Some(v) = self.tyctx.tyvals.get(tyvar) {
                Some(*v)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn static_width(&self, x: &Expr) -> Option<usize> {
        match self.get_type(x) {
            Some(Type::BitVector(w)) => *w,
            _ => None,
        }
    }

    pub fn assume_same_width(&mut self, x: &Expr, y: &Expr) {
        let xw = self.get_expr_width_var(&x).unwrap().clone();
        let yw = self.get_expr_width_var(&y).unwrap().clone();
        self.width_assumptions.push(format!("(= {} {})", xw, yw));
    }

    pub fn assume_same_width_from_string(&mut self, x: &String, y: &Expr) {
        let yw = self.get_expr_width_var(&y).unwrap().clone();
        self.width_assumptions.push(format!("(= {} {})", x, yw));
    }

    pub fn assume_comparable_types(&mut self, x: &Expr, y: &Expr) {
        match (self.get_type(x), self.get_type(y)) {
            (None, _) | (_, None) => panic!("Missing type(s) {:?} {:?}", x, y),
            (Some(Type::Bool), Some(Type::Bool)) | (Some(Type::Int), Some(Type::Int)) => (),
            (Some(Type::BitVector(Some(xw))), Some(Type::BitVector(Some(yw)))) => {
                assert_eq!(xw, yw, "incompatible {:?} {:?}", x, y)
            }
            (_, _) => self.assume_same_width(x, y),
        }
    }

    pub fn a64clz32(&mut self, x: &String, id: u32) -> String {
        // extract to ensure we have a 32 bit input
        self.additional_decls
            .push((format!("a64x_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_assumptions.push(format!(
            "(= a64x_{id} ((_ extract 31 0) {x}))",
            id = id,
            x = x
        ));

        // total zeros counter
        self.additional_decls
            .push((format!("ret0_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_assumptions
            .push(format!("(= ret0_{id} (_ bv0 64))", id = id));

        // round 1
        self.additional_decls
            .push((format!("ret1_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y16_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x16_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions.push(format!(
            "(= y16_{id} (bvlshr a64x_{id} #x00000010))",
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y16_{id} (_ bv0 32))) (= ret1_{id} ret0_{id}) (= ret1_{id} (bvadd ret0_{id} (_ bv16 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y16_{id} (_ bv0 32))) (= x16_{id} y16_{id}) (= x16_{id} a64x_{id}))",
            id = id
        ));

        // round 2
        self.additional_decls
            .push((format!("ret2_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y8_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x8_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions
            .push(format!("(= y8_{id} (bvlshr x16_{id} #x00000008))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y8_{id} (_ bv0 32))) (= ret2_{id} ret1_{id}) (= ret2_{id} (bvadd ret1_{id} (_ bv8 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y8_{id} (_ bv0 32))) (= x8_{id} y8_{id}) (= x8_{id} x16_{id}))",
            id = id
        ));

        // round 3
        self.additional_decls
            .push((format!("ret3_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y4_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x4_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions
            .push(format!("(= y4_{id} (bvlshr x8_{id} #x00000004))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y4_{id} (_ bv0 32))) (= ret3_{id} ret2_{id}) (= ret3_{id} (bvadd ret2_{id} (_ bv4 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y4_{id} (_ bv0 32))) (= x4_{id} y4_{id}) (= x4_{id} x8_{id}))",
            id = id
        ));

        // round 4
        self.additional_decls
            .push((format!("ret4_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y2_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x2_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions
            .push(format!("(= y2_{id} (bvlshr x4_{id} #x00000002))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y2_{id} (_ bv0 32))) (= ret4_{id} ret3_{id}) (= ret4_{id} (bvadd ret3_{id} (_ bv2 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y2_{id} (_ bv0 32))) (= x2_{id} y2_{id}) (= x2_{id} x4_{id}))",
            id = id
        ));

        // round 5
        self.additional_decls
            .push((format!("ret5_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y1_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x1_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions
            .push(format!("(= y1_{id} (bvlshr x2_{id} #x00000001))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y1_{id} (_ bv0 32))) (= ret5_{id} ret4_{id}) (= ret5_{id} (bvadd ret4_{id} (_ bv1 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y1_{id} (_ bv0 32))) (= x1_{id} y1_{id}) (= x1_{id} x2_{id}))",
            id = id
        ));

        // last round
        self.additional_decls
            .push((format!("ret6_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_assumptions.push(format!("(ite (not (= x1_{id} (_ bv0 32))) (= ret6_{id} ret5_{id}) (= ret6_{id} (bvadd ret5_{id} (_ bv1 64))))", id = id));

        // final return
        format!("ret6_{id}", id = id)
    }

    pub fn clz64(&mut self, x: &String, id: u32) -> String {
        // total zeros counter
        self.additional_decls
            .push((format!("ret0_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_assumptions
            .push(format!("(= ret0_{id} (_ bv0 64))", id = id));

        // round 1
        self.additional_decls
            .push((format!("ret1_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y32_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("x32_{id}", id = id), String::from("(_ BitVec 64)")));

        self.additional_assumptions.push(format!(
            "(= y32_{id} (bvlshr {x} #x0000000000000020))",
            x = x,
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y32_{id} (_ bv0 64))) (= ret1_{id} ret0_{id}) (= ret1_{id} (bvadd ret0_{id} (_ bv32 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y32_{id} (_ bv0 64))) (= x32_{id} y32_{id}) (= x32_{id} {x}))",
            x = x,
            id = id
        ));

        // round 2
        self.additional_decls
            .push((format!("ret2_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y16_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("x16_{id}", id = id), String::from("(_ BitVec 64)")));

        self.additional_assumptions.push(format!(
            "(= y16_{id} (bvlshr x32_{id} #x0000000000000010))",
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y16_{id} (_ bv0 64))) (= ret2_{id} ret1_{id}) (= ret2_{id} (bvadd ret1_{id} (_ bv16 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y16_{id} (_ bv0 64))) (= x16_{id} y16_{id}) (= x16_{id} x32_{id}))",
            id = id
        ));

        // round 3
        self.additional_decls
            .push((format!("ret3_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y8_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("x8_{id}", id = id), String::from("(_ BitVec 64)")));

        self.additional_assumptions.push(format!(
            "(= y8_{id} (bvlshr x16_{id} #x0000000000000008))",
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y8_{id} (_ bv0 64))) (= ret3_{id} ret2_{id}) (= ret3_{id} (bvadd ret2_{id} (_ bv8 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y8_{id} (_ bv0 64))) (= x8_{id} y8_{id}) (= x8_{id} x16_{id}))",
            id = id
        ));

        // round 4
        self.additional_decls
            .push((format!("ret4_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y4_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("x4_{id}", id = id), String::from("(_ BitVec 64)")));

        self.additional_assumptions.push(format!(
            "(= y4_{id} (bvlshr x8_{id} #x0000000000000004))",
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y4_{id} (_ bv0 64))) (= ret4_{id} ret3_{id}) (= ret4_{id} (bvadd ret3_{id} (_ bv4 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y4_{id} (_ bv0 64))) (= x4_{id} y4_{id}) (= x4_{id} x8_{id}))",
            id = id
        ));

        // round 5
        self.additional_decls
            .push((format!("ret5_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y2_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("x2_{id}", id = id), String::from("(_ BitVec 64)")));

        self.additional_assumptions.push(format!(
            "(= y2_{id} (bvlshr x4_{id} #x0000000000000002))",
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y2_{id} (_ bv0 64))) (= ret5_{id} ret4_{id}) (= ret5_{id} (bvadd ret4_{id} (_ bv2 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y2_{id} (_ bv0 64))) (= x2_{id} y2_{id}) (= x2_{id} x4_{id}))",
            id = id
        ));

        // round 6
        self.additional_decls
            .push((format!("ret6_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("y1_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_decls
            .push((format!("x1_{id}", id = id), String::from("(_ BitVec 64)")));

        self.additional_assumptions.push(format!(
            "(= y1_{id} (bvlshr x2_{id} #x0000000000000001))",
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y1_{id} (_ bv0 64))) (= ret6_{id} ret5_{id}) (= ret6_{id} (bvadd ret5_{id} (_ bv1 64))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y1_{id} (_ bv0 64))) (= x1_{id} y1_{id}) (= x1_{id} x2_{id}))",
            id = id
        ));

        // last round
        self.additional_decls
            .push((format!("ret7_{id}", id = id), String::from("(_ BitVec 64)")));
        self.additional_assumptions.push(format!("(ite (not (= x1_{id} (_ bv0 64))) (= ret7_{id} ret6_{id}) (= ret7_{id} (bvadd ret6_{id} (_ bv1 64))))", id = id));

        // final return
        format!("ret7_{id}", id = id)
    }

    pub fn clz32(&mut self, x: &String, id: u32) -> String {
        let x = format!("((_ extract 31 0) {})", x);

        // total zeros counter
        self.additional_decls
            .push((format!("ret0_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_assumptions
            .push(format!("(= ret0_{id} (_ bv0 32))", id = id));

        // round 1
        self.additional_decls
            .push((format!("ret1_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("y16_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x16_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions.push(format!(
            "(= y16_{id} (bvlshr {x} #x00000010))",
            x = x,
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y16_{id} (_ bv0 32))) (= ret1_{id} ret0_{id}) (= ret1_{id} (bvadd ret0_{id} (_ bv16 32))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y16_{id} (_ bv0 32))) (= x16_{id} y16_{id}) (= x16_{id} {x}))",
            x = x,
            id = id
        ));

        // round 2
        self.additional_decls
            .push((format!("ret2_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("y8_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x8_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions
            .push(format!("(= y8_{id} (bvlshr x16_{id} #x00000008))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y8_{id} (_ bv0 32))) (= ret2_{id} ret1_{id}) (= ret2_{id} (bvadd ret1_{id} (_ bv8 32))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y8_{id} (_ bv0 32))) (= x8_{id} y8_{id}) (= x8_{id} x16_{id}))",
            id = id
        ));

        // round 3
        self.additional_decls
            .push((format!("ret3_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("y4_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x4_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions
            .push(format!("(= y4_{id} (bvlshr x8_{id} #x00000004))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y4_{id} (_ bv0 32))) (= ret3_{id} ret2_{id}) (= ret3_{id} (bvadd ret2_{id} (_ bv4 32))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y4_{id} (_ bv0 32))) (= x4_{id} y4_{id}) (= x4_{id} x8_{id}))",
            id = id
        ));

        // round 4
        self.additional_decls
            .push((format!("ret4_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("y2_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x2_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions
            .push(format!("(= y2_{id} (bvlshr x4_{id} #x00000002))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y2_{id} (_ bv0 32))) (= ret4_{id} ret3_{id}) (= ret4_{id} (bvadd ret3_{id} (_ bv2 32))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y2_{id} (_ bv0 32))) (= x2_{id} y2_{id}) (= x2_{id} x4_{id}))",
            id = id
        ));

        // round 5
        self.additional_decls
            .push((format!("ret5_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("y1_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_decls
            .push((format!("x1_{id}", id = id), String::from("(_ BitVec 32)")));

        self.additional_assumptions
            .push(format!("(= y1_{id} (bvlshr x2_{id} #x00000001))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y1_{id} (_ bv0 32))) (= ret5_{id} ret4_{id}) (= ret5_{id} (bvadd ret4_{id} (_ bv1 32))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y1_{id} (_ bv0 32))) (= x1_{id} y1_{id}) (= x1_{id} x2_{id}))",
            id = id
        ));

        // last round
        self.additional_decls
            .push((format!("ret6_{id}", id = id), String::from("(_ BitVec 32)")));
        self.additional_assumptions.push(format!("(ite (not (= x1_{id} (_ bv0 32))) (= ret6_{id} ret5_{id}) (= ret6_{id} (bvadd ret5_{id} (_ bv1 32))))", id = id));

        // final return
        let padding = self.new_fresh_bits(self.bitwidth - 32);
        format!("(concat {padding} ret6_{id})", padding = padding, id = id)
    }

    pub fn clz16(&mut self, x: &String, id: u32) -> String {
        let x = format!("((_ extract 15 0) {})", x);

        // total zeros counter
        self.additional_decls
            .push((format!("ret1_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_assumptions
            .push(format!("(= ret1_{id} (_ bv0 16))", id = id));

        // round 1
        self.additional_decls
            .push((format!("ret2_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_decls
            .push((format!("y8_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_decls
            .push((format!("x8_{id}", id = id), String::from("(_ BitVec 16)")));

        self.additional_assumptions.push(format!(
            "(= y8_{id} (bvlshr {x} #x0008))",
            x = x,
            id = id
        ));
        self.additional_assumptions.push(format!("(ite (not (= y8_{id} (_ bv0 16))) (= ret2_{id} ret1_{id}) (= ret2_{id} (bvadd ret1_{id} (_ bv8 16))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y8_{id} (_ bv0 16))) (= x8_{id} y8_{id}) (= x8_{id} {x}))",
            x = x,
            id = id
        ));

        // round 2
        self.additional_decls
            .push((format!("ret3_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_decls
            .push((format!("y4_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_decls
            .push((format!("x4_{id}", id = id), String::from("(_ BitVec 16)")));

        self.additional_assumptions
            .push(format!("(= y4_{id} (bvlshr x8_{id} #x0004))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y4_{id} (_ bv0 16))) (= ret3_{id} ret2_{id}) (= ret3_{id} (bvadd ret2_{id} (_ bv4 16))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y4_{id} (_ bv0 16))) (= x4_{id} y4_{id}) (= x4_{id} x8_{id}))",
            id = id
        ));

        // round 3
        self.additional_decls
            .push((format!("ret4_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_decls
            .push((format!("y2_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_decls
            .push((format!("x2_{id}", id = id), String::from("(_ BitVec 16)")));

        self.additional_assumptions
            .push(format!("(= y2_{id} (bvlshr x4_{id} #x0002))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y2_{id} (_ bv0 16))) (= ret4_{id} ret3_{id}) (= ret4_{id} (bvadd ret3_{id} (_ bv2 16))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y2_{id} (_ bv0 16))) (= x2_{id} y2_{id}) (= x2_{id} x4_{id}))",
            id = id
        ));

        // round 4
        self.additional_decls
            .push((format!("ret5_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_decls
            .push((format!("y1_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_decls
            .push((format!("x1_{id}", id = id), String::from("(_ BitVec 16)")));

        self.additional_assumptions
            .push(format!("(= y1_{id} (bvlshr x2_{id} #x0001))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y1_{id} (_ bv0 16))) (= ret5_{id} ret4_{id}) (= ret5_{id} (bvadd ret4_{id} (_ bv1 16))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y1_{id} (_ bv0 16))) (= x1_{id} y1_{id}) (= x1_{id} x2_{id}))",
            id = id
        ));

        // last round
        self.additional_decls
            .push((format!("ret6_{id}", id = id), String::from("(_ BitVec 16)")));
        self.additional_assumptions.push(format!("(ite (not (= x1_{id} (_ bv0 16))) (= ret6_{id} ret5_{id}) (= ret6_{id} (bvadd ret5_{id} (_ bv1 16))))", id = id));

        // final return
        let padding = self.new_fresh_bits(self.bitwidth - 16);
        format!("(concat {padding} ret6_{id})", padding = padding, id = id)
    }

    pub fn clz8(&mut self, x: &String, id: u32) -> String {
        let x = format!("((_ extract 7 0) {})", x);

        // total zeros counter
        self.additional_decls
            .push((format!("ret0_{id}", id = id), String::from("(_ BitVec 8)")));
        self.additional_assumptions
            .push(format!("(= ret0_{id} (_ bv0 8))", id = id));

        // round 1
        self.additional_decls
            .push((format!("ret3_{id}", id = id), String::from("(_ BitVec 8)")));
        self.additional_decls
            .push((format!("y4_{id}", id = id), String::from("(_ BitVec 8)")));
        self.additional_decls
            .push((format!("x4_{id}", id = id), String::from("(_ BitVec 8)")));

        self.additional_assumptions
            .push(format!("(= y4_{id} (bvlshr {x} #x04))", x = x, id = id));
        self.additional_assumptions.push(format!("(ite (not (= y4_{id} (_ bv0 8))) (= ret3_{id} ret0_{id}) (= ret3_{id} (bvadd ret0_{id} (_ bv4 8))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y4_{id} (_ bv0 8))) (= x4_{id} y4_{id}) (= x4_{id} {x}))",
            x = x,
            id = id
        ));

        // round 2
        self.additional_decls
            .push((format!("ret4_{id}", id = id), String::from("(_ BitVec 8)")));
        self.additional_decls
            .push((format!("y2_{id}", id = id), String::from("(_ BitVec 8)")));
        self.additional_decls
            .push((format!("x2_{id}", id = id), String::from("(_ BitVec 8)")));

        self.additional_assumptions
            .push(format!("(= y2_{id} (bvlshr x4_{id} #x02))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y2_{id} (_ bv0 8))) (= ret4_{id} ret3_{id}) (= ret4_{id} (bvadd ret3_{id} (_ bv2 8))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y2_{id} (_ bv0 8))) (= x2_{id} y2_{id}) (= x2_{id} x4_{id}))",
            id = id
        ));

        // round 3
        self.additional_decls
            .push((format!("ret5_{id}", id = id), String::from("(_ BitVec 8)")));
        self.additional_decls
            .push((format!("y1_{id}", id = id), String::from("(_ BitVec 8)")));
        self.additional_decls
            .push((format!("x1_{id}", id = id), String::from("(_ BitVec 8)")));

        self.additional_assumptions
            .push(format!("(= y1_{id} (bvlshr x2_{id} #x01))", id = id));
        self.additional_assumptions.push(format!("(ite (not (= y1_{id} (_ bv0 8))) (= ret5_{id} ret4_{id}) (= ret5_{id} (bvadd ret4_{id} (_ bv1 8))))", id = id));
        self.additional_assumptions.push(format!(
            "(ite (not (= y1_{id} (_ bv0 8))) (= x1_{id} y1_{id}) (= x1_{id} x2_{id}))",
            id = id
        ));

        // last round
        self.additional_decls
            .push((format!("ret6_{id}", id = id), String::from("(_ BitVec 8)")));
        self.additional_assumptions.push(format!("(ite (not (= x1_{id} (_ bv0 8))) (= ret6_{id} ret5_{id}) (= ret6_{id} (bvadd ret5_{id} (_ bv1 8))))", id = id));

        // final return
        let padding = self.new_fresh_bits(self.bitwidth - 8);
        format!("(concat {padding} ret6_{id})", padding = padding, id = id)
    }

    pub fn clz1(&mut self, x: &String, id: u32) -> String {
        let extract = format!("((_ extract 0 0) {})", x);
        self.additional_decls
            .push((format!("ret_{id}", id = id), String::from("(_ BitVec 1)")));
        self.additional_assumptions
            .push(format!("(= ret_{id} (bvnot {x}))", id = id, x = extract));

        let padding = self.new_fresh_bits(self.bitwidth - 1);
        format!("(concat {padding} ret_{id})", padding = padding, id = id)
    }

    pub fn vir_expr_to_rsmt2_str(&mut self, e: Expr) -> String {
        let tyvar = self.tyctx.tyvars.get(&e);
        let ty = &self.get_type(&e);
        let width = self.get_expr_width_var(&e).map(|s| s.clone());
        let static_expr_width = self.static_width(&e);
        match e {
            Expr::Terminal(t) => match t {
                Terminal::Var(v) => match self.var_map.get(&v) {
                    Some(o) => o.clone(),
                    None => v,
                },
                Terminal::Const(i, _) => match ty.unwrap() {
                    Type::BitVector(w) => {
                        let var = *tyvar.unwrap();
                        let width = w.unwrap_or(self.bitwidth);
                        let narrow_decl = format!("(_ bv{} {})", i, width);
                        self.widen_to_query_width(var, width, narrow_decl, None)
                    }
                    Type::Int => i.to_string(),
                    Type::Bool => {
                        if i == 0 {
                            "false".to_string()
                        } else {
                            "true".to_string()
                        }
                    }
                },
                Terminal::True => "true".to_string(),
                Terminal::False => "false".to_string(),
                Terminal::Wildcard => match ty.unwrap() {
                    Type::BitVector(_) => self.new_fresh_bits(self.bitwidth),
                    Type::Int => self.new_fresh_int(),
                    Type::Bool => self.new_fresh_bool(),
                },
            },
            Expr::Unary(op, arg) => {
                let op = match op {
                    UnaryOp::Not => "not",
                    UnaryOp::BVNeg => {
                        self.assume_same_width_from_string(&width.unwrap(), &*arg);
                        "bvneg"
                    }
                    UnaryOp::BVNot => {
                        self.assume_same_width_from_string(&width.unwrap(), &*arg);
                        "bvnot"
                    }
                };
                format!("({} {})", op, self.vir_expr_to_rsmt2_str(*arg))
            }
            Expr::Binary(op, x, y) => {
                match op {
                    BinaryOp::BVAdd | BinaryOp::BVSub | BinaryOp::BVAnd => {
                        self.assume_comparable_types(&*x, &*y)
                    }
                    _ => (),
                };
                match op {
                    BinaryOp::BVMul
                    | BinaryOp::BVAdd
                    | BinaryOp::BVSub
                    | BinaryOp::BVAnd
                    | BinaryOp::BVOr
                    | BinaryOp::BVShl
                    | BinaryOp::BVShr
                    | BinaryOp::BVRotl => self.assume_same_width_from_string(&width.unwrap(), &*x),
                    _ => (),
                };
                match op {
                    BinaryOp::BVRotl => {
                        let arg_width = self.get_expr_width_var(&*x).unwrap().clone();
                        let xs = self.vir_expr_to_rsmt2_str(*x);
                        let ys = self.vir_expr_to_rsmt2_str(*y);
                        return self.rotate_symbolic(&xs, &arg_width, &ys, "rotate_left");
                        // // SMT bitvector rotate_left requires that the rotate amount be
                        // // statically specified. Instead, to use a dynamic amount, desugar
                        // // to shifts and bit arithmetic.
                        // return format!(
                        //     "(bvor (bvshl {x} {y}) (bvlshr {x} (bvsub {width} {y})))",
                        //     x = self.vir_expr_to_rsmt2_str(*x),
                        //     y = self.vir_expr_to_rsmt2_str(*y),
                        //     width = format!("(_ bv{} {})", self.bitwidth, self.bitwidth)
                        // );
                    }
                    BinaryOp::BVRotr => {
                        let arg_width = self.get_expr_width_var(&*x).unwrap().clone();
                        let xs = self.vir_expr_to_rsmt2_str(*x);
                        let ys = self.vir_expr_to_rsmt2_str(*y);
                        return self.rotate_symbolic(&xs, &arg_width, &ys, "rotate_right");
                    }
                    // To shift right, we need to make sure the bits to the right get zeroed. Shift left first.
                    BinaryOp::BVShr => {
                        let arg_width = self.get_expr_width_var(&*x).unwrap().clone();
                        let xs = self.vir_expr_to_rsmt2_str(*x);
                        let ys = self.vir_expr_to_rsmt2_str(*y);

                        // Strategy: shift right by (bitwidth - arg width) to zero bits to the right
                        // of the bits in the argument size. Then shift right by (amt + (bitwidth - arg width))

                        // Width math
                        let arg_width_as_bv =
                            format!("((_ int2bv {}) {})", self.bitwidth, arg_width);
                        let bitwidth_as_bv = format!("(_ bv{} {})", self.bitwidth, self.bitwidth);
                        let extra_shift =
                            format!(" (bvsub {} {})", bitwidth_as_bv, arg_width_as_bv);
                        let shl_to_zero = format!("(bvshl {} {})", xs, extra_shift);

                        let amt_plus_extra = format!("(bvadd {} {})", ys, extra_shift);
                        return format!("(bvlshr {} {})", shl_to_zero, amt_plus_extra);
                    }
                    _ => (),
                };
                let op = match op {
                    BinaryOp::And => "and",
                    BinaryOp::Or => "or",
                    BinaryOp::Imp => "=>",
                    BinaryOp::Eq => "=",
                    // TODO: this comparison only works for Ints!!
                    BinaryOp::Lte => "<=",
                    BinaryOp::BVMul => "bvmul",
                    BinaryOp::BVAdd => "bvadd",
                    BinaryOp::BVSub => "bvsub",
                    BinaryOp::BVAnd => "bvand",
                    BinaryOp::BVOr => "bvor",
                    BinaryOp::BVXor => "bvxor",
                    BinaryOp::BVShl => "bvshl",
                    _ => unreachable!("{:?}", op),
                };
                format!(
                    "({} {} {})",
                    op,
                    self.vir_expr_to_rsmt2_str(*x),
                    self.vir_expr_to_rsmt2_str(*y)
                )
            }
            Expr::BVIntToBV(w, x) => {
                let padded_width = self.bitwidth - w;
                format!(
                    "((_ zero_extend {}) ((_ int2bv {}) {}))",
                    padded_width,
                    w,
                    self.vir_expr_to_rsmt2_str(*x)
                )
            }
            Expr::BVConvTo(y) => {
                // For static convto, width constraints are handling during inference
                self.vir_expr_to_rsmt2_str(*y)
            }
            Expr::BVZeroExtTo(i, x) => {
                let arg_width = self.get_expr_width_var(&*x).unwrap().clone();
                let static_width = self.static_width(&*x);
                let expr_width = width.unwrap().clone();
                self.width_assumptions
                    .push(format!("(= {} {})", expr_width, i));
                let xs = self.vir_expr_to_rsmt2_str(*x);
                if let Some(size) = static_width {
                    self.extend_concrete(i, &xs, size, &"zero_extend")
                } else {
                    let is = i.to_string();
                    self.extend_symbolic(&is, &xs, &arg_width, &"zero_extend")
                }
            }
            Expr::BVZeroExtToVarWidth(i, x) => {
                let arg_width = self.get_expr_width_var(&*x).unwrap().clone();
                let static_arg_width = self.static_width(&*x);
                let expr_width = width.unwrap().clone();
                let is = self.vir_expr_to_rsmt2_str(*i);
                let xs = self.vir_expr_to_rsmt2_str(*x);
                self.width_assumptions
                    .push(format!("(= {} {})", expr_width, is));
                if let (Some(arg_size), Some(e_size)) = (static_arg_width, static_expr_width) {
                    self.extend_concrete(e_size, &xs, arg_size, &"zero_extend")
                } else {
                    self.extend_symbolic(&is, &xs, &arg_width, &"zero_extend")
                }
            }
            Expr::BVSignExtTo(i, x) => {
                let arg_width = self.get_expr_width_var(&*x).unwrap().clone();
                let static_width = self.static_width(&*x);
                let expr_width = width.unwrap().clone();
                self.width_assumptions
                    .push(format!("(= {} {})", expr_width, i));
                let xs = self.vir_expr_to_rsmt2_str(*x);
                if let Some(size) = static_width {
                    self.extend_concrete(i, &xs, size, &"sign_extend")
                } else {
                    let is = i.to_string();
                    self.extend_symbolic(&is, &xs, &arg_width, &"sign_extend")
                }
            }
            Expr::BVSignExtToVarWidth(i, x) => {
                let arg_width = self.get_expr_width_var(&*x).unwrap().clone();
                let static_arg_width = self.static_width(&*x);
                let expr_width = width.unwrap().clone();
                let is = self.vir_expr_to_rsmt2_str(*i);
                let xs = self.vir_expr_to_rsmt2_str(*x);
                self.width_assumptions
                    .push(format!("(= {} {})", expr_width, is));
                if let (Some(arg_size), Some(e_size)) = (static_arg_width, static_expr_width) {
                    self.extend_concrete(e_size, &xs, arg_size, &"sign_extend")
                } else {
                    self.extend_symbolic(&is, &xs, &arg_width, &"sign_extend")
                }
            }
            Expr::BVConvToVarWidth(x, y) => {
                let expr_width = width.unwrap().clone();
                let dyn_width = self.vir_expr_to_rsmt2_str(*x);
                self.width_assumptions
                    .push(format!("(= {} {})", expr_width, dyn_width));
                self.vir_expr_to_rsmt2_str(*y)
            }
            Expr::UndefinedTerm(term) => term.ret.name,
            Expr::WidthOf(x) => self.get_expr_width_var(&*x).unwrap().clone(),
            Expr::BVExtract(i, j, x) => {
                assert!(i > j);
                if let Type::BitVector(x_width) = ty.unwrap() {
                    assert!(i < x_width.unwrap());
                    let xs = self.vir_expr_to_rsmt2_str(*x);
                    let extract = format!("((_ extract {} {}) {})", i, j, xs);
                    let new_width = i - j + 1;
                    let padding =
                        self.new_fresh_bits(self.bitwidth.checked_sub(new_width).unwrap());
                    format!("(concat {} {})", padding, extract)
                } else {
                    unreachable!("Must perform extraction on bv with known width")
                }
            }
            Expr::Conditional(c, t, e) => {
                format!(
                    "(ite {} {} {})",
                    self.vir_expr_to_rsmt2_str(*c),
                    self.vir_expr_to_rsmt2_str(*t),
                    self.vir_expr_to_rsmt2_str(*e)
                )
            }
            Expr::CLZ(e) => {
                let tyvar = *tyvar.unwrap();
                let es = self.vir_expr_to_rsmt2_str(*e);
                match static_expr_width {
                    Some(1) => self.clz1(&es, tyvar),
                    Some(8) => self.clz8(&es, tyvar),
                    Some(16) => self.clz16(&es, tyvar),
                    Some(32) => self.clz32(&es, tyvar),
                    Some(64) => self.clz64(&es, tyvar),
                    Some(w) => unreachable!("Unexpected CLZ width {}", w),
                    None => unreachable!("Need static CLZ width"),
                }
            }
            Expr::A64CLZ(ty, e) => {
                let tyvar = *tyvar.unwrap();
                let es = self.vir_expr_to_rsmt2_str(*e);
                let val = self.get_expr_value(&*ty);
                match val {
                    Some(32) => self.a64clz32(&es, tyvar),
                    Some(64) => self.clz64(&es, tyvar),
                    Some(w) => {
                        println!("Unexpected A64CLZ width {}", w);
                        self.additional_assumptions.push("false".to_string());
                        es
                    }
                    None => {
                        println!("Need static A64CLZ width");
                        self.additional_assumptions.push("false".to_string());
                        es
                    }
                }
            }
        }
    }

    // Checks whether the assumption list is always false
    fn check_assumptions_feasibility<Parser>(
        &self,
        solver: &mut Solver<Parser>,
        assumptions: Vec<String>,
    ) -> bool {
        println!("Checking assumption feasibility");
        solver.push(1).unwrap();
        for a in assumptions {
            // println!("{}", &a);
            solver.assert(a).unwrap();

            // Uncomment to debug specific asserts
            // solver.push(2).unwrap();
            // let _ = match solver.check_sat() {
            //     Ok(true) => {
            //         println!("Assertion list is feasible");
            //         true
            //     }
            //     Ok(false) => {
            //         println!("Assertion list is infeasible!");
            //         panic!();
            //         false
            //     }
            //     Err(err) => {
            //         unreachable!("Error! {:?}", err);
            //     }
            // };
            // solver.pop(2).unwrap();
        }
        let res = match solver.check_sat() {
            Ok(true) => {
                println!("Assertion list is feasible");
                true
            }
            Ok(false) => {
                println!("Assertion list is infeasible!");
                false
            }
            Err(err) => {
                unreachable!("Error! {:?}", err);
            }
        };
        solver.pop(1).unwrap();
        res
    }
}

/// Overall query for single rule:
/// <declare vars>
/// (not (=> <assumptions> (= <LHS> <RHS>))))))
/// Overall query for multiple rules (out of date):
/// <declare vars>
/// (not (=> (and
///             <all rules' assumptions>
///             <between rule assumptions>
///             <all but first rule's <LHS> = <RHS>>)
///          (= <first rule LHS> <first rule RHS>))))))
pub fn run_solver(rule_sem: RuleSemantics, query_width: usize) -> VerificationResult {
    println!("Verifying with query width: {}", query_width);
    let mut solver = Solver::default_z3(()).unwrap();

    let mut assumptions: Vec<String> = vec![];

    let mut ctx = SolverCtx {
        tyctx: rule_sem.tyctx,
        bitwidth: REG_WIDTH,
        var_map: HashMap::new(),
        width_vars: HashMap::new(),
        width_assumptions: vec![],
        additional_decls: vec![],
        additional_assumptions: vec![],
        fresh_bits_idx: 0,
    };

    // Use the query width for any free variables with unspecified bitwidths
    let mut query_width_used = false;
    let mut query_bv_set_idxs = HashSet::new();
    for v in &rule_sem.free_vars {
        let ty = &ctx.tyctx.tymap[&v.tyvar];
        if let Type::BitVector(None) = ty {
            query_width_used = true;
            ctx.tyctx
                .tymap
                .insert(v.tyvar, Type::BitVector(Some(query_width)));
            let bv_set_idx = ctx.tyctx.bv_unknown_width_sets[&v.tyvar];
            query_bv_set_idxs.insert(bv_set_idx);
        }
    }
    if !query_width_used {
        panic!("Query width unused, check rule!");
    }

    for (_e, t) in &ctx.tyctx.tyvars {
        // dbg!(t);
        // dbg!(&_e);
        let ty = &ctx.tyctx.tymap[&t];
        match ty {
            Type::BitVector(w) => {
                let width_name = format!("width__{}", t);
                ctx.additional_decls
                    .push((width_name.clone(), "Int".to_string()));
                match w {
                    Some(bitwidth) => {
                        ctx.width_assumptions
                            .push(format!("(= {} {})", width_name, bitwidth));
                    }
                    None => {
                        let bv_set_idx = ctx.tyctx.bv_unknown_width_sets[&t];
                        if query_bv_set_idxs.contains(&bv_set_idx) {
                            ctx.tyctx
                                .tymap
                                .insert(*t, Type::BitVector(Some(query_width)));
                            ctx.width_assumptions
                                .push(format!("(= {} {})", width_name, query_width));
                        }
                    }
                };
                ctx.width_vars.insert(*t, width_name.clone());
            }
            _ => (),
        }
    }

    println!("Declaring quantified variables");
    for v in &rule_sem.quantified_vars {
        let name = &v.name;
        let ty = ctx.tyctx.tymap[&v.tyvar].clone();
        let var_ty = ctx.vir_to_rsmt2_constant_ty(&ty);
        println!("\t{} : {:?}", name, var_ty);
        if let Type::BitVector(w) = ty {
            let wide = ctx.widen_to_query_width(
                v.tyvar,
                w.unwrap_or(ctx.bitwidth),
                name.clone(),
                Some(name.to_string()),
            );
            ctx.var_map.insert(name.clone(), wide);
        }
        solver.declare_const(name, var_ty).unwrap();
    }

    println!("Adding explicit assumptions");
    for a in &rule_sem.assumptions {
        let p = ctx.vir_expr_to_rsmt2_str(a.clone());
        // println!("\t{}", p);
        assumptions.push(p)
    }
    println!("Adding width assumptions");
    for a in &ctx.width_assumptions {
        // println!("\t{}", a);
        assumptions.push(a.clone());
    }
    println!("Adding additional assumptions");
    for a in &ctx.additional_assumptions {
        // println!("\t{}", a);
        assumptions.push(a.clone());
    }

    println!("Declaring additional variables");
    for (name, ty) in &ctx.additional_decls {
        println!("\t{} : {:?}", name, ty);
        solver.declare_const(name, ty).unwrap();
    }

    let assumption_str = format!("(and {})", assumptions.join(" "));

    // Check whether the assumptions are possible
    if !ctx.check_assumptions_feasibility(&mut solver, assumptions) {
        println!("Rule not applicable as written for rule assumptions, skipping full query");
        return VerificationResult::InapplicableRule;
    }

    // Correctness query
    // Verification condition: first rule's LHS and RHS are equal
    let width = match (
        ctx.static_width(&rule_sem.lhs),
        ctx.static_width(&rule_sem.rhs),
    ) {
        (Some(w), None) | (None, Some(w)) => w,
        (Some(w1), Some(w2)) => {
            assert_eq!(w1, w2);
            w1
        }
        (None, None) => {
            println!(
                "Width of relevant bits of LHS and RHS unknown, using full register bitwidth: {}",
                REG_WIDTH
            );
            REG_WIDTH
        }
    };

    let lhs = ctx.vir_expr_to_rsmt2_str(rule_sem.lhs);
    let rhs = ctx.vir_expr_to_rsmt2_str(rule_sem.rhs);

    let lhs_care_bits = format!("((_ extract {} {}) {})", width - 1, 0, &lhs);
    let rhs_care_bits = format!("((_ extract {} {}) {})", width - 1, 0, &rhs);

    let side_equality = format!("(= {} {})", lhs_care_bits, rhs_care_bits);
    println!("LHS and RHS equality condition:\n\t{}\n", side_equality);

    let query = format!("(not (=> {} {}))", assumption_str, side_equality);
    println!("Running query");
    // println!("Running query:\n\t{}\n", query);
    solver.assert(query).unwrap();

    match solver.check_sat() {
        Ok(true) => {
            println!("Verification failed");
            let model = solver.get_model().unwrap();
            dbg!(model);
            VerificationResult::Failure(Counterexample {})
        }
        Ok(false) => {
            println!("Verification succeeded");
            VerificationResult::Success
        }
        Err(err) => {
            unreachable!("Error! {:?}", err);
        }
    }
}

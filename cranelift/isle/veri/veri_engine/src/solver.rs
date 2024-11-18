/// Convert our internal Verification IR to an external SMT AST and pass
/// queries to that solver.
///
/// This uses the easy-smt crate to interact with any solver.
///
use cranelift_isle as isle;
use isle::sema::{Pattern, Rule, TermEnv, TypeEnv};

use crate::solver::encoded_ops::popcnt::popcnt;
use crate::type_inference::RuleSemantics;
use crate::Config;
use easy_smt::{Response, SExpr};
use std::cmp::Ordering;
use std::collections::HashMap;
use veri_ir::{
    BinaryOp, ConcreteTest, Counterexample, Expr, TermSignature, Terminal, Type, TypeContext,
    UnaryOp, VerificationResult,
};

mod encoded_ops;

use encoded_ops::cls;
use encoded_ops::clz;
use encoded_ops::rev;
use encoded_ops::subs;

use crate::MAX_WIDTH;

pub struct SolverCtx {
    smt: easy_smt::Context,
    pub find_widths: bool,
    tyctx: TypeContext,
    pub bitwidth: usize,
    var_map: HashMap<String, SExpr>,
    width_vars: HashMap<u32, String>,
    width_assumptions: Vec<SExpr>,
    pub additional_decls: Vec<(String, SExpr)>,
    pub additional_assumptions: Vec<SExpr>,
    pub additional_assertions: Vec<SExpr>,
    fresh_bits_idx: usize,
    lhs_load_args: Option<Vec<SExpr>>,
    rhs_load_args: Option<Vec<SExpr>>,
    lhs_store_args: Option<Vec<SExpr>>,
    rhs_store_args: Option<Vec<SExpr>>,
    load_return: Option<SExpr>,
    lhs_flag: bool,
}

pub struct RuleCtx<'a> {
    rule_sem: &'a RuleSemantics,
    rule: &'a Rule,
    termenv: &'a TermEnv,
    typeenv: &'a TypeEnv,
    config: &'a Config,
}

impl SolverCtx {
    pub fn new_fresh_bits(&mut self, width: usize) -> SExpr {
        let name = format!("fresh{}", self.fresh_bits_idx);
        self.fresh_bits_idx += 1;
        self.additional_decls
            .push((name.clone(), self.smt.bit_vec_sort(self.smt.numeral(width))));
        self.smt.atom(name)
    }

    fn new_fresh_int(&mut self) -> SExpr {
        let name = format!("fresh{}", self.fresh_bits_idx);
        self.fresh_bits_idx += 1;
        self.additional_decls
            .push((name.clone(), self.smt.int_sort()));
        self.smt.atom(name)
    }

    fn new_fresh_bool(&mut self) -> SExpr {
        let name = format!("fresh{}", self.fresh_bits_idx);
        self.fresh_bits_idx += 1;
        self.additional_decls
            .push((name.clone(), self.smt.bool_sort()));
        self.smt.atom(name)
    }

    fn declare(&mut self, name: String, typ: SExpr) -> SExpr {
        let atom = self.smt.atom(&name);
        self.additional_decls.push((name, typ));
        atom
    }

    fn assume(&mut self, expr: SExpr) {
        self.additional_assumptions.push(expr);
    }

    fn assert(&mut self, expr: SExpr) {
        self.additional_assertions.push(expr);
    }

    /// Construct a constant bit-vector value of the given width. (This is used so pervasively that
    /// perhaps we should submit it for inclusion in the easy_smt library...)
    fn bv(&self, value: i128, width: usize) -> SExpr {
        if value < 0 {
            return self
                .smt
                .list(vec![self.smt.atom("bvneg"), self.bv(-value, width)]);
        }
        self.smt.list(vec![
            self.smt.atoms().und,
            self.smt.atom(format!("bv{}", value)),
            self.smt.numeral(width),
        ])
    }

    /// Convert an SMT integer to a bit vector of a given width.
    fn int2bv(&self, width: usize, value: SExpr) -> SExpr {
        self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom("int2bv"),
                self.smt.numeral(width),
            ]),
            value,
        ])
    }

    /// Convert an SMT bit vector to a nat.
    fn bv2nat(&self, value: SExpr) -> SExpr {
        self.smt.list(vec![self.smt.atom("bv2nat"), value])
    }

    /// Zero-extend an SMT bit vector to a wider bit vector by adding `padding` zeroes to the
    /// front.
    fn zero_extend(&self, padding: usize, value: SExpr) -> SExpr {
        if padding == 0 {
            return value;
        }
        self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom("zero_extend"),
                self.smt.numeral(padding),
            ]),
            value,
        ])
    }

    /// Sign-extend an SMT bit vector to a wider bit vector by adding `padding` zeroes to the
    /// front.
    fn sign_extend(&self, padding: usize, value: SExpr) -> SExpr {
        self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom("sign_extend"),
                self.smt.numeral(padding),
            ]),
            value,
        ])
    }

    // Extend with concrete source and destination sizes. Includes extracting relevant bits.
    fn extend_concrete(
        &mut self,
        dest_width: usize,
        source: SExpr,
        source_width: usize,
        op: &str,
    ) -> SExpr {
        if dest_width < source_width {
            log::warn!(
                "Unexpected extend widths for {}: dest {} source {} ",
                self.smt.display(source),
                dest_width,
                source_width,
            );
            self.assert(self.smt.false_());
            return self.bv(
                0,
                if self.find_widths {
                    self.bitwidth
                } else {
                    dest_width
                },
            );
        }

        let delta = dest_width - source_width;
        if !self.find_widths {
            return self.smt.list(vec![
                self.smt.list(vec![
                    self.smt.atoms().und,
                    self.smt.atom(op),
                    self.smt.numeral(delta),
                ]),
                source,
            ]);
        }

        // Extract the relevant bits of the source (which is modeled with a wider,
        // register-width bitvector).
        let extract = self
            .smt
            .extract(source_width.wrapping_sub(1).try_into().unwrap(), 0, source);

        // Do the extend itself.
        let extend = self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom(op),
                self.smt.numeral(delta),
            ]),
            extract,
        ]);

        // Pad the extended result back to the full register bitwidth. Use the bits
        // that were already in the source register. That is, given:
        //                       reg - source width              source width
        //                                |                           |
        // SOURCE: [               don't care bits           |   care bits    ]
        //
        //                             dest width
        //                                |
        // OUT:    [ same don't care bits |  defined extend  |   care bits     ]
        let mut unconstrained_bits = 0;
        if dest_width < self.bitwidth {
            unconstrained_bits = self
                .bitwidth
                .checked_sub(delta)
                .unwrap()
                .checked_sub(source_width)
                .unwrap();
        }

        // If we are extending to the full register width, no padding needed
        if unconstrained_bits == 0 {
            extend
        } else {
            let padding = self.smt.extract(
                self.bitwidth.checked_sub(1).unwrap().try_into().unwrap(),
                self.bitwidth
                    .checked_sub(unconstrained_bits)
                    .unwrap()
                    .try_into()
                    .unwrap(),
                source,
            );
            self.smt.concat(padding, extend)
        }
    }

    // SMT-LIB only supports extends (zero or sign) by concrete amounts, but we
    // need symbolic ones. This method essentially does if-conversion over possible
    // concrete forms, outputting nested ITE blocks. We consider both the starting
    // width and the destination width to be potentially symbolic.
    // For safety, we add an assertion that some arm of this ITE must match.
    fn extend_symbolic(
        &mut self,
        dest_width: SExpr,
        source: SExpr,
        source_width: SExpr,
        op: &str,
    ) -> SExpr {
        if self.find_widths {
            return source;
        }
        // Symbolic expression for amount to shift
        let shift = self.smt.sub(dest_width, source_width);

        let mut some_match = vec![];
        let mut ite_str = source;

        // Special case: if we are asked to extend by 0, just return the source
        let matching = self.smt.eq(self.smt.numeral(0), shift);
        some_match.push(matching);
        ite_str = self.smt.ite(matching, source, ite_str);

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
                let matching = self.smt.and(
                    self.smt.eq(self.smt.numeral(possible_delta), shift),
                    self.smt.eq(self.smt.numeral(possible_source), source_width),
                );
                some_match.push(matching);
                let extend = self.extend_concrete(
                    possible_source + possible_delta,
                    source,
                    possible_source,
                    op,
                );
                ite_str = self.smt.ite(matching, extend, ite_str);
            }
        }
        let some_shift_matches = self.smt.or_many(some_match);
        self.width_assumptions.push(some_shift_matches);
        ite_str
    }

    fn encode_rotate(&self, op: &str, source: SExpr, amount: SExpr, width: usize) -> SExpr {
        // SMT bitvector rotate_left requires that the rotate amount be
        // statically specified. Instead, to use a dynamic amount, desugar
        // to shifts and bit arithmetic.
        let width_as_bv = self.bv(width.try_into().unwrap(), width);
        let wrapped_amount = self.smt.bvurem(amount, width_as_bv);
        let wrapped_delta = self.smt.bvsub(width_as_bv, wrapped_amount);
        match op {
            "rotate_left" => self.smt.bvor(
                self.smt.bvshl(source, wrapped_amount),
                self.smt.bvlshr(source, wrapped_delta),
            ),
            "rotate_right" => self.smt.bvor(
                self.smt.bvshl(source, wrapped_delta),
                self.smt.bvlshr(source, wrapped_amount),
            ),
            _ => unreachable!(),
        }
    }

    // SMT bitvector rotate requires that the rotate amount be
    // statically specified. Instead, to use a dynamic amount, desugar
    // to shifts and bit arithmetic.
    fn rotate_symbolic(
        &mut self,
        source: SExpr,
        source_width: usize,
        amount: SExpr,
        op: &str,
    ) -> SExpr {
        if self.find_widths {
            return source;
        }
        let (s, a) = if self.find_widths {
            // Extract the relevant bits of the source (which is modeled with a wider,
            // register-width bitvector).
            let extract_source = self.smt.extract(
                source_width.checked_sub(1).unwrap().try_into().unwrap(),
                0,
                source,
            );

            let extract_amount = self.smt.extract(
                source_width.checked_sub(1).unwrap().try_into().unwrap(),
                0,
                amount,
            );
            (extract_source, extract_amount)
        } else {
            (source, amount)
        };

        // Do the rotate itself.
        let rotate = self.encode_rotate(op, s, a, source_width);

        // Pad the extended result back to the full register bitwidth. Use the bits
        // that were already in the source register. That is, given:
        //                       reg - source width              source width
        //                                |                           |
        // SOURCE: [               don't care bits           |   care bits    ]
        //
        //                             dest width
        //                                |
        // OUT:    [ same don't care bits                   |   care bits     ]
        let unconstrained_bits = self.bitwidth.checked_sub(source_width).unwrap();

        // If we are extending to the full register width, no padding needed
        if unconstrained_bits == 0 || !self.find_widths {
            rotate
        } else {
            let padding = self.smt.extract(
                self.bitwidth.checked_sub(1).unwrap().try_into().unwrap(),
                self.bitwidth
                    .checked_sub(unconstrained_bits)
                    .unwrap()
                    .try_into()
                    .unwrap(),
                source,
            );
            self.smt.concat(padding, rotate)
        }
    }

    // SMT-LIB only supports rotates by concrete amounts, but we
    // need symbolic ones. This method essentially does if-conversion over possible
    // concrete forms, outputting nested ITE blocks. We consider both the starting
    // width and the rotate amount to be potentially symbolic.
    // For safety, we add an assertion that some arm of this ITE must match.
    fn rotate_symbolic_dyn_source_width(
        &mut self,
        source: SExpr,
        source_width: SExpr,
        amount: SExpr,
        op: &str,
    ) -> SExpr {
        if self.find_widths {
            return source;
        }
        let mut some_match = vec![];
        let mut ite_str = source;

        // Special case: if we are asked to rotate by 0, just return the source
        let matching = self.smt.eq(self.bv(0, self.bitwidth), amount);
        some_match.push(matching);
        ite_str = self.smt.ite(matching, source, ite_str);

        // Possible starting widths
        for possible_source in [8usize, 16, 32, 64] {
            // Statement meaning the symbolic case matches this concrete case
            let matching = self.smt.eq(self.smt.numeral(possible_source), source_width);
            some_match.push(matching);

            // Extract the relevant bits of the source (which is modeled with a wider,
            // register-width bitvector).
            let extract_source = self.smt.extract(
                possible_source.checked_sub(1).unwrap().try_into().unwrap(),
                0,
                source,
            );
            let extract_amount = self.smt.extract(
                possible_source.checked_sub(1).unwrap().try_into().unwrap(),
                0,
                amount,
            );

            // SMT bitvector rotate_left requires that the rotate amount be
            // statically specified. Instead, to use a dynamic amount, desugar
            // to shifts and bit arithmetic.
            let rotate = self.encode_rotate(op, extract_source, extract_amount, possible_source);

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
            let rotate = if unconstrained_bits == 0 {
                rotate
            } else {
                let padding = self.smt.extract(
                    self.bitwidth.checked_sub(1).unwrap().try_into().unwrap(),
                    self.bitwidth
                        .checked_sub(unconstrained_bits)
                        .unwrap()
                        .try_into()
                        .unwrap(),
                    source,
                );
                self.smt.concat(padding, rotate)
            };

            ite_str = self.smt.ite(matching, rotate, ite_str);
        }
        let some_shift_matches = self.smt.or_many(some_match);
        self.width_assumptions.push(some_shift_matches);
        ite_str
    }

    pub fn widen_to_register_width(
        &mut self,
        tyvar: u32,
        narrow_width: usize,
        narrow_decl: SExpr,
        name: Option<String>,
    ) -> SExpr {
        let width = self.bitwidth.checked_sub(narrow_width).unwrap();
        if width > 0 {
            let mut narrow_name = format!("narrow__{}", tyvar);
            let mut wide_name = format!("wide__{}", tyvar);
            if let Some(s) = name {
                narrow_name = format!("{}_{}", s, narrow_name);
                wide_name = format!("{}_{}", s, wide_name);
            }
            self.assume(self.smt.eq(self.smt.atom(&narrow_name), narrow_decl));
            self.additional_decls.push((
                narrow_name.clone(),
                self.smt.bit_vec_sort(self.smt.numeral(narrow_width)),
            ));
            self.additional_decls.push((
                wide_name.clone(),
                self.smt.bit_vec_sort(self.smt.numeral(self.bitwidth)),
            ));
            let padding = self.new_fresh_bits(width);
            self.assume(self.smt.eq(
                self.smt.atom(&wide_name),
                self.smt.concat(padding, self.smt.atom(narrow_name)),
            ));
            self.smt.atom(wide_name)
        } else if let Some(s) = name {
            self.assume(self.smt.eq(self.smt.atom(&s), narrow_decl));
            self.smt.atom(&s)
        } else {
            narrow_decl
        }
    }

    pub fn get_expr_width_var(&self, e: &Expr) -> Option<SExpr> {
        if let Some(tyvar) = self.tyctx.tyvars.get(e) {
            self.width_vars.get(tyvar).map(|s| self.smt.atom(s))
        } else {
            None
        }
    }

    pub fn vir_to_smt_ty(&self, ty: &Type) -> SExpr {
        match ty {
            Type::BitVector(w) => {
                let width = w.unwrap_or(self.bitwidth);
                self.smt.bit_vec_sort(self.smt.numeral(width))
            }
            Type::Int => self.smt.int_sort(),
            Type::Bool | Type::Unit => self.smt.bool_sort(),
        }
    }

    pub fn get_type(&self, x: &Expr) -> Option<&Type> {
        self.tyctx.tymap.get(self.tyctx.tyvars.get(x)?)
    }

    pub fn get_expr_value(&self, e: &Expr) -> Option<i128> {
        if let Some(tyvar) = self.tyctx.tyvars.get(e) {
            self.tyctx.tyvals.get(tyvar).copied()
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
        let xw = self.get_expr_width_var(x).unwrap();
        let yw = self.get_expr_width_var(y).unwrap();
        self.width_assumptions.push(self.smt.eq(xw, yw));
    }

    pub fn assume_same_width_from_sexpr(&mut self, x: SExpr, y: &Expr) {
        let yw = self.get_expr_width_var(y).unwrap();
        self.width_assumptions.push(self.smt.eq(x, yw));
    }

    pub fn assume_comparable_types(&mut self, x: &Expr, y: &Expr) {
        match (self.get_type(x), self.get_type(y)) {
            (None, _) | (_, None) => panic!("Missing type(s) {:?} {:?}", x, y),
            (Some(Type::Bool), Some(Type::Bool))
            | (Some(Type::Int), Some(Type::Int))
            | (Some(Type::Unit), Some(Type::Unit)) => (),
            (Some(Type::BitVector(Some(xw))), Some(Type::BitVector(Some(yw)))) => {
                assert_eq!(xw, yw, "incompatible {:?} {:?}", x, y)
            }
            (_, _) => self.assume_same_width(x, y),
        }
    }

    pub fn vir_expr_to_sexp(&mut self, e: Expr) -> SExpr {
        let tyvar = self.tyctx.tyvars.get(&e);
        let ty = self.get_type(&e);
        let width = self.get_expr_width_var(&e);
        let static_expr_width = self.static_width(&e);
        match e {
            Expr::Terminal(t) => match t {
                Terminal::Literal(v, tyvar) => {
                    let lit = self.smt.atom(v);
                    if self.find_widths && matches!(ty.unwrap(), Type::BitVector(_)) {
                        self.widen_to_register_width(tyvar, static_expr_width.unwrap(), lit, None)
                    } else {
                        lit
                    }
                }
                Terminal::Var(v) => match self.var_map.get(&v) {
                    Some(o) => *o,
                    None => self.smt.atom(v),
                },
                Terminal::Const(i, _) => match ty.unwrap() {
                    Type::BitVector(w) => {
                        let width = w.unwrap_or(self.bitwidth);
                        let narrow_decl = self.bv(i, width);
                        if self.find_widths {
                            self.zero_extend(self.bitwidth - width, narrow_decl)
                        } else {
                            narrow_decl
                        }
                    }
                    Type::Int => self.smt.numeral(i),
                    Type::Bool => {
                        if i == 0 {
                            self.smt.false_()
                        } else {
                            self.smt.true_()
                        }
                    }
                    Type::Unit => self.smt.true_(),
                },
                Terminal::True => self.smt.true_(),
                Terminal::False => self.smt.false_(),
                Terminal::Wildcard(_) => match ty.unwrap() {
                    Type::BitVector(Some(w)) if !self.find_widths => self.new_fresh_bits(*w),
                    Type::BitVector(_) => self.new_fresh_bits(self.bitwidth),
                    Type::Int => self.new_fresh_int(),
                    Type::Bool => self.new_fresh_bool(),
                    Type::Unit => self.smt.true_(),
                },
            },
            Expr::Unary(op, arg) => {
                let op = match op {
                    UnaryOp::Not => "not",
                    UnaryOp::BVNeg => {
                        if self.find_widths {
                            self.assume_same_width_from_sexpr(width.unwrap(), &arg);
                        }
                        "bvneg"
                    }
                    UnaryOp::BVNot => {
                        if self.find_widths {
                            self.assume_same_width_from_sexpr(width.unwrap(), &arg);
                        }
                        "bvnot"
                    }
                };
                let subexp = self.vir_expr_to_sexp(*arg);
                self.smt.list(vec![self.smt.atom(op), subexp])
            }
            Expr::Binary(op, x, y) => {
                if self.find_widths {
                    match op {
                        BinaryOp::BVMul
                        | BinaryOp::BVUDiv
                        | BinaryOp::BVSDiv
                        | BinaryOp::BVUrem
                        | BinaryOp::BVSrem
                        | BinaryOp::BVAdd
                        | BinaryOp::BVSub
                        | BinaryOp::BVAnd
                        | BinaryOp::BVOr
                        | BinaryOp::BVShl
                        | BinaryOp::BVShr
                        | BinaryOp::BVAShr
                        | BinaryOp::BVRotl
                        | BinaryOp::BVRotr => self.assume_same_width_from_sexpr(width.unwrap(), &x),
                        BinaryOp::Eq => {
                            if let Some(Type::BitVector(_)) = self.get_type(&x) {
                                self.assume_comparable_types(&x, &y)
                            }
                        }
                        _ => (),
                    };
                    self.assume_comparable_types(&x, &y);
                }
                match op {
                    BinaryOp::BVRotl => {
                        let source_width = self.static_width(&x);
                        match source_width {
                            Some(w) => {
                                let xs = self.vir_expr_to_sexp(*x);
                                let ys = self.vir_expr_to_sexp(*y);
                                return self.rotate_symbolic(xs, w, ys, "rotate_left");
                            }
                            None => {
                                let arg_width = self.get_expr_width_var(&x).unwrap();
                                let xs = self.vir_expr_to_sexp(*x);
                                let ys = self.vir_expr_to_sexp(*y);
                                return self.rotate_symbolic_dyn_source_width(
                                    xs,
                                    arg_width,
                                    ys,
                                    "rotate_left",
                                );
                            }
                        }
                    }
                    BinaryOp::BVRotr => {
                        let source_width = self.static_width(&x);
                        match source_width {
                            Some(w) => {
                                let xs = self.vir_expr_to_sexp(*x);
                                let ys = self.vir_expr_to_sexp(*y);
                                return self.rotate_symbolic(xs, w, ys, "rotate_right");
                            }
                            None => {
                                let arg_width = self.get_expr_width_var(&x).unwrap();
                                let xs = self.vir_expr_to_sexp(*x);
                                let ys = self.vir_expr_to_sexp(*y);
                                return self.rotate_symbolic_dyn_source_width(
                                    xs,
                                    arg_width,
                                    ys,
                                    "rotate_right",
                                );
                            }
                        }
                    }
                    // To shift right, we need to make sure the bits to the right get zeroed. Shift left first.
                    BinaryOp::BVShr => {
                        let arg_width = if self.find_widths {
                            self.get_expr_width_var(&x).unwrap()
                        } else {
                            self.smt.numeral(self.static_width(&x).unwrap())
                        };
                        let xs = self.vir_expr_to_sexp(*x);

                        // Strategy: shift left by (bitwidth - arg width) to zero bits to the right
                        // of the bits in the argument size. Then shift right by (amt + (bitwidth - arg width))

                        // Width math
                        if self.find_widths {
                            // The shift arg needs to be extracted to the right width, default to 8 if unknown
                            let y_static_width = self.static_width(&y).unwrap_or(8);
                            let y_rec = self.vir_expr_to_sexp(*y);
                            if self.find_widths {
                                return xs;
                            }
                            let extract = self.smt.extract(
                                y_static_width.checked_sub(1).unwrap().try_into().unwrap(),
                                0,
                                y_rec,
                            );
                            let ys = self.zero_extend(self.bitwidth - y_static_width, extract);
                            let arg_width_as_bv = self.int2bv(self.bitwidth, arg_width);
                            let bitwidth_as_bv =
                                self.bv(self.bitwidth.try_into().unwrap(), self.bitwidth);
                            let extra_shift = self.smt.bvsub(bitwidth_as_bv, arg_width_as_bv);
                            let shl_to_zero = self.smt.bvshl(xs, extra_shift);

                            let amt_plus_extra = self.smt.bvadd(ys, extra_shift);
                            return self.smt.bvlshr(shl_to_zero, amt_plus_extra);
                        } else {
                            let ys = self.vir_expr_to_sexp(*y);
                            return self.smt.bvlshr(xs, ys);
                        }
                    }
                    BinaryOp::BVAShr => {
                        let arg_width = if self.find_widths {
                            self.get_expr_width_var(&x).unwrap()
                        } else {
                            self.smt.numeral(self.static_width(&x).unwrap())
                        };
                        let xs = self.vir_expr_to_sexp(*x);

                        // Strategy: shift left by (bitwidth - arg width) to eliminate bits to the left
                        // of the bits in the argument size. Then shift right by (amt + (bitwidth - arg width))

                        // Width math
                        if self.find_widths {
                            // The shift arg needs to be extracted to the right width, default to 8 if unknown
                            let y_static_width = self.static_width(&y).unwrap_or(8);
                            let ys = self.vir_expr_to_sexp(*y);
                            let extract = self.smt.extract(
                                y_static_width.checked_sub(1).unwrap().try_into().unwrap(),
                                0,
                                ys,
                            );
                            let ysext = self.zero_extend(self.bitwidth - y_static_width, extract);

                            let arg_width_as_bv = self.int2bv(self.bitwidth, arg_width);
                            let bitwidth_as_bv =
                                self.bv(self.bitwidth.try_into().unwrap(), self.bitwidth);
                            let extra_shift = self.smt.bvsub(bitwidth_as_bv, arg_width_as_bv);
                            let shl_to_zero = self.smt.bvshl(xs, extra_shift);

                            let amt_plus_extra = self.smt.bvadd(ysext, extra_shift);
                            return self.smt.bvashr(shl_to_zero, amt_plus_extra);
                        } else {
                            let ys = self.vir_expr_to_sexp(*y);
                            return self.smt.bvashr(xs, ys);
                        }
                    }
                    _ => (),
                };
                let op_str = match op {
                    BinaryOp::And => "and",
                    BinaryOp::Or => "or",
                    BinaryOp::Imp => "=>",
                    BinaryOp::Eq => "=",
                    BinaryOp::Lte => match (self.get_type(&x), self.get_type(&y)) {
                        (Some(Type::Int), Some(Type::Int)) => "<=",
                        (Some(Type::BitVector(_)), Some(Type::BitVector(_))) => "bvule",
                        _ => unreachable!(),
                    },
                    BinaryOp::Lt => match (self.get_type(&x), self.get_type(&y)) {
                        (Some(Type::Int), Some(Type::Int)) => "<",
                        (Some(Type::BitVector(_)), Some(Type::BitVector(_))) => "bvult",
                        _ => unreachable!(),
                    },
                    BinaryOp::BVSgt => "bvsgt",
                    BinaryOp::BVSgte => "bvsge",
                    BinaryOp::BVSlt => "bvslt",
                    BinaryOp::BVSlte => "bvsle",
                    BinaryOp::BVUgt => "bvugt",
                    BinaryOp::BVUgte => "bvuge",
                    BinaryOp::BVUlt => "bvult",
                    BinaryOp::BVUlte => "bvule",
                    BinaryOp::BVMul => "bvmul",
                    BinaryOp::BVUDiv => "bvudiv",
                    BinaryOp::BVSDiv => "bvsdiv",
                    BinaryOp::BVAdd => "bvadd",
                    BinaryOp::BVSub => "bvsub",
                    BinaryOp::BVUrem => "bvurem",
                    BinaryOp::BVSrem => "bvsrem",
                    BinaryOp::BVAnd => "bvand",
                    BinaryOp::BVOr => "bvor",
                    BinaryOp::BVXor => "bvxor",
                    BinaryOp::BVShl => "bvshl",
                    BinaryOp::BVSaddo => "bvsaddo",
                    _ => unreachable!("{:?}", op),
                };
                // If we have some static width that isn't the bitwidth, extract based on it
                // before performing the operation for the dynamic case.
                match static_expr_width {
                    Some(w) if w < self.bitwidth && self.find_widths => {
                        let h: i32 = (w - 1).try_into().unwrap();
                        let x_sexp = self.vir_expr_to_sexp(*x);
                        let y_sexp = self.vir_expr_to_sexp(*y);
                        self.zero_extend(
                            self.bitwidth.checked_sub(w).unwrap(),
                            self.smt.list(vec![
                                self.smt.atom(op_str),
                                self.smt.extract(h, 0, x_sexp),
                                self.smt.extract(h, 0, y_sexp),
                            ]),
                        )
                    }
                    _ => {
                        let x_sexp = self.vir_expr_to_sexp(*x);
                        let y_sexp = self.vir_expr_to_sexp(*y);
                        self.smt.list(vec![self.smt.atom(op_str), x_sexp, y_sexp])
                    }
                }
            }
            Expr::BVIntToBV(w, x) => {
                let x_sexp = self.vir_expr_to_sexp(*x);
                if self.find_widths {
                    let padded_width = self.bitwidth - w;
                    self.zero_extend(padded_width, self.int2bv(w, x_sexp))
                } else {
                    self.int2bv(w, x_sexp)
                }
            }
            Expr::BVToInt(x) => {
                let x_sexp = self.vir_expr_to_sexp(*x);
                self.bv2nat(x_sexp)
            }
            Expr::BVZeroExtTo(i, x) => {
                let arg_width = if self.find_widths {
                    let expr_width = width.unwrap();
                    self.width_assumptions
                        .push(self.smt.eq(expr_width, self.smt.numeral(i)));
                    self.get_expr_width_var(&x).unwrap()
                } else {
                    self.smt.numeral(self.static_width(&x).unwrap())
                };
                let static_width = self.static_width(&x);
                let xs = self.vir_expr_to_sexp(*x);
                if let Some(size) = static_width {
                    self.extend_concrete(i, xs, size, "zero_extend")
                } else {
                    self.extend_symbolic(self.smt.numeral(i), xs, arg_width, "zero_extend")
                }
            }
            Expr::BVZeroExtToVarWidth(i, x) => {
                let static_arg_width = self.static_width(&x);
                let arg_width = self.get_expr_width_var(&x);
                let is = self.vir_expr_to_sexp(*i);
                let xs = self.vir_expr_to_sexp(*x);
                if self.find_widths {
                    let expr_width = width.unwrap();
                    self.width_assumptions.push(self.smt.eq(expr_width, is));
                }
                if let (Some(arg_size), Some(e_size)) = (static_arg_width, static_expr_width) {
                    self.extend_concrete(e_size, xs, arg_size, "zero_extend")
                } else {
                    self.extend_symbolic(is, xs, arg_width.unwrap(), "zero_extend")
                }
            }
            Expr::BVSignExtTo(i, x) => {
                let arg_width = if self.find_widths {
                    let expr_width = width.unwrap();
                    self.width_assumptions
                        .push(self.smt.eq(expr_width, self.smt.numeral(i)));
                    self.get_expr_width_var(&x).unwrap()
                } else {
                    self.smt.numeral(self.static_width(&x).unwrap())
                };
                let static_width = self.static_width(&x);
                let xs = self.vir_expr_to_sexp(*x);
                if let Some(size) = static_width {
                    self.extend_concrete(i, xs, size, "sign_extend")
                } else {
                    self.extend_symbolic(self.smt.numeral(i), xs, arg_width, "sign_extend")
                }
            }
            Expr::BVSignExtToVarWidth(i, x) => {
                let static_arg_width = self.static_width(&x);
                let arg_width = self.get_expr_width_var(&x);
                let is = self.vir_expr_to_sexp(*i);
                let xs = self.vir_expr_to_sexp(*x);
                if self.find_widths {
                    let expr_width = width.unwrap();
                    self.width_assumptions.push(self.smt.eq(expr_width, is));
                }
                if let (Some(arg_size), Some(e_size)) = (static_arg_width, static_expr_width) {
                    self.extend_concrete(e_size, xs, arg_size, "sign_extend")
                } else {
                    self.extend_symbolic(is, xs, arg_width.unwrap(), "sign_extend")
                }
            }
            Expr::BVConvTo(x, y) => {
                if self.find_widths {
                    let expr_width = width.unwrap();
                    let dyn_width = self.vir_expr_to_sexp(*x);
                    let eq = self.smt.eq(expr_width, dyn_width);
                    self.width_assumptions.push(eq);
                    self.vir_expr_to_sexp(*y)
                } else {
                    let arg_width = self.static_width(&y).unwrap();
                    match ty {
                        Some(Type::BitVector(Some(w))) => match arg_width.cmp(w) {
                            Ordering::Less => {
                                let padding =
                                    self.new_fresh_bits(w.checked_sub(arg_width).unwrap());
                                let ys = self.vir_expr_to_sexp(*y);
                                self.smt.concat(padding, ys)
                            }
                            Ordering::Greater => {
                                let new = (w - 1).try_into().unwrap();
                                let ys = self.vir_expr_to_sexp(*y);
                                self.smt.extract(new, 0, ys)
                            }
                            Ordering::Equal => self.vir_expr_to_sexp(*y),
                        },
                        _ => unreachable!("{:?}, {:?}", x, y),
                    }
                }
            }
            Expr::WidthOf(x) => {
                if self.find_widths {
                    self.get_expr_width_var(&x).unwrap()
                } else {
                    self.smt.numeral(self.static_width(&x).unwrap())
                }
            }
            Expr::BVExtract(i, j, x) => {
                assert!(i >= j);
                if self.get_type(&x).is_some() {
                    let xs = self.vir_expr_to_sexp(*x);
                    // No-op if we are extracting exactly the full bitwidth
                    if j == 0 && i == self.bitwidth - 1 && self.find_widths {
                        return xs;
                    }
                    let extract =
                        self.smt
                            .extract(i.try_into().unwrap(), j.try_into().unwrap(), xs);
                    let new_width = i - j + 1;
                    if new_width < self.bitwidth && self.find_widths {
                        let padding =
                            self.new_fresh_bits(self.bitwidth.checked_sub(new_width).unwrap());
                        self.smt.concat(padding, extract)
                    } else {
                        extract
                    }
                } else {
                    unreachable!("Must perform extraction on bv with known width")
                }
            }
            Expr::Conditional(c, t, e) => {
                if self.find_widths && matches!(ty, Some(Type::BitVector(_))) {
                    self.assume_same_width_from_sexpr(width.unwrap(), &t);
                    self.assume_same_width_from_sexpr(width.unwrap(), &e);
                }
                let cs = self.vir_expr_to_sexp(*c);
                let ts = self.vir_expr_to_sexp(*t);
                let es = self.vir_expr_to_sexp(*e);
                self.smt.ite(cs, ts, es)
            }
            Expr::Switch(c, cases) => {
                if self.find_widths {
                    if matches!(ty, Some(Type::BitVector(_))) {
                        for (_, b) in &cases {
                            self.assume_same_width_from_sexpr(width.unwrap(), b);
                        }
                    }
                    let cty = self.get_type(&c);
                    if matches!(cty, Some(Type::BitVector(_))) {
                        let cwidth = self.get_expr_width_var(&c);
                        for (m, _) in &cases {
                            self.assume_same_width_from_sexpr(cwidth.unwrap(), m);
                        }
                    }
                }
                let cs = self.vir_expr_to_sexp(*c);
                let mut case_sexprs: Vec<(SExpr, SExpr)> = cases
                    .iter()
                    .map(|(m, b)| {
                        (
                            self.vir_expr_to_sexp(m.clone()),
                            self.vir_expr_to_sexp(b.clone()),
                        )
                    })
                    .collect();

                // Assert that some case must match
                let some_case_matches: Vec<SExpr> = case_sexprs
                    .iter()
                    .map(|(m, _)| self.smt.eq(cs, *m))
                    .collect();
                self.assert(self.smt.or_many(some_case_matches.clone()));

                let (_, last_body) = case_sexprs.remove(case_sexprs.len() - 1);

                // Reverse to keep the order of the switch
                case_sexprs.iter().rev().fold(last_body, |acc, (m, b)| {
                    self.smt.ite(self.smt.eq(cs, *m), *b, acc)
                })
            }
            Expr::CLZ(e) => {
                let tyvar = *tyvar.unwrap();
                if self.find_widths {
                    self.assume_same_width_from_sexpr(width.unwrap(), &e);
                }
                let es = self.vir_expr_to_sexp(*e);
                match static_expr_width {
                    Some(1) => clz::clz1(self, es, tyvar),
                    Some(8) => clz::clz8(self, es, tyvar),
                    Some(16) => clz::clz16(self, es, tyvar),
                    Some(32) => clz::clz32(self, es, tyvar),
                    Some(64) => clz::clz64(self, es, tyvar),
                    Some(w) => unreachable!("Unexpected CLZ width {}", w),
                    None => unreachable!("Need static CLZ width"),
                }
            }
            Expr::CLS(e) => {
                let tyvar = *tyvar.unwrap();
                if self.find_widths {
                    self.assume_same_width_from_sexpr(width.unwrap(), &e);
                }
                let es = self.vir_expr_to_sexp(*e);
                match static_expr_width {
                    Some(1) => cls::cls1(self, tyvar),
                    Some(8) => cls::cls8(self, es, tyvar),
                    Some(16) => cls::cls16(self, es, tyvar),
                    Some(32) => cls::cls32(self, es, tyvar),
                    Some(64) => cls::cls64(self, es, tyvar),
                    Some(w) => unreachable!("Unexpected CLS width {}", w),
                    None => unreachable!("Need static CLS width"),
                }
            }
            Expr::Rev(e) => {
                let tyvar = *tyvar.unwrap();
                if self.find_widths {
                    self.assume_same_width_from_sexpr(width.unwrap(), &e);
                }
                let es = self.vir_expr_to_sexp(*e);
                match static_expr_width {
                    Some(1) => rev::rev1(self, es, tyvar),
                    Some(8) => rev::rev8(self, es, tyvar),
                    Some(16) => rev::rev16(self, es, tyvar),
                    Some(32) => rev::rev32(self, es, tyvar),
                    Some(64) => rev::rev64(self, es, tyvar),
                    Some(w) => unreachable!("Unexpected CLS width {}", w),
                    None => unreachable!("Need static CLS width"),
                }
            }
            Expr::BVSubs(ty, x, y) => {
                let tyvar = *tyvar.unwrap();
                if self.find_widths {
                    self.assume_comparable_types(&x, &y);
                }
                let ety = self.vir_expr_to_sexp(*ty);
                let ex = self.vir_expr_to_sexp(*x);
                let ey = self.vir_expr_to_sexp(*y);

                let encoded_32 = subs::subs(self, 32, ex, ey, tyvar);
                let encoded_64 = subs::subs(self, 64, ex, ey, tyvar);

                self.smt.ite(
                    self.smt.eq(ety, self.smt.numeral(32)),
                    encoded_32,
                    encoded_64,
                )
            }
            Expr::BVPopcnt(x) => {
                let tyvar = *tyvar.unwrap();
                if self.find_widths {
                    self.assume_same_width_from_sexpr(width.unwrap(), &x);
                }
                let ex = self.vir_expr_to_sexp(*x);

                match static_expr_width {
                    Some(8) => {
                        let p = popcnt(self, 8, ex, tyvar);
                        if self.find_widths {
                            self.zero_extend(self.bitwidth - 8, p)
                        } else {
                            p
                        }
                    }
                    Some(16) => {
                        let p = popcnt(self, 16, ex, tyvar);
                        if self.find_widths {
                            self.zero_extend(self.bitwidth - 8, p)
                        } else {
                            self.zero_extend(8, p)
                        }
                    }
                    Some(32) => {
                        let p = popcnt(self, 32, ex, tyvar);
                        if self.find_widths {
                            self.zero_extend(self.bitwidth - 8, p)
                        } else {
                            self.zero_extend(24, p)
                        }
                    }
                    Some(64) => {
                        let p = popcnt(self, 64, ex, tyvar);
                        if self.find_widths {
                            self.zero_extend(self.bitwidth - 8, p)
                        } else {
                            self.zero_extend(56, p)
                        }
                    }
                    Some(w) => unreachable!("Unexpected popcnt width {}", w),
                    None => unreachable!("Need static popcnt width"),
                }
            }
            Expr::BVConcat(xs) => {
                if self.find_widths {
                    let widths: Vec<SExpr> = xs
                        .iter()
                        .map(|x| self.get_expr_width_var(x).unwrap())
                        .collect();
                    let sum = self.smt.plus_many(widths);
                    self.width_assumptions
                        .push(self.smt.eq(width.unwrap(), sum));
                }
                let mut sexprs: Vec<SExpr> = xs
                    .iter()
                    .map(|x| self.vir_expr_to_sexp(x.clone()))
                    .collect();
                let last = sexprs.remove(sexprs.len() - 1);

                // Width hack for now
                if self.find_widths {
                    return sexprs[0];
                }
                // Reverse to keep the order of the cases
                sexprs
                    .iter()
                    .rev()
                    .fold(last, |acc, x| self.smt.concat(*x, acc))
            }
            Expr::LoadEffect(x, y, z) => {
                let ex = self.vir_expr_to_sexp(*x);
                let ey = self.vir_expr_to_sexp(*y);
                let ez = self.vir_expr_to_sexp(*z);

                if self.find_widths {
                    self.width_assumptions.push(self.smt.eq(width.unwrap(), ey));
                }

                if self.lhs_flag {
                    if self.lhs_load_args.is_some() {
                        panic!("Only one load on the LHS currently supported, found multiple.")
                    }
                    self.lhs_load_args = Some(vec![ex, ey, ez]);
                    let load_ret = if self.find_widths {
                        self.new_fresh_bits(self.bitwidth)
                    } else {
                        self.new_fresh_bits(static_expr_width.unwrap())
                    };
                    self.load_return = Some(load_ret);
                    load_ret
                } else {
                    if self.rhs_load_args.is_some() {
                        panic!("Only one load on the RHS currently supported, found miltiple.")
                    }
                    self.rhs_load_args = Some(vec![ex, ey, ez]);
                    self.load_return.unwrap()
                }
            }
            Expr::StoreEffect(w, x, y, z) => {
                let ew = self.vir_expr_to_sexp(*w);
                let ex = self.vir_expr_to_sexp(*x);
                let ez = self.vir_expr_to_sexp(*z);

                if self.find_widths {
                    let y_width = self.get_expr_width_var(&y).unwrap();
                    self.width_assumptions.push(self.smt.eq(y_width, ex));
                }
                let ey = self.vir_expr_to_sexp(*y);

                if self.lhs_flag {
                    self.lhs_store_args = Some(vec![ew, ex, ey, ez]);
                } else {
                    self.rhs_store_args = Some(vec![ew, ex, ey, ez]);
                }
                self.smt.atom("true")
            }
        }
    }

    // Checks whether the assumption list is always false
    fn check_assumptions_feasibility(
        &mut self,
        assumptions: &[SExpr],
        term_input_bs: &[String],
        config: &Config,
    ) -> VerificationResult {
        log::debug!("Checking assumption feasibility");
        self.smt.push().unwrap();
        for (i, a) in assumptions.iter().enumerate() {
            self.smt
                .assert(self.smt.named(format!("assum{i}"), *a))
                .unwrap();
        }

        let res = match self.smt.check() {
            Ok(Response::Sat) => {
                if !config.distinct_check || term_input_bs.is_empty() {
                    log::debug!("Assertion list is feasible for at least one input!");
                    self.smt.pop().unwrap();
                    return VerificationResult::Success;
                }
                // Check that there is a model with distinct bitvector inputs
                let mut not_all_same = vec![];
                let atoms: Vec<SExpr> = term_input_bs.iter().map(|n| self.smt.atom(n)).collect();
                let solution = self.smt.get_value(atoms).unwrap();
                assert_eq!(term_input_bs.len(), solution.len());
                for (variable, value) in solution {
                    not_all_same.push(self.smt.not(self.smt.eq(variable, value)));
                }
                match not_all_same.len().cmp(&1) {
                    Ordering::Equal => self.smt.assert(not_all_same[0]).unwrap(),
                    Ordering::Greater => self.smt.assert(self.smt.and_many(not_all_same)).unwrap(),
                    Ordering::Less => unreachable!("must have some BV inputs"),
                }
                match self.smt.check() {
                    Ok(Response::Sat) => {
                        log::debug!("Assertion list is feasible for two distinct inputs");
                        VerificationResult::Success
                    }
                    Ok(Response::Unsat) => {
                        log::debug!("Assertion list is only feasible for one input with distinct BV values!");
                        VerificationResult::NoDistinctModels
                    }
                    Ok(Response::Unknown) => {
                        panic!("Solver said 'unk'");
                    }
                    Err(err) => {
                        unreachable!("Error! {:?}", err);
                    }
                }
            }
            Ok(Response::Unsat) => {
                log::debug!("Assertion list is infeasible!");
                let unsat = self.smt.get_unsat_core().unwrap();
                log::debug!("Unsat core:\n{}", self.smt.display(unsat));
                VerificationResult::InapplicableRule
            }
            Ok(Response::Unknown) => {
                panic!("Solver said 'unk'");
            }
            Err(err) => {
                unreachable!("Error! {:?}", err);
            }
        };
        self.smt.pop().unwrap();
        res
    }

    fn display_hex_to_bin(&self, value: SExpr) -> String {
        let sexpr_hex_prefix = "#x";
        let val_str = self.smt.display(value).to_string();
        if val_str.starts_with(sexpr_hex_prefix) {
            let without_prefix = val_str.trim_start_matches("#x");
            let as_unsigned = u128::from_str_radix(without_prefix, 16).unwrap();
            // SMT-LIB: bvhexX where X is a hexadecimal numeral of length m defines the bitvector
            // constant with value X and size 4*m.
            match without_prefix.len() {
                2 => format!("{}|{:#010b}", self.smt.display(value), as_unsigned),
                3 => format!("{}|{:#014b}", self.smt.display(value), as_unsigned),
                4 => format!("{}|{:#018b}", self.smt.display(value), as_unsigned),
                8 => format!("{}|{:#034b}", self.smt.display(value), as_unsigned),
                16 => format!("{}|{:#068b}", self.smt.display(value), as_unsigned),
                17 => format!("{}|{:#070b}", self.smt.display(value), as_unsigned),
                32 => format!("{}|{:#0130b}", self.smt.display(value), as_unsigned),
                _ => {
                    format!("{}|{:#b}", self.smt.display(value), as_unsigned)
                }
            }
        } else {
            val_str
        }
    }

    fn display_value(&self, variable: SExpr, value: SExpr) -> (String, String) {
        let var_str = self.smt.display(variable).to_string();
        (var_str, self.display_hex_to_bin(value))
    }

    fn display_isle_pattern(
        &mut self,
        termenv: &TermEnv,
        typeenv: &TypeEnv,
        vars: &Vec<(String, String)>,
        rule: &Rule,
        pat: &Pattern,
    ) -> SExpr {
        let mut to_sexpr = |p| self.display_isle_pattern(termenv, typeenv, vars, rule, p);

        match pat {
            isle::sema::Pattern::Term(_, term_id, args) => {
                let sym = termenv.terms[term_id.index()].name;
                let name = typeenv.syms[sym.index()].clone();

                let mut sexprs = args.iter().map(&mut to_sexpr).collect::<Vec<SExpr>>();

                sexprs.insert(0, self.smt.atom(name));
                self.smt.list(sexprs)
            }
            isle::sema::Pattern::Var(_, var_id) => {
                let sym = rule.vars[var_id.index()].name;
                let ident = typeenv.syms[sym.index()].clone();
                let smt_ident_prefix = format!("{}__clif{}__", ident, var_id.index());

                let var = self.display_var_from_smt_prefix(vars, &ident, &smt_ident_prefix);
                self.smt.atom(var)
            }
            isle::sema::Pattern::BindPattern(_, var_id, subpat) => {
                let sym = rule.vars[var_id.index()].name;
                let ident = &typeenv.syms[sym.index()];
                let smt_ident_prefix = format!("{}__clif{}__", ident, var_id.index(),);
                let subpat_node = to_sexpr(subpat);

                let var = self.display_var_from_smt_prefix(vars, ident, &smt_ident_prefix);

                // Special case: elide bind patterns to wildcars
                if matches!(**subpat, isle::sema::Pattern::Wildcard(_)) {
                    self.smt.atom(var)
                } else {
                    self.smt
                        .list(vec![self.smt.atom(var), self.smt.atom("@"), subpat_node])
                }
            }
            isle::sema::Pattern::Wildcard(_) => self.smt.list(vec![self.smt.atom("_")]),

            isle::sema::Pattern::ConstPrim(_, sym) => {
                let name = typeenv.syms[sym.index()].clone();
                self.smt.list(vec![self.smt.atom(name)])
            }
            isle::sema::Pattern::ConstBool(_, val) => {
                self.smt.list(vec![self.smt.atom(format!("{val}"))])
            }
            isle::sema::Pattern::ConstInt(_, num) => {
                let _smt_name_prefix = format!("{}__", num);
                self.smt.list(vec![self.smt.atom(num.to_string())])
            }
            isle::sema::Pattern::And(_, subpats) => {
                let mut sexprs = subpats.iter().map(to_sexpr).collect::<Vec<SExpr>>();

                sexprs.insert(0, self.smt.atom("and"));
                self.smt.list(sexprs)
            }
        }
    }

    fn display_var_from_smt_prefix(
        &self,
        vars: &Vec<(String, String)>,
        ident: &str,
        prefix: &str,
    ) -> String {
        let matches: Vec<&(String, String)> =
            vars.iter().filter(|(v, _)| v.starts_with(prefix)).collect();
        if matches.is_empty() {
            panic!("Can't find match for: {}\n{:?}", prefix, vars);
        } else if matches.len() == 3 {
            assert!(
                self.find_widths,
                "Only expect multiple matches with dynamic widths"
            );
            for (name, model) in matches {
                if name.contains("narrow") {
                    return format!("[{}|{}]", self.smt.display(self.smt.atom(ident)), model);
                }
            }
            panic!("narrow not found");
        } else if matches.len() == 1 {
            let model = &matches.first().unwrap().1;
            format!("[{}|{}]", self.smt.display(self.smt.atom(ident)), model)
        } else {
            panic!("Unexpected number of matches!")
        }
    }

    fn display_isle_expr(
        &self,
        termenv: &TermEnv,
        typeenv: &TypeEnv,
        vars: &Vec<(String, String)>,
        rule: &Rule,
        expr: &isle::sema::Expr,
    ) -> SExpr {
        let to_sexpr = |e| self.display_isle_expr(termenv, typeenv, vars, rule, e);

        match expr {
            isle::sema::Expr::Term(_, term_id, args) => {
                let sym = termenv.terms[term_id.index()].name;
                let name = typeenv.syms[sym.index()].clone();

                let mut sexprs = args.iter().map(to_sexpr).collect::<Vec<SExpr>>();

                sexprs.insert(0, self.smt.atom(name));
                self.smt.list(sexprs)
            }
            isle::sema::Expr::Var(_, var_id) => {
                let sym = rule.vars[var_id.index()].name;
                let ident = typeenv.syms[sym.index()].clone();
                let smt_ident_prefix = format!("{}__clif{}__", ident, var_id.index());

                let var = self.display_var_from_smt_prefix(vars, &ident, &smt_ident_prefix);
                self.smt.atom(var)
            }
            isle::sema::Expr::ConstPrim(_, sym) => {
                let name = typeenv.syms[sym.index()].clone();
                self.smt.list(vec![self.smt.atom(name)])
            }
            isle::sema::Expr::ConstBool(_, val) => {
                self.smt.list(vec![self.smt.atom(format!("{val}"))])
            }
            isle::sema::Expr::ConstInt(_, num) => {
                let _smt_name_prefix = format!("{}__", num);
                self.smt.list(vec![self.smt.atom(num.to_string())])
            }
            isle::sema::Expr::Let { bindings, body, .. } => {
                let mut sexprs = vec![];
                for (varid, _, expr) in bindings {
                    let sym = rule.vars[varid.index()].name;
                    let ident = typeenv.syms[sym.index()].clone();
                    let smt_prefix = format!("{}__clif{}__", ident, varid.index());
                    let var = self.display_var_from_smt_prefix(vars, &ident, &smt_prefix);

                    sexprs.push(self.smt.list(vec![self.smt.atom(var), to_sexpr(expr)]));
                }
                self.smt.list(vec![
                    self.smt.atom("let"),
                    self.smt.list(sexprs),
                    to_sexpr(body),
                ])
            }
        }
    }

    fn display_model(
        &mut self,
        termenv: &TermEnv,
        typeenv: &TypeEnv,
        rule: &Rule,
        lhs_sexpr: SExpr,
        rhs_sexpr: SExpr,
    ) {
        let mut vars = vec![];
        let mut lhs_value = None;
        let mut rhs_value = None;
        for (name, atom) in &self.var_map {
            let solution = self
                .smt
                .get_value(vec![self.smt.atom(name), *atom])
                .unwrap();
            for (variable, value) in solution {
                let display = self.display_value(variable, value);
                vars.push(display.clone());
                if variable == lhs_sexpr {
                    lhs_value = Some(display.1);
                } else if variable == rhs_sexpr {
                    rhs_value = Some(display.1);
                }
            }
        }
        for (name, _) in &self.additional_decls {
            let solution = self.smt.get_value(vec![self.smt.atom(name)]).unwrap();
            for (variable, value) in solution {
                vars.push(self.display_value(variable, value));
            }
        }
        vars.sort_by_key(|x| x.0.clone());
        vars.dedup();

        // TODO VERBOSE
        println!("Counterexample summary");
        let lhs = self.display_isle_pattern(
            termenv,
            typeenv,
            &vars,
            rule,
            &Pattern::Term(
                cranelift_isle::sema::TypeId(0),
                rule.root_term,
                rule.args.clone(),
            ),
        );
        println!("{}", self.smt.display(lhs));

        // if-let statement processing
        if !&rule.iflets.is_empty() {
            print!("(if-let ");
        }
        for if_let_struct in &rule.iflets {
            let if_lhs = &if_let_struct.lhs;
            let if_rhs: &cranelift_isle::sema::Expr = &if_let_struct.rhs;

            let if_lhs_expr = self.display_isle_pattern(termenv, typeenv, &vars, rule, if_lhs);

            let if_rhs_expr = self.display_isle_expr(termenv, typeenv, &vars, rule, if_rhs);

            println!(
                "({} {})",
                self.smt.display(if_lhs_expr),
                self.smt.display(if_rhs_expr)
            );
        }
        println!(")");

        println!("=>");
        let rhs = self.display_isle_expr(termenv, typeenv, &vars, rule, &rule.rhs);
        println!("{}", self.smt.display(rhs));

        println!("\n{} =>\n{}\n", lhs_value.unwrap(), rhs_value.unwrap(),);
    }

    fn declare_variables(
        &mut self,
        rule_sem: &RuleSemantics,
        config: &Config,
    ) -> (Vec<SExpr>, Vec<SExpr>) {
        let mut assumptions: Vec<SExpr> = vec![];
        log::trace!("Declaring quantified variables");
        for v in &rule_sem.quantified_vars {
            let name = &v.name;
            let ty = self.tyctx.tymap[&v.tyvar];
            let var_ty = self.vir_to_smt_ty(&ty);
            log::trace!("\t{} : {}", name, self.smt.display(var_ty));
            if let Type::BitVector(w) = ty {
                if self.find_widths {
                    let wide = self.widen_to_register_width(
                        v.tyvar,
                        w.unwrap_or(self.bitwidth),
                        self.smt.atom(name),
                        Some(name.to_string()),
                    );
                    self.var_map.insert(name.clone(), wide);
                } else {
                    self.var_map.insert(name.clone(), self.smt.atom(name));
                }
            } else {
                self.var_map.insert(name.clone(), self.smt.atom(name));
            }
            self.smt.declare_const(name, var_ty).unwrap();
        }
        self.lhs_flag = true;
        for a in &rule_sem.lhs_assumptions {
            let p = self.vir_expr_to_sexp(a.clone());
            assumptions.push(p)
        }
        self.lhs_flag = false;
        for a in &rule_sem.rhs_assumptions {
            let p = self.vir_expr_to_sexp(a.clone());
            assumptions.push(p)
        }
        if self.find_widths {
            for a in &self.width_assumptions {
                assumptions.push(*a);
            }
        }
        self.additional_assumptions.is_empty();
        for a in &self.additional_assumptions {
            assumptions.push(*a);
        }
        // Look at RHS assertions, which are checked, not trusted
        let assertions: Vec<SExpr> = rule_sem
            .rhs_assertions
            .iter()
            .map(|a| self.vir_expr_to_sexp(a.clone()))
            .collect();

        for (name, ty) in &self.additional_decls {
            self.smt.declare_const(name, *ty).unwrap();
        }

        if let Some(a) = &config.custom_assumptions {
            let term_args = rule_sem
                .term_args
                .iter()
                .map(|s| self.smt.atom(s))
                .collect();
            let custom_assumptions = a(&self.smt, term_args);
            log::debug!(
                "Custom assumptions:\n\t{}\n",
                self.smt.display(custom_assumptions)
            );
            assumptions.push(custom_assumptions);
        }
        (assumptions, assertions)
    }
}

/// Overall query for single rule:
/// <declare vars>
/// (not (=> <assumptions> (= <LHS> <RHS>))))))
pub fn run_solver(
    rule_sem: &RuleSemantics,
    rule: &Rule,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    concrete: &Option<ConcreteTest>,
    config: &Config,
    _types: &TermSignature,
) -> VerificationResult {
    if std::env::var("SKIP_SOLVER").is_ok() {
        log::debug!("Environment variable SKIP_SOLVER set, returning Unknown");
        return VerificationResult::Unknown;
    }

    let mut solver = easy_smt::ContextBuilder::new()
        .replay_file(Some(std::fs::File::create("dynamic_widths.smt2").unwrap()))
        .solver("z3", ["-smt2", "-in"])
        .build()
        .unwrap();

    solver
        .set_option(":produce-unsat-cores", solver.true_())
        .unwrap();

    // We start with logic to determine the width of all bitvectors
    let mut ctx = SolverCtx {
        smt: solver,
        // Always find widths at first
        find_widths: true,
        tyctx: rule_sem.tyctx.clone(),
        bitwidth: MAX_WIDTH,
        var_map: HashMap::new(),
        width_vars: HashMap::new(),
        width_assumptions: vec![],
        additional_decls: vec![],
        additional_assumptions: vec![],
        additional_assertions: vec![],
        fresh_bits_idx: 0,
        lhs_load_args: None,
        rhs_load_args: None,
        lhs_store_args: None,
        rhs_store_args: None,
        load_return: None,
        lhs_flag: true,
    };

    let mut unresolved_widths = vec![];

    // Check whether the non-solver type inference was able to resolve all bitvector widths,
    // and add assumptions for known widths
    for (_e, t) in &ctx.tyctx.tyvars {
        let ty = &ctx.tyctx.tymap[t];
        if let Type::BitVector(w) = ty {
            let width_name = format!("width__{}", t);
            ctx.additional_decls
                .push((width_name.clone(), ctx.smt.int_sort()));
            match *w {
                Some(bitwidth) => {
                    let eq = ctx
                        .smt
                        .eq(ctx.smt.atom(&width_name), ctx.smt.numeral(bitwidth));
                    ctx.width_assumptions.push(eq);
                }
                None => {
                    log::debug!("Unresolved width: {:?} ({})", &_e, *t);
                    ctx.width_assumptions
                        .push(ctx.smt.gt(ctx.smt.atom(&width_name), ctx.smt.numeral(0)));
                    unresolved_widths.push(width_name.clone());
                }
            };
            ctx.width_vars.insert(*t, width_name.clone());
        }
    }

    if unresolved_widths.is_empty() {
        log::debug!("All widths resolved after basic type inference");
        return run_solver_with_static_widths(
            &RuleCtx {
                rule_sem,
                rule,
                termenv,
                typeenv,
                config,
            },
            &ctx.tyctx,
            concrete,
        );
    }

    log::debug!("Some unresolved widths after basic type inference");
    log::debug!("Finding widths from the solver");
    ctx.find_widths = true;
    let (assumptions, _) = ctx.declare_variables(rule_sem, config);
    ctx.smt.push().unwrap();
    for (i, a) in assumptions.iter().enumerate() {
        ctx.smt
            .assert(ctx.smt.named(format!("dyn{i}"), *a))
            .unwrap();
    }

    resolve_dynamic_widths(
        RuleCtx {
            rule_sem,
            rule,
            termenv,
            typeenv,
            config,
        },
        concrete,
        &mut ctx,
        unresolved_widths,
        0,
    )
}

fn resolve_dynamic_widths(
    rulectx: RuleCtx,
    concrete: &Option<ConcreteTest>,
    ctx: &mut SolverCtx,
    unresolved_widths: Vec<String>,
    attempt: usize,
) -> VerificationResult {
    if attempt > 10 {
        panic!("Unexpected number of attempts to resolve dynamic widths!")
    }
    match ctx.smt.check() {
        Ok(Response::Sat) => {
            let mut cur_tyctx = ctx.tyctx.clone();
            let mut width_resolutions = HashMap::new();
            for (e, t) in &ctx.tyctx.tyvars {
                let ty = &ctx.tyctx.tymap[t];
                if let Type::BitVector(w) = ty {
                    let width_name = format!("width__{}", t);
                    let atom = ctx.smt.atom(&width_name);
                    let width = ctx.smt.get_value(vec![atom]).unwrap().first().unwrap().1;
                    let width_int = u8::try_from(ctx.smt.get(width)).unwrap();

                    // Check that we haven't contradicted previous widths
                    if let Some(before_width) = w {
                        assert_eq!(*before_width, width_int as usize)
                    };

                    // Check that the width is nonzero
                    if width_int == 0 {
                        panic!("Unexpected, zero width! {} {:?}", t, e);
                    }

                    if unresolved_widths.contains(&width_name) {
                        log::debug!("\tResolved width: {}, {}", width_name, width_int);
                        width_resolutions.insert(width_name, width_int);
                        cur_tyctx
                            .tymap
                            .insert(*t, Type::BitVector(Some(width_int as usize)));
                    }
                }
            }
            let static_result = run_solver_with_static_widths(&rulectx, &cur_tyctx, concrete);

            // If we have a failure or unknown, return right away
            if !matches!(static_result, VerificationResult::Success) {
                return static_result;
            }

            // Otherwise, try again, but adding the assertion that some width is
            // different than our current assigment
            let not_equals = width_resolutions.iter().map(|(s, w)| {
                ctx.smt.not(
                    ctx.smt
                        .eq(ctx.smt.atom(s.clone()), ctx.smt.atom((*w).to_string())),
                )
            });
            ctx.smt.assert(ctx.smt.or_many(not_equals)).unwrap();

            resolve_dynamic_widths(rulectx, concrete, ctx, unresolved_widths, attempt + 1)
        }
        Ok(Response::Unsat) => {
            if attempt == 0 {
                log::warn!(
                    "Rule not applicable as written for rule assumptions, skipping full query"
                );
                let unsat = ctx.smt.get_unsat_core().unwrap();
                log::warn!("Unsat core:\n{}", ctx.smt.display(unsat));
                VerificationResult::InapplicableRule
            } else {
                // If this is not the first attempt, some previous width assignment must
                // have succeeded.
                VerificationResult::Success
            }
        }
        Ok(Response::Unknown) => {
            panic!("Solver said 'unk'");
        }
        Err(err) => {
            unreachable!("Error! {:?}", err);
        }
    }
}

pub fn run_solver_with_static_widths(
    rulectx: &RuleCtx,
    tyctx: &TypeContext,
    concrete: &Option<ConcreteTest>,
) -> VerificationResult {
    // Declare variables again, this time with all static widths
    let mut solver = easy_smt::ContextBuilder::new()
        .replay_file(Some(std::fs::File::create("static_widths.smt2").unwrap()))
        .solver("z3", ["-smt2", "-in"])
        .build()
        .unwrap();
    solver
        .set_option(":produce-unsat-cores", solver.true_())
        .unwrap();
    let mut ctx = SolverCtx {
        smt: solver,
        find_widths: false,
        tyctx: tyctx.clone(),
        bitwidth: MAX_WIDTH,
        var_map: HashMap::new(),
        width_vars: HashMap::new(),
        width_assumptions: vec![],
        additional_decls: vec![],
        additional_assumptions: vec![],
        additional_assertions: vec![],
        fresh_bits_idx: 0,
        lhs_load_args: None,
        rhs_load_args: None,
        lhs_store_args: None,
        rhs_store_args: None,
        load_return: None,
        lhs_flag: true,
    };
    let (assumptions, mut assertions) = ctx.declare_variables(rulectx.rule_sem, rulectx.config);

    let lhs = ctx.vir_expr_to_sexp(rulectx.rule_sem.lhs.clone());
    ctx.lhs_flag = false;
    let rhs = ctx.vir_expr_to_sexp(rulectx.rule_sem.rhs.clone());

    // For debugging
    let unnamed_rule = String::from("<unnamed rule>");
    let rulename = rulectx
        .rule
        .name
        .map(|name| &rulectx.typeenv.syms[name.index()])
        .unwrap_or(&unnamed_rule);
    let unit = "()".to_string();
    let widthname = ctx
        .static_width(&rulectx.rule_sem.lhs)
        .map_or(unit, |s| format!("width {}", s));

    // Check whether the assumptions are possible
    let feasibility = ctx.check_assumptions_feasibility(
        &assumptions,
        &rulectx.rule_sem.term_input_bvs,
        rulectx.config,
    );
    if feasibility != VerificationResult::Success {
        log::warn!("Rule not applicable as written for rule assumptions, skipping full query");
        return feasibility;
    }

    // Correctness query
    // Verification condition: first rule's LHS and RHS are equal
    if let Some(concrete) = concrete {
        return test_concrete_with_static_widths(
            rulectx,
            concrete,
            lhs,
            rhs,
            &mut ctx,
            assumptions,
        );
    }

    let condition = if let Some(condition) = &rulectx.config.custom_verification_condition {
        let term_args = rulectx
            .rule_sem
            .term_args
            .iter()
            .map(|s| ctx.smt.atom(s))
            .collect();
        let custom_condition = condition(&ctx.smt, term_args, lhs, rhs);
        log::debug!(
            "Custom verification condition:\n\t{}\n",
            ctx.smt.display(custom_condition)
        );
        custom_condition
    } else {
        // Note: this is where we ask if the LHS and the RHS are equal
        let side_equality = ctx.smt.eq(lhs, rhs);
        log::debug!(
            "LHS and RHS equality condition:{}",
            ctx.smt.display(side_equality)
        );
        side_equality
    };

    for a in &ctx.additional_assertions {
        assertions.push(*a);
    }

    let assumption_conjunction = ctx.smt.and_many(assumptions);
    let mut full_condition = if !assertions.is_empty() {
        let assertion_conjunction = ctx.smt.and_many(assertions.clone());
        ctx.smt.and(condition, assertion_conjunction)
    } else {
        condition
    };

    let mut load_conditions = vec![];
    match (&ctx.lhs_load_args, &ctx.rhs_load_args) {
        (Some(_), Some(_)) => {
            let lhs_args_vec = ctx.lhs_load_args.clone().unwrap();
            let rhs_args_vec = ctx.rhs_load_args.clone().unwrap();
            log::debug!("Load argument conditions:");
            for i in 0..lhs_args_vec.len() {
                let arg_equal = ctx.smt.eq(lhs_args_vec[i], rhs_args_vec[i]);
                load_conditions.push(arg_equal);
                log::debug!("\t{}", ctx.smt.display(arg_equal));
                full_condition = ctx.smt.and(full_condition, arg_equal);
            }
        }
        (None, None) => (),
        (Some(_), None) => {
            log::error!("Verification failed for {}, {}", rulename, widthname);
            log::error!("Left hand side has load statement but right hand side does not.");
            return VerificationResult::Failure(Counterexample {});
        }
        (None, Some(_)) => {
            log::error!("Verification failed for {}, {}", rulename, widthname);
            log::error!("Right hand side has load statement but left hand side does not.");
            return VerificationResult::Failure(Counterexample {});
        }
    }

    let mut store_conditions = vec![];
    match (&ctx.lhs_store_args, &ctx.rhs_store_args) {
        (Some(_), Some(_)) => {
            let lhs_args_vec = ctx.lhs_store_args.clone().unwrap();
            let rhs_args_vec = ctx.rhs_store_args.clone().unwrap();
            log::debug!("Store argument conditions:");

            for i in 0..lhs_args_vec.len() {
                let arg_equal = ctx.smt.eq(lhs_args_vec[i], rhs_args_vec[i]);
                store_conditions.push(arg_equal);
                log::debug!("\t{}", ctx.smt.display(arg_equal));
                full_condition = ctx.smt.and(full_condition, arg_equal)
            }
        }
        (None, None) => (),
        (Some(_), None) => {
            log::error!("Verification failed for {}, {}", rulename, widthname);
            log::error!("Left hand side has store statement but right hand side does not.");
            return VerificationResult::Failure(Counterexample {});
        }
        (None, Some(_)) => {
            log::error!("Verification failed for {}, {}", rulename, widthname);
            log::error!("Right hand side has store statement but left hand side does not.");
            return VerificationResult::Failure(Counterexample {});
        }
    }

    log::trace!(
        "Full verification condition:{}",
        ctx.smt.display(full_condition)
    );
    let query = ctx
        .smt
        .not(ctx.smt.imp(assumption_conjunction, full_condition));
    log::trace!("Running query");
    ctx.smt.assert(query).unwrap();

    match ctx.smt.check() {
        Ok(Response::Sat) => {
            println!("Verification failed for {}, {}", rulename, widthname);
            ctx.display_model(rulectx.termenv, rulectx.typeenv, rulectx.rule, lhs, rhs);
            let vals = ctx.smt.get_value(vec![condition]).unwrap();
            for (variable, value) in vals {
                if value == ctx.smt.false_() {
                    println!("Failed condition:\n{}", ctx.smt.display(variable));
                } else if value == ctx.smt.true_() {
                    println!("Condition met, but failed some assertion(s).")
                }
            }

            if !assertions.is_empty() {
                let vals = ctx.smt.get_value(assertions).unwrap();
                for (variable, value) in vals {
                    if value == ctx.smt.false_() {
                        println!("Failed assertion:\n{}", ctx.smt.display(variable));
                    }
                }
            }

            if !load_conditions.is_empty() {
                let vals = ctx.smt.get_value(load_conditions).unwrap();
                for (variable, value) in vals {
                    if value == ctx.smt.false_() {
                        log::error!("Failed load condition:\n{}", ctx.smt.display(variable));
                    }
                }
            }
            VerificationResult::Failure(Counterexample {})
        }
        Ok(Response::Unsat) => {
            println!("Verification succeeded for {}, {}", rulename, widthname);
            VerificationResult::Success
        }
        Ok(Response::Unknown) => {
            panic!("Solver said 'unk'");
        }
        Err(err) => {
            unreachable!("Error! {:?}", err);
        }
    }
}

pub fn test_concrete_with_static_widths(
    rulectx: &RuleCtx,
    concrete: &ConcreteTest,
    lhs: SExpr,
    rhs: SExpr,
    ctx: &mut SolverCtx,
    assumptions: Vec<SExpr>,
) -> VerificationResult {
    // Test code only: test against concrete input/output
    // Check that our expected output is valid
    for (i, a) in assumptions.iter().enumerate() {
        ctx.smt
            .assert(ctx.smt.named(format!("conc{i}"), *a))
            .unwrap();
    }
    for (i, e) in ctx.additional_assertions.iter().enumerate() {
        ctx.smt
            .assert(ctx.smt.named(format!("conc_assert{i}"), *e))
            .unwrap();
    }
    ctx.smt.push().unwrap();
    let eq = ctx
        .smt
        .eq(rhs, ctx.smt.atom(concrete.output.literal.clone()));

    ctx.smt
        .assert(ctx.smt.named("conceq".to_string(), eq))
        .unwrap();

    for (i, a) in rulectx.rule_sem.rhs_assertions.iter().enumerate() {
        let p = ctx.vir_expr_to_sexp(a.clone());
        ctx.smt
            .assert(ctx.smt.named(format!("rhs_assert{i}"), p))
            .unwrap();
    }

    if !matches!(ctx.smt.check(), Ok(Response::Sat)) {
        // Bad! This is a bug!
        // Pop the output assertion
        ctx.smt.pop().unwrap();
        // Try again
        assert!(matches!(ctx.smt.check(), Ok(Response::Sat)));
        // Get the value for what output is to panic with a useful message
        let val = ctx.smt.get_value(vec![rhs]).unwrap()[0].1;
        ctx.display_model(rulectx.termenv, rulectx.typeenv, rulectx.rule, lhs, rhs);
        panic!(
            "Expected {}, got {}",
            concrete.output.literal,
            ctx.display_hex_to_bin(val)
        );
    } else {
        log::debug!(
            "Expected concrete result matched: {}",
            concrete.output.literal
        );
        ctx.smt.pop().unwrap();
    }

    // Check that there is no other possible output
    ctx.smt.push().unwrap();
    ctx.smt
        .assert(
            ctx.smt.not(
                ctx.smt
                    .eq(rhs, ctx.smt.atom(concrete.output.literal.clone())),
            ),
        )
        .unwrap();
    if !matches!(ctx.smt.check(), Ok(Response::Unsat)) {
        // Get the value for what output is to panic with a useful message
        let val = ctx.smt.get_value(vec![rhs]).unwrap()[0].1;
        ctx.display_model(rulectx.termenv, rulectx.typeenv, rulectx.rule, lhs, rhs);
        // AVH TODO: should probably elevate back to an error with custom verification condition
        log::error!(
            "WARNING: Expected ONLY {}, got POSSIBLE {}",
            concrete.output.literal,
            ctx.display_hex_to_bin(val)
        );
    }
    ctx.smt.pop().unwrap();
    VerificationResult::Success
}

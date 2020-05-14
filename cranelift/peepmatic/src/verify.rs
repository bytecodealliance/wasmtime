//! Verification and type checking of optimizations.
//!
//! For type checking, we compile the AST's type constraints down into Z3
//! variables and assertions. If Z3 finds the assertions satisfiable, then we're
//! done! If it finds them unsatisfiable, we use the `get_unsat_core` method to
//! get the minimal subset of assertions that are in conflict, and report a
//! best-effort type error message with them. These messages aren't perfect, but
//! they're Good Enough when embedded in the source text via our tracking of
//! `wast::Span`s.
//!
//! Verifying that there aren't any counter-examples (inputs for which the LHS
//! and RHS produce different results) for a particular optimization is not
//! implemented yet.

use crate::ast::{Span as _, *};
use crate::traversals::{Dfs, TraversalEvent};
use peepmatic_runtime::{
    operator::{Operator, TypingContext as TypingContextTrait},
    r#type::{BitWidth, Kind, Type},
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::hash::Hash;
use std::iter;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use wast::{Error as WastError, Id, Span};
use z3::ast::Ast;

/// A verification or type checking error.
#[derive(Debug)]
pub struct VerifyError {
    errors: Vec<anyhow::Error>,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for e in &self.errors {
            writeln!(f, "{}\n", e)?;
        }
        Ok(())
    }
}

impl std::error::Error for VerifyError {}

impl From<WastError> for VerifyError {
    fn from(e: WastError) -> Self {
        VerifyError {
            errors: vec![e.into()],
        }
    }
}

impl From<anyhow::Error> for VerifyError {
    fn from(e: anyhow::Error) -> Self {
        VerifyError { errors: vec![e] }
    }
}

impl VerifyError {
    /// To provide a more useful error this function can be used to extract
    /// relevant textual information about this error into the error itself.
    ///
    /// The `contents` here should be the full text of the original file being
    /// parsed, and this will extract a sub-slice as necessary to render in the
    /// `Display` implementation later on.
    pub fn set_text(&mut self, contents: &str) {
        for e in &mut self.errors {
            if let Some(e) = e.downcast_mut::<WastError>() {
                e.set_text(contents);
            }
        }
    }

    /// To provide a more useful error this function can be used to set
    /// the file name that this error is associated with.
    ///
    /// The `path` here will be stored in this error and later rendered in the
    /// `Display` implementation.
    pub fn set_path(&mut self, path: &Path) {
        for e in &mut self.errors {
            if let Some(e) = e.downcast_mut::<WastError>() {
                e.set_path(path);
            }
        }
    }
}

/// Either `Ok(T)` or `Err(VerifyError)`.
pub type VerifyResult<T> = Result<T, VerifyError>;

/// Verify and type check a set of optimizations.
pub fn verify(opts: &Optimizations) -> VerifyResult<()> {
    if opts.optimizations.is_empty() {
        return Err(anyhow::anyhow!("no optimizations").into());
    }

    verify_unique_left_hand_sides(opts)?;

    let z3 = &z3::Context::new(&z3::Config::new());
    for opt in &opts.optimizations {
        verify_optimization(z3, opt)?;
    }
    Ok(())
}

/// Check that every LHS in the given optimizations is unique.
///
/// If there were duplicates, then it would be nondeterministic which one we
/// applied and would make automata construction more difficult. It is better to
/// check for duplicates and reject them if found.
fn verify_unique_left_hand_sides(opts: &Optimizations) -> VerifyResult<()> {
    let mut lefts = HashMap::new();
    for opt in &opts.optimizations {
        let canon_lhs = canonicalized_lhs_key(&opt.lhs);
        let existing = lefts.insert(canon_lhs, opt.lhs.span());
        if let Some(span) = existing {
            return Err(VerifyError {
                errors: vec![
                    anyhow::anyhow!("error: two optimizations cannot have the same left-hand side"),
                    WastError::new(span, "note: first use of this left-hand side".into()).into(),
                    WastError::new(
                        opt.lhs.span(),
                        "note: second use of this left-hand side".into(),
                    )
                    .into(),
                ],
            });
        }
    }
    Ok(())
}

/// When checking for duplicate left-hand sides, we need to consider patterns
/// that are duplicates up to renaming identifiers. For example, these LHSes
/// should be considered duplicates of each other:
///
/// ```lisp
/// (=> (iadd $x $y) ...)
/// (=> (iadd $a $b) ...)
/// ```
///
/// This function creates an opaque, canonicalized hash key for left-hand sides
/// that sees through identifier renaming.
fn canonicalized_lhs_key(lhs: &Lhs) -> impl Hash + Eq {
    let mut var_to_canon = HashMap::new();
    let mut const_to_canon = HashMap::new();
    let mut canonicalized = vec![];

    for (event, ast) in Dfs::new(lhs) {
        if event != TraversalEvent::Enter {
            continue;
        }
        use CanonicalBit::*;
        canonicalized.push(match ast {
            DynAstRef::Lhs(_) => Other("Lhs"),
            DynAstRef::Pattern(_) => Other("Pattern"),
            DynAstRef::ValueLiteral(_) => Other("ValueLiteral"),
            DynAstRef::Integer(i) => Integer(i.value),
            DynAstRef::Boolean(b) => Boolean(b.value),
            DynAstRef::ConditionCode(cc) => ConditionCode(cc.cc),
            DynAstRef::PatternOperation(o) => Operation(o.operator, o.r#type.get()),
            DynAstRef::Precondition(p) => Precondition(p.constraint),
            DynAstRef::ConstraintOperand(_) => Other("ConstraintOperand"),
            DynAstRef::Variable(Variable { id, .. }) => {
                let new_id = var_to_canon.len() as u32;
                let canon_id = var_to_canon.entry(id).or_insert(new_id);
                Var(*canon_id)
            }
            DynAstRef::Constant(Constant { id, .. }) => {
                let new_id = const_to_canon.len() as u32;
                let canon_id = const_to_canon.entry(id).or_insert(new_id);
                Const(*canon_id)
            }
            other => unreachable!("unreachable ast node: {:?}", other),
        });
    }

    return canonicalized;

    #[derive(Hash, PartialEq, Eq)]
    enum CanonicalBit {
        Var(u32),
        Const(u32),
        Integer(i64),
        Boolean(bool),
        ConditionCode(peepmatic_runtime::cc::ConditionCode),
        Operation(Operator, Option<Type>),
        Precondition(Constraint),
        Other(&'static str),
    }
}

pub(crate) struct TypingContext<'a> {
    z3: &'a z3::Context,
    type_kind_sort: z3::DatatypeSort<'a>,
    solver: z3::Solver<'a>,

    // The type of the root of the optimization. Initialized when collecting
    // type constraints.
    root_ty: Option<TypeVar<'a>>,

    // See the comments above `enter_operation_scope`.
    operation_scope: HashMap<&'static str, TypeVar<'a>>,

    // A map from identifiers to the type variable describing its type.
    id_to_type_var: HashMap<Id<'a>, TypeVar<'a>>,

    // A list of type constraints, the span of the AST node where the constraint
    // originates from, and an optional message to be displayed if the
    // constraint is not satisfied.
    constraints: Vec<(z3::ast::Bool<'a>, Span, Option<Cow<'static, str>>)>,

    // Keep track of AST nodes that need to have their types assigned to
    // them. For these AST nodes, we know what bit width to use when
    // interpreting peephole optimization actions.
    boolean_literals: Vec<(&'a Boolean<'a>, TypeVar<'a>)>,
    integer_literals: Vec<(&'a Integer<'a>, TypeVar<'a>)>,
    rhs_operations: Vec<(&'a Operation<'a, Rhs<'a>>, TypeVar<'a>)>,
}

impl<'a> TypingContext<'a> {
    fn new(z3: &'a z3::Context) -> Self {
        let type_kind_sort = z3::DatatypeBuilder::new(z3)
            .variant("int", &[])
            .variant("bool", &[])
            .variant("cpu_flags", &[])
            .variant("cc", &[])
            .variant("void", &[])
            .finish("TypeKind");
        TypingContext {
            z3,
            solver: z3::Solver::new(z3),
            root_ty: None,
            operation_scope: Default::default(),
            id_to_type_var: Default::default(),
            type_kind_sort,
            constraints: vec![],
            boolean_literals: Default::default(),
            integer_literals: Default::default(),
            rhs_operations: Default::default(),
        }
    }

    fn init_root_type(&mut self, span: Span, root_ty: TypeVar<'a>) {
        assert!(self.root_ty.is_none());

        // Make sure the root is a valid kind, i.e. not a condition code.
        let is_int = self.is_int(&root_ty);
        let is_bool = self.is_bool(&root_ty);
        let is_void = self.is_void(&root_ty);
        let is_cpu_flags = self.is_cpu_flags(&root_ty);
        self.constraints.push((
            is_int.or(&[&is_bool, &is_void, &is_cpu_flags]),
            span,
            Some(
                "the root of an optimization must be an integer, a boolean, void, or CPU flags"
                    .into(),
            ),
        ));

        self.root_ty = Some(root_ty);
    }

    fn new_type_var(&self) -> TypeVar<'a> {
        let kind =
            z3::ast::Datatype::fresh_const(self.z3, "type-var-kind", &self.type_kind_sort.sort);
        let width = z3::ast::BV::fresh_const(self.z3, "type-var-width", 8);
        TypeVar { kind, width }
    }

    fn get_or_create_type_var_for_id(&mut self, id: Id<'a>) -> TypeVar<'a> {
        if let Some(ty) = self.id_to_type_var.get(&id) {
            ty.clone()
        } else {
            // Note: can't use the entry API because we reborrow `self` here.
            let ty = self.new_type_var();
            self.id_to_type_var.insert(id, ty.clone());
            ty
        }
    }

    fn get_type_var_for_id(&mut self, id: Id<'a>) -> VerifyResult<TypeVar<'a>> {
        if let Some(ty) = self.id_to_type_var.get(&id) {
            Ok(ty.clone())
        } else {
            Err(WastError::new(id.span(), format!("unknown identifier: ${}", id.name())).into())
        }
    }

    // The `#[peepmatic]` macro for operations allows defining operations' types
    // like `(iNN, iNN) -> iNN` where `iNN` all refer to the same integer type
    // variable that must have the same bit width. But other operations might
    // *also* have that type signature but be instantiated at a different bit
    // width. We don't want to mix up which `iNN` variables are and aren't the
    // same. We use this method to track scopes within which all uses of `iNN`
    // and similar refer to the same type variables.
    fn enter_operation_scope<'b>(
        &'b mut self,
    ) -> impl DerefMut<Target = TypingContext<'a>> + Drop + 'b {
        assert!(self.operation_scope.is_empty());
        return Scope(self);

        struct Scope<'a, 'b>(&'b mut TypingContext<'a>)
        where
            'a: 'b;

        impl<'a, 'b> Deref for Scope<'a, 'b>
        where
            'a: 'b,
        {
            type Target = TypingContext<'a>;
            fn deref(&self) -> &TypingContext<'a> {
                self.0
            }
        }

        impl<'a, 'b> DerefMut for Scope<'a, 'b>
        where
            'a: 'b,
        {
            fn deref_mut(&mut self) -> &mut TypingContext<'a> {
                self.0
            }
        }

        impl Drop for Scope<'_, '_> {
            fn drop(&mut self) {
                self.0.operation_scope.clear();
            }
        }
    }

    fn remember_boolean_literal(&mut self, b: &'a Boolean<'a>, ty: TypeVar<'a>) {
        self.assert_is_bool(b.span, &ty);
        self.boolean_literals.push((b, ty));
    }

    fn remember_integer_literal(&mut self, i: &'a Integer<'a>, ty: TypeVar<'a>) {
        self.assert_is_integer(i.span, &ty);
        self.integer_literals.push((i, ty));
    }

    fn remember_rhs_operation(&mut self, op: &'a Operation<'a, Rhs<'a>>, ty: TypeVar<'a>) {
        self.rhs_operations.push((op, ty));
    }

    fn is_int(&self, ty: &TypeVar<'a>) -> z3::ast::Bool<'a> {
        self.type_kind_sort.variants[0]
            .tester
            .apply(&[&ty.kind.clone().into()])
            .as_bool()
            .unwrap()
    }

    fn is_bool(&self, ty: &TypeVar<'a>) -> z3::ast::Bool<'a> {
        self.type_kind_sort.variants[1]
            .tester
            .apply(&[&ty.kind.clone().into()])
            .as_bool()
            .unwrap()
    }

    fn is_cpu_flags(&self, ty: &TypeVar<'a>) -> z3::ast::Bool<'a> {
        self.type_kind_sort.variants[2]
            .tester
            .apply(&[&ty.kind.clone().into()])
            .as_bool()
            .unwrap()
    }

    fn is_condition_code(&self, ty: &TypeVar<'a>) -> z3::ast::Bool<'a> {
        self.type_kind_sort.variants[3]
            .tester
            .apply(&[&ty.kind.clone().into()])
            .as_bool()
            .unwrap()
    }

    fn is_void(&self, ty: &TypeVar<'a>) -> z3::ast::Bool<'a> {
        self.type_kind_sort.variants[4]
            .tester
            .apply(&[&ty.kind.clone().into()])
            .as_bool()
            .unwrap()
    }

    fn assert_is_integer(&mut self, span: Span, ty: &TypeVar<'a>) {
        self.constraints.push((
            self.is_int(ty),
            span,
            Some("type error: expected integer".into()),
        ));
    }

    fn assert_is_bool(&mut self, span: Span, ty: &TypeVar<'a>) {
        self.constraints.push((
            self.is_bool(ty),
            span,
            Some("type error: expected bool".into()),
        ));
    }

    fn assert_is_cpu_flags(&mut self, span: Span, ty: &TypeVar<'a>) {
        self.constraints.push((
            self.is_cpu_flags(ty),
            span,
            Some("type error: expected CPU flags".into()),
        ));
    }

    fn assert_is_cc(&mut self, span: Span, ty: &TypeVar<'a>) {
        self.constraints.push((
            self.is_condition_code(ty),
            span,
            Some("type error: expected condition code".into()),
        ));
    }

    fn assert_is_void(&mut self, span: Span, ty: &TypeVar<'a>) {
        self.constraints.push((
            self.is_void(ty),
            span,
            Some("type error: expected void".into()),
        ));
    }

    fn assert_bit_width(&mut self, span: Span, ty: &TypeVar<'a>, width: u8) {
        debug_assert!(width == 0 || width.is_power_of_two());
        let width_var = z3::ast::BV::from_i64(self.z3, width as i64, 8);
        let is_width = width_var._eq(&ty.width);
        self.constraints.push((
            is_width,
            span,
            Some(format!("type error: expected bit width = {}", width).into()),
        ));
    }

    fn assert_bit_width_lt(&mut self, span: Span, a: &TypeVar<'a>, b: &TypeVar<'a>) {
        self.constraints.push((
            a.width.bvult(&b.width),
            span,
            Some("type error: expected narrower bit width".into()),
        ));
    }

    fn assert_bit_width_gt(&mut self, span: Span, a: &TypeVar<'a>, b: &TypeVar<'a>) {
        self.constraints.push((
            a.width.bvugt(&b.width),
            span,
            Some("type error: expected wider bit width".into()),
        ));
    }

    fn assert_type_eq(
        &mut self,
        span: Span,
        lhs: &TypeVar<'a>,
        rhs: &TypeVar<'a>,
        msg: Option<Cow<'static, str>>,
    ) {
        self.constraints
            .push((lhs.kind._eq(&rhs.kind), span, msg.clone()));
        self.constraints
            .push((lhs.width._eq(&rhs.width), span, msg));
    }

    fn type_check(&self, span: Span) -> VerifyResult<()> {
        let trackers = iter::repeat_with(|| z3::ast::Bool::fresh_const(self.z3, "type-constraint"))
            .take(self.constraints.len())
            .collect::<Vec<_>>();

        let mut tracker_to_diagnostics = HashMap::with_capacity(self.constraints.len());

        for (constraint_data, tracker) in self.constraints.iter().zip(trackers) {
            let (constraint, span, msg) = constraint_data;
            self.solver.assert_and_track(constraint, &tracker);
            tracker_to_diagnostics.insert(tracker, (*span, msg.clone()));
        }

        match self.solver.check() {
            z3::SatResult::Sat => Ok(()),
            z3::SatResult::Unsat => {
                let core = self.solver.get_unsat_core();
                if core.is_empty() {
                    return Err(WastError::new(
                        span,
                        "z3 determined the type constraints for this optimization are \
                         unsatisfiable, meaning there is a type error, but z3 did not give us any \
                         additional information"
                            .into(),
                    )
                    .into());
                }

                let mut errors = core
                    .iter()
                    .map(|tracker| {
                        let (span, msg) = &tracker_to_diagnostics[tracker];
                        (
                            *span,
                            WastError::new(
                                *span,
                                msg.clone().unwrap_or("type error".into()).into(),
                            )
                            .into(),
                        )
                    })
                    .collect::<Vec<_>>();
                errors.sort_by_key(|(span, _)| *span);
                let errors = errors.into_iter().map(|(_, e)| e).collect();

                Err(VerifyError { errors })
            }
            z3::SatResult::Unknown => Err(anyhow::anyhow!(
                "z3 returned 'unknown' when evaluating type constraints: {}",
                self.solver
                    .get_reason_unknown()
                    .unwrap_or_else(|| "<no reason given>".into())
            )
            .into()),
        }
    }

    fn assign_types(&mut self) -> VerifyResult<()> {
        for (int, ty) in mem::replace(&mut self.integer_literals, vec![]) {
            let width = self.ty_var_to_width(&ty)?;
            int.bit_width.set(Some(width));
        }

        for (b, ty) in mem::replace(&mut self.boolean_literals, vec![]) {
            let width = self.ty_var_to_width(&ty)?;
            b.bit_width.set(Some(width));
        }

        for (op, ty) in mem::replace(&mut self.rhs_operations, vec![]) {
            let kind = self.op_ty_var_to_kind(&ty);
            let bit_width = match kind {
                Kind::CpuFlags | Kind::Void => BitWidth::One,
                Kind::Int | Kind::Bool => self.ty_var_to_width(&ty)?,
            };
            debug_assert!(op.r#type.get().is_none());
            op.r#type.set(Some(Type { kind, bit_width }));
        }

        Ok(())
    }

    fn ty_var_to_width(&self, ty_var: &TypeVar<'a>) -> VerifyResult<BitWidth> {
        // Doing solver push/pops apparently clears out the model, so we have to
        // re-check each time to ensure that it exists, and Z3 doesn't helpfully
        // abort the process for us. This should be fast, since the solver
        // remembers inferences from earlier checks.
        assert_eq!(self.solver.check(), z3::SatResult::Sat);

        // Check if there is more than one satisfying assignment to
        // `ty_var`'s width variable. If so, then it must be polymorphic. If
        // not, then it must have a fixed value.
        let model = self.solver.get_model();
        let width_var = model.eval(&ty_var.width).unwrap();
        let bit_width: u8 = width_var.as_u64().unwrap().try_into().unwrap();

        self.solver.push();
        self.solver.assert(&ty_var.width._eq(&width_var).not());
        let is_polymorphic = match self.solver.check() {
            z3::SatResult::Sat => true,
            z3::SatResult::Unsat => false,
            z3::SatResult::Unknown => panic!("Z3 cannot determine bit width of type"),
        };
        self.solver.pop(1);

        if is_polymorphic {
            // If something is polymorphic over bit widths, it must be
            // polymorphic over the same bit width as the whole
            // optimization.
            //
            // TODO: We should have a better model for bit-width
            // polymorphism. The current setup works for all the use cases we
            // currently care about, and is relatively easy to implement when
            // matching and constructing the RHS, but is a bit ad-hoc. Maybe
            // allow each LHS variable a polymorphic bit width, augment the AST
            // with that info, and later emit match ops as necessary to express
            // their relative constraints? *hand waves*
            self.solver.push();
            self.solver
                .assert(&ty_var.width._eq(&self.root_ty.as_ref().unwrap().width));
            match self.solver.check() {
                z3::SatResult::Sat => {}
                z3::SatResult::Unsat => {
                    return Err(anyhow::anyhow!(
                        "AST node is bit width polymorphic, but not over the optimization's root \
                         width"
                    )
                    .into())
                }
                z3::SatResult::Unknown => panic!("Z3 cannot determine bit width of type"),
            };
            self.solver.pop(1);

            Ok(BitWidth::Polymorphic)
        } else {
            Ok(BitWidth::try_from(bit_width).unwrap())
        }
    }

    fn op_ty_var_to_kind(&self, ty_var: &TypeVar<'a>) -> Kind {
        for (predicate, kind) in [
            (Self::is_int as fn(_, _) -> _, Kind::Int),
            (Self::is_bool, Kind::Bool),
            (Self::is_cpu_flags, Kind::CpuFlags),
            (Self::is_void, Kind::Void),
        ]
        .iter()
        {
            self.solver.push();
            self.solver.assert(&predicate(self, ty_var));
            match self.solver.check() {
                z3::SatResult::Sat => {
                    self.solver.pop(1);
                    return *kind;
                }
                z3::SatResult::Unsat => {
                    self.solver.pop(1);
                    continue;
                }
                z3::SatResult::Unknown => panic!("Z3 cannot determine the type's kind"),
            }
        }

        // This would only happen if given a `TypeVar` whose kind was a
        // condition code, but we only use this function for RHS operations,
        // which cannot be condition codes.
        panic!("cannot convert type variable's kind to `peepmatic_runtime::type::Kind`")
    }
}

impl<'a> TypingContextTrait<'a> for TypingContext<'a> {
    type TypeVariable = TypeVar<'a>;

    fn cc(&mut self, span: Span) -> TypeVar<'a> {
        let ty = self.new_type_var();
        self.assert_is_cc(span, &ty);
        ty
    }

    fn bNN(&mut self, span: Span) -> TypeVar<'a> {
        if let Some(ty) = self.operation_scope.get("bNN") {
            return ty.clone();
        }

        let ty = self.new_type_var();
        self.assert_is_bool(span, &ty);
        self.operation_scope.insert("bNN", ty.clone());
        ty
    }

    fn iNN(&mut self, span: Span) -> TypeVar<'a> {
        if let Some(ty) = self.operation_scope.get("iNN") {
            return ty.clone();
        }

        let ty = self.new_type_var();
        self.assert_is_integer(span, &ty);
        self.operation_scope.insert("iNN", ty.clone());
        ty
    }

    fn iMM(&mut self, span: Span) -> TypeVar<'a> {
        if let Some(ty) = self.operation_scope.get("iMM") {
            return ty.clone();
        }

        let ty = self.new_type_var();
        self.assert_is_integer(span, &ty);
        self.operation_scope.insert("iMM", ty.clone());
        ty
    }

    fn cpu_flags(&mut self, span: Span) -> TypeVar<'a> {
        if let Some(ty) = self.operation_scope.get("cpu_flags") {
            return ty.clone();
        }

        let ty = self.new_type_var();
        self.assert_is_cpu_flags(span, &ty);
        self.assert_bit_width(span, &ty, 1);
        self.operation_scope.insert("cpu_flags", ty.clone());
        ty
    }

    fn b1(&mut self, span: Span) -> TypeVar<'a> {
        let b1 = self.new_type_var();
        self.assert_is_bool(span, &b1);
        self.assert_bit_width(span, &b1, 1);
        b1
    }

    fn void(&mut self, span: Span) -> TypeVar<'a> {
        let void = self.new_type_var();
        self.assert_is_void(span, &void);
        self.assert_bit_width(span, &void, 0);
        void
    }

    fn bool_or_int(&mut self, span: Span) -> TypeVar<'a> {
        let ty = self.new_type_var();
        let is_int = self.type_kind_sort.variants[0]
            .tester
            .apply(&[&ty.kind.clone().into()])
            .as_bool()
            .unwrap();
        let is_bool = self.type_kind_sort.variants[1]
            .tester
            .apply(&[&ty.kind.clone().into()])
            .as_bool()
            .unwrap();
        self.constraints.push((
            is_int.or(&[&is_bool]),
            span,
            Some("type error: must be either an int or a bool type".into()),
        ));
        ty
    }

    fn any_t(&mut self, _span: Span) -> TypeVar<'a> {
        if let Some(ty) = self.operation_scope.get("any_t") {
            return ty.clone();
        }

        let ty = self.new_type_var();
        self.operation_scope.insert("any_t", ty.clone());
        ty
    }
}

#[derive(Clone)]
pub(crate) struct TypeVar<'a> {
    kind: z3::ast::Datatype<'a>,
    width: z3::ast::BV<'a>,
}

fn verify_optimization(z3: &z3::Context, opt: &Optimization) -> VerifyResult<()> {
    let mut context = TypingContext::new(z3);
    collect_type_constraints(&mut context, opt)?;
    context.type_check(opt.span)?;
    context.assign_types()?;

    // TODO: add another pass here to check for counter-examples to this
    // optimization, i.e. inputs where the LHS and RHS are not equivalent.

    Ok(())
}

fn collect_type_constraints<'a>(
    context: &mut TypingContext<'a>,
    opt: &'a Optimization<'a>,
) -> VerifyResult<()> {
    use crate::traversals::TraversalEvent as TE;

    let lhs_ty = context.new_type_var();
    context.init_root_type(opt.lhs.span, lhs_ty.clone());

    let rhs_ty = context.new_type_var();
    context.assert_type_eq(
        opt.span,
        &lhs_ty,
        &rhs_ty,
        Some("type error: the left-hand side and right-hand side must have the same type".into()),
    );

    // A stack of type variables that we are constraining as we traverse the
    // AST. Operations push new type variables for their operands' expected
    // types, and exiting a `Pattern` in the traversal pops them off.
    let mut expected_types = vec![lhs_ty];

    // Build up the type constraints for the left-hand side.
    for (event, node) in Dfs::new(&opt.lhs) {
        match (event, node) {
            (TE::Enter, DynAstRef::Pattern(Pattern::Constant(Constant { id, span })))
            | (TE::Enter, DynAstRef::Pattern(Pattern::Variable(Variable { id, span }))) => {
                let id = context.get_or_create_type_var_for_id(*id);
                context.assert_type_eq(*span, expected_types.last().unwrap(), &id, None);
            }
            (TE::Enter, DynAstRef::Pattern(Pattern::ValueLiteral(ValueLiteral::Integer(i)))) => {
                let ty = expected_types.last().unwrap();
                context.remember_integer_literal(i, ty.clone());
            }
            (TE::Enter, DynAstRef::Pattern(Pattern::ValueLiteral(ValueLiteral::Boolean(b)))) => {
                let ty = expected_types.last().unwrap();
                context.remember_boolean_literal(b, ty.clone());
            }
            (
                TE::Enter,
                DynAstRef::Pattern(Pattern::ValueLiteral(ValueLiteral::ConditionCode(cc))),
            ) => {
                let ty = expected_types.last().unwrap();
                context.assert_is_cc(cc.span, ty);
            }
            (TE::Enter, DynAstRef::PatternOperation(op)) => {
                let result_ty;
                let mut operand_types = vec![];
                {
                    let mut scope = context.enter_operation_scope();
                    result_ty = op.operator.result_type(&mut *scope, op.span);
                    op.operator
                        .immediate_types(&mut *scope, op.span, &mut operand_types);
                    op.operator
                        .param_types(&mut *scope, op.span, &mut operand_types);
                }

                if op.operands.len() != operand_types.len() {
                    return Err(WastError::new(
                        op.span,
                        format!(
                            "Expected {} operands but found {}",
                            operand_types.len(),
                            op.operands.len()
                        ),
                    )
                    .into());
                }

                for imm in op
                    .operands
                    .iter()
                    .take(op.operator.immediates_arity() as usize)
                {
                    match imm {
                        Pattern::ValueLiteral(_) |
                        Pattern::Constant(_) |
                        Pattern::Variable(_) => continue,
                        Pattern::Operation(op) => return Err(WastError::new(
                            op.span,
                            "operations are invalid immediates; must be a value literal, constant, \
                             or variable".into()
                        ).into()),
                    }
                }

                match op.operator {
                    Operator::Ireduce | Operator::Uextend | Operator::Sextend => {
                        if op.r#type.get().is_none() {
                            return Err(WastError::new(
                                op.span,
                                "`ireduce`, `sextend`, and `uextend` require an ascribed type, \
                                 like `(sextend{i64} ...)`"
                                    .into(),
                            )
                            .into());
                        }
                    }
                    _ => {}
                }

                match op.operator {
                    Operator::Uextend | Operator::Sextend => {
                        context.assert_bit_width_gt(op.span, &result_ty, &operand_types[0]);
                    }
                    Operator::Ireduce => {
                        context.assert_bit_width_lt(op.span, &result_ty, &operand_types[0]);
                    }
                    _ => {}
                }

                if let Some(ty) = op.r#type.get() {
                    match ty.kind {
                        Kind::Bool => context.assert_is_bool(op.span, &result_ty),
                        Kind::Int => context.assert_is_integer(op.span, &result_ty),
                        Kind::Void => context.assert_is_void(op.span, &result_ty),
                        Kind::CpuFlags => {
                            unreachable!("no syntax for ascribing CPU flags types right now")
                        }
                    }
                    if let Some(w) = ty.bit_width.fixed_width() {
                        context.assert_bit_width(op.span, &result_ty, w);
                    }
                }

                context.assert_type_eq(op.span, expected_types.last().unwrap(), &result_ty, None);

                operand_types.reverse();
                expected_types.extend(operand_types);
            }
            (TE::Exit, DynAstRef::Pattern(..)) => {
                expected_types.pop().unwrap();
            }
            (TE::Enter, DynAstRef::Precondition(pre)) => {
                type_constrain_precondition(context, pre)?;
            }
            _ => continue,
        }
    }

    // We should have exited exactly as many patterns as we entered: one for the
    // root pattern and the initial `lhs_ty`, and then the rest for the operands
    // of pattern operations.
    assert!(expected_types.is_empty());

    // Collect the type constraints for the right-hand side.
    expected_types.push(rhs_ty);
    for (event, node) in Dfs::new(&opt.rhs) {
        match (event, node) {
            (TE::Enter, DynAstRef::Rhs(Rhs::ValueLiteral(ValueLiteral::Integer(i)))) => {
                let ty = expected_types.last().unwrap();
                context.remember_integer_literal(i, ty.clone());
            }
            (TE::Enter, DynAstRef::Rhs(Rhs::ValueLiteral(ValueLiteral::Boolean(b)))) => {
                let ty = expected_types.last().unwrap();
                context.remember_boolean_literal(b, ty.clone());
            }
            (TE::Enter, DynAstRef::Rhs(Rhs::ValueLiteral(ValueLiteral::ConditionCode(cc)))) => {
                let ty = expected_types.last().unwrap();
                context.assert_is_cc(cc.span, ty);
            }
            (TE::Enter, DynAstRef::Rhs(Rhs::Constant(Constant { span, id })))
            | (TE::Enter, DynAstRef::Rhs(Rhs::Variable(Variable { span, id }))) => {
                let id_ty = context.get_type_var_for_id(*id)?;
                context.assert_type_eq(*span, expected_types.last().unwrap(), &id_ty, None);
            }
            (TE::Enter, DynAstRef::RhsOperation(op)) => {
                let result_ty;
                let mut operand_types = vec![];
                {
                    let mut scope = context.enter_operation_scope();
                    result_ty = op.operator.result_type(&mut *scope, op.span);
                    op.operator
                        .immediate_types(&mut *scope, op.span, &mut operand_types);
                    op.operator
                        .param_types(&mut *scope, op.span, &mut operand_types);
                }

                if op.operands.len() != operand_types.len() {
                    return Err(WastError::new(
                        op.span,
                        format!(
                            "Expected {} operands but found {}",
                            operand_types.len(),
                            op.operands.len()
                        ),
                    )
                    .into());
                }

                for imm in op
                    .operands
                    .iter()
                    .take(op.operator.immediates_arity() as usize)
                {
                    match imm {
                        Rhs::ValueLiteral(_)
                        | Rhs::Constant(_)
                        | Rhs::Variable(_)
                        | Rhs::Unquote(_) => continue,
                        Rhs::Operation(op) => return Err(WastError::new(
                            op.span,
                            "operations are invalid immediates; must be a value literal, unquote, \
                             constant, or variable"
                                .into(),
                        )
                        .into()),
                    }
                }

                match op.operator {
                    Operator::Ireduce | Operator::Uextend | Operator::Sextend => {
                        if op.r#type.get().is_none() {
                            return Err(WastError::new(
                                op.span,
                                "`ireduce`, `sextend`, and `uextend` require an ascribed type, \
                                 like `(sextend{i64} ...)`"
                                    .into(),
                            )
                            .into());
                        }
                    }
                    _ => {}
                }

                match op.operator {
                    Operator::Uextend | Operator::Sextend => {
                        context.assert_bit_width_gt(op.span, &result_ty, &operand_types[0]);
                    }
                    Operator::Ireduce => {
                        context.assert_bit_width_lt(op.span, &result_ty, &operand_types[0]);
                    }
                    _ => {}
                }

                if let Some(ty) = op.r#type.get() {
                    match ty.kind {
                        Kind::Bool => context.assert_is_bool(op.span, &result_ty),
                        Kind::Int => context.assert_is_integer(op.span, &result_ty),
                        Kind::Void => context.assert_is_void(op.span, &result_ty),
                        Kind::CpuFlags => {
                            unreachable!("no syntax for ascribing CPU flags types right now")
                        }
                    }
                    if let Some(w) = ty.bit_width.fixed_width() {
                        context.assert_bit_width(op.span, &result_ty, w);
                    }
                }

                context.assert_type_eq(op.span, expected_types.last().unwrap(), &result_ty, None);
                if op.r#type.get().is_none() {
                    context.remember_rhs_operation(op, result_ty);
                }

                operand_types.reverse();
                expected_types.extend(operand_types);
            }
            (TE::Enter, DynAstRef::Unquote(unq)) => {
                let result_ty;
                let mut operand_types = vec![];
                {
                    let mut scope = context.enter_operation_scope();
                    result_ty = unq.operator.result_type(&mut *scope, unq.span);
                    unq.operator
                        .immediate_types(&mut *scope, unq.span, &mut operand_types);
                    unq.operator
                        .param_types(&mut *scope, unq.span, &mut operand_types);
                }

                if unq.operands.len() != operand_types.len() {
                    return Err(WastError::new(
                        unq.span,
                        format!(
                            "Expected {} unquote operands but found {}",
                            operand_types.len(),
                            unq.operands.len()
                        ),
                    )
                    .into());
                }

                for operand in &unq.operands {
                    match operand {
                        Rhs::ValueLiteral(_) | Rhs::Constant(_) => continue,
                        Rhs::Variable(_) | Rhs::Unquote(_) | Rhs::Operation(_) => {
                            return Err(WastError::new(
                                operand.span(),
                                "unquote operands must be value literals or constants".into(),
                            )
                            .into());
                        }
                    }
                }

                context.assert_type_eq(unq.span, expected_types.last().unwrap(), &result_ty, None);

                operand_types.reverse();
                expected_types.extend(operand_types);
            }
            (TE::Exit, DynAstRef::Rhs(..)) => {
                expected_types.pop().unwrap();
            }
            _ => continue,
        }
    }

    // Again, we should have popped off all the expected types when exiting
    // `Rhs` nodes in the traversal.
    assert!(expected_types.is_empty());

    Ok(())
}

fn type_constrain_precondition<'a>(
    context: &mut TypingContext<'a>,
    pre: &Precondition<'a>,
) -> VerifyResult<()> {
    match pre.constraint {
        Constraint::BitWidth => {
            if pre.operands.len() != 2 {
                return Err(WastError::new(
                    pre.span,
                    format!(
                        "the `bit-width` precondition requires exactly 2 operands, found \
                         {} operands",
                        pre.operands.len(),
                    ),
                )
                .into());
            }

            let id = match pre.operands[0] {
                ConstraintOperand::ValueLiteral(_) => {
                    return Err(anyhow::anyhow!(
                        "the `bit-width` precondition requires a constant or variable as \
                     its first operand"
                    )
                    .into())
                }
                ConstraintOperand::Constant(Constant { id, .. })
                | ConstraintOperand::Variable(Variable { id, .. }) => id,
            };

            let width = match pre.operands[1] {
                ConstraintOperand::ValueLiteral(ValueLiteral::Integer(Integer {
                    value, ..
                })) if value == 1
                    || value == 8
                    || value == 16
                    || value == 32
                    || value == 64
                    || value == 128 =>
                {
                    value as u8
                }
                ref op => return Err(WastError::new(
                    op.span(),
                    "the `bit-width` precondition requires a bit width of 1, 8, 16, 32, 64, or \
                     128"
                    .into(),
                )
                .into()),
            };

            let ty = context.get_type_var_for_id(id)?;
            context.assert_bit_width(pre.span, &ty, width);
            Ok(())
        }
        Constraint::IsPowerOfTwo => {
            if pre.operands.len() != 1 {
                return Err(WastError::new(
                    pre.span,
                    format!(
                        "the `is-power-of-two` precondition requires exactly 1 operand, found \
                         {} operands",
                        pre.operands.len(),
                    ),
                )
                .into());
            }
            match &pre.operands[0] {
                ConstraintOperand::Constant(Constant { id, .. }) => {
                    let ty = context.get_type_var_for_id(*id)?;
                    context.assert_is_integer(pre.span(), &ty);
                    Ok(())
                }
                op => Err(WastError::new(
                    op.span(),
                    "`is-power-of-two` operands must be constant bindings".into(),
                )
                .into()),
            }
        }
        Constraint::FitsInNativeWord => {
            if pre.operands.len() != 1 {
                return Err(WastError::new(
                    pre.span,
                    format!(
                        "the `fits-in-native-word` precondition requires exactly 1 operand, found \
                         {} operands",
                        pre.operands.len(),
                    ),
                )
                .into());
            }

            match pre.operands[0] {
                ConstraintOperand::ValueLiteral(_) => {
                    return Err(anyhow::anyhow!(
                        "the `fits-in-native-word` precondition requires a constant or variable as \
                         its first operand"
                    )
                    .into())
                }
                ConstraintOperand::Constant(Constant { id, .. })
                | ConstraintOperand::Variable(Variable { id, .. }) => {
                    context.get_type_var_for_id(id)?;
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! verify_ok {
        ($name:ident, $src:expr) => {
            #[test]
            fn $name() {
                let buf = wast::parser::ParseBuffer::new($src).expect("should lex OK");
                let opts = match wast::parser::parse::<Optimizations>(&buf) {
                    Ok(opts) => opts,
                    Err(mut e) => {
                        e.set_path(Path::new(stringify!($name)));
                        e.set_text($src);
                        eprintln!("{}", e);
                        panic!("should parse OK")
                    }
                };
                match verify(&opts) {
                    Ok(_) => return,
                    Err(mut e) => {
                        e.set_path(Path::new(stringify!($name)));
                        e.set_text($src);
                        eprintln!("{}", e);
                        panic!("should verify OK")
                    }
                }
            }
        };
    }

    macro_rules! verify_err {
        ($name:ident, $src:expr) => {
            #[test]
            fn $name() {
                let buf = wast::parser::ParseBuffer::new($src).expect("should lex OK");
                let opts = match wast::parser::parse::<Optimizations>(&buf) {
                    Ok(opts) => opts,
                    Err(mut e) => {
                        e.set_path(Path::new(stringify!($name)));
                        e.set_text($src);
                        eprintln!("{}", e);
                        panic!("should parse OK")
                    }
                };
                match verify(&opts) {
                    Ok(_) => panic!("expected a verification error, but it verified OK"),
                    Err(mut e) => {
                        e.set_path(Path::new(stringify!($name)));
                        e.set_text($src);
                        eprintln!("{}", e);
                        return;
                    }
                }
            }
        };
    }

    verify_ok!(bool_0, "(=> true true)");
    verify_ok!(bool_1, "(=> false false)");
    verify_ok!(bool_2, "(=> true false)");
    verify_ok!(bool_3, "(=> false true)");

    verify_err!(bool_is_not_int_0, "(=> true 42)");
    verify_err!(bool_is_not_int_1, "(=> 42 true)");

    verify_ok!(
        bit_width_0,
        "
(=> (when (iadd $x $y)
          (bit-width $x 32)
          (bit-width $y 32))
    (iadd $x $y))
"
    );
    verify_err!(
        bit_width_1,
        "
(=> (when (iadd $x $y)
          (bit-width $x 32)
          (bit-width $y 64))
    (iadd $x $y))
"
    );
    verify_err!(
        bit_width_2,
        "
(=> (when (iconst $C)
          (bit-width $C))
    5)
"
    );
    verify_err!(
        bit_width_3,
        "
(=> (when (iconst $C)
          (bit-width 32 32))
    5)
"
    );
    verify_err!(
        bit_width_4,
        "
(=> (when (iconst $C)
          (bit-width $C $C))
    5)
"
    );
    verify_err!(
        bit_width_5,
        "
(=> (when (iconst $C)
          (bit-width $C2 32))
    5)
"
    );
    verify_err!(
        bit_width_6,
        "
(=> (when (iconst $C)
          (bit-width $C2 33))
    5)
"
    );

    verify_ok!(
        is_power_of_two_0,
        "
(=> (when (imul $x $C)
          (is-power-of-two $C))
    (ishl $x $(log2 $C)))
"
    );
    verify_err!(
        is_power_of_two_1,
        "
(=> (when (imul $x $C)
          (is-power-of-two))
    5)
"
    );
    verify_err!(
        is_power_of_two_2,
        "
(=> (when (imul $x $C)
          (is-power-of-two $C $C))
    5)
"
    );

    verify_ok!(pattern_ops_0, "(=> (iadd $x $C) 5)");
    verify_err!(pattern_ops_1, "(=> (iadd $x) 5)");
    verify_err!(pattern_ops_2, "(=> (iadd $x $y $z) 5)");

    verify_ok!(unquote_0, "(=> $C $(log2 $C))");
    verify_err!(unquote_1, "(=> (iadd $C $D) $(log2 $C $D))");
    verify_err!(unquote_2, "(=> $x $(log2))");
    verify_ok!(unquote_3, "(=> $C $(neg $C))");
    verify_err!(unquote_4, "(=> $x $(neg))");
    verify_err!(unquote_5, "(=> (iadd $x $y) $(neg $x $y))");
    verify_err!(unquote_6, "(=> $x $(neg $x))");

    verify_ok!(rhs_0, "(=> $x (iadd $x (iconst 0)))");
    verify_err!(rhs_1, "(=> $x (iadd $x))");
    verify_err!(rhs_2, "(=> $x (iadd $x 0 0))");

    verify_err!(no_optimizations, "");

    verify_err!(
        duplicate_left_hand_sides,
        "
(=> (iadd $x $y) 0)
(=> (iadd $x $y) 1)
"
    );
    verify_err!(
        canonically_duplicate_left_hand_sides_0,
        "
(=> (iadd $x $y) 0)
(=> (iadd $y $x) 1)
"
    );
    verify_err!(
        canonically_duplicate_left_hand_sides_1,
        "
(=> (iadd $X $Y) 0)
(=> (iadd $Y $X) 1)
"
    );
    verify_err!(
        canonically_duplicate_left_hand_sides_2,
        "
(=> (iadd $x $x) 0)
(=> (iadd $y $y) 1)
"
    );

    verify_ok!(
        canonically_different_left_hand_sides_0,
        "
(=> (iadd $x $C) 0)
(=> (iadd $C $x) 1)
"
    );
    verify_ok!(
        canonically_different_left_hand_sides_1,
        "
(=> (iadd $x $x) 0)
(=> (iadd $x $y) 1)
"
    );

    verify_ok!(
        fits_in_native_word_0,
        "(=> (when (iadd $x $y) (fits-in-native-word $x)) 0)"
    );
    verify_err!(
        fits_in_native_word_1,
        "(=> (when (iadd $x $y) (fits-in-native-word)) 0)"
    );
    verify_err!(
        fits_in_native_word_2,
        "(=> (when (iadd $x $y) (fits-in-native-word $x $y)) 0)"
    );
    verify_err!(
        fits_in_native_word_3,
        "(=> (when (iadd $x $y) (fits-in-native-word true)) 0)"
    );

    verify_err!(reduce_extend_0, "(=> (sextend (ireduce -1)) 0)");
    verify_err!(reduce_extend_1, "(=> (uextend (ireduce -1)) 0)");
    verify_ok!(reduce_extend_2, "(=> (sextend{i64} (ireduce{i32} -1)) 0)");
    verify_ok!(reduce_extend_3, "(=> (uextend{i64} (ireduce{i32} -1)) 0)");
    verify_err!(reduce_extend_4, "(=> (sextend{i64} (ireduce{i64} -1)) 0)");
    verify_err!(reduce_extend_5, "(=> (uextend{i64} (ireduce{i64} -1)) 0)");
    verify_err!(reduce_extend_6, "(=> (sextend{i32} (ireduce{i64} -1)) 0)");
    verify_err!(reduce_extend_7, "(=> (uextend{i32} (ireduce{i64} -1)) 0)");

    verify_err!(
        using_an_operation_as_an_immediate_in_lhs,
        "(=> (iadd_imm (imul $x $y) $z) 0)"
    );
    verify_err!(
        using_an_operation_as_an_immediate_in_rhs,
        "(=> (iadd (imul $x $y) $z) (iadd_imm (imul $x $y) $z))"
    );

    verify_err!(
        using_a_condition_code_as_the_root_of_an_optimization,
        "(=> eq eq)"
    );
}

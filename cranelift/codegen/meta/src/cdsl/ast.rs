use crate::cdsl::formats::FormatRegistry;
use crate::cdsl::instructions::{InstSpec, Instruction, InstructionPredicate};
use crate::cdsl::operands::{OperandKind, OperandKindFields};
use crate::cdsl::types::ValueType;
use crate::cdsl::typevar::{TypeSetBuilder, TypeVar};

use cranelift_entity::{entity_impl, PrimaryMap};

use std::fmt;

pub enum Expr {
    Var(VarIndex),
    Literal(Literal),
    Apply(Apply),
}

impl Expr {
    pub fn maybe_literal(&self) -> Option<&Literal> {
        match &self {
            Expr::Literal(lit) => Some(lit),
            _ => None,
        }
    }

    pub fn maybe_var(&self) -> Option<VarIndex> {
        if let Expr::Var(var) = &self {
            Some(*var)
        } else {
            None
        }
    }

    pub fn unwrap_var(&self) -> VarIndex {
        self.maybe_var()
            .expect("tried to unwrap a non-Var content in Expr::unwrap_var")
    }

    pub fn to_rust_code(&self, var_pool: &VarPool) -> String {
        match self {
            Expr::Var(var_index) => var_pool.get(*var_index).to_rust_code(),
            Expr::Literal(literal) => literal.to_rust_code(),
            Expr::Apply(a) => a.to_rust_code(var_pool),
        }
    }
}

/// An AST definition associates a set of variables with the values produced by an expression.
pub struct Def {
    pub apply: Apply,
    pub defined_vars: Vec<VarIndex>,
}

impl Def {
    pub fn to_comment_string(&self, var_pool: &VarPool) -> String {
        let results = self
            .defined_vars
            .iter()
            .map(|&x| var_pool.get(x).name)
            .collect::<Vec<_>>();

        let results = if results.len() == 1 {
            results[0].to_string()
        } else {
            format!("({})", results.join(", "))
        };

        format!("{} << {}", results, self.apply.to_comment_string(var_pool))
    }
}

pub struct DefPool {
    pool: PrimaryMap<DefIndex, Def>,
}

impl DefPool {
    pub fn new() -> Self {
        Self {
            pool: PrimaryMap::new(),
        }
    }
    pub fn get(&self, index: DefIndex) -> &Def {
        self.pool.get(index).unwrap()
    }
    pub fn get_mut(&mut self, index: DefIndex) -> &mut Def {
        self.pool.get_mut(index).unwrap()
    }
    pub fn next_index(&self) -> DefIndex {
        self.pool.next_key()
    }
    pub fn create(&mut self, apply: Apply, defined_vars: Vec<VarIndex>) -> DefIndex {
        self.pool.push(Def {
            apply,
            defined_vars,
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefIndex(u32);
entity_impl!(DefIndex);

#[derive(Debug, Clone)]
enum LiteralValue {
    /// A value of an enumerated immediate operand.
    ///
    /// Some immediate operand kinds like `intcc` and `floatcc` have an enumerated range of values
    /// corresponding to a Rust enum type. An `Enumerator` object is an AST leaf node representing one
    /// of the values.
    Enumerator(&'static str),

    /// A bitwise value of an immediate operand, used for bitwise exact floating point constants.
    Bits(u64),

    /// A value of an integer immediate operand.
    Int(i64),
}

#[derive(Clone)]
pub struct Literal {
    kind: OperandKind,
    value: LiteralValue,
}

impl fmt::Debug for Literal {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt,
            "Literal(kind={}, value={:?})",
            self.kind.name, self.value
        )
    }
}

impl Literal {
    pub fn enumerator_for(kind: &OperandKind, value: &'static str) -> Self {
        if let OperandKindFields::ImmEnum(values) = &kind.fields {
            assert!(
                values.get(value).is_some(),
                format!(
                    "nonexistent value '{}' in enumeration '{}'",
                    value, kind.name
                )
            );
        } else {
            panic!("enumerator is for enum values");
        }
        Self {
            kind: kind.clone(),
            value: LiteralValue::Enumerator(value),
        }
    }

    pub fn bits(kind: &OperandKind, bits: u64) -> Self {
        match kind.fields {
            OperandKindFields::ImmValue => {}
            _ => panic!("bits_of is for immediate scalar types"),
        }
        Self {
            kind: kind.clone(),
            value: LiteralValue::Bits(bits),
        }
    }

    pub fn constant(kind: &OperandKind, value: i64) -> Self {
        match kind.fields {
            OperandKindFields::ImmValue => {}
            _ => panic!("bits_of is for immediate scalar types"),
        }
        Self {
            kind: kind.clone(),
            value: LiteralValue::Int(value),
        }
    }

    pub fn to_rust_code(&self) -> String {
        let maybe_values = match &self.kind.fields {
            OperandKindFields::ImmEnum(values) => Some(values),
            OperandKindFields::ImmValue => None,
            _ => panic!("impossible per construction"),
        };

        match self.value {
            LiteralValue::Enumerator(value) => {
                format!("{}::{}", self.kind.rust_type, maybe_values.unwrap()[value])
            }
            LiteralValue::Bits(bits) => format!("{}::with_bits({:#x})", self.kind.rust_type, bits),
            LiteralValue::Int(val) => val.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PatternPosition {
    Source,
    Destination,
}

/// A free variable.
///
/// When variables are used in `XForms` with source and destination patterns, they are classified
/// as follows:
///
/// Input values: Uses in the source pattern with no preceding def. These may appear as inputs in
/// the destination pattern too, but no new inputs can be introduced.
///
/// Output values: Variables that are defined in both the source and destination pattern.  These
/// values may have uses outside the source pattern, and the destination pattern must compute the
/// same value.
///
/// Intermediate values: Values that are defined in the source pattern, but not in the destination
/// pattern. These may have uses outside the source pattern, so the defining instruction can't be
/// deleted immediately.
///
/// Temporary values are defined only in the destination pattern.
pub struct Var {
    pub name: &'static str,

    /// The `Def` defining this variable in a source pattern.
    pub src_def: Option<DefIndex>,

    /// The `Def` defining this variable in a destination pattern.
    pub dst_def: Option<DefIndex>,

    /// TypeVar representing the type of this variable.
    type_var: Option<TypeVar>,

    /// Is this the original type variable, or has it be redefined with set_typevar?
    is_original_type_var: bool,
}

impl Var {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            src_def: None,
            dst_def: None,
            type_var: None,
            is_original_type_var: false,
        }
    }

    /// Is this an input value to the src pattern?
    pub fn is_input(&self) -> bool {
        self.src_def.is_none() && self.dst_def.is_none()
    }

    /// Is this an output value, defined in both src and dst patterns?
    pub fn is_output(&self) -> bool {
        self.src_def.is_some() && self.dst_def.is_some()
    }

    /// Is this an intermediate value, defined only in the src pattern?
    pub fn is_intermediate(&self) -> bool {
        self.src_def.is_some() && self.dst_def.is_none()
    }

    /// Is this a temp value, defined only in the dst pattern?
    pub fn is_temp(&self) -> bool {
        self.src_def.is_none() && self.dst_def.is_some()
    }

    /// Get the def of this variable according to the position.
    pub fn get_def(&self, position: PatternPosition) -> Option<DefIndex> {
        match position {
            PatternPosition::Source => self.src_def,
            PatternPosition::Destination => self.dst_def,
        }
    }

    pub fn set_def(&mut self, position: PatternPosition, def: DefIndex) {
        assert!(
            self.get_def(position).is_none(),
            format!("redefinition of variable {}", self.name)
        );
        match position {
            PatternPosition::Source => {
                self.src_def = Some(def);
            }
            PatternPosition::Destination => {
                self.dst_def = Some(def);
            }
        }
    }

    /// Get the type variable representing the type of this variable.
    pub fn get_or_create_typevar(&mut self) -> TypeVar {
        match &self.type_var {
            Some(tv) => tv.clone(),
            None => {
                // Create a new type var in which we allow all types.
                let tv = TypeVar::new(
                    format!("typeof_{}", self.name),
                    format!("Type of the pattern variable {:?}", self),
                    TypeSetBuilder::all(),
                );
                self.type_var = Some(tv.clone());
                self.is_original_type_var = true;
                tv
            }
        }
    }
    pub fn get_typevar(&self) -> Option<TypeVar> {
        self.type_var.clone()
    }
    pub fn set_typevar(&mut self, tv: TypeVar) {
        self.is_original_type_var = if let Some(previous_tv) = &self.type_var {
            *previous_tv == tv
        } else {
            false
        };
        self.type_var = Some(tv);
    }

    /// Check if this variable has a free type variable. If not, the type of this variable is
    /// computed from the type of another variable.
    pub fn has_free_typevar(&self) -> bool {
        match &self.type_var {
            Some(tv) => tv.base.is_none() && self.is_original_type_var,
            None => false,
        }
    }

    pub fn to_rust_code(&self) -> String {
        self.name.into()
    }
    fn rust_type(&self) -> String {
        self.type_var.as_ref().unwrap().to_rust_code()
    }
}

impl fmt::Debug for Var {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_fmt(format_args!(
            "Var({}{}{})",
            self.name,
            if self.src_def.is_some() { ", src" } else { "" },
            if self.dst_def.is_some() { ", dst" } else { "" }
        ))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VarIndex(u32);
entity_impl!(VarIndex);

pub struct VarPool {
    pool: PrimaryMap<VarIndex, Var>,
}

impl VarPool {
    pub fn new() -> Self {
        Self {
            pool: PrimaryMap::new(),
        }
    }
    pub fn get(&self, index: VarIndex) -> &Var {
        self.pool.get(index).unwrap()
    }
    pub fn get_mut(&mut self, index: VarIndex) -> &mut Var {
        self.pool.get_mut(index).unwrap()
    }
    pub fn create(&mut self, name: &'static str) -> VarIndex {
        self.pool.push(Var::new(name))
    }
}

/// Apply an instruction to arguments.
///
/// An `Apply` AST expression is created by using function call syntax on instructions. This
/// applies to both bound and unbound polymorphic instructions.
pub struct Apply {
    pub inst: Instruction,
    pub args: Vec<Expr>,
    pub value_types: Vec<ValueType>,
}

impl Apply {
    pub fn new(target: InstSpec, args: Vec<Expr>) -> Self {
        let (inst, value_types) = match target {
            InstSpec::Inst(inst) => (inst, Vec::new()),
            InstSpec::Bound(bound_inst) => (bound_inst.inst, bound_inst.value_types),
        };

        // Apply should only operate on concrete value types, not "any".
        let value_types = value_types
            .into_iter()
            .map(|vt| vt.expect("shouldn't be Any"))
            .collect();

        // Basic check on number of arguments.
        assert!(
            inst.operands_in.len() == args.len(),
            format!("incorrect number of arguments in instruction {}", inst.name)
        );

        // Check that the kinds of Literals arguments match the expected operand.
        for &imm_index in &inst.imm_opnums {
            let arg = &args[imm_index];
            if let Some(literal) = arg.maybe_literal() {
                let op = &inst.operands_in[imm_index];
                assert!(
                    op.kind.name == literal.kind.name,
                    format!(
                        "Passing literal of kind {} to field of wrong kind {}",
                        literal.kind.name, op.kind.name
                    )
                );
            }
        }

        Self {
            inst,
            args,
            value_types,
        }
    }

    fn to_comment_string(&self, var_pool: &VarPool) -> String {
        let args = self
            .args
            .iter()
            .map(|arg| arg.to_rust_code(var_pool))
            .collect::<Vec<_>>()
            .join(", ");

        let mut inst_and_bound_types = vec![self.inst.name.to_string()];
        inst_and_bound_types.extend(self.value_types.iter().map(|vt| vt.to_string()));
        let inst_name = inst_and_bound_types.join(".");

        format!("{}({})", inst_name, args)
    }

    fn to_rust_code(&self, var_pool: &VarPool) -> String {
        let args = self
            .args
            .iter()
            .map(|arg| arg.to_rust_code(var_pool))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})", self.inst.name, args)
    }

    pub fn inst_predicate(
        &self,
        format_registry: &FormatRegistry,
        var_pool: &VarPool,
    ) -> InstructionPredicate {
        let iform = format_registry.get(self.inst.format);

        let mut pred = InstructionPredicate::new();
        for (format_field, &op_num) in iform.imm_fields.iter().zip(self.inst.imm_opnums.iter()) {
            let arg = &self.args[op_num];
            if arg.maybe_var().is_some() {
                // Ignore free variables for now.
                continue;
            }
            pred = pred.and(InstructionPredicate::new_is_field_equal_ast(
                iform,
                &format_field,
                arg.to_rust_code(var_pool),
            ));
        }

        // Add checks for any bound secondary type variables.  We can't check the controlling type
        // variable this way since it may not appear as the type of an operand.
        if self.value_types.len() > 1 {
            let poly = self
                .inst
                .polymorphic_info
                .as_ref()
                .expect("must have polymorphic info if it has bounded types");
            for (bound_type, type_var) in
                self.value_types[1..].iter().zip(poly.other_typevars.iter())
            {
                pred = pred.and(InstructionPredicate::new_typevar_check(
                    &self.inst, type_var, bound_type,
                ));
            }
        }

        pred
    }

    /// Same as `inst_predicate()`, but also check the controlling type variable.
    pub fn inst_predicate_with_ctrl_typevar(
        &self,
        format_registry: &FormatRegistry,
        var_pool: &VarPool,
    ) -> InstructionPredicate {
        let mut pred = self.inst_predicate(format_registry, var_pool);

        if !self.value_types.is_empty() {
            let bound_type = &self.value_types[0];
            let poly = self.inst.polymorphic_info.as_ref().unwrap();
            let type_check = if poly.use_typevar_operand {
                InstructionPredicate::new_typevar_check(&self.inst, &poly.ctrl_typevar, bound_type)
            } else {
                InstructionPredicate::new_ctrl_typevar_check(&bound_type)
            };
            pred = pred.and(type_check);
        }

        pred
    }

    pub fn rust_builder(&self, defined_vars: &Vec<VarIndex>, var_pool: &VarPool) -> String {
        let mut args = self
            .args
            .iter()
            .map(|expr| expr.to_rust_code(var_pool))
            .collect::<Vec<_>>()
            .join(", ");

        // Do we need to pass an explicit type argument?
        if let Some(poly) = &self.inst.polymorphic_info {
            if !poly.use_typevar_operand {
                args = format!("{}, {}", var_pool.get(defined_vars[0]).rust_type(), args);
            }
        }

        format!("{}({})", self.inst.snake_name(), args)
    }
}

// Simple helpers for legalize actions construction.

pub enum DummyExpr {
    Var(DummyVar),
    Literal(Literal),
    Apply(InstSpec, Vec<DummyExpr>),
}

#[derive(Clone)]
pub struct DummyVar {
    pub name: &'static str,
}

impl Into<DummyExpr> for DummyVar {
    fn into(self) -> DummyExpr {
        DummyExpr::Var(self)
    }
}
impl Into<DummyExpr> for Literal {
    fn into(self) -> DummyExpr {
        DummyExpr::Literal(self)
    }
}

pub fn var(name: &'static str) -> DummyVar {
    DummyVar { name }
}

pub struct DummyDef {
    pub expr: DummyExpr,
    pub defined_vars: Vec<DummyVar>,
}

pub struct ExprBuilder {
    expr: DummyExpr,
}

impl ExprBuilder {
    pub fn apply(inst: InstSpec, args: Vec<DummyExpr>) -> Self {
        let expr = DummyExpr::Apply(inst, args);
        Self { expr }
    }

    pub fn assign_to(self, defined_vars: Vec<DummyVar>) -> DummyDef {
        DummyDef {
            expr: self.expr,
            defined_vars,
        }
    }
}

macro_rules! def_rhs {
    // inst(a, b, c)
    ($inst:ident($($src:expr),*)) => {
        ExprBuilder::apply($inst.into(), vec![$($src.clone().into()),*])
    };

    // inst.type(a, b, c)
    ($inst:ident.$type:ident($($src:expr),*)) => {
        ExprBuilder::apply($inst.bind($type).into(), vec![$($src.clone().into()),*])
    };
}

// Helper macro to define legalization recipes.
macro_rules! def {
    // x = ...
    ($dest:ident = $($tt:tt)*) => {
        def_rhs!($($tt)*).assign_to(vec![$dest.clone()])
    };

    // (x, y, ...) = ...
    (($($dest:ident),*) = $($tt:tt)*) => {
        def_rhs!($($tt)*).assign_to(vec![$($dest.clone()),*])
    };

    // An instruction with no results.
    ($($tt:tt)*) => {
        def_rhs!($($tt)*).assign_to(Vec::new())
    }
}

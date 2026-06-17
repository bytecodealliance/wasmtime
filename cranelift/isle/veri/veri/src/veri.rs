use crate::{
    expand::{Constrain, Expansion},
    program::Program,
    spec::{self, Arm, Constructor, Signature, State},
    trie::{BindingType, binding_type},
    types::{Compound, Const, Type, Variant, Width, field_name_by_index},
};
use anyhow::{Context, Error, Result, bail, format_err};
use cranelift_isle::{
    ast::Ident,
    lexer::Pos,
    sema::{self, Sym, TermId, TypeId, VariantId},
    trie_again::{Binding, BindingId, Constraint, TupleIndex},
};
use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    iter::zip,
};

declare_id!(
    /// The id of an expression within verification Conditions.
    #[must_use]
    ExprId
);

declare_id!(
    /// The id of a variable within verification Conditions.
    VariableId
);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Expr {
    // Terminals.
    Const(Const),
    Variable(VariableId),

    // Boolean.
    Not(ExprId),
    And(ExprId, ExprId),
    Or(ExprId, ExprId),
    Imp(ExprId, ExprId),
    Eq(ExprId, ExprId),
    Lt(ExprId, ExprId),
    Lte(ExprId, ExprId),

    BVUgt(ExprId, ExprId),
    BVUge(ExprId, ExprId),
    BVUlt(ExprId, ExprId),
    BVUle(ExprId, ExprId),

    BVSgt(ExprId, ExprId),
    BVSge(ExprId, ExprId),
    BVSlt(ExprId, ExprId),
    BVSle(ExprId, ExprId),

    BVSaddo(ExprId, ExprId),

    // Unary.
    BVNot(ExprId),
    BVNeg(ExprId),
    Cls(ExprId),
    Clz(ExprId),
    Rev(ExprId),
    Popcnt(ExprId),

    // Binary.
    Add(ExprId, ExprId),
    Sub(ExprId, ExprId),
    Mul(ExprId, ExprId),
    BVAdd(ExprId, ExprId),
    BVSub(ExprId, ExprId),
    BVMul(ExprId, ExprId),
    BVSDiv(ExprId, ExprId),
    BVUDiv(ExprId, ExprId),
    BVSRem(ExprId, ExprId),
    BVURem(ExprId, ExprId),
    BVAnd(ExprId, ExprId),
    BVOr(ExprId, ExprId),
    BVXor(ExprId, ExprId),
    BVShl(ExprId, ExprId),
    BVLShr(ExprId, ExprId),
    BVAShr(ExprId, ExprId),
    BVRotl(ExprId, ExprId),
    BVRotr(ExprId, ExprId),

    // ITE
    Conditional(ExprId, ExprId, ExprId),

    // Bitwidth conversion.
    BVZeroExt(ExprId, ExprId),
    BVSignExt(ExprId, ExprId),
    BVConvTo(ExprId, ExprId),

    // Extract specified bit range.
    BVExtract(usize, usize, ExprId),

    // Concatenate bitvectors.
    BVConcat(ExprId, ExprId),

    // Integer conversion.
    Int2BV(ExprId, ExprId),
    BV2Nat(ExprId),

    // Bitwidth.
    WidthOf(ExprId),

    // Floating point conversion.
    ToFP(ExprId, ExprId),
    ToFPUnsigned(ExprId, ExprId),
    ToFPFromFP(ExprId, ExprId),
    FPToUBV(ExprId, ExprId),
    FPToSBV(ExprId, ExprId),

    // Floating point.
    FPPositiveInfinity(ExprId),
    FPNegativeInfinity(ExprId),
    FPPositiveZero(ExprId),
    FPNegativeZero(ExprId),
    FPNaN(ExprId),
    FPEq(ExprId, ExprId),
    FPNe(ExprId, ExprId),
    FPLt(ExprId, ExprId),
    FPGt(ExprId, ExprId),
    FPLe(ExprId, ExprId),
    FPGe(ExprId, ExprId),
    FPAdd(ExprId, ExprId),
    FPSub(ExprId, ExprId),
    FPMul(ExprId, ExprId),
    FPDiv(ExprId, ExprId),
    FPMin(ExprId, ExprId),
    FPMax(ExprId, ExprId),
    FPNeg(ExprId),
    FPCeil(ExprId),
    FPFloor(ExprId),
    FPSqrt(ExprId),
    FPTrunc(ExprId),
    FPNearest(ExprId),
    FPIsZero(ExprId),
    FPIsInfinite(ExprId),
    FPIsNaN(ExprId),
    FPIsNegative(ExprId),
    FPIsPositive(ExprId),
}

impl Expr {
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable(_))
    }

    pub fn pure(&self) -> bool {
        !matches!(self, Expr::BVConvTo(..))
    }

    pub fn sources(&self) -> Vec<ExprId> {
        match self {
            Expr::Const(_) | Expr::Variable(_) => Vec::new(),
            // Unary
            Expr::Not(x)
            | Expr::BVNot(x)
            | Expr::BVNeg(x)
            | Expr::BVExtract(_, _, x)
            | Expr::BV2Nat(x)
            | Expr::Cls(x)
            | Expr::Clz(x)
            | Expr::Rev(x)
            | Expr::Popcnt(x)
            | Expr::WidthOf(x)
            | Expr::FPPositiveInfinity(x)
            | Expr::FPNegativeInfinity(x)
            | Expr::FPPositiveZero(x)
            | Expr::FPNegativeZero(x)
            | Expr::FPNaN(x)
            | Expr::FPNeg(x)
            | Expr::FPCeil(x)
            | Expr::FPFloor(x)
            | Expr::FPSqrt(x)
            | Expr::FPTrunc(x)
            | Expr::FPNearest(x)
            | Expr::FPIsZero(x)
            | Expr::FPIsInfinite(x)
            | Expr::FPIsNaN(x)
            | Expr::FPIsNegative(x)
            | Expr::FPIsPositive(x) => vec![*x],

            // Binary
            Expr::And(x, y)
            | Expr::Or(x, y)
            | Expr::Imp(x, y)
            | Expr::Eq(x, y)
            | Expr::Lt(x, y)
            | Expr::Lte(x, y)
            | Expr::Add(x, y)
            | Expr::Sub(x, y)
            | Expr::Mul(x, y)
            | Expr::BVUgt(x, y)
            | Expr::BVUge(x, y)
            | Expr::BVUlt(x, y)
            | Expr::BVUle(x, y)
            | Expr::BVSgt(x, y)
            | Expr::BVSge(x, y)
            | Expr::BVSlt(x, y)
            | Expr::BVSle(x, y)
            | Expr::BVSaddo(x, y)
            | Expr::BVAdd(x, y)
            | Expr::BVSub(x, y)
            | Expr::BVMul(x, y)
            | Expr::BVSDiv(x, y)
            | Expr::BVUDiv(x, y)
            | Expr::BVSRem(x, y)
            | Expr::BVURem(x, y)
            | Expr::BVAnd(x, y)
            | Expr::BVOr(x, y)
            | Expr::BVXor(x, y)
            | Expr::BVShl(x, y)
            | Expr::BVLShr(x, y)
            | Expr::BVAShr(x, y)
            | Expr::BVRotl(x, y)
            | Expr::BVRotr(x, y)
            | Expr::BVZeroExt(x, y)
            | Expr::BVSignExt(x, y)
            | Expr::BVConvTo(x, y)
            | Expr::Int2BV(x, y)
            | Expr::ToFP(x, y)
            | Expr::ToFPUnsigned(x, y)
            | Expr::ToFPFromFP(x, y)
            | Expr::FPToUBV(x, y)
            | Expr::FPToSBV(x, y)
            | Expr::BVConcat(x, y)
            | Expr::FPEq(x, y)
            | Expr::FPNe(x, y)
            | Expr::FPLt(x, y)
            | Expr::FPGt(x, y)
            | Expr::FPLe(x, y)
            | Expr::FPGe(x, y)
            | Expr::FPAdd(x, y)
            | Expr::FPSub(x, y)
            | Expr::FPMul(x, y)
            | Expr::FPDiv(x, y)
            | Expr::FPMin(x, y)
            | Expr::FPMax(x, y) => vec![*x, *y],

            // Ternary
            Expr::Conditional(c, t, e) => vec![*c, *t, *e],
        }
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Expr::Const(c) => write!(f, "const({c})"),
            Expr::Variable(v) => write!(f, "var({})", v.index()),
            Expr::Not(x) => write!(f, "!{}", x.index()),
            Expr::And(x, y) => write!(f, "{} && {}", x.index(), y.index()),
            Expr::Or(x, y) => write!(f, "{} || {}", x.index(), y.index()),
            Expr::Imp(x, y) => write!(f, "{} => {}", x.index(), y.index()),
            Expr::Eq(x, y) => write!(f, "{} == {}", x.index(), y.index()),
            Expr::Lt(x, y) => write!(f, "{} < {}", x.index(), y.index()),
            Expr::Lte(x, y) => write!(f, "{} <= {}", x.index(), y.index()),
            Expr::Add(x, y) => write!(f, "{} + {}", x.index(), y.index()),
            Expr::Sub(x, y) => write!(f, "{} - {}", x.index(), y.index()),
            Expr::Mul(x, y) => write!(f, "{} * {}", x.index(), y.index()),
            Expr::BVUgt(x, y) => write!(f, "bvugt({}, {})", x.index(), y.index()),
            Expr::BVUge(x, y) => write!(f, "bvuge({}, {})", x.index(), y.index()),
            Expr::BVUlt(x, y) => write!(f, "bvult({}, {})", x.index(), y.index()),
            Expr::BVUle(x, y) => write!(f, "bvule({}, {})", x.index(), y.index()),
            Expr::BVSgt(x, y) => write!(f, "bvsgt({}, {})", x.index(), y.index()),
            Expr::BVSge(x, y) => write!(f, "bvsge({}, {})", x.index(), y.index()),
            Expr::BVSlt(x, y) => write!(f, "bvslt({}, {})", x.index(), y.index()),
            Expr::BVSle(x, y) => write!(f, "bvsle({}, {})", x.index(), y.index()),
            Expr::BVSaddo(x, y) => write!(f, "bvsaddo({}, {})", x.index(), y.index()),
            Expr::BVNot(x) => write!(f, "bvnot({})", x.index()),
            Expr::BVNeg(x) => write!(f, "bvneg({})", x.index()),
            Expr::Cls(x) => write!(f, "cls({})", x.index()),
            Expr::Clz(x) => write!(f, "clz({})", x.index()),
            Expr::Rev(x) => write!(f, "rev({})", x.index()),
            Expr::Popcnt(x) => write!(f, "popcnt({})", x.index()),
            Expr::BVAdd(x, y) => write!(f, "bvadd({}, {})", x.index(), y.index()),
            Expr::BVSub(x, y) => write!(f, "bvsub({}, {})", x.index(), y.index()),
            Expr::BVMul(x, y) => write!(f, "bvmul({}, {})", x.index(), y.index()),
            Expr::BVSDiv(x, y) => write!(f, "bvsdiv({}, {})", x.index(), y.index()),
            Expr::BVUDiv(x, y) => write!(f, "bvudiv({}, {})", x.index(), y.index()),
            Expr::BVSRem(x, y) => write!(f, "bvsrem({}, {})", x.index(), y.index()),
            Expr::BVURem(x, y) => write!(f, "bvurem({}, {})", x.index(), y.index()),
            Expr::BVAnd(x, y) => write!(f, "bvand({}, {})", x.index(), y.index()),
            Expr::BVOr(x, y) => write!(f, "bvor({}, {})", x.index(), y.index()),
            Expr::BVXor(x, y) => write!(f, "bvxor({}, {})", x.index(), y.index()),
            Expr::BVShl(x, y) => write!(f, "bvshl({}, {})", x.index(), y.index()),
            Expr::BVLShr(x, y) => write!(f, "bvlshr({}, {})", x.index(), y.index()),
            Expr::BVAShr(x, y) => write!(f, "bvashr({}, {})", x.index(), y.index()),
            Expr::BVRotl(x, y) => write!(f, "bvrotl({}, {})", x.index(), y.index()),
            Expr::BVRotr(x, y) => write!(f, "bvrotr({}, {})", x.index(), y.index()),
            Expr::Conditional(c, t, e) => {
                write!(f, "{} ? {} : {}", c.index(), t.index(), e.index())
            }
            Expr::BVZeroExt(w, x) => write!(f, "bv_zero_ext({}, {})", w.index(), x.index()),
            Expr::BVSignExt(w, x) => write!(f, "bv_zero_ext({}, {})", w.index(), x.index()),
            Expr::BVConvTo(w, x) => write!(f, "bv_conv_to({}, {})", w.index(), x.index()),
            Expr::BVExtract(h, l, x) => write!(f, "bv_extract({h}, {l}, {})", x.index()),
            Expr::BVConcat(x, y) => write!(f, "bv_concat({}, {})", x.index(), y.index()),
            Expr::Int2BV(w, x) => write!(f, "int2bv({}, {})", w.index(), x.index()),
            Expr::ToFP(w, x) => write!(f, "to_fp({}, {})", w.index(), x.index()),
            Expr::ToFPUnsigned(w, x) => write!(f, "to_fp_unsigned({}, {})", w.index(), x.index()),
            Expr::ToFPFromFP(w, x) => write!(f, "to_fp_from_fp({}, {})", w.index(), x.index()),
            Expr::FPToUBV(w, x) => write!(f, "fp.to_ubv({}, {})", w.index(), x.index()),
            Expr::FPToSBV(w, x) => write!(f, "fp.to_sbv({}, {})", w.index(), x.index()),
            Expr::BV2Nat(x) => write!(f, "bv2nat({})", x.index()),
            Expr::WidthOf(x) => write!(f, "width_of({})", x.index()),
            Expr::FPPositiveInfinity(x) => write!(f, "fp.+oo({})", x.index()),
            Expr::FPNegativeInfinity(x) => write!(f, "fp.-oo({})", x.index()),
            Expr::FPPositiveZero(x) => write!(f, "fp.+zero({})", x.index()),
            Expr::FPNegativeZero(x) => write!(f, "fp.-zero({})", x.index()),
            Expr::FPNaN(x) => write!(f, "fp.NaN({})", x.index()),
            Expr::FPEq(x, y) => write!(f, "fp.eq({}, {})", x.index(), y.index()),
            Expr::FPNe(x, y) => write!(f, "fp.ne({}, {})", x.index(), y.index()),
            Expr::FPLt(x, y) => write!(f, "fp.lt({}, {})", x.index(), y.index()),
            Expr::FPGt(x, y) => write!(f, "fp.gt({}, {})", x.index(), y.index()),
            Expr::FPLe(x, y) => write!(f, "fp.le({}, {})", x.index(), y.index()),
            Expr::FPGe(x, y) => write!(f, "fp.ge({}, {})", x.index(), y.index()),
            Expr::FPAdd(x, y) => write!(f, "fp.add({}, {})", x.index(), y.index()),
            Expr::FPSub(x, y) => write!(f, "fp.sub({}, {})", x.index(), y.index()),
            Expr::FPMul(x, y) => write!(f, "fp.mul({}, {})", x.index(), y.index()),
            Expr::FPDiv(x, y) => write!(f, "fp.div({}, {})", x.index(), y.index()),
            Expr::FPMin(x, y) => write!(f, "fp.min({}, {})", x.index(), y.index()),
            Expr::FPMax(x, y) => write!(f, "fp.max({}, {})", x.index(), y.index()),
            Expr::FPNeg(x) => write!(f, "fp.neg({})", x.index()),
            Expr::FPCeil(x) => write!(f, "fp.ceil({})", x.index()),
            Expr::FPFloor(x) => write!(f, "fp.floor({})", x.index()),
            Expr::FPSqrt(x) => write!(f, "fp.sqrt({})", x.index()),
            Expr::FPTrunc(x) => write!(f, "fp.trunc({})", x.index()),
            Expr::FPNearest(x) => write!(f, "fp.nearest({})", x.index()),
            Expr::FPIsZero(x) => write!(f, "fp.isZero({})", x.index()),
            Expr::FPIsInfinite(x) => write!(f, "fp.isInfinite({})", x.index()),
            Expr::FPIsNaN(x) => write!(f, "fp.isNaN({})", x.index()),
            Expr::FPIsNegative(x) => write!(f, "fp.isNegative({})", x.index()),
            Expr::FPIsPositive(x) => write!(f, "fp.isPositive({})", x.index()),
        }
    }
}

pub type Model = HashMap<ExprId, Const>;

#[derive(Debug)]
pub struct Variable {
    pub ty: Type,
    pub name: String,
}

impl Variable {
    fn component_name(prefix: &str, field: &str) -> String {
        format!("{prefix}_{field}")
    }
}

#[derive(Debug, Clone)]
pub struct SymbolicOption {
    some: ExprId,
    inner: Box<Symbolic>,
}

#[derive(Debug, Clone)]
pub struct SymbolicField {
    pub name: String,
    pub value: Symbolic,
}

impl SymbolicField {
    fn eval(&self, model: &Model) -> Result<FieldValue> {
        Ok(FieldValue {
            name: self.name.clone(),
            value: self.value.eval(model)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SymbolicEnum {
    pub ty: TypeId,
    pub discriminant: ExprId,
    pub variants: Vec<SymbolicVariant>,
}

impl SymbolicEnum {
    fn try_variant_by_name(&self, name: &str) -> Result<&SymbolicVariant> {
        self.variants
            .iter()
            .find(|v| v.name == name)
            .ok_or(format_err!("no variant with name {name}"))
    }

    fn validate(&self) -> Result<()> {
        // Expect the variants to have distinct discriminants in the range [0, num_variants).
        for (expect, variant) in self.variants.iter().enumerate() {
            if variant.discriminant != expect {
                bail!(
                    "variant '{name}' has unexpected discriminant",
                    name = variant.name
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SymbolicVariant {
    pub name: String,
    pub id: VariantId,
    pub discriminant: usize,
    pub value: Symbolic,
}

impl SymbolicVariant {
    fn try_field_by_name(&self, name: &str) -> Result<&SymbolicField> {
        self.fields()?
            .iter()
            .find(|f| f.name == name)
            .ok_or(format_err!("no field with name {name}"))
    }

    fn field_values(&self) -> Result<Vec<Symbolic>> {
        Ok(self.fields()?.iter().map(|f| f.value.clone()).collect())
    }

    fn fields(&self) -> Result<&Vec<SymbolicField>> {
        self.value
            .as_struct()
            .ok_or(format_err!("variant value is not a struct"))
    }
}

impl std::fmt::Display for SymbolicVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{name} {value}", name = self.name, value = self.value)
    }
}

/// Inline spec expression macro.
///
/// Note that at this stage the spec expressions are preserved as
/// [`spec::Expr`]. Generation of [`Expr`] objects from them is deferred until
/// macro expansion.
#[derive(Debug, Clone)]
pub struct Macro {
    pub params: Vec<Ident>,
    pub body: spec::Expr,
}

#[derive(Debug, Clone)]
pub enum Symbolic {
    Scalar(ExprId),
    Struct(Vec<SymbolicField>),
    Enum(SymbolicEnum),
    Option(SymbolicOption),
    Tuple(Vec<Symbolic>),
    Macro(Macro),
}

impl Symbolic {
    /// Name of the symbolic value's variant, for use in diagnostics.
    fn kind(&self) -> &'static str {
        match self {
            Self::Scalar(_) => "scalar",
            Self::Struct(_) => "struct",
            Self::Enum(_) => "enum",
            Self::Option(_) => "option",
            Self::Tuple(_) => "tuple",
            Self::Macro(_) => "macro",
        }
    }

    fn as_scalar(&self) -> Option<ExprId> {
        match self {
            Self::Scalar(x) => Some(*x),
            _ => None,
        }
    }

    fn as_struct(&self) -> Option<&Vec<SymbolicField>> {
        match self {
            Self::Struct(fields) => Some(fields),
            _ => None,
        }
    }

    fn as_enum(&self) -> Option<&SymbolicEnum> {
        match self {
            Self::Enum(e) => Some(e),
            _ => None,
        }
    }

    fn as_option(&self) -> Option<&SymbolicOption> {
        match self {
            Self::Option(opt) => Some(opt),
            _ => None,
        }
    }

    fn as_tuple(&self) -> Option<&Vec<Symbolic>> {
        match self {
            Self::Tuple(fields) => Some(fields),
            _ => None,
        }
    }

    fn elements(&self) -> &[Symbolic] {
        match self {
            Self::Tuple(fields) => &fields[..],
            v => std::slice::from_ref(v),
        }
    }

    fn eval(&self, model: &Model) -> Result<Value> {
        match self {
            Symbolic::Scalar(x) => Ok(Value::Const(
                model
                    .get(x)
                    .ok_or(format_err!("undefined expression in model"))?
                    .clone(),
            )),
            Symbolic::Struct(fields) => Ok(Value::Struct(
                fields
                    .iter()
                    .map(|f| f.eval(model))
                    .collect::<Result<_>>()?,
            )),
            Symbolic::Enum(e) => {
                // Determine the enum variant by looking up the discriminant.
                let discriminant: usize = model
                    .get(&e.discriminant)
                    .ok_or(format_err!("undefined discriminant in model"))?
                    .as_int()
                    .ok_or(format_err!(
                        "model value for discriminant is not an integer"
                    ))?
                    .try_into()
                    .unwrap();
                let variant = e
                    .variants
                    .iter()
                    .find(|v| v.discriminant == discriminant)
                    .ok_or(format_err!("no variant with discriminant {discriminant}"))?;
                Ok(Value::Enum(Box::new(VariantValue {
                    name: variant.name.clone(),
                    value: variant.value.eval(model)?,
                })))
            }
            Symbolic::Option(opt) => match model.get(&opt.some) {
                Some(Const::Bool(true)) => {
                    Ok(Value::Option(Some(Box::new(opt.inner.eval(model)?))))
                }
                Some(Const::Bool(false)) => Ok(Value::Option(None)),
                Some(_) => bail!("model value for option some is not boolean"),
                None => bail!("undefined expression in model"),
            },
            Symbolic::Tuple(elements) => Ok(Value::Tuple(
                elements
                    .iter()
                    .map(|s| s.eval(model))
                    .collect::<Result<_>>()?,
            )),
            Symbolic::Macro(_) => bail!("cannot evaluate macro"),
        }
    }

    // Build a new value by applying the given map function to all constituent
    // scalars in this symbolic value.
    fn scalar_map<F>(&self, f: &mut F) -> Symbolic
    where
        F: FnMut(ExprId) -> ExprId,
    {
        match self {
            Symbolic::Scalar(x) => Symbolic::Scalar(f(*x)),
            Symbolic::Struct(fields) => Symbolic::Struct(
                fields
                    .iter()
                    .map(|field| SymbolicField {
                        name: field.name.clone(),
                        value: field.value.scalar_map(f),
                    })
                    .collect(),
            ),
            Symbolic::Enum(e) => Symbolic::Enum(SymbolicEnum {
                ty: e.ty,
                discriminant: f(e.discriminant),
                variants: e
                    .variants
                    .iter()
                    .map(|v| SymbolicVariant {
                        id: v.id,
                        name: v.name.clone(),
                        discriminant: v.discriminant,
                        value: v.value.scalar_map(f),
                    })
                    .collect(),
            }),
            v => todo!("scalar map: {v:?}"),
        }
    }

    fn merge<F>(a: &Symbolic, b: &Symbolic, merge: &mut F) -> Result<Symbolic>
    where
        F: FnMut(ExprId, ExprId) -> ExprId,
    {
        if std::mem::discriminant(a) != std::mem::discriminant(b) {
            bail!("conditional arms have incompatible types");
        }
        match (a, b) {
            (Symbolic::Scalar(a), Symbolic::Scalar(b)) => Ok(merge(*a, *b).into()),
            (Symbolic::Struct(a_fields), Symbolic::Struct(b_fields)) => {
                assert_eq!(a_fields.len(), b_fields.len());
                Ok(Symbolic::Struct(
                    zip(a_fields, b_fields)
                        .map(|(a, b)| {
                            assert_eq!(a.name, b.name);
                            Ok(SymbolicField {
                                name: a.name.clone(),
                                value: Symbolic::merge(&a.value, &b.value, merge)?,
                            })
                        })
                        .collect::<Result<_>>()?,
                ))
            }
            (Symbolic::Enum(a), Symbolic::Enum(b)) => {
                assert_eq!(a.ty, b.ty);
                let ty = a.ty;
                let discriminant = merge(a.discriminant, b.discriminant);
                assert_eq!(a.variants.len(), b.variants.len());
                let variants = zip(&a.variants, &b.variants)
                    .map(|(a, b)| {
                        assert_eq!(a.name, b.name);
                        assert_eq!(a.id, b.id);
                        assert_eq!(a.discriminant, b.discriminant);
                        Ok(SymbolicVariant {
                            name: a.name.clone(),
                            id: a.id,
                            discriminant: a.discriminant,
                            value: Symbolic::merge(&a.value, &b.value, merge)?,
                        })
                    })
                    .collect::<Result<_>>()?;
                Ok(Symbolic::Enum(SymbolicEnum {
                    ty,
                    discriminant,
                    variants,
                }))
            }
            case => todo!("symbolic merge types: {case:?}"),
        }
    }
}

impl From<ExprId> for Symbolic {
    fn from(x: ExprId) -> Self {
        Symbolic::Scalar(x)
    }
}

impl std::fmt::Display for Symbolic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbolic::Scalar(x) => write!(f, "{}", x.index()),
            Symbolic::Struct(fields) => write!(
                f,
                "{{{fields}}}",
                fields = fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name, f.value))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Symbolic::Enum(e) => write!(
                f,
                "{{{discriminant}, {variants}}}",
                discriminant = e.discriminant.index(),
                variants = e
                    .variants
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Symbolic::Option(SymbolicOption { some, inner }) => {
                write!(f, "Option{{some: {}, inner: {inner}}}", some.index())
            }
            Symbolic::Tuple(vs) => write!(
                f,
                "({vs})",
                vs = vs
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Symbolic::Macro(_) => write!(f, "macro"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Const(Const),
    Struct(Vec<FieldValue>),
    Enum(Box<VariantValue>),
    Option(Option<Box<Value>>),
    Tuple(Vec<Value>),
}

#[derive(Debug, Clone)]
pub struct FieldValue {
    name: String,
    value: Value,
}

#[derive(Debug, Clone)]
pub struct VariantValue {
    name: String,
    value: Value,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Const(c) => c.fmt(f),
            Value::Struct(fields) => write!(
                f,
                "{{{fields}}}",
                fields = fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name, f.value))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Enum(v) => write!(f, "{name} {value}", name = v.name, value = v.value),
            Value::Option(Some(v)) => write!(f, "Some({v})"),
            Value::Option(None) => write!(f, "None"),
            Value::Tuple(elements) => write!(
                f,
                "({})",
                elements
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

#[derive(Debug)]
pub struct Call {
    pub term: TermId,
    pub args: Vec<Symbolic>,
    pub ret: Symbolic,
    pub signatures: Vec<Signature>,
}

// Type qualifier, for example derived from an `(as ...)` expression.
#[derive(Debug)]
pub struct Qualifier {
    pub value: Symbolic,
    pub ty: Compound,
}

/// Verification conditions for an expansion.
#[derive(Debug, Default)]
pub struct Conditions {
    pub exprs: Vec<Expr>,
    pub assumptions: Vec<ExprId>,
    pub assertions: Vec<ExprId>,
    pub variables: Vec<Variable>,
    pub state: Variables,
    pub calls: Vec<Call>,
    pub qualifiers: Vec<Qualifier>,
    pub pos: HashMap<ExprId, Pos>,
}

impl Conditions {
    pub fn from_expansion(expansion: &Expansion, prog: &Program) -> Result<Self> {
        let builder = ConditionsBuilder::new(expansion, prog);
        builder.build()
    }

    pub fn pretty_print(&self, prog: &Program) {
        println!("conditions {{");

        // Expressions
        println!("\texprs = [");
        for (i, expr) in self.exprs.iter().enumerate() {
            println!("\t\t{i}:\t{expr}");
        }
        println!("\t]");

        // Assumptions
        println!("\tassumptions = [");
        for expr_id in &self.assumptions {
            println!("\t\t{}", expr_id.index());
        }
        println!("\t]");

        // Assertions
        println!("\tassertions = [");
        for expr_id in &self.assertions {
            println!("\t\t{}", expr_id.index());
        }
        println!("\t]");

        // Variables
        println!("\tvariables = [");
        for (i, v) in self.variables.iter().enumerate() {
            println!("\t\t{i}:\t{ty}\t{name}", ty = v.ty, name = v.name);
        }
        println!("\t]");

        // Calls
        // TODO(mbm): prettier pretty printing code
        println!("\tcalls = [");
        for call in &self.calls {
            println!("\t\tcall {{");
            println!("\t\t\tterm = {}", prog.term_name(call.term));
            if !call.args.is_empty() {
                println!("\t\t\targs = [");
                for arg in &call.args {
                    println!("\t\t\t\t{}", arg);
                }
                println!("\t\t\t]");
            }
            println!("\t\t\tret = {}", call.ret);
            if !call.signatures.is_empty() {
                println!("\t\t\tsignatures = [");
                for sig in &call.signatures {
                    println!("\t\t\t\tsignature {{");
                    if !sig.args.is_empty() {
                        println!("\t\t\t\t\targs = [");
                        for arg in &sig.args {
                            println!("\t\t\t\t\t\t{arg}");
                        }
                        println!("\t\t\t\t\t]");
                    }
                    println!("\t\t\t\t\tret = {}", sig.ret);
                    println!("\t\t\t\t}}");
                }
                println!("\t\t\t]");
            }
            println!("\t\t}}");
        }
        println!("\t]");

        println!("}}");
    }

    pub fn validate(&self) -> Result<()> {
        // Ensure there are no dangling expressions.
        let reachable = self.reachable();
        for x in (0..self.exprs.len()).map(ExprId) {
            if self.exprs[x.index()].is_variable() {
                continue;
            }
            if !reachable.contains(&x) {
                bail!("expression {x} is unreachable", x = x.index());
            }
        }

        Ok(())
    }

    fn reachable(&self) -> HashSet<ExprId> {
        let mut reach = HashSet::new();

        let mut stack: Vec<ExprId> = Vec::new();
        stack.extend(&self.assumptions);
        stack.extend(&self.assertions);

        while let Some(x) = stack.pop() {
            if reach.contains(&x) {
                continue;
            }

            reach.insert(x);
            let expr = &self.exprs[x.index()];
            stack.extend(expr.sources());
        }

        reach
    }

    pub fn print_model(&self, model: &Model, prog: &Program) -> Result<()> {
        self.write_model(&mut std::io::stdout(), model, prog)
    }

    pub fn write_model(
        &self,
        out: &mut dyn std::io::Write,
        model: &Model,
        prog: &Program,
    ) -> Result<()> {
        // State
        for (name, value) in &self.state.0 {
            writeln!(out, "state: {name} = {}", value.eval(model)?)?;
        }

        // Calls
        for call in &self.calls {
            // Skip unit enum variant terms, which may occur frequently and are
            // rarely informative.
            let term = prog.term(call.term);
            if term.is_enum_variant() && call.args.is_empty() {
                continue;
            }

            writeln!(
                out,
                "{term_name}({args}) -> {ret}",
                term_name = prog.term_name(call.term),
                args = call
                    .args
                    .iter()
                    .map(|a| Ok(a.eval(model)?.to_string()))
                    .collect::<Result<Vec<_>>>()?
                    .join(", "),
                ret = call.ret.eval(model)?
            )?;
        }

        Ok(())
    }

    pub fn error_at_expr(&self, prog: &Program, x: ExprId, msg: impl Into<String>) -> Error {
        if let Some(pos) = self.pos.get(&x) {
            prog.error_at_pos(*pos, msg).into()
        } else {
            Error::msg(msg.into())
        }
    }
}

enum TermKind {
    Constructor,
    Extractor,
}

#[derive(Copy, Clone)]
enum Invocation {
    Caller,
    Callee,
}

#[derive(Copy, Clone)]
enum Domain {
    Total,
    Partial(ExprId),
}

impl Domain {
    fn from_return_value(value: &Symbolic) -> (Self, Symbolic) {
        match value {
            Symbolic::Option(opt) => (Self::Partial(opt.some), (*opt.inner).clone()),
            v => (Self::Total, v.clone()),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Variables(HashMap<String, Symbolic>);

impl Variables {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn get(&self, name: &String) -> Option<&Symbolic> {
        self.0.get(name)
    }

    fn expect(&self, name: &String) -> Result<&Symbolic> {
        self.get(name)
            .ok_or(format_err!("undefined variable {name}"))
    }

    fn set(&mut self, name: String, value: Symbolic) -> Result<()> {
        match self.0.entry(name) {
            Entry::Occupied(e) => {
                bail!("redefinition of variable {name}", name = e.key());
            }
            Entry::Vacant(e) => {
                e.insert(value);
                Ok(())
            }
        }
    }
}

struct ConditionsBuilder<'a> {
    expansion: &'a Expansion,
    prog: &'a Program,

    state_modification_conds: HashMap<String, Vec<ExprId>>,
    binding_value: HashMap<BindingId, Symbolic>,
    expr_map: HashMap<Expr, ExprId>,
    conditions: Conditions,
    position_stack: Vec<Pos>,
}

impl<'a> ConditionsBuilder<'a> {
    fn new(expansion: &'a Expansion, prog: &'a Program) -> Self {
        Self {
            expansion,
            prog,
            state_modification_conds: HashMap::new(),
            binding_value: HashMap::new(),
            expr_map: HashMap::new(),
            conditions: Conditions::default(),
            position_stack: Vec::new(),
        }
    }

    fn build(mut self) -> Result<Conditions> {
        // State initialization.
        for state in &self.prog.specenv.state {
            self.init_state(state)?;
        }

        // Bindings.
        for (i, binding) in self.expansion.bindings.iter().enumerate() {
            if let Some(binding) = binding {
                self.add_binding(i.try_into().unwrap(), binding)?;
            }
        }

        // Callee contract for the term under expansion.
        self.constructor(
            self.expansion.result,
            self.expansion.term,
            &self.expansion.parameters,
            Invocation::Callee,
        )?;

        // Constraints.
        for constrain in &self.expansion.constraints {
            let holds = self.constrain(constrain)?;
            self.conditions.assumptions.push(holds);
        }

        // Equals.
        for (a, b) in self.expansion.equalities() {
            let eq = self.bindings_equal(a, b)?;
            self.conditions.assumptions.push(eq);
        }

        // State defaults.
        for state in &self.prog.specenv.state {
            self.state_default(state)?;
        }

        // Validate
        self.conditions.validate()?;

        Ok(self.conditions)
    }

    fn init_state(&mut self, state: &State) -> Result<()> {
        let name = &state.name.0;
        let value = self.alloc_value(&state.ty, name.clone())?;
        self.conditions.state.set(name.clone(), value)?;
        Ok(())
    }

    fn state_default(&mut self, state: &State) -> anyhow::Result<()> {
        // Evaluate the default spec expression in a scope that only defines
        // the state variable itself.
        let mut vars = Variables::new();
        let name = &state.name.0;
        vars.set(name.clone(), self.conditions.state.expect(name)?.clone())?;
        let mut default = self.spec_expr(&state.default, &vars)?;

        // Other specs may have declared conditions under which they modify the
        // state. The default only applies when none of them are true.
        if let Some(conds) = self.state_modification_conds.get(name) {
            let modified = self.any(conds.clone());
            let not_modified = self.dedup_expr(Expr::Not(modified));
            default = self.scalar(Expr::Imp(not_modified, self.as_scalar(default)?));
        }

        // The expression should define an assumption about the state variable,
        // so should be a scalar boolean.
        self.conditions.assumptions.push(self.as_scalar(default)?);

        Ok(())
    }

    fn add_binding(&mut self, id: BindingId, binding: &Binding) -> Result<()> {
        // Exit if already added.
        if self.binding_value.contains_key(&id) {
            return Ok(());
        }

        // Allocate a value.
        let binding_type = self.binding_type(binding);
        let name = format!("b{}", id.index());
        let value = self.alloc_binding(&binding_type, name)?;
        self.binding_value.insert(id, value);

        // Ensure dependencies have been added.
        for source in binding.sources() {
            let source_binding = self
                .expansion
                .binding(*source)
                .expect("source binding should be defined");
            self.add_binding(*source, source_binding)?;
        }

        // Generate conditions depending on binding type.
        match binding {
            Binding::ConstInt { val, ty } => self.const_int(id, *val, *ty),

            Binding::ConstBool { val, .. } => self.const_bool(id, *val),

            Binding::ConstPrim { val } => self.const_prim(id, *val),

            // Argument binding has no associated constraints.
            Binding::Argument { .. } => Ok(()),

            Binding::Extractor { term, parameter } => self.extractor(id, *term, *parameter),

            Binding::Constructor {
                term, parameters, ..
            } => self.constructor(id, *term, parameters, Invocation::Caller),

            Binding::Iterator { .. } => unimplemented!("iterator bindings"),

            Binding::MakeVariant {
                ty,
                variant,
                fields,
            } => self.make_variant(id, *ty, *variant, fields),

            Binding::MatchVariant {
                source,
                variant,
                field,
            } => self.match_variant(id, *source, *variant, *field),

            Binding::MakeStruct { ty, fields } => self.make_struct(id, *ty, fields),

            Binding::ExtractStruct { source, field } => self.extract_struct(id, *source, *field),

            Binding::MakeSome { inner } => self.make_some(id, *inner),

            Binding::MatchSome { source } => self.match_some(id, *source),

            Binding::MatchTuple { source, field } => self.match_tuple(id, *source, *field),
        }
    }

    fn const_int(&mut self, id: BindingId, val: i128, ty: TypeId) -> Result<()> {
        let eq = self.equals_const_int(id, val, ty)?;
        self.conditions.assumptions.push(eq);
        Ok(())
    }

    fn const_bool(&mut self, id: BindingId, val: bool) -> Result<()> {
        let eq = self.equals_const_bool(id, val)?;
        self.conditions.assumptions.push(eq);
        Ok(())
    }

    fn equals_const_int(&mut self, id: BindingId, val: i128, ty: TypeId) -> Result<ExprId> {
        // Determine modeled type.
        let ty_name = self.prog.type_name(ty);
        let ty = self
            .prog
            .specenv
            .type_model
            .get(&ty)
            .ok_or(self.error(format!("no model for type {ty_name}")))?
            .as_primitive()
            .ok_or(self.error("constant must have basic type"))?;

        // Construct value of the determined type.
        let value = self.spec_typed_value(val, ty)?.into();

        // Destination binding equals constant value.
        let eq = self.values_equal(self.binding_value[&id].clone(), value)?;
        Ok(eq)
    }

    fn const_prim(&mut self, id: BindingId, val: Sym) -> Result<()> {
        let eq = self.equals_const_prim(id, val)?;
        self.conditions.assumptions.push(eq);
        Ok(())
    }

    fn equals_const_prim(&mut self, id: BindingId, val: Sym) -> Result<ExprId> {
        // Lookup value.
        let spec_value = self
            .prog
            .specenv
            .const_value
            .get(&val)
            .ok_or(self.error(format!(
                "value of constant {const_name} is unspecified",
                const_name = self.prog.tyenv.syms[val.index()]
            )))?;
        let value = self.spec_expr_no_vars(spec_value)?;

        // Destination binding equals constant value.
        let eq = self.values_equal(self.binding_value[&id].clone(), value)?;
        Ok(eq)
    }

    fn equals_const_bool(&mut self, id: BindingId, val: bool) -> Result<ExprId> {
        // Destination binding equals constant value.
        let value = Symbolic::Scalar(self.boolean(val));
        let eq = self.values_equal(self.binding_value[&id].clone(), value)?;
        Ok(eq)
    }

    fn extractor(&mut self, id: BindingId, term: TermId, parameter: BindingId) -> Result<()> {
        // Arguments are the actually the return values of an
        // extractor, possibly wrapped in an Option<..> type.
        let (domain, ret) = Domain::from_return_value(&self.binding_value[&id]);
        let args = ret.elements();

        // Result maps to the parameter of an extractor.
        let result = self.binding_value[&parameter].clone();

        // Call extractor.
        self.call(
            term,
            TermKind::Extractor,
            args,
            result,
            Invocation::Caller,
            domain,
        )
        .with_context(|| {
            format!(
                "expanding extractor '{name}'",
                name = self.prog.term_name(term)
            )
        })
    }

    fn constructor(
        &mut self,
        id: BindingId,
        term: TermId,
        parameters: &[BindingId],
        invocation: Invocation,
    ) -> Result<()> {
        // Arguments.
        let mut args = Vec::new();
        for parameter_binding_id in parameters {
            let x = self
                .binding_value
                .get(parameter_binding_id)
                .expect("parameter binding should be defined")
                .clone();
            args.push(x);
        }

        // Return value.
        let (domain, result) = Domain::from_return_value(&self.binding_value[&id]);

        // Call constructor.
        self.call(
            term,
            TermKind::Constructor,
            &args,
            result,
            invocation,
            domain,
        )
        .with_context(|| {
            format!(
                "expanding constructor '{name}'",
                name = self.prog.term_name(term)
            )
        })
    }

    fn call(
        &mut self,
        term: TermId,
        kind: TermKind,
        args: &[Symbolic],
        ret: Symbolic,
        invocation: Invocation,
        domain: Domain,
    ) -> Result<()> {
        // Lookup spec.
        let term_name = self.prog.term_name(term);
        let term_spec = self
            .prog
            .specenv
            .term_spec
            .get(&term)
            .ok_or(self.error(format!("no spec for term {term_name}",)))?;

        // We are provided the arguments and return value as they appear
        // syntactically in the term declaration and specification. However,
        // whether these are the actual inputs and outputs of the corresponding
        // function depends on the term kind.
        if term_spec.args.len() != args.len() {
            bail!("incorrect number of arguments for term {term_name}");
        }
        let arguments: Vec<_> = zip(&term_spec.args, args).collect();
        let result = (&term_spec.ret, &ret);
        let (inputs, outputs) = match kind {
            TermKind::Constructor => (arguments.as_slice(), std::slice::from_ref(&result)),
            TermKind::Extractor => (std::slice::from_ref(&result), arguments.as_slice()),
        };

        // Scope for spec expression evaluation. State variables are always available.
        let mut vars = self.conditions.state.clone();

        // State modification conditions.
        for modifies in &term_spec.modifies {
            let cond = if let Some(cond_name) = &modifies.cond {
                // Allocate new boolean for the modification condition.
                let cond = self.alloc_variable(
                    Type::Bool,
                    format!("{name}_modification_cond", name = modifies.state.0),
                );
                // Bring into spec scope.
                vars.set(cond_name.0.clone(), cond.into())?;
                cond
            } else {
                // TODO(mbm): warn when state is both conditionally and unconditionaly modified.
                self.boolean(true)
            };

            // Record condition to determine when the default spec applies.
            self.state_modification_conds
                .entry(modifies.state.0.clone())
                .or_default()
                .push(cond);
        }

        // Inputs are available to the requires and matches clauses.
        for (name, input) in inputs {
            vars.set(name.0.clone(), (*input).clone())?;
        }

        // Requires.
        let mut requires: Vec<ExprId> = Vec::new();
        for require in &term_spec.requires {
            let require = self.spec_expr(require, &vars)?;
            requires.push(self.as_scalar(require)?);
        }

        // Matches.
        let mut matches: Vec<ExprId> = Vec::new();
        for m in &term_spec.matches {
            let m = self.spec_expr(m, &vars)?;
            matches.push(self.as_scalar(m)?);
        }

        // Outputs: only in scope for provides.
        for (name, output) in outputs {
            vars.set(name.0.clone(), (*output).clone())?;
        }

        // Provides.
        let mut provides: Vec<ExprId> = Vec::new();
        for provide in &term_spec.provides {
            let provide = self.spec_expr(provide, &vars)?;
            provides.push(self.as_scalar(provide)?);
        }

        // Partial function.
        // REVIEW(mbm): pin down semantics for partial function specifications.
        if let Domain::Partial(p) = domain {
            // Matches describe when the function applies.
            let all_matches = self.all(matches);
            let eq = self.exprs_equal(p, all_matches);
            self.conditions.assumptions.push(eq);

            // Provides are conditioned on the match.
            let all_provides = self.all(provides);
            let provide = self.dedup_expr(Expr::Imp(all_matches, all_provides));
            provides = vec![provide];
        } else if !matches.is_empty() {
            bail!("spec matches on non-partial function");
        }

        // Assert/assume depending on caller or callee.
        match invocation {
            Invocation::Caller => {
                self.conditions.assertions.extend(requires);
                self.conditions.assumptions.extend(provides);
            }
            Invocation::Callee => {
                self.conditions.assumptions.extend(requires);
                self.conditions.assertions.extend(provides);
            }
        }

        // Record callsite.
        self.record_term_instantiation(term, args.to_vec(), ret)?;

        Ok(())
    }

    fn record_term_instantiation(
        &mut self,
        term: TermId,
        args: Vec<Symbolic>,
        ret: Symbolic,
    ) -> Result<()> {
        let signatures = self
            .prog
            .specenv
            .resolve_term_instantiations(&term, &self.prog.tyenv)?;
        self.conditions.calls.push(Call {
            term,
            args,
            ret,
            signatures,
        });
        Ok(())
    }

    fn make_variant(
        &mut self,
        id: BindingId,
        ty: TypeId,
        variant: VariantId,
        fields: &[BindingId],
    ) -> Result<()> {
        // Lookup term corresponding to variant.
        let variant_term_id = self.prog.get_variant_term(ty, variant);

        // Invoke as a constructor.
        self.constructor(id, variant_term_id, fields, Invocation::Caller)?;

        Ok(())
    }

    fn match_variant(
        &mut self,
        id: BindingId,
        source: BindingId,
        variant: VariantId,
        field: TupleIndex,
    ) -> Result<()> {
        // Source binding should be an enum.
        let e = self.binding_value[&source]
            .as_enum()
            .ok_or(self.error("target of variant constraint should be an enum"))?
            .clone();

        // Lookup enum type via corresponding constriant,
        let tys: Vec<_> = self
            .expansion
            .constraints
            .iter()
            .flat_map(|c| match c {
                Constrain::Match(id, Constraint::Variant { ty, variant: v, .. })
                    if *id == source && *v == variant =>
                {
                    Some(ty)
                }
                _ => None,
            })
            .collect();
        if tys.len() != 1 {
            bail!("expected exactly one variant constraint for match variant binding");
        }
        let ty = tys[0];

        // Lookup variant and field.
        let variant_type = self.prog.tyenv.get_variant(*ty, variant);
        let variant_name = self.prog.tyenv.syms[variant_type.name.index()].as_str();

        let field_name = field_name_by_index(&variant_type.fields, field.index(), &self.prog.tyenv);

        // Destination binding.
        let v = self.binding_value[&id].clone();

        // Assumption: if the variant matches then the destination binding
        // equals the projected field.
        let variant = e.try_variant_by_name(variant_name)?;
        let field = variant.try_field_by_name(&field_name)?;

        let discriminator = self.discriminator(&e, variant);
        let eq = self.values_equal(v, field.value.clone())?;
        let constraint = self.dedup_expr(Expr::Imp(discriminator, eq));
        self.conditions.assumptions.push(constraint);

        Ok(())
    }

    fn make_struct(&mut self, id: BindingId, ty: TypeId, fields: &[BindingId]) -> Result<()> {
        // Destination binding should already be allocated as a struct.
        let dest_fields = self.binding_value[&id]
            .as_struct()
            .ok_or(self.error("target of make_struct should be a struct"))?
            .clone();

        // Lookup the struct type's field list for naming.
        let struct_ty = &self.prog.tyenv.types[ty.index()];
        let struct_fields = match struct_ty {
            sema::Type::Struct { fields, .. } => fields,
            _ => bail!("MakeStruct target type should be Type::Struct"),
        };

        if dest_fields.len() != fields.len() {
            bail!("make_struct: destination field count does not match binding count");
        }

        // Each input field binding's value equals the corresponding destination field's value.
        for (i, &field_binding_id) in fields.iter().enumerate() {
            let field_name = field_name_by_index(struct_fields, i, &self.prog.tyenv);
            let dest_field = dest_fields
                .iter()
                .find(|f| f.name == field_name)
                .ok_or(format_err!("no field with name {field_name}"))?;
            let input_value = self.binding_value[&field_binding_id].clone();
            let eq = self.values_equal(dest_field.value.clone(), input_value)?;
            self.conditions.assumptions.push(eq);
        }

        Ok(())
    }

    fn extract_struct(
        &mut self,
        id: BindingId,
        source: BindingId,
        field: TupleIndex,
    ) -> Result<()> {
        // Source binding should be a struct.
        let s = self.binding_value[&source]
            .as_struct()
            .ok_or(self.error("source of extract_struct should be a struct"))?
            .clone();

        // Lookup the struct type via the corresponding constraint on the source.
        let tys: Vec<_> = self
            .expansion
            .constraints
            .iter()
            .flat_map(|c| match c {
                Constrain::Match(cid, Constraint::Struct { ty, .. }) if *cid == source => Some(ty),
                _ => None,
            })
            .collect();
        if tys.len() != 1 {
            bail!("expected exactly one struct constraint for extract_struct binding");
        }
        let ty = tys[0];

        // Lookup field name from the struct type.
        let struct_ty = &self.prog.tyenv.types[ty.index()];
        let struct_fields = match struct_ty {
            sema::Type::Struct { fields, .. } => fields,
            _ => bail!("source of extract_struct should be Type::Struct"),
        };
        let field_name = field_name_by_index(struct_fields, field.index(), &self.prog.tyenv);

        // Locate the matching field in the source struct value.
        let symbolic_field = s
            .iter()
            .find(|f| f.name == field_name)
            .ok_or(format_err!("no field with name {field_name}"))?;

        // Assumption: destination binding equals the projected field value.
        let v = self.binding_value[&id].clone();
        let eq = self.values_equal(v, symbolic_field.value.clone())?;
        self.conditions.assumptions.push(eq);

        Ok(())
    }

    fn make_some(&mut self, id: BindingId, inner: BindingId) -> Result<()> {
        // Destination binding should be an option.
        let opt = self.binding_value[&id]
            .as_option()
            .expect("destination of make_some binding should be an option")
            .clone();

        // Inner binding.
        let inner = self.binding_value[&inner].clone();

        // Assumption: option is Some.
        self.conditions.assumptions.push(opt.some);

        // Assumption: option value is equal to this binding.
        let eq = self.values_equal(inner, (*opt.inner).clone())?;
        self.conditions.assumptions.push(eq);

        Ok(())
    }

    fn match_some(&mut self, id: BindingId, source: BindingId) -> Result<()> {
        // Source should be an option.
        let opt = self.binding_value[&source]
            .as_option()
            .expect("source of match_some binding should be an option")
            .clone();

        // Destination binding.
        let v = self.binding_value[&id].clone();

        // Assumption: if the option is some, then the inner value
        // equals this binding.
        let eq = self.values_equal(v, (*opt.inner).clone())?;
        let constraint = self.dedup_expr(Expr::Imp(opt.some, eq));
        self.conditions.assumptions.push(constraint);

        Ok(())
    }

    fn match_tuple(&mut self, id: BindingId, source: BindingId, field: TupleIndex) -> Result<()> {
        // Source should be a tuple. Access its fields.
        let fields = self.binding_value[&source]
            .as_tuple()
            .expect("source of match_tuple binding should be a tuple")
            .clone();

        // Destination binding.
        let v = self.binding_value[&id].clone();

        // Assumption: indexed field should equal this binding.
        let eq = self.values_equal(v, fields[field.index()].clone())?;
        self.conditions.assumptions.push(eq);

        Ok(())
    }

    fn constrain(&mut self, constrain: &Constrain) -> Result<ExprId> {
        match constrain {
            Constrain::Match(binding_id, constraint) => self.constraint(*binding_id, constraint),
            Constrain::NotAll(constrains) => {
                let cs = constrains
                    .iter()
                    .map(|c| self.constrain(c))
                    .collect::<Result<_>>()?;
                let all = self.all(cs);
                let not_all = self.dedup_expr(Expr::Not(all));
                Ok(not_all)
            }
        }
    }

    fn constraint(&mut self, binding_id: BindingId, constraint: &Constraint) -> Result<ExprId> {
        match constraint {
            Constraint::Some => self.constraint_some(binding_id),
            Constraint::ConstPrim { val } => self.equals_const_prim(binding_id, *val),
            Constraint::ConstBool { val, .. } => self.equals_const_bool(binding_id, *val),
            Constraint::ConstInt { val, ty } => self.equals_const_int(binding_id, *val, *ty),
            Constraint::Variant {
                ty,
                variant,
                fields: _,
            } => self.constraint_variant(binding_id, *ty, *variant),
            Constraint::Struct { ty, fields: _ } => self.constraint_struct(binding_id, *ty),
        }
    }

    fn constraint_some(&mut self, binding_id: BindingId) -> Result<ExprId> {
        // Constrained binding should be an option.
        let opt = self.binding_value[&binding_id]
            .as_option()
            .expect("target of some constraint should be an option")
            .clone();

        // Constraint: option is Some.
        Ok(opt.some)
    }

    fn constraint_variant(
        &mut self,
        binding_id: BindingId,
        ty: TypeId,
        variant: VariantId,
    ) -> Result<ExprId> {
        // Constrained binding should be an enum.
        let e = self.binding_value[&binding_id]
            .as_enum()
            .ok_or(self.error("target of variant constraint should be an enum"))?
            .clone();

        // TODO(mbm): check the enum type is correct?

        // Lookup variant.
        let variant_type = self.prog.tyenv.get_variant(ty, variant);
        let variant_name = self.prog.tyenv.syms[variant_type.name.index()].as_str();

        // Assumption: discriminant equals variant.
        let variant = e.try_variant_by_name(variant_name)?;
        let discriminator = self.discriminator(&e, variant);
        Ok(discriminator)
    }

    fn constraint_struct(&mut self, binding_id: BindingId, _ty: TypeId) -> Result<ExprId> {
        // Target binding should be a struct.
        self.binding_value[&binding_id]
            .as_struct()
            .ok_or(self.error("target of struct constraint should be a struct"))?;

        // A struct constraint is irrefutable: a value of the given struct type
        // always matches.
        Ok(self.boolean(true))
    }

    fn spec_expr(&mut self, expr: &spec::Expr, vars: &Variables) -> Result<Symbolic> {
        self.position_stack.push(expr.pos);
        let result = self.spec_expr_kind(&expr.x, vars);
        self.position_stack.pop();
        result
    }

    fn spec_expr_kind(&mut self, expr: &spec::ExprKind, vars: &Variables) -> Result<Symbolic> {
        macro_rules! unary_expr {
            ($expr:path, $x:ident) => {{
                let $x = self.spec_expr($x, vars)?;
                Ok(self.scalar($expr(self.as_scalar($x)?)))
            }};
        }

        macro_rules! binary_expr {
            ($expr:path, $x:ident, $y:ident) => {{
                let $x = self.spec_expr($x, vars)?;
                let $y = self.spec_expr($y, vars)?;
                Ok(self.scalar($expr(self.as_scalar($x)?, self.as_scalar($y)?)))
            }};
        }

        macro_rules! variadic_expr {
            ($expr:path, $xs:ident) => {{
                let exprs: Vec<ExprId> = $xs
                    .iter()
                    .map(|x| {
                        let x = self.spec_expr(x, vars)?;
                        self.as_scalar(x)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Symbolic::Scalar(
                    exprs
                        .into_iter()
                        .rev()
                        .reduce(|acc, e| self.dedup_expr($expr(e, acc)))
                        .ok_or(self.error("empty variadic expression"))?,
                ))
            }};
        }

        match expr {
            spec::ExprKind::Var(v) => {
                let v = vars.expect(&v.0)?;
                Ok(v.clone())
            }

            spec::ExprKind::Const(c) => Ok(self.constant(c.clone()).into()),

            spec::ExprKind::Constructor(constructor) => self.construct(constructor, vars),

            spec::ExprKind::Field(name, x) => {
                let x = self.spec_expr(x, vars)?;
                self.spec_field(name, x)
            }

            spec::ExprKind::Discriminator(variant, x) => {
                let x = self.spec_expr(x, vars)?;
                self.spec_discriminator(variant, x)
            }

            spec::ExprKind::Not(x) => unary_expr!(Expr::Not, x),
            spec::ExprKind::And(xs) => variadic_expr!(Expr::And, xs),
            spec::ExprKind::Or(xs) => variadic_expr!(Expr::Or, xs),
            spec::ExprKind::Imp(x, y) => binary_expr!(Expr::Imp, x, y),

            spec::ExprKind::Eq(x, y) => {
                let x = self.spec_expr(x, vars)?;
                let y = self.spec_expr(y, vars)?;
                Ok(self.values_equal(x, y)?.into())
            }

            spec::ExprKind::Lt(x, y) => binary_expr!(Expr::Lt, x, y),
            spec::ExprKind::Lte(x, y) => binary_expr!(Expr::Lte, x, y),
            spec::ExprKind::Gt(x, y) => binary_expr!(Expr::Lt, y, x),
            spec::ExprKind::Gte(x, y) => binary_expr!(Expr::Lte, y, x),
            spec::ExprKind::BVUlt(x, y) => binary_expr!(Expr::BVUlt, x, y),
            spec::ExprKind::BVUle(x, y) => binary_expr!(Expr::BVUle, x, y),
            spec::ExprKind::BVSge(x, y) => binary_expr!(Expr::BVSge, x, y),
            spec::ExprKind::BVSlt(x, y) => binary_expr!(Expr::BVSlt, x, y),
            spec::ExprKind::BVSle(x, y) => binary_expr!(Expr::BVSle, x, y),
            spec::ExprKind::BVSgt(x, y) => binary_expr!(Expr::BVSgt, x, y),
            spec::ExprKind::BVUgt(x, y) => binary_expr!(Expr::BVUgt, x, y),
            spec::ExprKind::BVUge(x, y) => binary_expr!(Expr::BVUge, x, y),
            spec::ExprKind::BVSaddo(x, y) => binary_expr!(Expr::BVSaddo, x, y),
            spec::ExprKind::BVNot(x) => unary_expr!(Expr::BVNot, x),
            spec::ExprKind::BVNeg(x) => unary_expr!(Expr::BVNeg, x),
            spec::ExprKind::Cls(x) => unary_expr!(Expr::Cls, x),
            spec::ExprKind::Clz(x) => unary_expr!(Expr::Clz, x),
            spec::ExprKind::Rev(x) => unary_expr!(Expr::Rev, x),
            spec::ExprKind::Popcnt(x) => unary_expr!(Expr::Popcnt, x),
            spec::ExprKind::Add(x, y) => binary_expr!(Expr::Add, x, y),
            spec::ExprKind::Sub(x, y) => binary_expr!(Expr::Sub, x, y),
            spec::ExprKind::Mul(x, y) => binary_expr!(Expr::Mul, x, y),
            spec::ExprKind::BVAdd(x, y) => binary_expr!(Expr::BVAdd, x, y),
            spec::ExprKind::BVSub(x, y) => binary_expr!(Expr::BVSub, x, y),
            spec::ExprKind::BVMul(x, y) => binary_expr!(Expr::BVMul, x, y),
            spec::ExprKind::BVSDiv(x, y) => binary_expr!(Expr::BVSDiv, x, y),
            spec::ExprKind::BVAnd(x, y) => binary_expr!(Expr::BVAnd, x, y),
            spec::ExprKind::BVOr(x, y) => binary_expr!(Expr::BVOr, x, y),
            spec::ExprKind::BVXor(x, y) => binary_expr!(Expr::BVXor, x, y),
            spec::ExprKind::BVShl(x, y) => binary_expr!(Expr::BVShl, x, y),
            spec::ExprKind::BVLShr(x, y) => binary_expr!(Expr::BVLShr, x, y),
            spec::ExprKind::BVAShr(x, y) => binary_expr!(Expr::BVAShr, x, y),
            spec::ExprKind::BVUDiv(x, y) => binary_expr!(Expr::BVUDiv, x, y),
            spec::ExprKind::BVURem(x, y) => binary_expr!(Expr::BVURem, x, y),
            spec::ExprKind::BVSRem(x, y) => binary_expr!(Expr::BVSRem, x, y),
            spec::ExprKind::BVRotl(x, y) => binary_expr!(Expr::BVRotl, x, y),
            spec::ExprKind::BVRotr(x, y) => binary_expr!(Expr::BVRotr, x, y),

            spec::ExprKind::Conditional(c, t, e) => {
                let c = self.spec_expr(c, vars)?;
                let t = self.spec_expr(t, vars)?;
                let e = self.spec_expr(e, vars)?;
                self.conditional(self.as_scalar(c)?, t, e)
            }

            spec::ExprKind::Switch(on, arms) => self.spec_switch(on, arms, vars),

            spec::ExprKind::Match(on, arms) => self.spec_match(on, arms, vars),

            spec::ExprKind::Let(defs, body) => self.spec_let(defs, body, vars),

            spec::ExprKind::With(decls, body) => self.spec_with(decls, body, vars),

            spec::ExprKind::Expand(ident, args) => self.spec_expand(ident, args, vars),

            spec::ExprKind::BVZeroExt(w, x) => binary_expr!(Expr::BVZeroExt, w, x),
            spec::ExprKind::BVSignExt(w, x) => binary_expr!(Expr::BVSignExt, w, x),
            spec::ExprKind::BVConvTo(w, x) => binary_expr!(Expr::BVConvTo, w, x),

            spec::ExprKind::BVExtract(h, l, x) => {
                let x = self.spec_expr(x, vars)?;
                Ok(self.scalar(Expr::BVExtract(*h, *l, self.as_scalar(x)?)))
            }

            spec::ExprKind::BVConcat(xs) => variadic_expr!(Expr::BVConcat, xs),
            spec::ExprKind::BVReplicate(x, n) => {
                let x = self.spec_expr(x, vars)?;
                let r = self.replicate(self.as_scalar(x)?, *n)?;
                Ok(Symbolic::Scalar(r))
            }
            spec::ExprKind::Int2BV(w, x) => binary_expr!(Expr::Int2BV, w, x),
            spec::ExprKind::BV2Nat(x) => unary_expr!(Expr::BV2Nat, x),
            spec::ExprKind::ToFP(w, x) => binary_expr!(Expr::ToFP, w, x),
            spec::ExprKind::ToFPUnsigned(w, x) => binary_expr!(Expr::ToFPUnsigned, w, x),
            spec::ExprKind::ToFPFromFP(w, x) => binary_expr!(Expr::ToFPFromFP, w, x),
            spec::ExprKind::FPToUBV(w, x) => binary_expr!(Expr::FPToUBV, w, x),
            spec::ExprKind::FPToSBV(w, x) => binary_expr!(Expr::FPToSBV, w, x),
            spec::ExprKind::WidthOf(x) => unary_expr!(Expr::WidthOf, x),

            spec::ExprKind::As(x, ty) => {
                let x = self.spec_expr(x, vars)?;
                self.conditions.qualifiers.push(Qualifier {
                    value: x.clone(),
                    ty: ty.clone(),
                });
                Ok(x)
            }

            spec::ExprKind::FPPositiveInfinity(x) => unary_expr!(Expr::FPPositiveInfinity, x),
            spec::ExprKind::FPNegativeInfinity(x) => unary_expr!(Expr::FPNegativeInfinity, x),
            spec::ExprKind::FPPositiveZero(x) => unary_expr!(Expr::FPPositiveZero, x),
            spec::ExprKind::FPNegativeZero(x) => unary_expr!(Expr::FPNegativeZero, x),
            spec::ExprKind::FPNaN(x) => unary_expr!(Expr::FPNaN, x),
            spec::ExprKind::FPEq(x, y) => binary_expr!(Expr::FPEq, x, y),
            spec::ExprKind::FPNe(x, y) => binary_expr!(Expr::FPNe, x, y),
            spec::ExprKind::FPLt(x, y) => binary_expr!(Expr::FPLt, x, y),
            spec::ExprKind::FPGt(x, y) => binary_expr!(Expr::FPGt, x, y),
            spec::ExprKind::FPLe(x, y) => binary_expr!(Expr::FPLe, x, y),
            spec::ExprKind::FPGe(x, y) => binary_expr!(Expr::FPGe, x, y),
            spec::ExprKind::FPAdd(x, y) => binary_expr!(Expr::FPAdd, x, y),
            spec::ExprKind::FPSub(x, y) => binary_expr!(Expr::FPSub, x, y),
            spec::ExprKind::FPMul(x, y) => binary_expr!(Expr::FPMul, x, y),
            spec::ExprKind::FPDiv(x, y) => binary_expr!(Expr::FPDiv, x, y),
            spec::ExprKind::FPMin(x, y) => binary_expr!(Expr::FPMin, x, y),
            spec::ExprKind::FPMax(x, y) => binary_expr!(Expr::FPMax, x, y),
            spec::ExprKind::FPNeg(x) => unary_expr!(Expr::FPNeg, x),
            spec::ExprKind::FPCeil(x) => unary_expr!(Expr::FPCeil, x),
            spec::ExprKind::FPFloor(x) => unary_expr!(Expr::FPFloor, x),
            spec::ExprKind::FPSqrt(x) => unary_expr!(Expr::FPSqrt, x),
            spec::ExprKind::FPTrunc(x) => unary_expr!(Expr::FPTrunc, x),
            spec::ExprKind::FPNearest(x) => unary_expr!(Expr::FPNearest, x),
            spec::ExprKind::FPIsZero(x) => unary_expr!(Expr::FPIsZero, x),
            spec::ExprKind::FPIsInfinite(x) => unary_expr!(Expr::FPIsInfinite, x),
            spec::ExprKind::FPIsNaN(x) => unary_expr!(Expr::FPIsNaN, x),
            spec::ExprKind::FPIsNegative(x) => unary_expr!(Expr::FPIsNegative, x),
            spec::ExprKind::FPIsPositive(x) => unary_expr!(Expr::FPIsPositive, x),

            spec::ExprKind::Macro(params, body) => Ok(Symbolic::Macro(Macro {
                params: params.clone(),
                body: body.clone(),
            })),
        }
    }

    fn spec_expr_no_vars(&mut self, expr: &spec::Expr) -> Result<Symbolic> {
        let no_vars = Variables::new();
        self.spec_expr(expr, &no_vars)
    }

    fn spec_typed_value(&mut self, val: i128, ty: &Type) -> Result<ExprId> {
        match ty {
            Type::Bool => Ok(self.boolean(match val {
                0 => false,
                1 => true,
                _ => bail!("boolean value must be zero or one"),
            })),
            Type::Int => Ok(self.constant(Const::Int(val))),
            Type::BitVector(Width::Bits(w)) => {
                Ok(self.constant(Const::BitVector(*w, val.try_into()?)))
            }
            _ => bail!("cannot construct constant of type {ty}"),
        }
    }

    fn construct(&mut self, constructor: &Constructor, vars: &Variables) -> Result<Symbolic> {
        match constructor {
            Constructor::Enum {
                name,
                variant,
                args,
            } => {
                // Lookup ISLE type by name.
                let type_id = self
                    .prog
                    .tyenv
                    .get_type_by_name(name)
                    .ok_or(self.error(format!("unknown enum type {name}", name = name.0)))?;

                // Determine type model.
                let model = self
                    .prog
                    .specenv
                    .type_model
                    .get(&type_id)
                    .ok_or(self.error(format!(
                        "unspecified model for type `{name}`: this enum type is being \
                         constructed here, but has no `(model ...)` declaration. Add a \
                         `(model {name} (enum ...))` form in a spec file listing its variants \
                         so the verifier knows its representation.",
                        name = name.0
                    )))?;

                // Should be an enum.
                let e = model.as_enum().ok_or(
                    self.error(format!("{name} expected to have enum type", name = name.0)),
                )?;

                // Lookup variant.
                let variant =
                    e.variants.iter().find(|v| v.name.0 == variant.0).ok_or(
                        self.error(format!("unknown variant {variant}", variant = variant.0)),
                    )?;

                // Discriminant: constant value since we are constructing a known variant.
                let discriminant = self.constant(Const::Int(variant.id.index().try_into()?));

                // Variants: undefined except for the variant under construction.
                let variants = e
                    .variants
                    .iter()
                    .map(|v| {
                        // For all except the variant under construction, allocate an undefined variant.
                        if v.id != variant.id {
                            return self.alloc_variant(v, "undef".to_string());
                        }

                        // Construct a variant provided arguments.
                        assert_eq!(args.len(), v.fields.len());
                        let fields = zip(&v.fields, args)
                            .map(|(f, a)| {
                                Ok(SymbolicField {
                                    name: f.name.0.clone(),
                                    value: self.spec_expr(a, vars)?,
                                })
                            })
                            .collect::<Result<_>>()?;
                        Ok(SymbolicVariant {
                            name: v.name.0.clone(),
                            id: v.id,
                            discriminant: v.id.index(),
                            value: Symbolic::Struct(fields),
                        })
                    })
                    .collect::<Result<_>>()?;

                Ok(self.new_enum(type_id, discriminant, variants)?)
            }
            Constructor::Struct { fields } => Ok(Symbolic::Struct(
                fields
                    .iter()
                    .map(|f| {
                        Ok(SymbolicField {
                            name: f.name.0.clone(),
                            value: self.spec_expr(&f.value, vars)?,
                        })
                    })
                    .collect::<Result<_>>()?,
            )),
        }
    }

    fn spec_field(&mut self, name: &Ident, v: Symbolic) -> Result<Symbolic> {
        log::trace!("access field {name} from {v}", name = name.0);

        let fields = v
            .as_struct()
            .ok_or(self.error("field access from non-struct value"))?;

        let field = fields
            .iter()
            .find(|f| f.name == name.0)
            .ok_or(self.error(format!(
                "attempt to access nonexistent struct field: {}",
                name.0
            )))?;

        Ok(field.value.clone())
    }

    fn spec_discriminator(&mut self, name: &Ident, v: Symbolic) -> Result<Symbolic> {
        let e = v
            .as_enum()
            .ok_or(self.error("discriminator for non-enum value"))?;
        let variant = e.try_variant_by_name(&name.0)?;
        let discriminator = self.discriminator(e, variant);
        Ok(discriminator.into())
    }

    fn discriminator(&mut self, e: &SymbolicEnum, variant: &SymbolicVariant) -> ExprId {
        let discriminant = self.constant(Const::Int(variant.discriminant.try_into().unwrap()));
        self.exprs_equal(e.discriminant, discriminant)
    }

    fn spec_switch(
        &mut self,
        on: &spec::Expr,
        arms: &[(spec::Expr, spec::Expr)],
        vars: &Variables,
    ) -> Result<Symbolic> {
        // Generate branch arms.
        let on = self.spec_expr(on, vars)?;
        let cases = arms
            .iter()
            .map(|(value, then)| {
                let value = self.spec_expr(value, vars)?;
                let cond = self.values_equal(on.clone(), value)?;
                Ok((cond, self.spec_expr(then, vars)?))
            })
            .collect::<Result<Vec<_>>>()?;

        // Build an expression splitting over cases.
        self.cases(&cases)
    }

    fn spec_match(&mut self, on: &spec::Expr, arms: &[Arm], vars: &Variables) -> Result<Symbolic> {
        // Generate the enum value to match on.
        let on = self.spec_expr(on, vars)?;
        let e = on.as_enum().ok_or(self.error("match on non-enum value"))?;

        // Generate cases.
        let mut cases = Vec::new();
        for arm in arms {
            // Lookup the variant.
            let variant = e.try_variant_by_name(&arm.variant.0)?;

            // Arm condition is that the discriminant matches the variant.
            let cond = self.discriminator(e, variant);

            // Arm value is the result of the body expression, evaluated with
            // the variants fields brought into scope.
            let Some(fields) = variant.value.as_struct() else {
                bail!("variant {name} must have struct value", name = variant.name);
            };
            if arm.args.len() != fields.len() {
                bail!(
                    "incorrect number of arguments for variant {name}",
                    name = variant.name
                );
            }
            let mut arm_vars = vars.clone();
            for (arg, field) in zip(&arm.args, fields) {
                arm_vars.set(arg.0.clone(), field.value.clone())?;
            }
            let body = self.spec_expr(&arm.body, &arm_vars)?;

            // Add case for this match arm.
            cases.push((cond, body));
        }

        // Build an expression splitting over cases.
        self.cases(&cases)
    }

    fn cases(&mut self, cases: &[(ExprId, Symbolic)]) -> Result<Symbolic> {
        // Build an undefined fallback value.
        let Some((_, value)) = cases.last() else {
            bail!("must have at least one case");
        };
        let fallback = value.scalar_map(&mut |_| self.undef_variable());

        // Represent as nested conditionals.
        cases
            .iter()
            .rev()
            .cloned()
            .try_fold(fallback, |acc, (cond, then)| {
                self.conditional(cond, then, acc)
            })
    }

    fn spec_let(
        &mut self,
        defs: &[(Ident, spec::Expr)],
        body: &spec::Expr,
        vars: &Variables,
    ) -> Result<Symbolic> {
        // Evaluate let defs.
        let mut let_vars = vars.clone();
        for (name, expr) in defs {
            let expr = self.spec_expr(expr, &let_vars)?;
            let_vars.set(name.0.clone(), expr)?;
        }

        // Evaluate body in let-binding scope.
        self.spec_expr(body, &let_vars)
    }

    fn spec_with(
        &mut self,
        decls: &[Ident],
        body: &spec::Expr,
        vars: &Variables,
    ) -> Result<Symbolic> {
        // Declare new variables.
        let mut with_vars = vars.clone();
        for name in decls {
            let expr = Symbolic::Scalar(self.alloc_variable(Type::Unknown, name.0.clone()));
            with_vars.set(name.0.clone(), expr)?;
        }

        // Evaluate body in new scope.
        self.spec_expr(body, &with_vars)
    }

    fn spec_expand(
        &mut self,
        name: &Ident,
        args: &[spec::Expr],
        vars: &Variables,
    ) -> Result<Symbolic> {
        // Lookup macro.
        //
        // Could be an inline macro in a local variable, or a macro defined at global scope.
        let (params, body) = if let Some(v) = vars.get(&name.0) {
            let Symbolic::Macro(m) = v else {
                bail!("variable {name} is not a macro", name = name.0);
            };
            (&m.params, &m.body)
        } else {
            let defn = self
                .prog
                .specenv
                .macros
                .get(&name.0)
                .ok_or(self.error(format!("unknown macro {name}", name = name.0)))?;
            (&defn.params, &defn.body)
        };

        // Build macro expansion scope.
        let mut macro_vars = Variables::new();
        if params.len() != args.len() {
            bail!(
                "incorrect number of arguments for macro {name}",
                name = name.0
            );
        }
        for (param, arg) in zip(params, args) {
            let arg = self.spec_expr(arg, vars)?;
            macro_vars.set(param.0.clone(), arg)?;
        }

        // Evaluate macro body.
        self.spec_expr(body, &macro_vars)
    }

    fn replicate(&mut self, x: ExprId, n: usize) -> Result<ExprId> {
        match n {
            0 => bail!("cannot replicate zero times"),
            1 => Ok(x),
            _ => {
                let h = n / 2;
                let l = self.replicate(x, h)?;
                let r = self.replicate(x, n - h)?;
                Ok(self.dedup_expr(Expr::BVConcat(l, r)))
            }
        }
    }

    fn conditional(&mut self, c: ExprId, t: Symbolic, e: Symbolic) -> Result<Symbolic> {
        Symbolic::merge(&t, &e, &mut |t, e| {
            self.dedup_expr(Expr::Conditional(c, t, e))
        })
    }

    fn bindings_equal(&mut self, a: BindingId, b: BindingId) -> Result<ExprId> {
        // TODO(mbm): can this be done without clones?
        let a = self.binding_value[&a].clone();
        let b = self.binding_value[&b].clone();
        self.values_equal(a, b)
    }

    fn values_equal(&mut self, a: Symbolic, b: Symbolic) -> Result<ExprId> {
        if std::mem::discriminant(&a) != std::mem::discriminant(&b) {
            return Err(self.error(format!(
                "equality on different symbolic types: {} != {}",
                a.kind(),
                b.kind()
            )));
        }
        match (a, b) {
            (Symbolic::Scalar(u), Symbolic::Scalar(v)) => Ok(self.exprs_equal(u, v)),

            (Symbolic::Struct(us), Symbolic::Struct(vs)) => {
                // Field-wise equality.
                // TODO(mbm): can we expect that structs are the same length?
                assert_eq!(us.len(), vs.len(), "field length mismatch");
                let fields_eq = zip(us, vs)
                    .map(|(fu, fv)| {
                        assert_eq!(fu.name, fv.name, "field name mismatch");
                        self.values_equal(fu.value, fv.value)
                    })
                    .collect::<Result<_>>()?;

                // All fields must be equal.
                Ok(self.all(fields_eq))
            }

            (Symbolic::Enum(u), Symbolic::Enum(v)) => {
                // Discriminant equality.
                let discriminants_eq = self.exprs_equal(u.discriminant, v.discriminant);
                let mut equalities = vec![discriminants_eq];

                // Variant equality conditions.
                assert_eq!(u.variants.len(), v.variants.len(), "variant count mismatch");
                let variants_eq = zip(&u.variants, &v.variants)
                    .map(|(uv, vv)| {
                        assert_eq!(uv.name, vv.name, "variant name mismatch");
                        let ud = self.discriminator(&u, uv);
                        let eq = self.values_equal(uv.value.clone(), vv.value.clone())?;
                        Ok(self.dedup_expr(Expr::Imp(ud, eq)))
                    })
                    .collect::<Result<Vec<_>>>()?;
                equalities.extend(variants_eq);

                // Combine discriminant and variant conditions.
                Ok(self.all(equalities))
            }

            (Symbolic::Tuple(us), Symbolic::Tuple(vs)) => {
                // Field-wise equality.
                // TODO(mbm): can we expect that tuples are the same length?
                assert_eq!(us.len(), vs.len(), "tuple length mismatch");
                let fields_eq = zip(us, vs)
                    .map(|(u, v)| self.values_equal(u, v))
                    .collect::<Result<_>>()?;

                // All fields must be equal.
                Ok(self.all(fields_eq))
            }

            ref c => todo!("values equal: {c:?}"),
        }
    }

    fn exprs_equal(&mut self, lhs: ExprId, rhs: ExprId) -> ExprId {
        self.dedup_expr(Expr::Eq(lhs, rhs))
    }

    fn all(&mut self, exprs: Vec<ExprId>) -> ExprId {
        exprs
            .into_iter()
            .reduce(|acc, e| self.dedup_expr(Expr::And(acc, e)))
            .unwrap_or_else(|| self.boolean(true))
    }

    fn any(&mut self, exprs: Vec<ExprId>) -> ExprId {
        exprs
            .into_iter()
            .reduce(|acc, e| self.dedup_expr(Expr::Or(acc, e)))
            .unwrap_or_else(|| self.boolean(false))
    }

    fn boolean(&mut self, value: bool) -> ExprId {
        self.constant(Const::Bool(value))
    }

    fn constant(&mut self, c: Const) -> ExprId {
        self.dedup_expr(Expr::Const(c))
    }

    /// Determine the type of the given binding in the context of the
    /// [Expansion] we are constructing verification conditions for.
    fn binding_type(&self, binding: &Binding) -> BindingType {
        binding_type(
            binding,
            self.expansion.term,
            self.prog,
            |binding_id: BindingId| self.expansion.bindings[binding_id.index()].clone().unwrap(),
        )
    }

    fn alloc_binding(&mut self, binding_type: &BindingType, name: String) -> Result<Symbolic> {
        match binding_type {
            BindingType::Base(type_id) => self.alloc_model(*type_id, name),
            BindingType::Option(inner_type) => {
                let some = self.alloc_variable(Type::Bool, Variable::component_name(&name, "some"));
                let inner = Box::new(
                    self.alloc_binding(inner_type, Variable::component_name(&name, "inner"))?,
                );
                Ok(Symbolic::Option(SymbolicOption { some, inner }))
            }
            BindingType::Tuple(inners) => {
                let inners = inners
                    .iter()
                    .enumerate()
                    .map(|(i, inner_type)| {
                        self.alloc_binding(
                            inner_type,
                            Variable::component_name(&name, &i.to_string()),
                        )
                    })
                    .collect::<Result<_>>()?;
                Ok(Symbolic::Tuple(inners))
            }
        }
    }

    fn alloc_value(&mut self, ty: &Compound, name: String) -> Result<Symbolic> {
        match ty {
            Compound::Primitive(ty) => Ok(Symbolic::Scalar(self.alloc_variable(ty.clone(), name))),
            Compound::Struct(fields) => Ok(Symbolic::Struct(
                fields
                    .iter()
                    .map(|f| {
                        Ok(SymbolicField {
                            name: f.name.0.clone(),
                            value: self
                                .alloc_value(&f.ty, Variable::component_name(&name, &f.name.0))?,
                        })
                    })
                    .collect::<Result<_>>()?,
            )),
            Compound::Enum(e) => {
                let discriminant =
                    self.alloc_variable(Type::Int, Variable::component_name(&name, "discriminant"));
                let variants = e
                    .variants
                    .iter()
                    .map(|v| self.alloc_variant(v, name.clone()))
                    .collect::<Result<_>>()?;
                Ok(self.new_enum(e.id, discriminant, variants)?)
            }
            Compound::Named(_) => {
                let ty = self.prog.specenv.resolve_type(ty, &self.prog.tyenv)?;
                self.alloc_value(&ty, name)
            }
        }
    }

    fn new_enum(
        &mut self,
        ty: TypeId,
        discriminant: ExprId,
        variants: Vec<SymbolicVariant>,
    ) -> Result<Symbolic> {
        // Construct symbolic enum and ensure it's valid.
        let e = SymbolicEnum {
            ty,
            discriminant,
            variants,
        };
        e.validate()?;

        // Assume discriminant invariant: positive integer less than number of
        // variants.
        let zero = self.constant(Const::Int(0));
        let num_variants = self.constant(Const::Int(e.variants.len().try_into()?));
        let discriminant_positive = self.dedup_expr(Expr::Lte(zero, discriminant));
        let discriminant_less_than_num_variants =
            self.dedup_expr(Expr::Lt(discriminant, num_variants));
        let discriminant_in_range = self.dedup_expr(Expr::And(
            discriminant_positive,
            discriminant_less_than_num_variants,
        ));
        self.conditions.assumptions.push(discriminant_in_range);

        // Variant term instantiations.
        let ret = Symbolic::Enum(e.clone());
        for variant in &e.variants {
            let term = self.prog.get_variant_term(e.ty, variant.id);
            let args = variant.field_values()?;
            self.record_term_instantiation(term, args, ret.clone())?;
        }

        Ok(ret)
    }

    fn alloc_variant(&mut self, variant: &Variant, name: String) -> Result<SymbolicVariant> {
        let name = Variable::component_name(&name, &variant.name.0);
        Ok(SymbolicVariant {
            name: variant.name.0.clone(),
            id: variant.id,
            discriminant: variant.id.index(),
            value: self.alloc_value(&variant.ty(), name)?,
        })
    }

    fn alloc_model(&mut self, type_id: TypeId, name: String) -> Result<Symbolic> {
        let type_name = self.prog.type_name(type_id);
        let term_name = self.prog.term_name(self.expansion.term);
        let ty = self
            .prog
            .specenv
            .type_model
            .get(&type_id)
            .ok_or(self.error(format!(
                "unspecified model for type `{type_name}`: while building verification \
             conditions for term `{term_name}`, the binding `{name}` has type `{type_name}`, \
             but that type has no `(model ...)` declaration. Add a `(model {type_name} ...)` \
             form in a spec file describing its representation (for example a bitvector of \
             some width, or an enum listing its variants) so the verifier can allocate a \
             symbolic value for it."
            )))?;
        self.alloc_value(ty, name)
    }

    fn undef_variable(&mut self) -> ExprId {
        self.alloc_variable(Type::Unknown, "undef".to_string())
    }

    fn alloc_variable(&mut self, ty: Type, name: String) -> ExprId {
        let v = VariableId(self.conditions.variables.len());
        self.conditions.variables.push(Variable { ty, name });
        self.dedup_expr(Expr::Variable(v))
    }

    fn scalar(&mut self, expr: Expr) -> Symbolic {
        Symbolic::Scalar(self.dedup_expr(expr))
    }

    fn as_scalar(&self, v: Symbolic) -> Result<ExprId> {
        v.as_scalar().ok_or(self.error("expected scalar value"))
    }

    fn dedup_expr(&mut self, expr: Expr) -> ExprId {
        // Dedupe, if pure.
        let maybe_id = if expr.pure() {
            self.expr_map.get(&expr)
        } else {
            None
        };

        // Otherwise, allocate new one.
        let id = if let Some(id) = maybe_id {
            *id
        } else {
            let id = ExprId(self.conditions.exprs.len());
            self.conditions.exprs.push(expr.clone());
            self.expr_map.insert(expr, id);
            id
        };

        if let Some(pos) = self.position_stack.last() {
            self.conditions.pos.insert(id, *pos);
        }

        id
    }

    fn error(&self, msg: impl Into<String>) -> Error {
        if let Some(pos) = self.position_stack.last() {
            self.prog.error_at_pos(*pos, msg).into()
        } else {
            Error::msg(msg.into())
        }
    }
}

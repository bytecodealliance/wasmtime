use std::collections::BTreeSet;
use std::iter::FromIterator;
use std::ops;
use std::rc::Rc;

use crate::cdsl::types::{BVType, LaneType, SpecialType, ValueType};

const MAX_LANES: u16 = 256;
const MAX_BITS: u16 = 64;
const MAX_BITVEC: u16 = MAX_BITS * MAX_LANES;

/// Type variables can be used in place of concrete types when defining
/// instructions. This makes the instructions *polymorphic*.
///
/// A type variable is restricted to vary over a subset of the value types.
/// This subset is specified by a set of flags that control the permitted base
/// types and whether the type variable can assume scalar or vector types, or
/// both.
#[derive(Debug)]
pub struct TypeVarContent {
    /// Short name of type variable used in instruction descriptions.
    pub name: String,

    /// Documentation string.
    pub doc: String,

    /// Type set associated to the type variable.
    /// This field must remain private; use `get_typeset()` or `get_raw_typeset()` to get the
    /// information you want.
    type_set: Rc<TypeSet>,

    pub base: Option<TypeVarParent>,
}

#[derive(Clone, Debug)]
pub struct TypeVar {
    content: Rc<TypeVarContent>,
}

impl TypeVar {
    pub fn new(name: impl Into<String>, doc: impl Into<String>, type_set: TypeSet) -> Self {
        Self {
            content: Rc::new(TypeVarContent {
                name: name.into(),
                doc: doc.into(),
                type_set: Rc::new(type_set),
                base: None,
            }),
        }
    }

    pub fn new_singleton(value_type: ValueType) -> Self {
        let (name, doc) = (value_type.to_string(), value_type.doc());
        let mut builder = TypeSetBuilder::new();

        let (scalar_type, num_lanes) = match value_type {
            ValueType::BV(bitvec_type) => {
                let bits = bitvec_type.lane_bits() as RangeBound;
                return TypeVar::new(name, doc, builder.bitvecs(bits..bits).finish());
            }
            ValueType::Special(special_type) => {
                return TypeVar::new(name, doc, builder.specials(vec![special_type]).finish());
            }
            ValueType::Lane(lane_type) => (lane_type, 1),
            ValueType::Vector(vec_type) => {
                (vec_type.lane_type(), vec_type.lane_count() as RangeBound)
            }
        };

        builder = builder.simd_lanes(num_lanes..num_lanes);

        let builder = match scalar_type {
            LaneType::IntType(int_type) => {
                let bits = int_type as RangeBound;
                builder.ints(bits..bits)
            }
            LaneType::FloatType(float_type) => {
                let bits = float_type as RangeBound;
                builder.floats(bits..bits)
            }
            LaneType::BoolType(bool_type) => {
                let bits = bool_type as RangeBound;
                builder.bools(bits..bits)
            }
        };
        TypeVar::new(name, doc, builder.finish())
    }

    /// Returns this typevar's type set, maybe computing it from the parent.
    fn get_typeset(&self) -> Rc<TypeSet> {
        // TODO Can this be done in a non-lazy way in derived() and we can remove this function and
        // the one below?
        match &self.content.base {
            Some(base) => Rc::new(base.type_var.get_typeset().image(base.derived_func)),
            None => self.content.type_set.clone(),
        }
    }

    /// Returns this typevar's type set, assuming this type var has no parent.
    pub fn get_raw_typeset(&self) -> &TypeSet {
        assert_eq!(self.content.type_set, self.get_typeset());
        &*self.content.type_set
    }

    /// If the associated typeset has a single type return it. Otherwise return None.
    pub fn singleton_type(&self) -> Option<ValueType> {
        let type_set = self.get_typeset();
        if type_set.size() == 1 {
            Some(type_set.get_singleton())
        } else {
            None
        }
    }

    /// Get the free type variable controlling this one.
    pub fn free_typevar(&self) -> Option<TypeVar> {
        match &self.content.base {
            Some(base) => base.type_var.free_typevar(),
            None => {
                match self.singleton_type() {
                    // A singleton type isn't a proper free variable.
                    Some(_) => None,
                    None => Some(self.clone()),
                }
            }
        }
    }

    /// Create a type variable that is a function of another.
    fn derived(&self, derived_func: DerivedFunc) -> TypeVar {
        let ts = self.get_typeset();

        // Safety checks to avoid over/underflows.
        assert!(ts.specials.len() == 0, "can't derive from special types");
        match derived_func {
            DerivedFunc::HalfWidth => {
                assert!(
                    ts.ints.len() == 0 || *ts.ints.iter().min().unwrap() > 8,
                    "can't halve all integer types"
                );
                assert!(
                    ts.floats.len() == 0 || *ts.floats.iter().min().unwrap() > 32,
                    "can't halve all float types"
                );
                assert!(
                    ts.bools.len() == 0 || *ts.bools.iter().min().unwrap() > 8,
                    "can't halve all boolean types"
                );
            }
            DerivedFunc::DoubleWidth => {
                assert!(
                    ts.ints.len() == 0 || *ts.ints.iter().max().unwrap() < MAX_BITS,
                    "can't double all integer types"
                );
                assert!(
                    ts.floats.len() == 0 || *ts.floats.iter().max().unwrap() < MAX_BITS,
                    "can't double all float types"
                );
                assert!(
                    ts.bools.len() == 0 || *ts.bools.iter().max().unwrap() < MAX_BITS,
                    "can't double all boolean types"
                );
            }
            DerivedFunc::HalfVector => {
                assert!(
                    *ts.lanes.iter().min().unwrap() > 1,
                    "can't halve a scalar type"
                );
            }
            DerivedFunc::DoubleVector => {
                assert!(
                    *ts.lanes.iter().max().unwrap() < MAX_LANES,
                    "can't double 256 lanes"
                );
            }
            DerivedFunc::LaneOf | DerivedFunc::ToBitVec | DerivedFunc::AsBool => {
                /* no particular assertions */
            }
        }

        return TypeVar {
            content: Rc::new(TypeVarContent {
                name: format!("{}({})", derived_func.name(), self.name),
                doc: "".into(),
                type_set: ts,
                base: Some(TypeVarParent {
                    type_var: self.clone(),
                    derived_func,
                }),
            }),
        };
    }

    pub fn lane_of(&self) -> TypeVar {
        return self.derived(DerivedFunc::LaneOf);
    }
    pub fn as_bool(&self) -> TypeVar {
        return self.derived(DerivedFunc::AsBool);
    }
    pub fn half_width(&self) -> TypeVar {
        return self.derived(DerivedFunc::HalfWidth);
    }
    pub fn double_width(&self) -> TypeVar {
        return self.derived(DerivedFunc::DoubleWidth);
    }
    pub fn half_vector(&self) -> TypeVar {
        return self.derived(DerivedFunc::HalfVector);
    }
    pub fn double_vector(&self) -> TypeVar {
        return self.derived(DerivedFunc::DoubleVector);
    }
    pub fn to_bitvec(&self) -> TypeVar {
        return self.derived(DerivedFunc::ToBitVec);
    }
}

impl Into<TypeVar> for &TypeVar {
    fn into(self) -> TypeVar {
        self.clone()
    }
}
impl Into<TypeVar> for ValueType {
    fn into(self) -> TypeVar {
        TypeVar::new_singleton(self)
    }
}

impl PartialEq for TypeVar {
    fn eq(&self, other: &TypeVar) -> bool {
        match (&self.content.base, &other.content.base) {
            (Some(base1), Some(base2)) => base1.type_var.eq(&base2.type_var),
            (None, None) => Rc::ptr_eq(&self.content, &other.content),
            _ => false,
        }
    }
}

impl ops::Deref for TypeVar {
    type Target = TypeVarContent;
    fn deref(&self) -> &Self::Target {
        &*self.content
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DerivedFunc {
    LaneOf,
    AsBool,
    HalfWidth,
    DoubleWidth,
    HalfVector,
    DoubleVector,
    ToBitVec,
}

impl DerivedFunc {
    pub fn name(&self) -> &'static str {
        match self {
            DerivedFunc::LaneOf => "lane_of",
            DerivedFunc::AsBool => "as_bool",
            DerivedFunc::HalfWidth => "half_width",
            DerivedFunc::DoubleWidth => "double_width",
            DerivedFunc::HalfVector => "half_vector",
            DerivedFunc::DoubleVector => "double_vector",
            DerivedFunc::ToBitVec => "to_bitvec",
        }
    }
}

#[derive(Debug)]
pub struct TypeVarParent {
    pub type_var: TypeVar,
    pub derived_func: DerivedFunc,
}

/// A set of types.
///
/// We don't allow arbitrary subsets of types, but use a parametrized approach
/// instead.
///
/// Objects of this class can be used as dictionary keys.
///
/// Parametrized type sets are specified in terms of ranges:
/// - The permitted range of vector lanes, where 1 indicates a scalar type.
/// - The permitted range of integer types.
/// - The permitted range of floating point types, and
/// - The permitted range of boolean types.
///
/// The ranges are inclusive from smallest bit-width to largest bit-width.
///
/// Finally, a type set can contain special types (derived from `SpecialType`)
/// which can't appear as lane types.

type RangeBound = u16;
type Range = ops::Range<RangeBound>;
type NumSet = BTreeSet<RangeBound>;

macro_rules! num_set {
    ($($expr:expr),*) => {
        NumSet::from_iter(vec![$($expr),*])
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeSet {
    pub lanes: NumSet,
    pub ints: NumSet,
    pub floats: NumSet,
    pub bools: NumSet,
    pub bitvecs: NumSet,
    pub specials: Vec<SpecialType>,
}

impl TypeSet {
    fn new(
        lanes: NumSet,
        ints: NumSet,
        floats: NumSet,
        bools: NumSet,
        bitvecs: NumSet,
        specials: Vec<SpecialType>,
    ) -> Self {
        Self {
            lanes,
            ints,
            floats,
            bools,
            bitvecs,
            specials,
        }
    }

    /// Return the number of concrete types represented by this typeset.
    fn size(&self) -> usize {
        self.lanes.len()
            * (self.ints.len() + self.floats.len() + self.bools.len() + self.bitvecs.len())
            + self.specials.len()
    }

    /// Return the image of self across the derived function func.
    fn image(&self, derived_func: DerivedFunc) -> TypeSet {
        match derived_func {
            DerivedFunc::LaneOf => self.lane_of(),
            DerivedFunc::AsBool => self.as_bool(),
            DerivedFunc::HalfWidth => self.half_width(),
            DerivedFunc::DoubleWidth => self.double_width(),
            DerivedFunc::HalfVector => self.half_vector(),
            DerivedFunc::DoubleVector => self.double_vector(),
            DerivedFunc::ToBitVec => self.to_bitvec(),
        }
    }

    /// Return a TypeSet describing the image of self across lane_of.
    fn lane_of(&self) -> TypeSet {
        let mut copy = self.clone();
        copy.lanes = num_set![1];
        copy.bitvecs = NumSet::new();
        copy
    }

    /// Return a TypeSet describing the image of self across as_bool.
    fn as_bool(&self) -> TypeSet {
        let mut copy = self.clone();
        copy.ints = NumSet::new();
        copy.floats = NumSet::new();
        copy.bitvecs = NumSet::new();
        if (&self.lanes - &num_set![1]).len() > 0 {
            copy.bools = &self.ints | &self.floats;
            copy.bools = &copy.bools | &self.bools;
        }
        if self.lanes.contains(&1) {
            copy.bools.insert(1);
        }
        copy
    }

    /// Return a TypeSet describing the image of self across halfwidth.
    fn half_width(&self) -> TypeSet {
        let mut copy = self.clone();
        copy.ints = NumSet::from_iter(self.ints.iter().filter(|&&x| x > 8).map(|&x| x / 2));
        copy.floats = NumSet::from_iter(self.floats.iter().filter(|&&x| x > 32).map(|&x| x / 2));
        copy.bools = NumSet::from_iter(self.bools.iter().filter(|&&x| x > 8).map(|&x| x / 2));
        copy.bitvecs = NumSet::from_iter(self.bitvecs.iter().filter(|&&x| x > 1).map(|&x| x / 2));
        copy.specials = Vec::new();
        copy
    }

    /// Return a TypeSet describing the image of self across doublewidth.
    fn double_width(&self) -> TypeSet {
        let mut copy = self.clone();
        copy.ints = NumSet::from_iter(self.ints.iter().filter(|&&x| x < MAX_BITS).map(|&x| x * 2));
        copy.floats = NumSet::from_iter(
            self.floats
                .iter()
                .filter(|&&x| x < MAX_BITS)
                .map(|&x| x * 2),
        );
        copy.bools = NumSet::from_iter(
            self.bools
                .iter()
                .filter(|&&x| x < MAX_BITS)
                .map(|&x| x * 2)
                .filter(legal_bool),
        );
        copy.bitvecs = NumSet::from_iter(
            self.bitvecs
                .iter()
                .filter(|&&x| x < MAX_BITVEC)
                .map(|&x| x * 2),
        );
        copy.specials = Vec::new();
        copy
    }

    /// Return a TypeSet describing the image of self across halfvector.
    fn half_vector(&self) -> TypeSet {
        let mut copy = self.clone();
        copy.bitvecs = NumSet::new();
        copy.lanes = NumSet::from_iter(self.lanes.iter().filter(|&&x| x > 1).map(|&x| x / 2));
        copy.specials = Vec::new();
        copy
    }

    /// Return a TypeSet describing the image of self across doublevector.
    fn double_vector(&self) -> TypeSet {
        let mut copy = self.clone();
        copy.bitvecs = NumSet::new();
        copy.lanes = NumSet::from_iter(
            self.lanes
                .iter()
                .filter(|&&x| x < MAX_LANES)
                .map(|&x| x * 2),
        );
        copy.specials = Vec::new();
        copy
    }

    /// Return a TypeSet describing the image of self across to_bitvec.
    fn to_bitvec(&self) -> TypeSet {
        assert!(self.bitvecs.is_empty());
        let all_scalars = &(&self.ints | &self.floats) | &self.bools;

        let mut copy = self.clone();
        copy.lanes = num_set![1];
        copy.ints = NumSet::new();
        copy.bools = NumSet::new();
        copy.floats = NumSet::new();
        copy.bitvecs = self
            .lanes
            .iter()
            .cycle()
            .zip(all_scalars.iter())
            .map(|(num_lanes, lane_width)| num_lanes * lane_width)
            .collect();

        copy.specials = Vec::new();
        copy
    }

    fn concrete_types(&self) -> Vec<ValueType> {
        let mut ret = Vec::new();
        for &num_lanes in &self.lanes {
            for &bits in &self.ints {
                ret.push(LaneType::int_from_bits(bits).by(num_lanes));
            }
            for &bits in &self.floats {
                ret.push(LaneType::float_from_bits(bits).by(num_lanes));
            }
            for &bits in &self.bools {
                ret.push(LaneType::bool_from_bits(bits).by(num_lanes));
            }
            for &bits in &self.bitvecs {
                assert_eq!(num_lanes, 1);
                ret.push(BVType::new(bits).into());
            }
        }
        for &special in &self.specials {
            ret.push(special.into());
        }
        ret
    }

    /// Return the singleton type represented by self. Can only call on typesets containing 1 type.
    fn get_singleton(&self) -> ValueType {
        let mut types = self.concrete_types();
        assert_eq!(types.len(), 1);
        return types.remove(0);
    }
}

pub struct TypeSetBuilder {
    ints: Interval,
    floats: Interval,
    bools: Interval,
    bitvecs: Interval,
    includes_scalars: bool,
    simd_lanes: Interval,
    specials: Vec<SpecialType>,
}

impl TypeSetBuilder {
    pub fn new() -> Self {
        Self {
            ints: Interval::None,
            floats: Interval::None,
            bools: Interval::None,
            bitvecs: Interval::None,
            includes_scalars: true,
            simd_lanes: Interval::None,
            specials: Vec::new(),
        }
    }

    pub fn ints(mut self, interval: impl Into<Interval>) -> Self {
        assert!(self.ints == Interval::None);
        self.ints = interval.into();
        self
    }
    pub fn floats(mut self, interval: impl Into<Interval>) -> Self {
        assert!(self.floats == Interval::None);
        self.floats = interval.into();
        self
    }
    pub fn bools(mut self, interval: impl Into<Interval>) -> Self {
        assert!(self.bools == Interval::None);
        self.bools = interval.into();
        self
    }
    pub fn includes_scalars(mut self, includes_scalars: bool) -> Self {
        self.includes_scalars = includes_scalars;
        self
    }
    pub fn simd_lanes(mut self, interval: impl Into<Interval>) -> Self {
        assert!(self.simd_lanes == Interval::None);
        self.simd_lanes = interval.into();
        self
    }
    pub fn bitvecs(mut self, interval: impl Into<Interval>) -> Self {
        assert!(self.bitvecs == Interval::None);
        self.bitvecs = interval.into();
        self
    }
    pub fn specials(mut self, specials: Vec<SpecialType>) -> Self {
        assert!(self.specials.is_empty());
        self.specials = specials;
        self
    }

    pub fn finish(self) -> TypeSet {
        let min_lanes = if self.includes_scalars { 1 } else { 2 };
;
        let bools = range_to_set(self.bools.to_range(1..MAX_BITS, None))
            .into_iter()
            .filter(legal_bool)
            .collect();

        TypeSet::new(
            range_to_set(self.simd_lanes.to_range(min_lanes..MAX_LANES, Some(1))),
            range_to_set(self.ints.to_range(8..MAX_BITS, None)),
            range_to_set(self.floats.to_range(32..64, None)),
            bools,
            range_to_set(self.bitvecs.to_range(1..MAX_BITVEC, None)),
            self.specials,
        )
    }
}

#[derive(PartialEq)]
pub enum Interval {
    None,
    All,
    Range(Range),
}

impl Interval {
    fn to_range(&self, full_range: Range, default: Option<RangeBound>) -> Option<Range> {
        match self {
            Interval::None => {
                if let Some(default_val) = default {
                    Some(default_val..default_val)
                } else {
                    None
                }
            }

            Interval::All => Some(full_range),

            Interval::Range(range) => {
                let (low, high) = (range.start, range.end);
                assert!(low.is_power_of_two());
                assert!(high.is_power_of_two());
                assert!(low <= high);
                assert!(low >= full_range.start);
                assert!(high <= full_range.end);
                Some(low..high)
            }
        }
    }
}

impl Into<Interval> for Range {
    fn into(self) -> Interval {
        Interval::Range(self)
    }
}

fn legal_bool(bits: &RangeBound) -> bool {
    // Only allow legal bit widths for bool types.
    *bits == 1 || (*bits >= 8 && *bits <= MAX_BITS && bits.is_power_of_two())
}

/// Generates a set with all the powers of two included in the range.
fn range_to_set(range: Option<Range>) -> NumSet {
    let mut set = NumSet::new();

    let (low, high) = match range {
        Some(range) => (range.start, range.end),
        None => return set,
    };

    assert!(low.is_power_of_two());
    assert!(high.is_power_of_two());
    assert!(low <= high);

    for i in low.trailing_zeros()..high.trailing_zeros() + 1 {
        assert!(1 << i <= RangeBound::max_value());
        set.insert(1 << i);
    }
    set
}

#[test]
fn test_typevar_builder() {
    let type_set = TypeSetBuilder::new().ints(Interval::All).finish();
    assert_eq!(type_set.lanes, num_set![1]);
    assert!(type_set.floats.is_empty());
    assert_eq!(type_set.ints, num_set![8, 16, 32, 64]);
    assert!(type_set.bools.is_empty());
    assert!(type_set.bitvecs.is_empty());
    assert!(type_set.specials.is_empty());

    let type_set = TypeSetBuilder::new().bools(Interval::All).finish();
    assert_eq!(type_set.lanes, num_set![1]);
    assert!(type_set.floats.is_empty());
    assert!(type_set.ints.is_empty());
    assert_eq!(type_set.bools, num_set![1, 8, 16, 32, 64]);
    assert!(type_set.bitvecs.is_empty());
    assert!(type_set.specials.is_empty());

    let type_set = TypeSetBuilder::new().floats(Interval::All).finish();
    assert_eq!(type_set.lanes, num_set![1]);
    assert_eq!(type_set.floats, num_set![32, 64]);
    assert!(type_set.ints.is_empty());
    assert!(type_set.bools.is_empty());
    assert!(type_set.bitvecs.is_empty());
    assert!(type_set.specials.is_empty());

    let type_set = TypeSetBuilder::new()
        .floats(Interval::All)
        .simd_lanes(Interval::All)
        .includes_scalars(false)
        .finish();
    assert_eq!(type_set.lanes, num_set![2, 4, 8, 16, 32, 64, 128, 256]);
    assert_eq!(type_set.floats, num_set![32, 64]);
    assert!(type_set.ints.is_empty());
    assert!(type_set.bools.is_empty());
    assert!(type_set.bitvecs.is_empty());
    assert!(type_set.specials.is_empty());

    let type_set = TypeSetBuilder::new()
        .floats(Interval::All)
        .simd_lanes(Interval::All)
        .includes_scalars(true)
        .finish();
    assert_eq!(type_set.lanes, num_set![1, 2, 4, 8, 16, 32, 64, 128, 256]);
    assert_eq!(type_set.floats, num_set![32, 64]);
    assert!(type_set.ints.is_empty());
    assert!(type_set.bools.is_empty());
    assert!(type_set.bitvecs.is_empty());
    assert!(type_set.specials.is_empty());

    let type_set = TypeSetBuilder::new().ints(16..64).finish();
    assert_eq!(type_set.lanes, num_set![1]);
    assert_eq!(type_set.ints, num_set![16, 32, 64]);
    assert!(type_set.floats.is_empty());
    assert!(type_set.bools.is_empty());
    assert!(type_set.bitvecs.is_empty());
    assert!(type_set.specials.is_empty());
}

#[test]
#[should_panic]
fn test_typevar_builder_too_high_bound_panic() {
    TypeSetBuilder::new().ints(16..2 * MAX_BITS).finish();
}

#[test]
#[should_panic]
fn test_typevar_builder_inverted_bounds_panic() {
    TypeSetBuilder::new().ints(32..16).finish();
}

#[test]
fn test_as_bool() {
    let a = TypeSetBuilder::new()
        .simd_lanes(2..8)
        .ints(8..8)
        .floats(32..32)
        .finish();
    assert_eq!(
        a.lane_of(),
        TypeSetBuilder::new().ints(8..8).floats(32..32).finish()
    );

    // Test as_bool with disjoint intervals.
    let mut a_as_bool = TypeSetBuilder::new().simd_lanes(2..8).finish();
    a_as_bool.bools = num_set![8, 32];
    assert_eq!(a.as_bool(), a_as_bool);

    let b = TypeSetBuilder::new()
        .simd_lanes(1..8)
        .ints(8..8)
        .floats(32..32)
        .finish();
    let mut b_as_bool = TypeSetBuilder::new().simd_lanes(1..8).finish();
    b_as_bool.bools = num_set![1, 8, 32];
    assert_eq!(b.as_bool(), b_as_bool);
}

#[test]
fn test_forward_images() {
    let empty_set = TypeSetBuilder::new().finish();

    // Half vector.
    assert_eq!(
        TypeSetBuilder::new()
            .simd_lanes(1..32)
            .finish()
            .half_vector(),
        TypeSetBuilder::new().simd_lanes(1..16).finish()
    );

    // Double vector.
    assert_eq!(
        TypeSetBuilder::new()
            .simd_lanes(1..32)
            .finish()
            .double_vector(),
        TypeSetBuilder::new().simd_lanes(2..64).finish()
    );
    assert_eq!(
        TypeSetBuilder::new()
            .simd_lanes(128..256)
            .finish()
            .double_vector(),
        TypeSetBuilder::new().simd_lanes(256..256).finish()
    );

    // Half width.
    assert_eq!(
        TypeSetBuilder::new().ints(8..32).finish().half_width(),
        TypeSetBuilder::new().ints(8..16).finish()
    );
    assert_eq!(
        TypeSetBuilder::new().floats(32..32).finish().half_width(),
        empty_set
    );
    assert_eq!(
        TypeSetBuilder::new().floats(32..64).finish().half_width(),
        TypeSetBuilder::new().floats(32..32).finish()
    );
    assert_eq!(
        TypeSetBuilder::new().bools(1..8).finish().half_width(),
        empty_set
    );
    assert_eq!(
        TypeSetBuilder::new().bools(1..32).finish().half_width(),
        TypeSetBuilder::new().bools(8..16).finish()
    );

    // Double width.
    assert_eq!(
        TypeSetBuilder::new().ints(8..32).finish().double_width(),
        TypeSetBuilder::new().ints(16..64).finish()
    );
    assert_eq!(
        TypeSetBuilder::new().ints(32..64).finish().double_width(),
        TypeSetBuilder::new().ints(64..64).finish()
    );
    assert_eq!(
        TypeSetBuilder::new().floats(32..32).finish().double_width(),
        TypeSetBuilder::new().floats(64..64).finish()
    );
    assert_eq!(
        TypeSetBuilder::new().floats(32..64).finish().double_width(),
        TypeSetBuilder::new().floats(64..64).finish()
    );
    assert_eq!(
        TypeSetBuilder::new().bools(1..16).finish().double_width(),
        TypeSetBuilder::new().bools(16..32).finish()
    );
    assert_eq!(
        TypeSetBuilder::new().bools(32..64).finish().double_width(),
        TypeSetBuilder::new().bools(64..64).finish()
    );
}

#[test]
#[should_panic]
fn test_typeset_singleton_panic_nonsingleton_types() {
    TypeSetBuilder::new()
        .ints(8..8)
        .floats(32..32)
        .finish()
        .get_singleton();
}

#[test]
#[should_panic]
fn test_typeset_singleton_panic_nonsingleton_lanes() {
    TypeSetBuilder::new()
        .simd_lanes(1..2)
        .floats(32..32)
        .finish()
        .get_singleton();
}

#[test]
fn test_typeset_singleton() {
    use crate::shared::types as shared_types;
    assert_eq!(
        TypeSetBuilder::new().ints(16..16).finish().get_singleton(),
        ValueType::Lane(shared_types::Int::I16.into())
    );
    assert_eq!(
        TypeSetBuilder::new()
            .floats(64..64)
            .finish()
            .get_singleton(),
        ValueType::Lane(shared_types::Float::F64.into())
    );
    assert_eq!(
        TypeSetBuilder::new().bools(1..1).finish().get_singleton(),
        ValueType::Lane(shared_types::Bool::B1.into())
    );
    assert_eq!(
        TypeSetBuilder::new()
            .simd_lanes(4..4)
            .ints(32..32)
            .finish()
            .get_singleton(),
        LaneType::from(shared_types::Int::I32).by(4)
    );
}

#[test]
fn test_typevar_functions() {
    let x = TypeVar::new(
        "x",
        "i16 and up",
        TypeSetBuilder::new().ints(16..64).finish(),
    );
    assert_eq!(x.half_width().name, "half_width(x)");
    assert_eq!(
        x.half_width().double_width().name,
        "double_width(half_width(x))"
    );

    let x = TypeVar::new("x", "up to i32", TypeSetBuilder::new().ints(8..32).finish());
    assert_eq!(x.double_width().name, "double_width(x)");
}

#[test]
fn test_typevar_singleton() {
    use crate::cdsl::types::VectorType;
    use crate::shared::types as shared_types;

    // Test i32.
    let typevar =
        TypeVar::new_singleton(ValueType::Lane(LaneType::IntType(shared_types::Int::I32)));
    assert_eq!(typevar.name, "i32");
    assert_eq!(typevar.type_set.ints, num_set![32]);
    assert!(typevar.type_set.floats.is_empty());
    assert!(typevar.type_set.bools.is_empty());
    assert!(typevar.type_set.bitvecs.is_empty());
    assert!(typevar.type_set.specials.is_empty());
    assert_eq!(typevar.type_set.lanes, num_set![1]);

    // Test f32x4.
    let typevar = TypeVar::new_singleton(ValueType::Vector(VectorType::new(
        LaneType::FloatType(shared_types::Float::F32),
        4,
    )));
    assert_eq!(typevar.name, "f32x4");
    assert!(typevar.type_set.ints.is_empty());
    assert_eq!(typevar.type_set.floats, num_set![32]);
    assert_eq!(typevar.type_set.lanes, num_set![4]);
    assert!(typevar.type_set.bools.is_empty());
    assert!(typevar.type_set.bitvecs.is_empty());
    assert!(typevar.type_set.specials.is_empty());
}

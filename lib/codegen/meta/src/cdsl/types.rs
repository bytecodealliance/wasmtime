//! Cranelift ValueType hierarchy

// Temporary disabled: Unused at the moment.
// use std::collections::HashMap;

use std::fmt;

use base::types as base_types;

// Numbering scheme for value types:
//
// 0: Void
// 0x01-0x6f: Special types
// 0x70-0x7f: Lane types
// 0x80-0xff: Vector types
//
// Vector types are encoded with the lane type in the low 4 bits and log2(lanes)
// in the high 4 bits, giving a range of 2-256 lanes.
static LANE_BASE: u8 = 0x70;

// Rust name prefix used for the `rust_name` method.
static _RUST_NAME_PREFIX: &'static str = "ir::types::";

// ValueType variants (i8, i32, ...) are provided in `base::types.rs`.

/// A concrete SSA value type.
///
/// All SSA values have a type that is described by an instance of `ValueType`
/// or one of its subclasses.
#[derive(Debug)]
pub enum ValueType {
    BV(BVType),
    Lane(LaneType),
    Special(SpecialType),
    Vector(VectorType),
}

impl ValueType {
    /// Iterate through all of the lane types.
    pub fn all_lane_types() -> LaneTypeIterator {
        LaneTypeIterator::new()
    }

    /// Iterate through all of the special types (neither lanes nor vectors).
    pub fn all_special_types() -> SpecialTypeIterator {
        SpecialTypeIterator::new()
    }

    /// Return a string containing the documentation comment for this type.
    pub fn doc(&self) -> String {
        match *self {
            ValueType::BV(ref b) => b.doc(),
            ValueType::Lane(l) => l.doc(),
            ValueType::Special(s) => s.doc(),
            ValueType::Vector(ref v) => v.doc(),
        }
    }

    /// Return the number of bits in a lane.
    pub fn lane_bits(&self) -> u64 {
        match *self {
            ValueType::BV(ref b) => b.lane_bits(),
            ValueType::Lane(l) => l.lane_bits(),
            ValueType::Special(s) => s.lane_bits(),
            ValueType::Vector(ref v) => v.lane_bits(),
        }
    }

    /// Return the number of lanes.
    pub fn lane_count(&self) -> u64 {
        match *self {
            ValueType::Vector(ref v) => v.lane_count(),
            _ => 1,
        }
    }

    /// Find the number of bytes that this type occupies in memory.
    pub fn membytes(&self) -> u64 {
        self.width() / 8
    }

    /// Get the name of this type.
    pub fn name(&self) -> String {
        match *self {
            ValueType::BV(ref b) => b.name(),
            ValueType::Lane(l) => l.name(),
            ValueType::Special(s) => s.name(),
            ValueType::Vector(ref v) => v.name(),
        }
    }

    /// Find the unique number associated with this type.
    pub fn number(&self) -> Option<u8> {
        match *self {
            ValueType::BV(_) => None,
            ValueType::Lane(l) => Some(l.number()),
            ValueType::Special(s) => Some(s.number()),
            ValueType::Vector(ref v) => Some(v.number()),
        }
    }

    /// Return the name of this type for generated Rust source files.
    pub fn _rust_name(&self) -> String {
        format!("{}{}", _RUST_NAME_PREFIX, self.name().to_uppercase())
    }

    /// Return true iff:
    ///     1. self and other have equal number of lanes
    ///     2. each lane in self has at least as many bits as a lane in other
    pub fn _wider_or_equal(&self, rhs: &ValueType) -> bool {
        (self.lane_count() == rhs.lane_count()) && (self.lane_bits() >= rhs.lane_bits())
    }

    /// Return the total number of bits of an instance of this type.
    pub fn width(&self) -> u64 {
        self.lane_count() * self.lane_bits()
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Create a ValueType from a given bitvector type.
impl From<BVType> for ValueType {
    fn from(bv: BVType) -> Self {
        ValueType::BV(bv)
    }
}

/// Create a ValueType from a given lane type.
impl From<LaneType> for ValueType {
    fn from(lane: LaneType) -> Self {
        ValueType::Lane(lane)
    }
}

/// Create a ValueType from a given special type.
impl From<SpecialType> for ValueType {
    fn from(spec: SpecialType) -> Self {
        ValueType::Special(spec)
    }
}

/// Create a ValueType from a given vector type.
impl From<VectorType> for ValueType {
    fn from(vector: VectorType) -> Self {
        ValueType::Vector(vector)
    }
}

/// A concrete scalar type that can appear as a vector lane too.
#[derive(Clone, Copy)]
pub enum LaneType {
    BoolType(base_types::Bool),
    FloatType(base_types::Float),
    IntType(base_types::Int),
}

impl LaneType {
    /// Return a string containing the documentation comment for this lane type.
    pub fn doc(&self) -> String {
        match *self {
            LaneType::BoolType(_) => format!("A boolean type with {} bits.", self.lane_bits()),
            LaneType::FloatType(base_types::Float::F32) => String::from(
                "A 32-bit floating point type represented in the IEEE 754-2008
                *binary32* interchange format. This corresponds to the :c:type:`float`
                type in most C implementations.",
            ),
            LaneType::FloatType(base_types::Float::F64) => String::from(
                "A 64-bit floating point type represented in the IEEE 754-2008
                *binary64* interchange format. This corresponds to the :c:type:`double`
                type in most C implementations.",
            ),
            LaneType::IntType(_) if self.lane_bits() < 32 => format!(
                "An integer type with {} bits.
                WARNING: arithmetic on {}bit integers is incomplete",
                self.lane_bits(),
                self.lane_bits()
            ),
            LaneType::IntType(_) => format!("An integer type with {} bits.", self.lane_bits()),
        }
    }

    /// Return the number of bits in a lane.
    pub fn lane_bits(&self) -> u64 {
        match *self {
            LaneType::BoolType(ref b) => *b as u64,
            LaneType::FloatType(ref f) => *f as u64,
            LaneType::IntType(ref i) => *i as u64,
        }
    }

    /// Get the name of this lane type.
    pub fn name(&self) -> String {
        match *self {
            LaneType::BoolType(_) => format!("b{}", self.lane_bits()),
            LaneType::FloatType(_) => format!("f{}", self.lane_bits()),
            LaneType::IntType(_) => format!("i{}", self.lane_bits()),
        }
    }

    /// Find the unique number associated with this lane type.
    pub fn number(&self) -> u8 {
        LANE_BASE + match *self {
            LaneType::BoolType(base_types::Bool::B1) => 0,
            LaneType::BoolType(base_types::Bool::B8) => 1,
            LaneType::BoolType(base_types::Bool::B16) => 2,
            LaneType::BoolType(base_types::Bool::B32) => 3,
            LaneType::BoolType(base_types::Bool::B64) => 4,
            LaneType::IntType(base_types::Int::I8) => 5,
            LaneType::IntType(base_types::Int::I16) => 6,
            LaneType::IntType(base_types::Int::I32) => 7,
            LaneType::IntType(base_types::Int::I64) => 8,
            LaneType::FloatType(base_types::Float::F32) => 9,
            LaneType::FloatType(base_types::Float::F64) => 10,
        }
    }
}

impl fmt::Debug for LaneType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner_msg = format!("bits={}", self.lane_bits());
        write!(
            f,
            "{}",
            match *self {
                LaneType::BoolType(_) => format!("BoolType({})", inner_msg),
                LaneType::FloatType(_) => format!("FloatType({})", inner_msg),
                LaneType::IntType(_) => format!("IntType({})", inner_msg),
            }
        )
    }
}

/// Create a LaneType from a given bool variant.
impl From<base_types::Bool> for LaneType {
    fn from(b: base_types::Bool) -> Self {
        LaneType::BoolType(b)
    }
}

/// Create a LaneType from a given float variant.
impl From<base_types::Float> for LaneType {
    fn from(f: base_types::Float) -> Self {
        LaneType::FloatType(f)
    }
}

/// Create a LaneType from a given int variant.
impl From<base_types::Int> for LaneType {
    fn from(i: base_types::Int) -> Self {
        LaneType::IntType(i)
    }
}

/// An iterator for different lane types.
pub struct LaneTypeIterator {
    bool_iter: base_types::BoolIterator,
    int_iter: base_types::IntIterator,
    float_iter: base_types::FloatIterator,
}

impl LaneTypeIterator {
    /// Create a new lane type iterator.
    fn new() -> Self {
        Self {
            bool_iter: base_types::BoolIterator::new(),
            int_iter: base_types::IntIterator::new(),
            float_iter: base_types::FloatIterator::new(),
        }
    }
}

impl Iterator for LaneTypeIterator {
    type Item = LaneType;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(b) = self.bool_iter.next() {
            Some(LaneType::from(b))
        } else if let Some(i) = self.int_iter.next() {
            Some(LaneType::from(i))
        } else if let Some(f) = self.float_iter.next() {
            Some(LaneType::from(f))
        } else {
            None
        }
    }
}

/// A concrete SIMD vector type.
///
/// A vector type has a lane type which is an instance of `LaneType`,
/// and a positive number of lanes.
pub struct VectorType {
    base: LaneType,
    lanes: u64,
}

impl VectorType {
    /// Initialize a new integer type with `n` bits.
    pub fn new(base: LaneType, lanes: u64) -> VectorType {
        VectorType { base, lanes }
    }

    /// Return a string containing the documentation comment for this vector type.
    pub fn doc(&self) -> String {
        format!(
            "A SIMD vector with {} lanes containing a `{}` each.",
            self.lane_count(),
            self.base.name()
        )
    }

    /// Return the number of bits in a lane.
    pub fn lane_bits(&self) -> u64 {
        self.base.lane_bits()
    }

    /// Return the number of lanes.
    pub fn lane_count(&self) -> u64 {
        self.lanes
    }

    /// Get the name of this vector type.
    pub fn name(&self) -> String {
        format!("{}x{}", self.base.name(), self.lane_count())
    }

    /// Find the unique number associated with this vector type.
    ///
    /// Vector types are encoded with the lane type in the low 4 bits and
    /// log2(lanes) in the high 4 bits, giving a range of 2-256 lanes.
    pub fn number(&self) -> u8 {
        let lanes_log_2: u32 = 63 - self.lane_count().leading_zeros();
        let base_num = u32::from(self.base.number());
        let num = (lanes_log_2 << 4) + base_num;
        num as u8
    }
}

impl fmt::Debug for VectorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VectorType(base={}, lanes={})",
            self.base.name(),
            self.lane_count()
        )
    }
}

/// A flat bitvector type. Used for semantics description only.
pub struct BVType {
    bits: u64,
}

impl BVType {
    /// Initialize a new bitvector type with `n` bits.
    pub fn _new(bits: u64) -> Self {
        Self { bits }
    }

    /// Return a string containing the documentation comment for this bitvector type.
    pub fn doc(&self) -> String {
        format!("A bitvector type with {} bits.", self.bits)
    }

    /// Return the number of bits in a lane.
    pub fn lane_bits(&self) -> u64 {
        self.bits
    }

    /// Get the name of this bitvector type.
    pub fn name(&self) -> String {
        format!("bv{}", self.bits)
    }
}

impl fmt::Debug for BVType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BVType(bits={})", self.lane_bits())
    }
}

/// A concrete scalar type that is neither a vector nor a lane type.
///
/// Special types cannot be used to form vectors.
#[derive(Clone, Copy)]
pub enum SpecialType {
    Flag(base_types::Flag),
}

impl SpecialType {
    /// Return a string containing the documentation comment for this special type.
    pub fn doc(&self) -> String {
        match *self {
            SpecialType::Flag(base_types::Flag::IFlags) => String::from(
                "CPU flags representing the result of an integer comparison. These flags
                can be tested with an :type:`intcc` condition code.",
            ),
            SpecialType::Flag(base_types::Flag::FFlags) => String::from(
                "CPU flags representing the result of a floating point comparison. These
                flags can be tested with a :type:`floatcc` condition code.",
            ),
        }
    }

    /// Return the number of bits in a lane.
    pub fn lane_bits(&self) -> u64 {
        match *self {
            SpecialType::Flag(_) => 0,
        }
    }

    /// Get the name of this special type.
    pub fn name(&self) -> String {
        match *self {
            SpecialType::Flag(base_types::Flag::IFlags) => "iflags".to_string(),
            SpecialType::Flag(base_types::Flag::FFlags) => "fflags".to_string(),
        }
    }

    /// Find the unique number associated with this special type.
    pub fn number(&self) -> u8 {
        match *self {
            SpecialType::Flag(base_types::Flag::IFlags) => 1,
            SpecialType::Flag(base_types::Flag::FFlags) => 2,
        }
    }
}

impl fmt::Debug for SpecialType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                SpecialType::Flag(_) => format!("FlagsType({})", self.name()),
            }
        )
    }
}

impl From<base_types::Flag> for SpecialType {
    fn from(f: base_types::Flag) -> Self {
        SpecialType::Flag(f)
    }
}

pub struct SpecialTypeIterator {
    flag_iter: base_types::FlagIterator,
}

impl SpecialTypeIterator {
    fn new() -> Self {
        Self {
            flag_iter: base_types::FlagIterator::new(),
        }
    }
}

impl Iterator for SpecialTypeIterator {
    type Item = SpecialType;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(f) = self.flag_iter.next() {
            Some(SpecialType::from(f))
        } else {
            None
        }
    }
}

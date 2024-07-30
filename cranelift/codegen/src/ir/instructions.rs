//! Instruction formats and opcodes.
//!
//! The `instructions` module contains definitions for instruction formats, opcodes, and the
//! in-memory representation of IR instructions.
//!
//! A large part of this module is auto-generated from the instruction descriptions in the meta
//! directory.

use crate::constant_hash::Table;
use alloc::vec::Vec;
use core::fmt::{self, Display, Formatter};
use core::ops::{Deref, DerefMut};
use core::str::FromStr;

#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

use crate::bitset::ScalarBitSet;
use crate::entity;
use crate::ir::{
    self,
    condcodes::{FloatCC, IntCC},
    trapcode::TrapCode,
    types, Block, FuncRef, MemFlags, SigRef, StackSlot, Type, Value,
};

/// Some instructions use an external list of argument values because there is not enough space in
/// the 16-byte `InstructionData` struct. These value lists are stored in a memory pool in
/// `dfg.value_lists`.
pub type ValueList = entity::EntityList<Value>;

/// Memory pool for holding value lists. See `ValueList`.
pub type ValueListPool = entity::ListPool<Value>;

/// A pair of a Block and its arguments, stored in a single EntityList internally.
///
/// NOTE: We don't expose either value_to_block or block_to_value outside of this module because
/// this operation is not generally safe. However, as the two share the same underlying layout,
/// they can be stored in the same value pool.
///
/// BlockCall makes use of this shared layout by storing all of its contents (a block and its
/// argument) in a single EntityList. This is a bit better than introducing a new entity type for
/// the pair of a block name and the arguments entity list, as we don't pay any indirection penalty
/// to get to the argument values -- they're stored in-line with the block in the same list.
///
/// The BlockCall::new function guarantees this layout by requiring a block argument that's written
/// in as the first element of the EntityList. Any subsequent entries are always assumed to be real
/// Values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct BlockCall {
    /// The underlying storage for the BlockCall. The first element of the values EntityList is
    /// guaranteed to always be a Block encoded as a Value via BlockCall::block_to_value.
    /// Consequently, the values entity list is never empty.
    values: entity::EntityList<Value>,
}

impl BlockCall {
    // NOTE: the only uses of this function should be internal to BlockCall. See the block comment
    // on BlockCall for more context.
    fn value_to_block(val: Value) -> Block {
        Block::from_u32(val.as_u32())
    }

    // NOTE: the only uses of this function should be internal to BlockCall. See the block comment
    // on BlockCall for more context.
    fn block_to_value(block: Block) -> Value {
        Value::from_u32(block.as_u32())
    }

    /// Construct a BlockCall with the given block and arguments.
    pub fn new(block: Block, args: &[Value], pool: &mut ValueListPool) -> Self {
        let mut values = ValueList::default();
        values.push(Self::block_to_value(block), pool);
        values.extend(args.iter().copied(), pool);
        Self { values }
    }

    /// Return the block for this BlockCall.
    pub fn block(&self, pool: &ValueListPool) -> Block {
        let val = self.values.first(pool).unwrap();
        Self::value_to_block(val)
    }

    /// Replace the block for this BlockCall.
    pub fn set_block(&mut self, block: Block, pool: &mut ValueListPool) {
        *self.values.get_mut(0, pool).unwrap() = Self::block_to_value(block);
    }

    /// Append an argument to the block args.
    pub fn append_argument(&mut self, arg: Value, pool: &mut ValueListPool) {
        self.values.push(arg, pool);
    }

    /// Return a slice for the arguments of this block.
    pub fn args_slice<'a>(&self, pool: &'a ValueListPool) -> &'a [Value] {
        &self.values.as_slice(pool)[1..]
    }

    /// Return a slice for the arguments of this block.
    pub fn args_slice_mut<'a>(&'a mut self, pool: &'a mut ValueListPool) -> &'a mut [Value] {
        &mut self.values.as_mut_slice(pool)[1..]
    }

    /// Remove the argument at ix from the argument list.
    pub fn remove(&mut self, ix: usize, pool: &mut ValueListPool) {
        self.values.remove(1 + ix, pool)
    }

    /// Clear out the arguments list.
    pub fn clear(&mut self, pool: &mut ValueListPool) {
        self.values.truncate(1, pool)
    }

    /// Appends multiple elements to the arguments.
    pub fn extend<I>(&mut self, elements: I, pool: &mut ValueListPool)
    where
        I: IntoIterator<Item = Value>,
    {
        self.values.extend(elements, pool)
    }

    /// Return a value that can display this block call.
    pub fn display<'a>(&self, pool: &'a ValueListPool) -> DisplayBlockCall<'a> {
        DisplayBlockCall { block: *self, pool }
    }

    /// Deep-clone the underlying list in the same pool. The returned
    /// list will have identical contents but changes to this list
    /// will not change its contents or vice-versa.
    pub fn deep_clone(&self, pool: &mut ValueListPool) -> Self {
        Self {
            values: self.values.deep_clone(pool),
        }
    }
}

/// Wrapper for the context needed to display a [BlockCall] value.
pub struct DisplayBlockCall<'a> {
    block: BlockCall,
    pool: &'a ValueListPool,
}

impl<'a> Display for DisplayBlockCall<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.block.block(&self.pool))?;
        let args = self.block.args_slice(&self.pool);
        if !args.is_empty() {
            write!(f, "(")?;
            for (ix, arg) in args.iter().enumerate() {
                if ix > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

// Include code generated by `cranelift-codegen/meta/src/gen_inst.rs`. This file contains:
//
// - The `pub enum InstructionFormat` enum with all the instruction formats.
// - The `pub enum InstructionData` enum with all the instruction data fields.
// - The `pub enum Opcode` definition with all known opcodes,
// - The `const OPCODE_FORMAT: [InstructionFormat; N]` table.
// - The private `fn opcode_name(Opcode) -> &'static str` function, and
// - The hash table `const OPCODE_HASH_TABLE: [Opcode; N]`.
//
// For value type constraints:
//
// - The `const OPCODE_CONSTRAINTS : [OpcodeConstraints; N]` table.
// - The `const TYPE_SETS : [ValueTypeSet; N]` table.
// - The `const OPERAND_CONSTRAINTS : [OperandConstraint; N]` table.
//
include!(concat!(env!("OUT_DIR"), "/opcodes.rs"));

impl Display for Opcode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", opcode_name(*self))
    }
}

impl Opcode {
    /// Get the instruction format for this opcode.
    pub fn format(self) -> InstructionFormat {
        OPCODE_FORMAT[self as usize - 1]
    }

    /// Get the constraint descriptor for this opcode.
    /// Panic if this is called on `NotAnOpcode`.
    pub fn constraints(self) -> OpcodeConstraints {
        OPCODE_CONSTRAINTS[self as usize - 1]
    }
}

// This trait really belongs in cranelift-reader where it is used by the `.clif` file parser, but since
// it critically depends on the `opcode_name()` function which is needed here anyway, it lives in
// this module. This also saves us from running the build script twice to generate code for the two
// separate crates.
impl FromStr for Opcode {
    type Err = &'static str;

    /// Parse an Opcode name from a string.
    fn from_str(s: &str) -> Result<Self, &'static str> {
        use crate::constant_hash::{probe, simple_hash};

        match probe::<&str, [Option<Self>]>(&OPCODE_HASH_TABLE, s, simple_hash(s)) {
            Err(_) => Err("Unknown opcode"),
            // We unwrap here because probe() should have ensured that the entry
            // at this index is not None.
            Ok(i) => Ok(OPCODE_HASH_TABLE[i].unwrap()),
        }
    }
}

impl<'a> Table<&'a str> for [Option<Opcode>] {
    fn len(&self) -> usize {
        self.len()
    }

    fn key(&self, idx: usize) -> Option<&'a str> {
        self[idx].map(opcode_name)
    }
}

/// A variable list of `Value` operands used for function call arguments and passing arguments to
/// basic blocks.
#[derive(Clone, Debug)]
pub struct VariableArgs(Vec<Value>);

impl VariableArgs {
    /// Create an empty argument list.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Add an argument to the end.
    pub fn push(&mut self, v: Value) {
        self.0.push(v)
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Convert this to a value list in `pool` with `fixed` prepended.
    pub fn into_value_list(self, fixed: &[Value], pool: &mut ValueListPool) -> ValueList {
        let mut vlist = ValueList::default();
        vlist.extend(fixed.iter().cloned(), pool);
        vlist.extend(self.0, pool);
        vlist
    }
}

// Coerce `VariableArgs` into a `&[Value]` slice.
impl Deref for VariableArgs {
    type Target = [Value];

    fn deref(&self) -> &[Value] {
        &self.0
    }
}

impl DerefMut for VariableArgs {
    fn deref_mut(&mut self) -> &mut [Value] {
        &mut self.0
    }
}

impl Display for VariableArgs {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        for (i, val) in self.0.iter().enumerate() {
            if i == 0 {
                write!(fmt, "{}", val)?;
            } else {
                write!(fmt, ", {}", val)?;
            }
        }
        Ok(())
    }
}

impl Default for VariableArgs {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyzing an instruction.
///
/// Avoid large matches on instruction formats by using the methods defined here to examine
/// instructions.
impl InstructionData {
    /// Get the destinations of this instruction, if it's a branch.
    ///
    /// `br_table` returns the empty slice.
    pub fn branch_destination<'a>(&'a self, jump_tables: &'a ir::JumpTables) -> &[BlockCall] {
        match self {
            Self::Jump {
                ref destination, ..
            } => std::slice::from_ref(destination),
            Self::Brif { blocks, .. } => blocks.as_slice(),
            Self::BranchTable { table, .. } => jump_tables.get(*table).unwrap().all_branches(),
            _ => {
                debug_assert!(!self.opcode().is_branch());
                &[]
            }
        }
    }

    /// Get a mutable slice of the destinations of this instruction, if it's a branch.
    ///
    /// `br_table` returns the empty slice.
    pub fn branch_destination_mut<'a>(
        &'a mut self,
        jump_tables: &'a mut ir::JumpTables,
    ) -> &mut [BlockCall] {
        match self {
            Self::Jump {
                ref mut destination,
                ..
            } => std::slice::from_mut(destination),
            Self::Brif { blocks, .. } => blocks.as_mut_slice(),
            Self::BranchTable { table, .. } => {
                jump_tables.get_mut(*table).unwrap().all_branches_mut()
            }
            _ => {
                debug_assert!(!self.opcode().is_branch());
                &mut []
            }
        }
    }

    /// Replace the values used in this instruction according to the given
    /// function.
    pub fn map_values(
        &mut self,
        pool: &mut ValueListPool,
        jump_tables: &mut ir::JumpTables,
        mut f: impl FnMut(Value) -> Value,
    ) {
        for arg in self.arguments_mut(pool) {
            *arg = f(*arg);
        }

        for block in self.branch_destination_mut(jump_tables) {
            for arg in block.args_slice_mut(pool) {
                *arg = f(*arg);
            }
        }
    }

    /// If this is a trapping instruction, get its trap code. Otherwise, return
    /// `None`.
    pub fn trap_code(&self) -> Option<TrapCode> {
        match *self {
            Self::CondTrap { code, .. } | Self::Trap { code, .. } => Some(code),
            _ => None,
        }
    }

    /// If this is a control-flow instruction depending on an integer condition, gets its
    /// condition.  Otherwise, return `None`.
    pub fn cond_code(&self) -> Option<IntCC> {
        match self {
            &InstructionData::IntCompare { cond, .. }
            | &InstructionData::IntCompareImm { cond, .. } => Some(cond),
            _ => None,
        }
    }

    /// If this is a control-flow instruction depending on a floating-point condition, gets its
    /// condition.  Otherwise, return `None`.
    pub fn fp_cond_code(&self) -> Option<FloatCC> {
        match self {
            &InstructionData::FloatCompare { cond, .. } => Some(cond),
            _ => None,
        }
    }

    /// If this is a trapping instruction, get an exclusive reference to its
    /// trap code. Otherwise, return `None`.
    pub fn trap_code_mut(&mut self) -> Option<&mut TrapCode> {
        match self {
            Self::CondTrap { code, .. } | Self::Trap { code, .. } => Some(code),
            _ => None,
        }
    }

    /// If this is an atomic read/modify/write instruction, return its subopcode.
    pub fn atomic_rmw_op(&self) -> Option<ir::AtomicRmwOp> {
        match self {
            &InstructionData::AtomicRmw { op, .. } => Some(op),
            _ => None,
        }
    }

    /// If this is a load/store instruction, returns its immediate offset.
    pub fn load_store_offset(&self) -> Option<i32> {
        match self {
            &InstructionData::Load { offset, .. }
            | &InstructionData::StackLoad { offset, .. }
            | &InstructionData::Store { offset, .. }
            | &InstructionData::StackStore { offset, .. } => Some(offset.into()),
            _ => None,
        }
    }

    /// If this is a load/store instruction, return its memory flags.
    pub fn memflags(&self) -> Option<MemFlags> {
        match self {
            &InstructionData::Load { flags, .. }
            | &InstructionData::LoadNoOffset { flags, .. }
            | &InstructionData::Store { flags, .. }
            | &InstructionData::StoreNoOffset { flags, .. }
            | &InstructionData::AtomicCas { flags, .. }
            | &InstructionData::AtomicRmw { flags, .. } => Some(flags),
            _ => None,
        }
    }

    /// If this instruction references a stack slot, return it
    pub fn stack_slot(&self) -> Option<StackSlot> {
        match self {
            &InstructionData::StackStore { stack_slot, .. }
            | &InstructionData::StackLoad { stack_slot, .. } => Some(stack_slot),
            _ => None,
        }
    }

    /// Return information about a call instruction.
    ///
    /// Any instruction that can call another function reveals its call signature here.
    pub fn analyze_call<'a>(&'a self, pool: &'a ValueListPool) -> CallInfo<'a> {
        match *self {
            Self::Call {
                func_ref, ref args, ..
            } => CallInfo::Direct(func_ref, args.as_slice(pool)),
            Self::CallIndirect {
                sig_ref, ref args, ..
            } => CallInfo::Indirect(sig_ref, &args.as_slice(pool)[1..]),
            _ => {
                debug_assert!(!self.opcode().is_call());
                CallInfo::NotACall
            }
        }
    }

    #[inline]
    pub(crate) fn mask_immediates(&mut self, ctrl_typevar: Type) {
        if ctrl_typevar.is_invalid() {
            return;
        }

        let bit_width = ctrl_typevar.bits();

        match self {
            Self::UnaryImm { opcode: _, imm } => {
                imm.mask_to_width(bit_width);
            }
            Self::BinaryImm64 {
                opcode,
                arg: _,
                imm,
            } => {
                if *opcode == Opcode::SdivImm || *opcode == Opcode::SremImm {
                    imm.mask_to_width(bit_width);
                }
            }
            Self::IntCompareImm {
                opcode,
                arg: _,
                cond,
                imm,
            } => {
                debug_assert_eq!(*opcode, Opcode::IcmpImm);
                if cond.unsigned() != *cond {
                    imm.mask_to_width(bit_width);
                }
            }
            _ => {}
        }
    }
}

/// Information about call instructions.
pub enum CallInfo<'a> {
    /// This is not a call instruction.
    NotACall,

    /// This is a direct call to an external function declared in the preamble. See
    /// `DataFlowGraph.ext_funcs`.
    Direct(FuncRef, &'a [Value]),

    /// This is an indirect call with the specified signature. See `DataFlowGraph.signatures`.
    Indirect(SigRef, &'a [Value]),
}

/// Value type constraints for a given opcode.
///
/// The `InstructionFormat` determines the constraints on most operands, but `Value` operands and
/// results are not determined by the format. Every `Opcode` has an associated
/// `OpcodeConstraints` object that provides the missing details.
#[derive(Clone, Copy)]
pub struct OpcodeConstraints {
    /// Flags for this opcode encoded as a bit field:
    ///
    /// Bits 0-2:
    ///     Number of fixed result values. This does not include `variable_args` results as are
    ///     produced by call instructions.
    ///
    /// Bit 3:
    ///     This opcode is polymorphic and the controlling type variable can be inferred from the
    ///     designated input operand. This is the `typevar_operand` index given to the
    ///     `InstructionFormat` meta language object. When this bit is not set, the controlling
    ///     type variable must be the first output value instead.
    ///
    /// Bit 4:
    ///     This opcode is polymorphic and the controlling type variable does *not* appear as the
    ///     first result type.
    ///
    /// Bits 5-7:
    ///     Number of fixed value arguments. The minimum required number of value operands.
    flags: u8,

    /// Permitted set of types for the controlling type variable as an index into `TYPE_SETS`.
    typeset_offset: u8,

    /// Offset into `OPERAND_CONSTRAINT` table of the descriptors for this opcode. The first
    /// `num_fixed_results()` entries describe the result constraints, then follows constraints for
    /// the fixed `Value` input operands. (`num_fixed_value_arguments()` of them).
    constraint_offset: u16,
}

impl OpcodeConstraints {
    /// Can the controlling type variable for this opcode be inferred from the designated value
    /// input operand?
    /// This also implies that this opcode is polymorphic.
    pub fn use_typevar_operand(self) -> bool {
        (self.flags & 0x8) != 0
    }

    /// Is it necessary to look at the designated value input operand in order to determine the
    /// controlling type variable, or is it good enough to use the first return type?
    ///
    /// Most polymorphic instructions produce a single result with the type of the controlling type
    /// variable. A few polymorphic instructions either don't produce any results, or produce
    /// results with a fixed type. These instructions return `true`.
    pub fn requires_typevar_operand(self) -> bool {
        (self.flags & 0x10) != 0
    }

    /// Get the number of *fixed* result values produced by this opcode.
    /// This does not include `variable_args` produced by calls.
    pub fn num_fixed_results(self) -> usize {
        (self.flags & 0x7) as usize
    }

    /// Get the number of *fixed* input values required by this opcode.
    ///
    /// This does not include `variable_args` arguments on call and branch instructions.
    ///
    /// The number of fixed input values is usually implied by the instruction format, but
    /// instruction formats that use a `ValueList` put both fixed and variable arguments in the
    /// list. This method returns the *minimum* number of values required in the value list.
    pub fn num_fixed_value_arguments(self) -> usize {
        ((self.flags >> 5) & 0x7) as usize
    }

    /// Get the offset into `TYPE_SETS` for the controlling type variable.
    /// Returns `None` if the instruction is not polymorphic.
    fn typeset_offset(self) -> Option<usize> {
        let offset = usize::from(self.typeset_offset);
        if offset < TYPE_SETS.len() {
            Some(offset)
        } else {
            None
        }
    }

    /// Get the offset into OPERAND_CONSTRAINTS where the descriptors for this opcode begin.
    fn constraint_offset(self) -> usize {
        self.constraint_offset as usize
    }

    /// Get the value type of result number `n`, having resolved the controlling type variable to
    /// `ctrl_type`.
    pub fn result_type(self, n: usize, ctrl_type: Type) -> Type {
        debug_assert!(n < self.num_fixed_results(), "Invalid result index");
        match OPERAND_CONSTRAINTS[self.constraint_offset() + n].resolve(ctrl_type) {
            ResolvedConstraint::Bound(t) => t,
            ResolvedConstraint::Free(ts) => panic!("Result constraints can't be free: {:?}", ts),
        }
    }

    /// Get the value type of input value number `n`, having resolved the controlling type variable
    /// to `ctrl_type`.
    ///
    /// Unlike results, it is possible for some input values to vary freely within a specific
    /// `ValueTypeSet`. This is represented with the `ArgumentConstraint::Free` variant.
    pub fn value_argument_constraint(self, n: usize, ctrl_type: Type) -> ResolvedConstraint {
        debug_assert!(
            n < self.num_fixed_value_arguments(),
            "Invalid value argument index"
        );
        let offset = self.constraint_offset() + self.num_fixed_results();
        OPERAND_CONSTRAINTS[offset + n].resolve(ctrl_type)
    }

    /// Get the typeset of allowed types for the controlling type variable in a polymorphic
    /// instruction.
    pub fn ctrl_typeset(self) -> Option<ValueTypeSet> {
        self.typeset_offset().map(|offset| TYPE_SETS[offset])
    }

    /// Is this instruction polymorphic?
    pub fn is_polymorphic(self) -> bool {
        self.ctrl_typeset().is_some()
    }
}

type BitSet8 = ScalarBitSet<u8>;
type BitSet16 = ScalarBitSet<u16>;

/// A value type set describes the permitted set of types for a type variable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ValueTypeSet {
    /// Allowed lane sizes
    pub lanes: BitSet16,
    /// Allowed int widths
    pub ints: BitSet8,
    /// Allowed float widths
    pub floats: BitSet8,
    /// Allowed ref widths
    pub refs: BitSet8,
    /// Allowed dynamic vectors minimum lane sizes
    pub dynamic_lanes: BitSet16,
}

impl ValueTypeSet {
    /// Is `scalar` part of the base type set?
    ///
    /// Note that the base type set does not have to be included in the type set proper.
    fn is_base_type(self, scalar: Type) -> bool {
        let l2b = u8::try_from(scalar.log2_lane_bits()).unwrap();
        if scalar.is_int() {
            self.ints.contains(l2b)
        } else if scalar.is_float() {
            self.floats.contains(l2b)
        } else if scalar.is_ref() {
            self.refs.contains(l2b)
        } else {
            false
        }
    }

    /// Does `typ` belong to this set?
    pub fn contains(self, typ: Type) -> bool {
        if typ.is_dynamic_vector() {
            let l2l = u8::try_from(typ.log2_min_lane_count()).unwrap();
            self.dynamic_lanes.contains(l2l) && self.is_base_type(typ.lane_type())
        } else {
            let l2l = u8::try_from(typ.log2_lane_count()).unwrap();
            self.lanes.contains(l2l) && self.is_base_type(typ.lane_type())
        }
    }

    /// Get an example member of this type set.
    ///
    /// This is used for error messages to avoid suggesting invalid types.
    pub fn example(self) -> Type {
        let t = if self.ints.max().unwrap_or(0) > 5 {
            types::I32
        } else if self.floats.max().unwrap_or(0) > 5 {
            types::F32
        } else {
            types::I8
        };
        t.by(1 << self.lanes.min().unwrap()).unwrap()
    }
}

/// Operand constraints. This describes the value type constraints on a single `Value` operand.
enum OperandConstraint {
    /// This operand has a concrete value type.
    Concrete(Type),

    /// This operand can vary freely within the given type set.
    /// The type set is identified by its index into the TYPE_SETS constant table.
    Free(u8),

    /// This operand is the same type as the controlling type variable.
    Same,

    /// This operand is `ctrlType.lane_of()`.
    LaneOf,

    /// This operand is `ctrlType.as_truthy()`.
    AsTruthy,

    /// This operand is `ctrlType.half_width()`.
    HalfWidth,

    /// This operand is `ctrlType.double_width()`.
    DoubleWidth,

    /// This operand is `ctrlType.split_lanes()`.
    SplitLanes,

    /// This operand is `ctrlType.merge_lanes()`.
    MergeLanes,

    /// This operands is `ctrlType.dynamic_to_vector()`.
    DynamicToVector,

    /// This operand is `ctrlType.narrower()`.
    Narrower,

    /// This operand is `ctrlType.wider()`.
    Wider,
}

impl OperandConstraint {
    /// Resolve this operand constraint into a concrete value type, given the value of the
    /// controlling type variable.
    pub fn resolve(&self, ctrl_type: Type) -> ResolvedConstraint {
        use self::OperandConstraint::*;
        use self::ResolvedConstraint::Bound;
        match *self {
            Concrete(t) => Bound(t),
            Free(vts) => ResolvedConstraint::Free(TYPE_SETS[vts as usize]),
            Same => Bound(ctrl_type),
            LaneOf => Bound(ctrl_type.lane_of()),
            AsTruthy => Bound(ctrl_type.as_truthy()),
            HalfWidth => Bound(ctrl_type.half_width().expect("invalid type for half_width")),
            DoubleWidth => Bound(
                ctrl_type
                    .double_width()
                    .expect("invalid type for double_width"),
            ),
            SplitLanes => {
                if ctrl_type.is_dynamic_vector() {
                    Bound(
                        ctrl_type
                            .dynamic_to_vector()
                            .expect("invalid type for dynamic_to_vector")
                            .split_lanes()
                            .expect("invalid type for split_lanes")
                            .vector_to_dynamic()
                            .expect("invalid dynamic type"),
                    )
                } else {
                    Bound(
                        ctrl_type
                            .split_lanes()
                            .expect("invalid type for split_lanes"),
                    )
                }
            }
            MergeLanes => {
                if ctrl_type.is_dynamic_vector() {
                    Bound(
                        ctrl_type
                            .dynamic_to_vector()
                            .expect("invalid type for dynamic_to_vector")
                            .merge_lanes()
                            .expect("invalid type for merge_lanes")
                            .vector_to_dynamic()
                            .expect("invalid dynamic type"),
                    )
                } else {
                    Bound(
                        ctrl_type
                            .merge_lanes()
                            .expect("invalid type for merge_lanes"),
                    )
                }
            }
            DynamicToVector => Bound(
                ctrl_type
                    .dynamic_to_vector()
                    .expect("invalid type for dynamic_to_vector"),
            ),
            Narrower => {
                let ctrl_type_bits = ctrl_type.log2_lane_bits();
                let mut tys = ValueTypeSet::default();

                // We're testing scalar values, only.
                tys.lanes = ScalarBitSet::from_range(0, 1);

                if ctrl_type.is_int() {
                    // The upper bound in from_range is exclusive, and we want to exclude the
                    // control type to construct the interval of [I8, ctrl_type).
                    tys.ints = BitSet8::from_range(3, ctrl_type_bits as u8);
                } else if ctrl_type.is_float() {
                    // The upper bound in from_range is exclusive, and we want to exclude the
                    // control type to construct the interval of [F16, ctrl_type).
                    tys.floats = BitSet8::from_range(4, ctrl_type_bits as u8);
                } else {
                    panic!("The Narrower constraint only operates on floats or ints");
                }
                ResolvedConstraint::Free(tys)
            }
            Wider => {
                let ctrl_type_bits = ctrl_type.log2_lane_bits();
                let mut tys = ValueTypeSet::default();

                // We're testing scalar values, only.
                tys.lanes = ScalarBitSet::from_range(0, 1);

                if ctrl_type.is_int() {
                    let lower_bound = ctrl_type_bits as u8 + 1;
                    // The largest integer type we can represent in `BitSet8` is I128, which is
                    // represented by bit 7 in the bit set. Adding one to exclude I128 from the
                    // lower bound would overflow as 2^8 doesn't fit in a u8, but this would
                    // already describe the empty set so instead we leave `ints` in its default
                    // empty state.
                    if lower_bound < BitSet8::capacity() {
                        // The interval should include all types wider than `ctrl_type`, so we use
                        // `2^8` as the upper bound, and add one to the bits of `ctrl_type` to define
                        // the interval `(ctrl_type, I128]`.
                        tys.ints = BitSet8::from_range(lower_bound, 8);
                    }
                } else if ctrl_type.is_float() {
                    // Same as above but for `tys.floats`, as the largest float type is F128.
                    let lower_bound = ctrl_type_bits as u8 + 1;
                    if lower_bound < BitSet8::capacity() {
                        tys.floats = BitSet8::from_range(lower_bound, 8);
                    }
                } else {
                    panic!("The Wider constraint only operates on floats or ints");
                }

                ResolvedConstraint::Free(tys)
            }
        }
    }
}

/// The type constraint on a value argument once the controlling type variable is known.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResolvedConstraint {
    /// The operand is bound to a known type.
    Bound(Type),
    /// The operand type can vary freely within the given set.
    Free(ValueTypeSet),
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn inst_data_is_copy() {
        fn is_copy<T: Copy>() {}
        is_copy::<InstructionData>();
    }

    #[test]
    fn inst_data_size() {
        // The size of `InstructionData` is performance sensitive, so make sure
        // we don't regress it unintentionally.
        assert_eq!(std::mem::size_of::<InstructionData>(), 16);
    }

    #[test]
    fn opcodes() {
        use core::mem;

        let x = Opcode::Iadd;
        let mut y = Opcode::Isub;

        assert!(x != y);
        y = Opcode::Iadd;
        assert_eq!(x, y);
        assert_eq!(x.format(), InstructionFormat::Binary);

        assert_eq!(format!("{:?}", Opcode::IaddImm), "IaddImm");
        assert_eq!(Opcode::IaddImm.to_string(), "iadd_imm");

        // Check the matcher.
        assert_eq!("iadd".parse::<Opcode>(), Ok(Opcode::Iadd));
        assert_eq!("iadd_imm".parse::<Opcode>(), Ok(Opcode::IaddImm));
        assert_eq!("iadd\0".parse::<Opcode>(), Err("Unknown opcode"));
        assert_eq!("".parse::<Opcode>(), Err("Unknown opcode"));
        assert_eq!("\0".parse::<Opcode>(), Err("Unknown opcode"));

        // Opcode is a single byte, and because Option<Opcode> originally came to 2 bytes, early on
        // Opcode included a variant NotAnOpcode to avoid the unnecessary bloat. Since then the Rust
        // compiler has brought in NonZero optimization, meaning that an enum not using the 0 value
        // can be optional for no size cost. We want to ensure Option<Opcode> remains small.
        assert_eq!(mem::size_of::<Opcode>(), mem::size_of::<Option<Opcode>>());
    }

    #[test]
    fn instruction_data() {
        use core::mem;
        // The size of the `InstructionData` enum is important for performance. It should not
        // exceed 16 bytes. Use `Box<FooData>` out-of-line payloads for instruction formats that
        // require more space than that. It would be fine with a data structure smaller than 16
        // bytes, but what are the odds of that?
        assert_eq!(mem::size_of::<InstructionData>(), 16);
    }

    #[test]
    fn constraints() {
        let a = Opcode::Iadd.constraints();
        assert!(a.use_typevar_operand());
        assert!(!a.requires_typevar_operand());
        assert_eq!(a.num_fixed_results(), 1);
        assert_eq!(a.num_fixed_value_arguments(), 2);
        assert_eq!(a.result_type(0, types::I32), types::I32);
        assert_eq!(a.result_type(0, types::I8), types::I8);
        assert_eq!(
            a.value_argument_constraint(0, types::I32),
            ResolvedConstraint::Bound(types::I32)
        );
        assert_eq!(
            a.value_argument_constraint(1, types::I32),
            ResolvedConstraint::Bound(types::I32)
        );

        let b = Opcode::Bitcast.constraints();
        assert!(!b.use_typevar_operand());
        assert!(!b.requires_typevar_operand());
        assert_eq!(b.num_fixed_results(), 1);
        assert_eq!(b.num_fixed_value_arguments(), 1);
        assert_eq!(b.result_type(0, types::I32), types::I32);
        assert_eq!(b.result_type(0, types::I8), types::I8);
        match b.value_argument_constraint(0, types::I32) {
            ResolvedConstraint::Free(vts) => assert!(vts.contains(types::F32)),
            _ => panic!("Unexpected constraint from value_argument_constraint"),
        }

        let c = Opcode::Call.constraints();
        assert_eq!(c.num_fixed_results(), 0);
        assert_eq!(c.num_fixed_value_arguments(), 0);

        let i = Opcode::CallIndirect.constraints();
        assert_eq!(i.num_fixed_results(), 0);
        assert_eq!(i.num_fixed_value_arguments(), 1);

        let cmp = Opcode::Icmp.constraints();
        assert!(cmp.use_typevar_operand());
        assert!(cmp.requires_typevar_operand());
        assert_eq!(cmp.num_fixed_results(), 1);
        assert_eq!(cmp.num_fixed_value_arguments(), 2);
        assert_eq!(cmp.result_type(0, types::I64), types::I8);
    }

    #[test]
    fn value_set() {
        use crate::ir::types::*;

        let vts = ValueTypeSet {
            lanes: BitSet16::from_range(0, 8),
            ints: BitSet8::from_range(4, 7),
            floats: BitSet8::from_range(0, 0),
            refs: BitSet8::from_range(5, 7),
            dynamic_lanes: BitSet16::from_range(0, 4),
        };
        assert!(!vts.contains(I8));
        assert!(vts.contains(I32));
        assert!(vts.contains(I64));
        assert!(vts.contains(I32X4));
        assert!(vts.contains(I32X4XN));
        assert!(!vts.contains(F16));
        assert!(!vts.contains(F32));
        assert!(!vts.contains(F128));
        assert!(vts.contains(R32));
        assert!(vts.contains(R64));
        assert_eq!(vts.example().to_string(), "i32");

        let vts = ValueTypeSet {
            lanes: BitSet16::from_range(0, 8),
            ints: BitSet8::from_range(0, 0),
            floats: BitSet8::from_range(5, 7),
            refs: BitSet8::from_range(0, 0),
            dynamic_lanes: BitSet16::from_range(0, 8),
        };
        assert_eq!(vts.example().to_string(), "f32");

        let vts = ValueTypeSet {
            lanes: BitSet16::from_range(1, 8),
            ints: BitSet8::from_range(0, 0),
            floats: BitSet8::from_range(5, 7),
            refs: BitSet8::from_range(0, 0),
            dynamic_lanes: BitSet16::from_range(0, 8),
        };
        assert_eq!(vts.example().to_string(), "f32x2");

        let vts = ValueTypeSet {
            lanes: BitSet16::from_range(2, 8),
            ints: BitSet8::from_range(3, 7),
            floats: BitSet8::from_range(0, 0),
            refs: BitSet8::from_range(0, 0),
            dynamic_lanes: BitSet16::from_range(0, 8),
        };
        assert_eq!(vts.example().to_string(), "i32x4");

        let vts = ValueTypeSet {
            // TypeSet(lanes=(1, 256), ints=(8, 64))
            lanes: BitSet16::from_range(0, 9),
            ints: BitSet8::from_range(3, 7),
            floats: BitSet8::from_range(0, 0),
            refs: BitSet8::from_range(0, 0),
            dynamic_lanes: BitSet16::from_range(0, 8),
        };
        assert!(vts.contains(I32));
        assert!(vts.contains(I32X4));
        assert!(!vts.contains(R32));
        assert!(!vts.contains(R64));
    }
}

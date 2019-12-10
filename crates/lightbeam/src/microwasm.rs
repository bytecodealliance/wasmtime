use crate::error::Error;
use crate::module::{ModuleContext, SigType, Signature};
use smallvec::{smallvec, SmallVec};
use std::{
    convert::TryInto,
    fmt,
    iter::{self, FromIterator},
    ops::RangeInclusive,
};
use wasmparser::{
    BinaryReaderError, FunctionBody, Ieee32 as WasmIeee32, Ieee64 as WasmIeee64,
    MemoryImmediate as WasmMemoryImmediate, Operator as WasmOperator, OperatorsReader,
};

pub fn dis<L>(
    mut out: impl std::io::Write,
    function_name: impl fmt::Display,
    microwasm: impl IntoIterator<Item = Operator<L>>,
) -> std::io::Result<()>
where
    BrTarget<L>: fmt::Display,
    L: Clone,
{
    writeln!(out, ".fn_{}:", function_name)?;

    let p = "      ";
    for op in microwasm {
        if op.is_label() || op.is_block() {
            writeln!(out, "{}", op)?;
        } else {
            writeln!(out, "{}{}", p, op)?;
        }
    }

    Ok(())
}

/// A constant value embedded in the instructions
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(Ieee32),
    F64(Ieee64),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::I32(v) => write!(f, "{}i32", v),
            Value::I64(v) => write!(f, "{}i64", v),
            Value::F32(v) => write!(f, "{}f32", f32::from_bits(v.to_bits())),
            Value::F64(v) => write!(f, "{}f64", f64::from_bits(v.to_bits())),
        }
    }
}

impl Value {
    pub fn as_int(self) -> Option<i64> {
        self.as_i64().or_else(|| self.as_i32().map(|i| i as _))
    }

    pub fn as_bytes(self) -> i64 {
        match self {
            Value::I32(val) => val as _,
            Value::I64(val) => val,
            Value::F32(val) => val.0 as _,
            Value::F64(val) => val.0 as _,
        }
    }

    pub fn as_i32(self) -> Option<i32> {
        match self {
            Value::I32(val) => Some(val),
            _ => None,
        }
    }

    pub fn as_i64(self) -> Option<i64> {
        match self {
            Value::I64(val) => Some(val),
            _ => None,
        }
    }

    pub fn as_f32(self) -> Option<Ieee32> {
        match self {
            Value::F32(val) => Some(val),
            _ => None,
        }
    }

    pub fn as_f64(self) -> Option<Ieee64> {
        match self {
            Value::F64(val) => Some(val),
            _ => None,
        }
    }

    pub fn type_(&self) -> SignlessType {
        match self {
            Value::I32(_) => Type::Int(Size::_32),
            Value::I64(_) => Type::Int(Size::_64),
            Value::F32(Ieee32(_)) => Type::Float(Size::_32),
            Value::F64(Ieee64(_)) => Type::Float(Size::_64),
        }
    }

    fn default_for_type(ty: SignlessType) -> Self {
        match ty {
            Type::Int(Size::_32) => Value::I32(0),
            Type::Int(Size::_64) => Value::I64(0),
            Type::Float(Size::_32) => Value::F32(Ieee32(0)),
            Type::Float(Size::_64) => Value::F64(Ieee64(0)),
        }
    }
}

impl From<i32> for Value {
    fn from(other: i32) -> Self {
        Value::I32(other)
    }
}
impl From<i64> for Value {
    fn from(other: i64) -> Self {
        Value::I64(other)
    }
}
impl From<u32> for Value {
    fn from(other: u32) -> Self {
        Value::I32(other as _)
    }
}
impl From<u64> for Value {
    fn from(other: u64) -> Self {
        Value::I64(other as _)
    }
}
impl From<Ieee32> for Value {
    fn from(other: Ieee32) -> Self {
        Value::F32(other)
    }
}
impl From<Ieee64> for Value {
    fn from(other: Ieee64) -> Self {
        Value::F64(other)
    }
}

/// Whether to interpret an integer as signed or unsigned
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Signedness {
    Signed,
    Unsigned,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Size {
    _32,
    _64,
}

type Int = Size;
type Float = Size;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SignfulInt(pub Signedness, pub Size);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Type<I> {
    Int(I),
    Float(Size),
}

pub trait IntoType<T> {
    fn into_type() -> T;
}

impl IntoType<SignlessType> for i32 {
    fn into_type() -> SignlessType {
        I32
    }
}

impl IntoType<SignlessType> for i64 {
    fn into_type() -> SignlessType {
        I64
    }
}

impl IntoType<SignlessType> for u32 {
    fn into_type() -> SignlessType {
        I32
    }
}

impl IntoType<SignlessType> for u64 {
    fn into_type() -> SignlessType {
        I64
    }
}

impl IntoType<SignlessType> for f32 {
    fn into_type() -> SignlessType {
        F32
    }
}

impl IntoType<SignlessType> for f64 {
    fn into_type() -> SignlessType {
        F64
    }
}

impl<I> Type<I> {
    pub fn for_<T: IntoType<Self>>() -> Self {
        T::into_type()
    }
}

impl fmt::Display for SignfulType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int(i) => write!(f, "{}", i),
            Type::Float(Size::_32) => write!(f, "f32"),
            Type::Float(Size::_64) => write!(f, "f64"),
        }
    }
}

impl fmt::Display for SignlessType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int(Size::_32) => write!(f, "i32"),
            Type::Int(Size::_64) => write!(f, "i64"),
            Type::Float(Size::_32) => write!(f, "f32"),
            Type::Float(Size::_64) => write!(f, "f64"),
        }
    }
}

impl fmt::Display for SignfulInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SignfulInt(Signedness::Signed, Size::_32) => write!(f, "i32"),
            SignfulInt(Signedness::Unsigned, Size::_32) => write!(f, "u32"),
            SignfulInt(Signedness::Signed, Size::_64) => write!(f, "i64"),
            SignfulInt(Signedness::Unsigned, Size::_64) => write!(f, "u64"),
        }
    }
}

pub type SignlessType = Type<Size>;
pub type SignfulType = Type<SignfulInt>;

pub const I32: SignlessType = Type::Int(Size::_32);
pub const I64: SignlessType = Type::Int(Size::_64);
pub const F32: SignlessType = Type::Float(Size::_32);
pub const F64: SignlessType = Type::Float(Size::_64);

pub mod sint {
    use super::{Signedness, SignfulInt, Size};

    pub const I32: SignfulInt = SignfulInt(Signedness::Signed, Size::_32);
    pub const I64: SignfulInt = SignfulInt(Signedness::Signed, Size::_64);
    pub const U32: SignfulInt = SignfulInt(Signedness::Unsigned, Size::_32);
    pub const U64: SignfulInt = SignfulInt(Signedness::Unsigned, Size::_64);
}

pub const SI32: SignfulType = Type::Int(sint::I32);
pub const SI64: SignfulType = Type::Int(sint::I64);
pub const SU32: SignfulType = Type::Int(sint::U32);
pub const SU64: SignfulType = Type::Int(sint::U64);
pub const SF32: SignfulType = Type::Float(Size::_32);
pub const SF64: SignfulType = Type::Float(Size::_64);

impl SignlessType {
    pub fn from_wasm(other: wasmparser::Type) -> Result<Self, BinaryReaderError> {
        use wasmparser::Type;

        match other {
            Type::I32 => Ok(I32),
            Type::I64 => Ok(I64),
            Type::F32 => Ok(F32),
            Type::F64 => Ok(F64),
            Type::EmptyBlockType => Err(BinaryReaderError {
                message: "SignlessType with EmptyBlockType",
                offset: -1isize as usize,
            }),
            _ => Err(BinaryReaderError {
                message: "SignlessType unimplemented",
                offset: -1isize as usize,
            }),
        }
    }
}

fn create_returns_from_wasm_type(
    ty: wasmparser::TypeOrFuncType,
) -> Result<Vec<SignlessType>, BinaryReaderError> {
    match ty {
        wasmparser::TypeOrFuncType::Type(ty) => Ok(Vec::from_iter(Type::from_wasm(ty))),
        wasmparser::TypeOrFuncType::FuncType(_) => Err(BinaryReaderError {
            message: "Unsupported func type",
            offset: -1isize as usize,
        }),
    }
}

#[derive(Debug, Clone)]
pub struct BrTable<L> {
    pub targets: Vec<BrTargetDrop<L>>,
    pub default: BrTargetDrop<L>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum NameTag {
    Header,
    Else,
    End,
}

pub type WasmLabel = (u32, NameTag);

pub type OperatorFromWasm = Operator<WasmLabel>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum BrTarget<L> {
    Return,
    Label(L),
}

impl<L> BrTarget<L> {
    pub fn label(&self) -> Option<&L> {
        match self {
            BrTarget::Return => None,
            BrTarget::Label(l) => Some(l),
        }
    }
}

impl<L> From<L> for BrTarget<L> {
    fn from(other: L) -> Self {
        BrTarget::Label(other)
    }
}

impl fmt::Display for BrTarget<WasmLabel> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BrTarget::Return => write!(f, ".return"),
            BrTarget::Label((i, NameTag::Header)) => write!(f, ".L{}", i),
            BrTarget::Label((i, NameTag::Else)) => write!(f, ".L{}_else", i),
            BrTarget::Label((i, NameTag::End)) => write!(f, ".L{}_end", i),
        }
    }
}

impl fmt::Display for BrTarget<&str> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BrTarget::Return => write!(f, ".return"),
            BrTarget::Label(l) => write!(f, ".L{}", l),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BrTargetDrop<L> {
    pub target: BrTarget<L>,
    pub to_drop: Option<RangeInclusive<u32>>,
}

impl<L> From<BrTarget<L>> for BrTargetDrop<L> {
    fn from(other: BrTarget<L>) -> Self {
        BrTargetDrop {
            target: other,
            to_drop: None,
        }
    }
}

impl<L> fmt::Display for BrTargetDrop<L>
where
    BrTarget<L>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(drop) = &self.to_drop {
            write!(
                f,
                "({}, drop {}..={})",
                self.target,
                drop.start(),
                drop.end()
            )
        } else {
            write!(f, "{}", self.target)
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Ieee32(u32);
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Ieee64(u64);

impl Ieee32 {
    pub fn to_bits(self) -> u32 {
        self.0
    }

    pub fn from_bits(other: u32) -> Self {
        Ieee32(other)
    }
}

impl From<WasmIeee32> for Ieee32 {
    fn from(other: WasmIeee32) -> Self {
        Self::from_bits(other.bits())
    }
}

impl Ieee64 {
    pub fn to_bits(self) -> u64 {
        self.0
    }

    pub fn from_bits(other: u64) -> Self {
        Ieee64(other)
    }
}

impl From<WasmIeee64> for Ieee64 {
    fn from(other: WasmIeee64) -> Self {
        Self::from_bits(other.bits())
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct MemoryImmediate {
    pub flags: u32,
    pub offset: u32,
}

impl From<WasmMemoryImmediate> for MemoryImmediate {
    fn from(other: WasmMemoryImmediate) -> Self {
        MemoryImmediate {
            flags: other.flags,
            offset: other.offset,
        }
    }
}

// TODO: Explicit VmCtx?
#[derive(Debug, Clone)]
pub enum Operator<Label> {
    /// Explicit trap instruction
    Unreachable,
    /// Define metadata for a block - its label, its signature, whether it has backwards callers etc. It
    /// is an error to branch to a block that has yet to be defined.
    Block {
        label: Label,
        // TODO: Do we need this?
        params: Vec<SignlessType>,
        // TODO: Ideally we'd have `num_backwards_callers` but we can't know that for WebAssembly
        has_backwards_callers: bool,
        num_callers: Option<u32>,
    },
    /// Start a new block. It is an error if the previous block has not been closed by emitting a `Br` or
    /// `BrTable`.
    Label(Label),
    /// Unconditionally break to a new block. This the parameters off the stack and passes them into
    /// the new block. Any remaining elements on the stack are discarded.
    Br {
        /// Returning from the function is just calling the "return" block
        target: BrTarget<Label>,
    },
    /// Pop a value off the top of the stack, jump to the `else_` label if this value is `true`
    /// and the `then` label otherwise. The `then` and `else_` blocks must have the same parameters.
    BrIf {
        /// Label to jump to if the value at the top of the stack is true
        then: BrTargetDrop<Label>,
        /// Label to jump to if the value at the top of the stack is false
        else_: BrTargetDrop<Label>,
    },
    /// Pop a value off the top of the stack, jump to `table[value.min(table.len() - 1)]`. All elements
    /// in the table must have the same parameters.
    BrTable(
        /// The table of labels to jump to - the index should be clamped to the length of the table
        BrTable<Label>,
    ),
    /// Call a function
    Call {
        function_index: u32,
    },
    /// Pop an `i32` off the top of the stack, index into the table at `table_index` and call that function
    CallIndirect {
        type_index: u32,
        table_index: u32,
    },
    /// Pop an element off of the stack and discard it.
    Drop(RangeInclusive<u32>),
    /// Pop an `i32` off of the stack and 2 elements off of the stack, call them `A` and `B` where `A` is the
    /// first element popped and `B` is the second. If the `i32` is 0 then discard `B` and push `A` back onto
    /// the stack, otherwise discard `A` and push `B` back onto the stack.
    Select,
    /// Duplicate the element at depth `depth` to the top of the stack. This can be used to implement
    /// `GetLocal`.
    Pick(u32),
    /// Swap the top element of the stack with the element at depth `depth`. This can be used to implement
    /// `SetLocal`.
    // TODO: Is it better to have `Swap`, to have `Pull` (which moves the `nth` element instead of swapping)
    //       or to have both?
    Swap(u32),
    GetGlobal(u32),
    SetGlobal(u32),
    Load {
        ty: SignlessType,
        memarg: MemoryImmediate,
    },
    Load8 {
        ty: SignfulInt,
        memarg: MemoryImmediate,
    },
    Load16 {
        ty: SignfulInt,
        memarg: MemoryImmediate,
    },
    // Only available for {I,U}64
    // TODO: Roll this into `Load` somehow?
    Load32 {
        sign: Signedness,
        memarg: MemoryImmediate,
    },
    Store {
        ty: SignlessType,
        memarg: MemoryImmediate,
    },
    Store8 {
        /// `ty` on integers
        ty: Int,
        memarg: MemoryImmediate,
    },
    Store16 {
        /// `ty` on integers
        ty: Int,
        memarg: MemoryImmediate,
    },
    // Only available for I64
    // TODO: Roll this into `Store` somehow?
    Store32 {
        memarg: MemoryImmediate,
    },
    MemorySize {
        reserved: u32,
    },
    MemoryGrow {
        reserved: u32,
    },
    Const(Value),
    Eq(SignlessType),
    Ne(SignlessType),
    /// `eqz` on integers
    Eqz(Int),
    Lt(SignfulType),
    Gt(SignfulType),
    Le(SignfulType),
    Ge(SignfulType),
    Add(SignlessType),
    Sub(SignlessType),
    Mul(SignlessType),
    /// `clz` on integers
    Clz(Int),
    /// `ctz` on integers
    Ctz(Int),
    /// `popcnt` on integers
    Popcnt(Int),
    Div(SignfulType),
    Rem(SignfulInt),
    And(Int),
    Or(Int),
    Xor(Int),
    Shl(Int),
    Shr(SignfulInt),
    Rotl(Int),
    Rotr(Int),
    Abs(Float),
    Neg(Float),
    Ceil(Float),
    Floor(Float),
    Trunc(Float),
    Nearest(Float),
    Sqrt(Float),
    Min(Float),
    Max(Float),
    Copysign(Float),
    I32WrapFromI64,
    ITruncFromF {
        input_ty: Float,
        output_ty: SignfulInt,
    },
    FConvertFromI {
        input_ty: SignfulInt,
        output_ty: Float,
    },
    F32DemoteFromF64,
    F64PromoteFromF32,
    I32ReinterpretFromF32,
    I64ReinterpretFromF64,
    F32ReinterpretFromI32,
    F64ReinterpretFromI64,
    // Only available for input I32 and output I64
    Extend {
        sign: Signedness,
    },
}

impl<L> Operator<L> {
    pub fn is_label(&self) -> bool {
        match self {
            Operator::Label(..) => true,
            _ => false,
        }
    }

    pub fn is_block(&self) -> bool {
        match self {
            Operator::Block { .. } => true,
            _ => false,
        }
    }

    pub fn end(params: Vec<SignlessType>, label: L) -> Self {
        Operator::Block {
            params,
            label,
            has_backwards_callers: false,
            // TODO
            num_callers: None,
        }
    }

    pub fn block(params: Vec<SignlessType>, label: L) -> Self {
        Operator::Block {
            params,
            label,
            has_backwards_callers: false,
            num_callers: Some(1),
        }
    }

    pub fn loop_(params: Vec<SignlessType>, label: L) -> Self {
        Operator::Block {
            params,
            label,
            has_backwards_callers: true,
            num_callers: None,
        }
    }
}

impl<L> fmt::Display for Operator<L>
where
    BrTarget<L>: fmt::Display,
    L: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operator::Unreachable => write!(f, "unreachable"),
            Operator::Label(label) => write!(f, "{}:", BrTarget::Label(label.clone())),
            Operator::Block {
                label,
                params,
                has_backwards_callers,
                num_callers,
            } => {
                write!(f, "def {} :: [", BrTarget::Label(label.clone()))?;
                let mut iter = params.iter();
                if let Some(p) = iter.next() {
                    write!(f, "{}", p)?;
                    for p in iter {
                        write!(f, ", {}", p)?;
                    }
                }
                write!(f, "]")?;

                if *has_backwards_callers {
                    write!(f, " has_backwards_callers")?;
                }

                if let Some(n) = num_callers {
                    write!(f, " num_callers={}", n)?;
                }

                Ok(())
            }
            Operator::Br { target } => write!(f, "br {}", target),
            Operator::BrIf { then, else_ } => write!(f, "br_if {}, {}", then, else_),
            Operator::BrTable(BrTable { targets, default }) => {
                write!(f, "br_table [")?;
                let mut iter = targets.iter();
                if let Some(p) = iter.next() {
                    write!(f, "{}", p)?;
                    for p in iter {
                        write!(f, ", {}", p)?;
                    }
                }

                write!(f, "], {}", default)
            }
            Operator::Call { function_index } => write!(f, "call {}", function_index),
            Operator::CallIndirect { .. } => write!(f, "call_indirect"),
            Operator::Drop(range) => {
                write!(f, "drop")?;

                match range.clone().into_inner() {
                    (0, 0) => {}
                    (start, end) if start == end => {
                        write!(f, " {}", start)?;
                    }
                    (start, end) => {
                        write!(f, " {}..={}", start, end)?;
                    }
                }

                Ok(())
            }
            Operator::Select => write!(f, "select"),
            Operator::Pick(depth) => write!(f, "pick {}", depth),
            Operator::Swap(depth) => write!(f, "swap {}", depth),
            Operator::Load { ty, memarg } => {
                write!(f, "{}.load {}, {}", ty, memarg.flags, memarg.offset)
            }
            Operator::Load8 { ty, memarg } => {
                write!(f, "{}.load8 {}, {}", ty, memarg.flags, memarg.offset)
            }
            Operator::Load16 { ty, memarg } => {
                write!(f, "{}.load16 {}, {}", ty, memarg.flags, memarg.offset)
            }
            Operator::Load32 { sign, memarg } => write!(
                f,
                "{}.load32 {}, {}",
                SignfulInt(*sign, Size::_64),
                memarg.flags,
                memarg.offset
            ),
            Operator::Store { ty, memarg } => {
                write!(f, "{}.store {}, {}", ty, memarg.flags, memarg.offset)
            }
            Operator::Store8 { ty, memarg } => write!(
                f,
                "{}.store8 {}, {}",
                SignfulInt(Signedness::Unsigned, *ty),
                memarg.flags,
                memarg.offset
            ),
            Operator::Store16 { ty, memarg } => write!(
                f,
                "{}.store16 {}, {}",
                SignfulInt(Signedness::Unsigned, *ty),
                memarg.flags,
                memarg.offset
            ),
            Operator::Store32 { memarg } => {
                write!(f, "u64.store32 {}, {}", memarg.flags, memarg.offset)
            }
            Operator::MemorySize { .. } => write!(f, "memory.size"),
            Operator::MemoryGrow { .. } => write!(f, "memory.grow"),
            Operator::Const(val) => write!(f, "const {}", val),
            Operator::Eq(ty) => write!(f, "{}.eq", ty),
            Operator::Ne(ty) => write!(f, "{}.ne", ty),
            Operator::Eqz(ty) => write!(f, "{}.eqz", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Lt(ty) => write!(f, "{}.lt", ty),
            Operator::Gt(ty) => write!(f, "{}.gt", ty),
            Operator::Le(ty) => write!(f, "{}.le", ty),
            Operator::Ge(ty) => write!(f, "{}.ge", ty),
            Operator::Add(ty) => write!(f, "{}.add", ty),
            Operator::Sub(ty) => write!(f, "{}.sub", ty),
            Operator::Mul(ty) => write!(f, "{}.mul", ty),
            Operator::Clz(ty) => write!(f, "{}.clz", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Ctz(ty) => write!(f, "{}.ctz", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Popcnt(ty) => write!(f, "{}.popcnt", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Div(ty) => write!(f, "{}.div", ty),
            Operator::Rem(ty) => write!(f, "{}.rem", ty),
            Operator::And(ty) => write!(f, "{}.and", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Or(ty) => write!(f, "{}.or", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Xor(ty) => write!(f, "{}.xor", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Shl(ty) => write!(f, "{}.shl", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Shr(ty) => write!(f, "{}.shr", ty),
            Operator::Rotl(ty) => write!(f, "{}.rotl", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Rotr(ty) => write!(f, "{}.rotr", SignfulInt(Signedness::Unsigned, *ty)),
            Operator::Abs(ty) => write!(f, "{}.abs", Type::<Int>::Float(*ty)),
            Operator::Neg(ty) => write!(f, "{}.neg", Type::<Int>::Float(*ty)),
            Operator::Ceil(ty) => write!(f, "{}.ceil", Type::<Int>::Float(*ty)),
            Operator::Floor(ty) => write!(f, "{}.floor", Type::<Int>::Float(*ty)),
            Operator::Trunc(ty) => write!(f, "{}.trunc", Type::<Int>::Float(*ty)),
            Operator::Nearest(ty) => write!(f, "{}.nearest", Type::<Int>::Float(*ty)),
            Operator::Sqrt(ty) => write!(f, "{}.sqrt", Type::<Int>::Float(*ty)),
            Operator::Min(ty) => write!(f, "{}.min", Type::<Int>::Float(*ty)),
            Operator::Max(ty) => write!(f, "{}.max", Type::<Int>::Float(*ty)),
            Operator::Copysign(ty) => write!(f, "{}.copysign", Type::<Int>::Float(*ty)),
            Operator::I32WrapFromI64 => write!(f, "i32.wrap_from.i64"),
            Operator::F32DemoteFromF64 => write!(f, "f32.demote_from.f64"),
            Operator::F64PromoteFromF32 => write!(f, "f64.promote_from.f32"),
            Operator::I32ReinterpretFromF32 => write!(f, "i32.reinterpret_from.f32"),
            Operator::I64ReinterpretFromF64 => write!(f, "i64.reinterpret_from.f64"),
            Operator::F32ReinterpretFromI32 => write!(f, "f32.reinterpret_from.i32"),
            Operator::F64ReinterpretFromI64 => write!(f, "f64.reinterpret_from.i64"),
            Operator::FConvertFromI {
                input_ty,
                output_ty,
            } => write!(
                f,
                "{}.convert_from.{}",
                Type::Float::<Int>(*output_ty),
                input_ty,
            ),
            Operator::GetGlobal(index) => write!(f, "global.get {}", index),
            Operator::SetGlobal(index) => write!(f, "global.set {}", index),
            Operator::ITruncFromF {
                input_ty,
                output_ty,
            } => write!(
                f,
                "{}.truncate_from.{}",
                output_ty,
                Type::<Int>::Float(*input_ty)
            ),
            Operator::Extend { sign } => write!(
                f,
                "{}.extend_from.{}",
                SignfulInt(*sign, Size::_64),
                SignfulInt(*sign, Size::_32)
            ),
        }
    }
}

// TODO: If we return a `Vec<<T as MicrowasmReceiver>::Item>` will that convert to (essentially) a no-op
//       in the case that `Item` is a ZST? That is important for ensuring that we don't do unnecessary
//       work when we're directly generating asm.
/// WIP: Trait to abstract over either producing a stream of Microwasm or directly producing assembly
/// from the Wasm. This should give a significant speedup since we don't need to allocate any vectors
/// or pay the cost of branches - we can just use iterators and direct function calls.
pub trait MicrowasmReceiver<Label> {
    type Item;

    fn unreachable(&mut self) -> Self::Item;
    fn block(
        &mut self,
        label: Label,
        params: impl Iterator<Item = SignlessType>,
        has_backwards_callers: bool,
        num_callers: Option<u32>,
    ) -> Self::Item;
    fn label(&mut self, _: Label) -> Self::Item;
    fn br(&mut self, target: BrTarget<Label>) -> Self::Item;
    fn br_if(&mut self, then: BrTargetDrop<Label>, else_: BrTargetDrop<Label>) -> Self::Item;
    fn br_table(&mut self, _: BrTable<Label>) -> Self::Item;
    fn call(&mut self, function_index: u32) -> Self::Item;
    fn call_indirect(&mut self, type_index: u32, table_index: u32) -> Self::Item;
    fn drop(&mut self, _: RangeInclusive<u32>) -> Self::Item;
    fn select(&mut self) -> Self::Item;
    fn pick(&mut self, _: u32) -> Self::Item;
    fn swap(&mut self, _: u32) -> Self::Item;
    fn get_global(&mut self, index: u32) -> Self::Item;
    fn set_global(&mut self, index: u32) -> Self::Item;
    fn load(&mut self, ty: SignlessType, memarg: MemoryImmediate) -> Self::Item;
    fn load8(&mut self, ty: SignfulInt, memarg: MemoryImmediate) -> Self::Item;
    fn load16(&mut self, ty: SignfulInt, memarg: MemoryImmediate) -> Self::Item;
    fn load32(&mut self, sign: Signedness, memarg: MemoryImmediate) -> Self::Item;
    fn store(&mut self, ty: SignlessType, memarg: MemoryImmediate) -> Self::Item;
    fn store8(&mut self, ty: Int, memarg: MemoryImmediate) -> Self::Item;
    fn store16(&mut self, ty: Int, memarg: MemoryImmediate) -> Self::Item;
    fn store32(&mut self, memarg: MemoryImmediate) -> Self::Item;
    fn memory_size(&mut self, reserved: u32) -> Self::Item;
    fn memory_grow(&mut self, reserved: u32) -> Self::Item;
    fn const_(&mut self, _: Value) -> Self::Item;
    fn ref_null(&mut self) -> Self::Item;
    fn ref_is_null(&mut self) -> Self::Item;
    fn eq(&mut self, _: SignlessType) -> Self::Item;
    fn ne(&mut self, _: SignlessType) -> Self::Item;
    fn eqz(&mut self, _: Int) -> Self::Item;
    fn lt(&mut self, _: SignfulType) -> Self::Item;
    fn gt(&mut self, _: SignfulType) -> Self::Item;
    fn le(&mut self, _: SignfulType) -> Self::Item;
    fn ge(&mut self, _: SignfulType) -> Self::Item;
    fn add(&mut self, _: SignlessType) -> Self::Item;
    fn sub(&mut self, _: SignlessType) -> Self::Item;
    fn mul(&mut self, _: SignlessType) -> Self::Item;
    fn clz(&mut self, _: Int) -> Self::Item;
    fn ctz(&mut self, _: Int) -> Self::Item;
    fn popcnt(&mut self, _: Int) -> Self::Item;
    fn div(&mut self, _: SignfulType) -> Self::Item;
    fn rem(&mut self, _: SignfulInt) -> Self::Item;
    fn and(&mut self, _: Int) -> Self::Item;
    fn or(&mut self, _: Int) -> Self::Item;
    fn xor(&mut self, _: Int) -> Self::Item;
    fn shl(&mut self, _: Int) -> Self::Item;
    fn shr(&mut self, _: SignfulInt) -> Self::Item;
    fn rotl(&mut self, _: Int) -> Self::Item;
    fn rotr(&mut self, _: Int) -> Self::Item;
    fn abs(&mut self, _: Float) -> Self::Item;
    fn neg(&mut self, _: Float) -> Self::Item;
    fn ceil(&mut self, _: Float) -> Self::Item;
    fn floor(&mut self, _: Float) -> Self::Item;
    fn trunc(&mut self, _: Float) -> Self::Item;
    fn nearest(&mut self, _: Float) -> Self::Item;
    fn sqrt(&mut self, _: Float) -> Self::Item;
    fn min(&mut self, _: Float) -> Self::Item;
    fn max(&mut self, _: Float) -> Self::Item;
    fn copysign(&mut self, _: Float) -> Self::Item;
    fn i32_wrap_from_i64(&mut self) -> Self::Item;
    fn i_trunc_from_f(&mut self, input_ty: Float, output_ty: SignfulInt) -> Self::Item;
    fn f_convert_from_i(&mut self, input_ty: SignfulInt, output_ty: Float) -> Self::Item;
    fn f32_demote_from_f64(&mut self) -> Self::Item;
    fn f64_promote_from_f32(&mut self) -> Self::Item;
    fn i32_reinterpret_from_f32(&mut self) -> Self::Item;
    fn i64_reinterpret_from_f64(&mut self) -> Self::Item;
    fn f32_reinterpret_from_i32(&mut self) -> Self::Item;
    fn f64_reinterpret_from_i64(&mut self) -> Self::Item;
    fn extend(&mut self, sign: Signedness) -> Self::Item;
    fn i_sat_trunc_from_f(&mut self, input_ty: Float, output_ty: SignfulInt) -> Self::Item;
    fn memory_init(&mut self, segment: u32) -> Self::Item;
    fn data_drop(&mut self, segment: u32) -> Self::Item;
    fn memory_copy(&mut self) -> Self::Item;
    fn memory_fill(&mut self) -> Self::Item;
    fn table_init(&mut self, segment: u32) -> Self::Item;
    fn elem_drop(&mut self, segment: u32) -> Self::Item;
    fn table_copy(&mut self) -> Self::Item;
}

/// Type of a control frame.
#[derive(Debug, Clone, PartialEq)]
enum ControlFrameKind {
    /// A regular block frame.
    ///
    /// Can be used for an implicit function block.
    Block {
        needs_end_label: bool,
    },
    Function,
    /// Loop frame (branching to the beginning of block).
    Loop,
    /// True-subblock of if expression.
    If {
        has_else: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct ControlFrame {
    id: u32,
    arguments: u32,
    returns: Vec<SignlessType>,
    kind: ControlFrameKind,
}

impl ControlFrame {
    fn needs_end_label(&self) -> bool {
        match self.kind {
            ControlFrameKind::Block { needs_end_label } => needs_end_label,
            ControlFrameKind::If { .. } => true,
            ControlFrameKind::Loop | ControlFrameKind::Function => false,
        }
    }

    fn mark_branched_to(&mut self) {
        if let ControlFrameKind::Block { needs_end_label } = &mut self.kind {
            *needs_end_label = true
        }
    }

    fn br_target(&self) -> BrTarget<(u32, NameTag)> {
        match self.kind {
            ControlFrameKind::Loop => BrTarget::Label((self.id, NameTag::Header)),
            ControlFrameKind::Function => BrTarget::Return,
            ControlFrameKind::Block { .. } | ControlFrameKind::If { .. } => {
                BrTarget::Label((self.id, NameTag::End))
            }
        }
    }
}

pub struct MicrowasmConv<'a, 'b, M> {
    // TODO: Maybe have a `ConvInner` type and have this wrap an `Option` so that
    //       we can dealloc everything when we've finished emitting
    is_done: bool,
    consts_to_emit: Option<Vec<Value>>,
    stack: Vec<SignlessType>,
    internal: OperatorsReader<'a>,
    module: &'b M,
    current_id: u32,
    control_frames: Vec<ControlFrame>,
    unreachable: bool,
}

#[derive(Debug)]
enum SigT {
    T,
    Concrete(SignlessType),
}

impl From<SignlessType> for SigT {
    fn from(other: SignlessType) -> SigT {
        SigT::Concrete(other)
    }
}

#[derive(Debug)]
pub struct OpSig {
    input: SmallVec<[SigT; 3]>,
    output: SmallVec<[SigT; 3]>,
}

impl OpSig {
    #[inline(always)]
    fn new<I0, I1>(input: I0, output: I1) -> Self
    where
        I0: IntoIterator<Item = SigT>,
        I1: IntoIterator<Item = SigT>,
    {
        OpSig {
            input: SmallVec::from_iter(input),
            output: SmallVec::from_iter(output),
        }
    }

    fn none() -> Self {
        Self::new(None, None)
    }
}

impl<T> From<&'_ T> for OpSig
where
    T: Signature,
{
    fn from(other: &T) -> Self {
        OpSig::new(
            other
                .params()
                .iter()
                .map(|t| SigT::Concrete(t.to_microwasm_type())),
            other
                .returns()
                .iter()
                .map(|t| SigT::Concrete(t.to_microwasm_type())),
        )
    }
}

impl<'a, 'b, M: ModuleContext> MicrowasmConv<'a, 'b, M>
where
    for<'any> &'any M::Signature: Into<OpSig>,
{
    pub fn new(
        context: &'b M,
        params: impl IntoIterator<Item = SignlessType>,
        returns: impl IntoIterator<Item = SignlessType>,
        reader: &'a FunctionBody,
    ) -> Result<Self, Error> {
        let locals_reader = reader
            .get_locals_reader()
            .map_err(|_| Error::Microwasm("Failed to get locals reader".to_string()))?;
        let mut locals = Vec::from_iter(params);
        let mut consts = Vec::new();

        for loc in locals_reader {
            let (count, ty) =
                loc.map_err(|_| Error::Microwasm("Getting local failed".to_string()))?;
            let ty = Type::from_wasm(ty)
                .map_err(|_| Error::Microwasm("Invalid local type".to_string()))?;

            locals.extend(std::iter::repeat(ty).take(count as _));
            consts.extend(
                std::iter::repeat(ty)
                    .map(Value::default_for_type)
                    .take(count as _),
            )
        }

        let num_locals = locals.len() as _;

        let operators_reader = reader
            .get_operators_reader()
            .map_err(|_| Error::Microwasm("Failed to get operators reader".to_string()))?;
        let mut out = Self {
            is_done: false,
            stack: locals,
            module: context,
            consts_to_emit: Some(consts),
            internal: operators_reader,
            current_id: 0,
            control_frames: vec![],
            unreachable: false,
        };

        let id = out.next_id();
        out.control_frames.push(ControlFrame {
            id,
            arguments: num_locals,
            returns: returns.into_iter().collect(),
            kind: ControlFrameKind::Function,
        });

        Ok(out)
    }

    fn op_sig(&self, op: &WasmOperator) -> Result<OpSig, BinaryReaderError> {
        use self::SigT::T;
        use std::iter::{empty as none, once};

        #[inline(always)]
        fn one<A>(a: A) -> impl IntoIterator<Item = SigT>
        where
            A: Into<SigT>,
        {
            once(a.into())
        }

        #[inline(always)]
        fn two<A, B>(a: A, b: B) -> impl IntoIterator<Item = SigT>
        where
            A: Into<SigT>,
            B: Into<SigT>,
        {
            once(a.into()).chain(once(b.into()))
        }

        #[inline(always)]
        fn three<A, B, C>(a: A, b: B, c: C) -> impl IntoIterator<Item = SigT>
        where
            A: Into<SigT>,
            B: Into<SigT>,
            C: Into<SigT>,
        {
            once(a.into()).chain(once(b.into())).chain(once(c.into()))
        }

        macro_rules! sig {
            (@iter $a:expr, $b:expr, $c:expr) => { three($a, $b, $c) };
            (@iter $a:expr, $b:expr) => { two($a, $b) };
            (@iter $a:expr) => { one($a) };
            (@iter) => { none() };
            (($($t:expr),*) -> ($($o:expr),*)) => {
                OpSig::new(sig!(@iter $($t),*), sig!(@iter $($o),*))
            };
        }

        let o = match op {
            WasmOperator::Unreachable => OpSig::none(),
            WasmOperator::Nop => OpSig::none(),

            WasmOperator::Block { .. } => OpSig::none(),
            WasmOperator::Loop { .. } => OpSig::none(),
            WasmOperator::If { .. } => sig!((I32) -> ()),
            WasmOperator::Else => OpSig::none(),
            WasmOperator::End => OpSig::none(),

            WasmOperator::Br { .. } => OpSig::none(),
            WasmOperator::BrIf { .. } => sig!((I32) -> ()),
            WasmOperator::BrTable { .. } => sig!((I32) -> ()),
            WasmOperator::Return => OpSig::none(),

            WasmOperator::Call { function_index } => {
                let func_type = self.module.func_type(*function_index);
                func_type.into()
            }
            WasmOperator::CallIndirect { index, .. } => {
                let func_type = self.module.signature(*index);
                let mut out = func_type.into();
                out.input.push(I32.into());
                out
            }

            WasmOperator::Drop => sig!((T) -> ()),

            // `Select` pops 3 elements and pushes 1
            WasmOperator::Select => sig!((T, T, I32) -> (T)),

            WasmOperator::LocalGet { local_index } => {
                let ty = self.stack[*local_index as usize];

                sig!(() -> (ty))
            }
            WasmOperator::LocalSet { local_index } => {
                let ty = self.stack[*local_index as usize];

                sig!((ty) -> ())
            }
            WasmOperator::LocalTee { local_index } => {
                let ty = self.stack[*local_index as usize];

                sig!((ty) -> (ty))
            }

            WasmOperator::GlobalGet { global_index } => {
                sig!(() -> (self.module.global_type(*global_index).to_microwasm_type()))
            }
            WasmOperator::GlobalSet { global_index } => {
                sig!((self.module.global_type(*global_index).to_microwasm_type()) -> ())
            }

            WasmOperator::F32Load { .. } => sig!((I32) -> (F32)),
            WasmOperator::F64Load { .. } => sig!((I32) -> (F64)),

            WasmOperator::I32Load { .. }
            | WasmOperator::I32Load8S { .. }
            | WasmOperator::I32Load8U { .. }
            | WasmOperator::I32Load16S { .. }
            | WasmOperator::I32Load16U { .. } => sig!((I32) -> (I32)),

            WasmOperator::I64Load { .. }
            | WasmOperator::I64Load8S { .. }
            | WasmOperator::I64Load8U { .. }
            | WasmOperator::I64Load16S { .. }
            | WasmOperator::I64Load16U { .. }
            | WasmOperator::I64Load32S { .. }
            | WasmOperator::I64Load32U { .. } => sig!((I32) -> (I64)),

            WasmOperator::F32Store { .. } => sig!((I32, F32) -> ()),
            WasmOperator::F64Store { .. } => sig!((I32, F64) -> ()),
            WasmOperator::I32Store { .. }
            | WasmOperator::I32Store8 { .. }
            | WasmOperator::I32Store16 { .. } => sig!((I32, I32) -> ()),
            WasmOperator::I64Store { .. }
            | WasmOperator::I64Store8 { .. }
            | WasmOperator::I64Store16 { .. }
            | WasmOperator::I64Store32 { .. } => sig!((I32, I64) -> ()),

            WasmOperator::MemorySize { .. } => sig!(() -> (I32)),
            WasmOperator::MemoryGrow { .. } => sig!((I32) -> (I32)),

            WasmOperator::I32Const { .. } => sig!(() -> (I32)),
            WasmOperator::I64Const { .. } => sig!(() -> (I64)),
            WasmOperator::F32Const { .. } => sig!(() -> (F32)),
            WasmOperator::F64Const { .. } => sig!(() -> (F64)),

            WasmOperator::RefNull => {
                return Err(BinaryReaderError {
                    message: "RefNull unimplemented",
                    offset: -1isize as usize,
                })
            }
            WasmOperator::RefIsNull => {
                return Err(BinaryReaderError {
                    message: "RefIsNull unimplemented",
                    offset: -1isize as usize,
                })
            }

            // All comparison operators remove 2 elements and push 1
            WasmOperator::I32Eqz => sig!((I32) -> (I32)),
            WasmOperator::I32Eq
            | WasmOperator::I32Ne
            | WasmOperator::I32LtS
            | WasmOperator::I32LtU
            | WasmOperator::I32GtS
            | WasmOperator::I32GtU
            | WasmOperator::I32LeS
            | WasmOperator::I32LeU
            | WasmOperator::I32GeS
            | WasmOperator::I32GeU => sig!((I32, I32) -> (I32)),

            WasmOperator::I64Eqz => sig!((I64) -> (I32)),
            WasmOperator::I64Eq
            | WasmOperator::I64Ne
            | WasmOperator::I64LtS
            | WasmOperator::I64LtU
            | WasmOperator::I64GtS
            | WasmOperator::I64GtU
            | WasmOperator::I64LeS
            | WasmOperator::I64LeU
            | WasmOperator::I64GeS
            | WasmOperator::I64GeU => sig!((I64, I64) -> (I32)),

            WasmOperator::F32Eq
            | WasmOperator::F32Ne
            | WasmOperator::F32Lt
            | WasmOperator::F32Gt
            | WasmOperator::F32Le
            | WasmOperator::F32Ge => sig!((F32, F32) -> (I32)),

            WasmOperator::F64Eq
            | WasmOperator::F64Ne
            | WasmOperator::F64Lt
            | WasmOperator::F64Gt
            | WasmOperator::F64Le
            | WasmOperator::F64Ge => sig!((F64, F64) -> (I32)),

            WasmOperator::I32Clz | WasmOperator::I32Ctz | WasmOperator::I32Popcnt => {
                sig!((I32) -> (I32))
            }
            WasmOperator::I64Clz | WasmOperator::I64Ctz | WasmOperator::I64Popcnt => {
                sig!((I64) -> (I64))
            }

            WasmOperator::I32Add
            | WasmOperator::I32Sub
            | WasmOperator::I32Mul
            | WasmOperator::I32DivS
            | WasmOperator::I32DivU
            | WasmOperator::I32RemS
            | WasmOperator::I32RemU
            | WasmOperator::I32And
            | WasmOperator::I32Or
            | WasmOperator::I32Xor
            | WasmOperator::I32Shl
            | WasmOperator::I32ShrS
            | WasmOperator::I32ShrU
            | WasmOperator::I32Rotl
            | WasmOperator::I32Rotr => sig!((I32, I32) -> (I32)),

            WasmOperator::I64Add
            | WasmOperator::I64Sub
            | WasmOperator::I64Mul
            | WasmOperator::I64DivS
            | WasmOperator::I64DivU
            | WasmOperator::I64RemS
            | WasmOperator::I64RemU
            | WasmOperator::I64And
            | WasmOperator::I64Or
            | WasmOperator::I64Xor
            | WasmOperator::I64Shl
            | WasmOperator::I64ShrS
            | WasmOperator::I64ShrU
            | WasmOperator::I64Rotl
            | WasmOperator::I64Rotr => sig!((I64, I64) -> (I64)),

            WasmOperator::F32Abs
            | WasmOperator::F32Neg
            | WasmOperator::F32Ceil
            | WasmOperator::F32Floor
            | WasmOperator::F32Trunc
            | WasmOperator::F32Nearest
            | WasmOperator::F32Sqrt => sig!((F32) -> (F32)),

            WasmOperator::F64Abs
            | WasmOperator::F64Neg
            | WasmOperator::F64Ceil
            | WasmOperator::F64Floor
            | WasmOperator::F64Trunc
            | WasmOperator::F64Nearest
            | WasmOperator::F64Sqrt => sig!((F64) -> (F64)),

            WasmOperator::F32Add
            | WasmOperator::F32Sub
            | WasmOperator::F32Mul
            | WasmOperator::F32Div
            | WasmOperator::F32Min
            | WasmOperator::F32Max
            | WasmOperator::F32Copysign => sig!((F32, F32) -> (F32)),

            WasmOperator::F64Add
            | WasmOperator::F64Sub
            | WasmOperator::F64Mul
            | WasmOperator::F64Div
            | WasmOperator::F64Min
            | WasmOperator::F64Max
            | WasmOperator::F64Copysign => sig!((F64, F64) -> (F64)),

            WasmOperator::I32WrapI64 => sig!((I64) -> (I32)),
            WasmOperator::I32TruncF32S | WasmOperator::I32TruncF32U => sig!((F32) -> (I32)),
            WasmOperator::I32TruncF64S | WasmOperator::I32TruncF64U => sig!((F64) -> (I32)),
            WasmOperator::I64ExtendI32S | WasmOperator::I64ExtendI32U => sig!((I32) -> (I64)),
            WasmOperator::I64TruncF32S | WasmOperator::I64TruncF32U => sig!((F32) -> (I64)),
            WasmOperator::I64TruncF64S | WasmOperator::I64TruncF64U => sig!((F64) -> (I64)),
            WasmOperator::F32ConvertI32S | WasmOperator::F32ConvertI32U => sig!((I32) -> (F32)),
            WasmOperator::F32ConvertI64S | WasmOperator::F32ConvertI64U => sig!((I64) -> (F32)),
            WasmOperator::F32DemoteF64 => sig!((F64) -> (F32)),
            WasmOperator::F64ConvertI32S | WasmOperator::F64ConvertI32U => sig!((I32) -> (F64)),
            WasmOperator::F64ConvertI64S | WasmOperator::F64ConvertI64U => sig!((I64) -> (F64)),
            WasmOperator::F64PromoteF32 => sig!((F32) -> (F64)),
            WasmOperator::I32ReinterpretF32 => sig!((F32) -> (I32)),
            WasmOperator::I64ReinterpretF64 => sig!((F64) -> (I64)),
            WasmOperator::F32ReinterpretI32 => sig!((I32) -> (F32)),
            WasmOperator::F64ReinterpretI64 => sig!((I64) -> (F64)),

            WasmOperator::I32Extend8S => sig!((I32) -> (I32)),
            WasmOperator::I32Extend16S => sig!((I32) -> (I32)),
            WasmOperator::I64Extend8S => sig!((I32) -> (I64)),
            WasmOperator::I64Extend16S => sig!((I32) -> (I64)),
            WasmOperator::I64Extend32S => sig!((I32) -> (I64)),

            _ => {
                return Err(BinaryReaderError {
                    message: "Opcode Unimplemented",
                    offset: -1isize as usize,
                })
            }
        };
        Ok(o)
    }

    fn next_id(&mut self) -> u32 {
        let id = self.current_id;
        self.current_id += 1;
        id
    }

    fn nth_block(&self, n: usize) -> &ControlFrame {
        self.control_frames.iter().rev().nth(n).unwrap()
    }

    fn nth_block_mut(&mut self, n: usize) -> &mut ControlFrame {
        self.control_frames.iter_mut().rev().nth(n).unwrap()
    }

    fn function_block(&self) -> &ControlFrame {
        self.control_frames.first().unwrap()
    }

    fn local_depth(&self, idx: u32) -> i32 {
        self.stack.len() as i32 - 1 - idx as i32
    }

    fn apply_op(&mut self, sig: OpSig) -> Result<(), BinaryReaderError> {
        let mut ty_param = None;

        for p in sig.input.iter().rev() {
            let stack_ty = match self.stack.pop() {
                Some(e) => e,
                None => {
                    return Err(BinaryReaderError {
                        message: "Stack is empty",
                        offset: -1isize as usize,
                    })
                }
            };

            let ty = match p {
                SigT::T => {
                    if let Some(t) = ty_param {
                        t
                    } else {
                        ty_param = Some(stack_ty);
                        stack_ty
                    }
                }
                SigT::Concrete(ty) => *ty,
            };

            debug_assert_eq!(ty, stack_ty);
        }

        for p in sig.output.into_iter().rev() {
            let ty = match p {
                SigT::T => match ty_param {
                    Some(e) => e,
                    None => {
                        return Err(BinaryReaderError {
                            message: "Type parameter was not set",
                            offset: -1isize as usize,
                        })
                    }
                },
                SigT::Concrete(ty) => ty,
            };
            self.stack.push(ty);
        }
        Ok(())
    }

    fn block_params(&self) -> Vec<SignlessType> {
        self.stack.clone()
    }

    fn block_params_with_wasm_type(
        &self,
        ty: wasmparser::TypeOrFuncType,
    ) -> Result<Vec<SignlessType>, BinaryReaderError> {
        let mut out = self.block_params();
        let return_wasm_type = create_returns_from_wasm_type(ty)?;
        out.extend(return_wasm_type);
        Ok(out)
    }
}

impl<'a, 'b, M: ModuleContext> Iterator for MicrowasmConv<'a, 'b, M>
where
    for<'any> &'any M::Signature: Into<OpSig>,
{
    type Item = wasmparser::Result<SmallVec<[OperatorFromWasm; 1]>>;

    fn next(&mut self) -> Option<wasmparser::Result<SmallVec<[OperatorFromWasm; 1]>>> {
        macro_rules! to_drop {
            ($block:expr) => {{
                let block = &$block;
                let first_non_local_depth = block.returns.len() as u32;

                (|| {
                    let last_non_local_depth = (self.stack.len() as u32)
                        .checked_sub(1)?
                        .checked_sub(block.arguments)?;

                    if first_non_local_depth <= last_non_local_depth {
                        Some(first_non_local_depth..=last_non_local_depth)
                    } else {
                        None
                    }
                })()
            }};
        }

        if self.is_done {
            return None;
        }

        if let Some(consts) = self.consts_to_emit.take() {
            return Some(Ok(consts.into_iter().map(Operator::Const).collect()));
        }

        if self.unreachable {
            self.unreachable = false;
            let mut depth = 0;

            // `if..then..else`/`br_if` means that there may be branches in which
            // the instruction that caused us to mark this as unreachable to not
            // be executed. Tracking this in the microwasm translation step is
            // very complicated so we just do basic code removal here and leave
            // the removal of uncalled blocks to the backend.
            return Some(Ok(loop {
                let op = match self.internal.read() {
                    Err(e) => return Some(Err(e)),
                    Ok(o) => o,
                };
                match op {
                    WasmOperator::Block { .. }
                    | WasmOperator::Loop { .. }
                    | WasmOperator::If { .. } => {
                        depth += 1;
                    }
                    WasmOperator::Else => {
                        if depth == 0 {
                            let block = match self.control_frames.last_mut() {
                                Some(e) => e,
                                None => {
                                    return Some(Err(BinaryReaderError {
                                        message: "unreachable Block else Failed",
                                        offset: -1isize as usize,
                                    }))
                                }
                            };

                            self.stack.truncate(block.arguments as _);

                            if let ControlFrameKind::If { has_else, .. } = &mut block.kind {
                                *has_else = true;
                            }

                            break smallvec![Operator::Label((block.id, NameTag::Else))];
                        }
                    }
                    WasmOperator::End => {
                        if depth == 0 {
                            let block = match self.control_frames.pop() {
                                Some(e) => e,
                                None => {
                                    return Some(Err(BinaryReaderError {
                                        message: "unreachable Block end Failed",
                                        offset: -1isize as usize,
                                    }))
                                }
                            };

                            if self.control_frames.is_empty() {
                                self.is_done = true;
                                return None;
                            }

                            self.stack.truncate(block.arguments as _);
                            self.stack.extend(block.returns);

                            let end_label = (block.id, NameTag::End);

                            if let ControlFrameKind::If {
                                has_else: false, ..
                            } = block.kind
                            {
                                break smallvec![
                                    Operator::Label((block.id, NameTag::Else)),
                                    Operator::Br {
                                        target: BrTarget::Label(end_label),
                                    },
                                    Operator::Label(end_label),
                                ];
                            } else {
                                break smallvec![Operator::Label((block.id, NameTag::End))];
                            }
                        } else {
                            depth -= 1;
                        }
                    }
                    _ => {}
                }
            }));
        }

        let op = match self.internal.read() {
            Err(e) => return Some(Err(e)),
            Ok(o) => o,
        };

        let op_sig = match self.op_sig(&op) {
            Ok(o) => o,
            Err(e) => return Some(Err(e)),
        };

        match self.apply_op(op_sig) {
            Ok(o) => o,
            Err(e) => return Some(Err(e)),
        };

        Some(Ok(match op {
            WasmOperator::Unreachable => {
                self.unreachable = true;
                smallvec![Operator::Unreachable]
            }
            WasmOperator::Nop => smallvec![],
            WasmOperator::Block { ty } => {
                let id = self.next_id();
                let return_type_wasm = match create_returns_from_wasm_type(ty) {
                    Ok(o) => o,
                    Err(e) => return Some(Err(e)),
                };
                let block_param_type_wasm = match self.block_params_with_wasm_type(ty) {
                    Err(e) => return Some(Err(e)),
                    Ok(o) => o,
                };
                self.control_frames.push(ControlFrame {
                    id,
                    arguments: self.stack.len() as u32,
                    returns: return_type_wasm,
                    kind: ControlFrameKind::Block {
                        needs_end_label: false,
                    },
                });
                smallvec![Operator::end(block_param_type_wasm, (id, NameTag::End))]
            }
            WasmOperator::Loop { ty } => {
                let id = self.next_id();
                let return_type_wasm = match create_returns_from_wasm_type(ty) {
                    Ok(o) => o,
                    Err(e) => return Some(Err(e)),
                };
                let block_param_type_wasm = match self.block_params_with_wasm_type(ty) {
                    Ok(o) => o,
                    Err(e) => return Some(Err(e)),
                };
                self.control_frames.push(ControlFrame {
                    id,
                    arguments: self.stack.len() as u32,
                    returns: return_type_wasm,
                    kind: ControlFrameKind::Loop,
                });
                let label = (id, NameTag::Header);
                smallvec![
                    Operator::loop_(self.block_params(), label),
                    Operator::end(block_param_type_wasm, (id, NameTag::End)),
                    Operator::Br {
                        target: BrTarget::Label(label),
                    },
                    Operator::Label(label),
                ]
            }
            WasmOperator::If { ty } => {
                let id = self.next_id();
                let return_type_wasm = match create_returns_from_wasm_type(ty) {
                    Ok(o) => o,
                    Err(e) => return Some(Err(e)),
                };
                let block_param_type_wasm = match self.block_params_with_wasm_type(ty) {
                    Ok(o) => o,
                    Err(e) => return Some(Err(e)),
                };
                self.control_frames.push(ControlFrame {
                    id,
                    arguments: self.stack.len() as u32,
                    returns: return_type_wasm,
                    kind: ControlFrameKind::If { has_else: false },
                });
                let (then, else_, end) = (
                    (id, NameTag::Header),
                    (id, NameTag::Else),
                    (id, NameTag::End),
                );
                smallvec![
                    Operator::block(self.block_params(), then),
                    Operator::block(self.block_params(), else_),
                    Operator::end(block_param_type_wasm, end),
                    Operator::BrIf {
                        then: BrTarget::Label(then).into(),
                        else_: BrTarget::Label(else_).into()
                    },
                    Operator::Label(then),
                ]
            }
            WasmOperator::Else => {
                // We don't pop it since we're still in the second block.
                let block = match self.control_frames.last() {
                    Some(e) => e,
                    None => {
                        return Some(Err(BinaryReaderError {
                            message: "Block else Failed",
                            offset: -1isize as usize,
                        }))
                    }
                };
                let to_drop = to_drop!(block);
                let block = match self.control_frames.last_mut() {
                    Some(e) => e,
                    None => {
                        return Some(Err(BinaryReaderError {
                            message: "Block else Failed",
                            offset: -1isize as usize,
                        }))
                    }
                };

                if let ControlFrameKind::If { has_else, .. } = &mut block.kind {
                    *has_else = true;
                }

                self.stack.truncate(block.arguments as _);

                let label = (block.id, NameTag::Else);

                SmallVec::from_iter(
                    to_drop
                        .into_iter()
                        .map(Operator::Drop)
                        .chain(iter::once(Operator::Br {
                            target: BrTarget::Label((block.id, NameTag::End)),
                        }))
                        .chain(iter::once(Operator::Label(label))),
                )
            }
            WasmOperator::End => {
                let block = match self.control_frames.pop() {
                    Some(e) => e,
                    None => {
                        return Some(Err(BinaryReaderError {
                            message: "Block End Failed",
                            offset: -1isize as usize,
                        }))
                    }
                };

                let to_drop = to_drop!(block);

                self.stack.truncate(block.arguments as _);
                self.stack.extend(block.returns.iter().cloned());

                if let ControlFrameKind::If {
                    has_else: false, ..
                } = block.kind
                {
                    let else_ = (block.id, NameTag::Else);
                    let end = (block.id, NameTag::End);

                    to_drop
                        .map(Operator::Drop)
                        .into_iter()
                        .chain::<SmallVec<[_; 4]>>(smallvec![
                            Operator::Br {
                                target: BrTarget::Label(end),
                            },
                            Operator::Label(else_),
                            Operator::Br {
                                target: BrTarget::Label(end),
                            },
                            Operator::Label(end),
                        ])
                        .collect()
                } else {
                    SmallVec::from_iter(if self.control_frames.is_empty() {
                        self.is_done = true;

                        None.into_iter()
                            .chain(Some(Operator::Br {
                                target: BrTarget::Return,
                            }))
                            .chain(None)
                    } else if block.needs_end_label() {
                        let label = (block.id, NameTag::End);

                        to_drop
                            .map(Operator::Drop)
                            .into_iter()
                            .chain(Some(Operator::Br {
                                target: BrTarget::Label(label),
                            }))
                            .chain(Some(Operator::Label(label)))
                    } else {
                        to_drop
                            .map(Operator::Drop)
                            .into_iter()
                            .chain(None)
                            .chain(None)
                    })
                }
            }
            // TODO: If we're breaking out of the function block we want
            //       to drop locals too (see code for `WasmOperator::End`)
            WasmOperator::Br { relative_depth } => {
                self.unreachable = true;
                let to_drop = to_drop!(self.nth_block(relative_depth as _));

                let block = self.nth_block_mut(relative_depth as _);
                block.mark_branched_to();
                SmallVec::from_iter(to_drop.into_iter().map(Operator::Drop).chain(iter::once(
                    Operator::Br {
                        target: block.br_target(),
                    },
                )))
            }
            WasmOperator::BrIf { relative_depth } => {
                let to_drop = to_drop!(self.nth_block(relative_depth as _));

                let label = (self.next_id(), NameTag::Header);
                let params = self.block_params();
                let block = self.nth_block_mut(relative_depth as _);
                block.mark_branched_to();

                smallvec![
                    Operator::block(params, label),
                    Operator::BrIf {
                        then: BrTargetDrop {
                            to_drop,
                            target: block.br_target()
                        },
                        else_: BrTarget::Label(label).into(),
                    },
                    Operator::Label(label),
                ]
            }
            WasmOperator::BrTable { table } => {
                self.unreachable = true;
                let (entries, default) = match table.read_table() {
                    Ok(o) => o,
                    Err(e) => return Some(Err(e)),
                };
                let targets = entries
                    .iter()
                    .map(|depth| {
                        self.nth_block_mut(*depth as _).mark_branched_to();
                        let block = self.nth_block(*depth as _);

                        let target = block.br_target();
                        BrTargetDrop {
                            to_drop: to_drop!(block),
                            target,
                        }
                    })
                    .collect();

                self.nth_block_mut(default as _).mark_branched_to();

                let default = self.nth_block(default as _);
                let target = default.br_target();
                let default = BrTargetDrop {
                    to_drop: to_drop!(default),
                    target,
                };

                smallvec![Operator::BrTable(BrTable { targets, default })]
            }
            WasmOperator::Return => {
                self.unreachable = true;

                let block = self.function_block();
                let to_drop = to_drop!(block);

                SmallVec::from_iter(to_drop.into_iter().map(Operator::Drop).chain(iter::once(
                    Operator::Br {
                        target: block.br_target(),
                    },
                )))
            }
            WasmOperator::Call { function_index } => smallvec![Operator::Call { function_index }],
            WasmOperator::CallIndirect { index, table_index } => {
                smallvec![Operator::CallIndirect {
                    type_index: index,
                    table_index,
                }]
            }
            WasmOperator::Drop => smallvec![Operator::Drop(0..=0)],
            WasmOperator::Select => smallvec![Operator::Select],

            WasmOperator::LocalGet { local_index } => {
                // `- 1` because we apply the stack difference _before_ this point
                let depth = self.local_depth(local_index).checked_sub(1)?;
                let depth = match depth.try_into() {
                    Ok(o) => o,
                    Err(_) => {
                        return Some(Err(BinaryReaderError {
                            message: "GetLocal - Local out of range",
                            offset: -1isize as usize,
                        }))
                    }
                };
                smallvec![Operator::Pick(depth)]
            }
            WasmOperator::LocalSet { local_index } => {
                // `+ 1` because we apply the stack difference _before_ this point
                let depth = self.local_depth(local_index).checked_add(1)?;
                let depth = match depth.try_into() {
                    Ok(o) => o,
                    Err(_) => {
                        return Some(Err(BinaryReaderError {
                            message: "SetLocal - Local out of range",
                            offset: -1isize as usize,
                        }))
                    }
                };
                smallvec![Operator::Swap(depth), Operator::Drop(0..=0)]
            }
            WasmOperator::LocalTee { local_index } => {
                // `+ 1` because we `pick` before `swap`
                let depth = self.local_depth(local_index).checked_add(1)?;
                let depth = match depth.try_into() {
                    Ok(o) => o,
                    Err(_) => {
                        return Some(Err(BinaryReaderError {
                            message: "SetLocal - Local out of range",
                            offset: -1isize as usize,
                        }))
                    }
                };
                smallvec![
                    Operator::Pick(0),
                    Operator::Swap(depth),
                    Operator::Drop(0..=0),
                ]
            }
            WasmOperator::GlobalGet { global_index } => {
                smallvec![Operator::GetGlobal(global_index)]
            }
            WasmOperator::GlobalSet { global_index } => {
                smallvec![Operator::SetGlobal(global_index)]
            }

            WasmOperator::I32Load { memarg } => smallvec![Operator::Load {
                ty: I32,
                memarg: memarg.into()
            }],
            WasmOperator::I64Load { memarg } => smallvec![Operator::Load {
                ty: I64,
                memarg: memarg.into()
            }],
            WasmOperator::F32Load { memarg } => smallvec![Operator::Load {
                ty: F32,
                memarg: memarg.into()
            }],
            WasmOperator::F64Load { memarg } => smallvec![Operator::Load {
                ty: F64,
                memarg: memarg.into()
            }],
            WasmOperator::I32Load8S { memarg } => smallvec![Operator::Load8 {
                ty: sint::I32,
                memarg: memarg.into(),
            }],
            WasmOperator::I32Load8U { memarg } => smallvec![Operator::Load8 {
                ty: sint::U32,
                memarg: memarg.into(),
            }],
            WasmOperator::I32Load16S { memarg } => smallvec![Operator::Load16 {
                ty: sint::I32,
                memarg: memarg.into(),
            }],
            WasmOperator::I32Load16U { memarg } => smallvec![Operator::Load16 {
                ty: sint::U32,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Load8S { memarg } => smallvec![Operator::Load8 {
                ty: sint::I64,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Load8U { memarg } => smallvec![Operator::Load8 {
                ty: sint::U64,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Load16S { memarg } => smallvec![Operator::Load16 {
                ty: sint::I64,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Load16U { memarg } => smallvec![Operator::Load16 {
                ty: sint::U64,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Load32S { memarg } => smallvec![Operator::Load32 {
                sign: Signedness::Signed,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Load32U { memarg } => smallvec![Operator::Load32 {
                sign: Signedness::Unsigned,
                memarg: memarg.into(),
            }],

            WasmOperator::I32Store { memarg } => smallvec![Operator::Store {
                ty: I32,
                memarg: memarg.into()
            }],
            WasmOperator::I64Store { memarg } => smallvec![Operator::Store {
                ty: I64,
                memarg: memarg.into()
            }],
            WasmOperator::F32Store { memarg } => smallvec![Operator::Store {
                ty: F32,
                memarg: memarg.into()
            }],
            WasmOperator::F64Store { memarg } => smallvec![Operator::Store {
                ty: F64,
                memarg: memarg.into()
            }],

            WasmOperator::I32Store8 { memarg } => smallvec![Operator::Store8 {
                ty: Size::_32,
                memarg: memarg.into(),
            }],
            WasmOperator::I32Store16 { memarg } => smallvec![Operator::Store16 {
                ty: Size::_32,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Store8 { memarg } => smallvec![Operator::Store8 {
                ty: Size::_64,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Store16 { memarg } => smallvec![Operator::Store16 {
                ty: Size::_64,
                memarg: memarg.into(),
            }],
            WasmOperator::I64Store32 { memarg } => smallvec![Operator::Store32 {
                memarg: memarg.into()
            }],
            WasmOperator::MemorySize { reserved } => smallvec![Operator::MemorySize { reserved }],
            WasmOperator::MemoryGrow { reserved } => smallvec![Operator::MemoryGrow { reserved }],
            WasmOperator::I32Const { value } => smallvec![Operator::Const(Value::I32(value))],
            WasmOperator::I64Const { value } => smallvec![Operator::Const(Value::I64(value))],
            WasmOperator::F32Const { value } => {
                smallvec![Operator::Const(Value::F32(value.into()))]
            }
            WasmOperator::F64Const { value } => {
                smallvec![Operator::Const(Value::F64(value.into()))]
            }
            WasmOperator::RefNull => {
                return Some(Err(BinaryReaderError {
                    message: "RefNull unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::RefIsNull => {
                return Some(Err(BinaryReaderError {
                    message: "RefIsNull unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I32Eqz => smallvec![Operator::Eqz(Size::_32)],
            WasmOperator::I32Eq => smallvec![Operator::Eq(I32)],
            WasmOperator::I32Ne => smallvec![Operator::Ne(I32)],
            WasmOperator::I32LtS => smallvec![Operator::Lt(SI32)],
            WasmOperator::I32LtU => smallvec![Operator::Lt(SU32)],
            WasmOperator::I32GtS => smallvec![Operator::Gt(SI32)],
            WasmOperator::I32GtU => smallvec![Operator::Gt(SU32)],
            WasmOperator::I32LeS => smallvec![Operator::Le(SI32)],
            WasmOperator::I32LeU => smallvec![Operator::Le(SU32)],
            WasmOperator::I32GeS => smallvec![Operator::Ge(SI32)],
            WasmOperator::I32GeU => smallvec![Operator::Ge(SU32)],
            WasmOperator::I64Eqz => smallvec![Operator::Eqz(Size::_64)],
            WasmOperator::I64Eq => smallvec![Operator::Eq(I64)],
            WasmOperator::I64Ne => smallvec![Operator::Ne(I64)],
            WasmOperator::I64LtS => smallvec![Operator::Lt(SI64)],
            WasmOperator::I64LtU => smallvec![Operator::Lt(SU64)],
            WasmOperator::I64GtS => smallvec![Operator::Gt(SI64)],
            WasmOperator::I64GtU => smallvec![Operator::Gt(SU64)],
            WasmOperator::I64LeS => smallvec![Operator::Le(SI64)],
            WasmOperator::I64LeU => smallvec![Operator::Le(SU64)],
            WasmOperator::I64GeS => smallvec![Operator::Ge(SI64)],
            WasmOperator::I64GeU => smallvec![Operator::Ge(SU64)],
            WasmOperator::F32Eq => smallvec![Operator::Eq(F32)],
            WasmOperator::F32Ne => smallvec![Operator::Ne(F32)],
            WasmOperator::F32Lt => smallvec![Operator::Lt(SF32)],
            WasmOperator::F32Gt => smallvec![Operator::Gt(SF32)],
            WasmOperator::F32Le => smallvec![Operator::Le(SF32)],
            WasmOperator::F32Ge => smallvec![Operator::Ge(SF32)],
            WasmOperator::F64Eq => smallvec![Operator::Eq(F64)],
            WasmOperator::F64Ne => smallvec![Operator::Ne(F64)],
            WasmOperator::F64Lt => smallvec![Operator::Lt(SF64)],
            WasmOperator::F64Gt => smallvec![Operator::Gt(SF64)],
            WasmOperator::F64Le => smallvec![Operator::Le(SF64)],
            WasmOperator::F64Ge => smallvec![Operator::Ge(SF64)],
            WasmOperator::I32Clz => smallvec![Operator::Clz(Size::_32)],
            WasmOperator::I32Ctz => smallvec![Operator::Ctz(Size::_32)],
            WasmOperator::I32Popcnt => smallvec![Operator::Popcnt(Size::_32)],
            WasmOperator::I32Add => smallvec![Operator::Add(I32)],
            WasmOperator::I32Sub => smallvec![Operator::Sub(I32)],
            WasmOperator::I32Mul => smallvec![Operator::Mul(I32)],
            WasmOperator::I32DivS => smallvec![Operator::Div(SI32)],
            WasmOperator::I32DivU => smallvec![Operator::Div(SU32)],
            // Unlike Wasm, our `rem_s` instruction _does_ trap on `-1`. Instead
            // of handling this complexity in the backend, we handle it here
            // (where it's way easier to debug).
            WasmOperator::I32RemS => smallvec![Operator::Rem(sint::I32),],

            WasmOperator::I32RemU => smallvec![Operator::Rem(sint::U32),],
            WasmOperator::I32And => smallvec![Operator::And(Size::_32)],
            WasmOperator::I32Or => smallvec![Operator::Or(Size::_32)],
            WasmOperator::I32Xor => smallvec![Operator::Xor(Size::_32)],
            WasmOperator::I32Shl => smallvec![Operator::Shl(Size::_32)],
            WasmOperator::I32ShrS => smallvec![Operator::Shr(sint::I32)],
            WasmOperator::I32ShrU => smallvec![Operator::Shr(sint::U32)],
            WasmOperator::I32Rotl => smallvec![Operator::Rotl(Size::_32)],
            WasmOperator::I32Rotr => smallvec![Operator::Rotr(Size::_32)],
            WasmOperator::I64Clz => smallvec![Operator::Clz(Size::_64)],
            WasmOperator::I64Ctz => smallvec![Operator::Ctz(Size::_64)],
            WasmOperator::I64Popcnt => smallvec![Operator::Popcnt(Size::_64)],
            WasmOperator::I64Add => smallvec![Operator::Add(I64)],
            WasmOperator::I64Sub => smallvec![Operator::Sub(I64)],
            WasmOperator::I64Mul => smallvec![Operator::Mul(I64)],
            WasmOperator::I64DivS => smallvec![Operator::Div(SI64)],
            WasmOperator::I64DivU => smallvec![Operator::Div(SU64)],
            WasmOperator::I64RemS => smallvec![Operator::Rem(sint::I64),],

            WasmOperator::I64RemU => smallvec![Operator::Rem(sint::U64)],
            WasmOperator::I64And => smallvec![Operator::And(Size::_64)],
            WasmOperator::I64Or => smallvec![Operator::Or(Size::_64)],
            WasmOperator::I64Xor => smallvec![Operator::Xor(Size::_64)],
            WasmOperator::I64Shl => smallvec![Operator::Shl(Size::_64)],
            WasmOperator::I64ShrS => smallvec![Operator::Shr(sint::I64)],
            WasmOperator::I64ShrU => smallvec![Operator::Shr(sint::U64)],
            WasmOperator::I64Rotl => smallvec![Operator::Rotl(Size::_64)],
            WasmOperator::I64Rotr => smallvec![Operator::Rotr(Size::_64)],
            WasmOperator::F32Abs => smallvec![Operator::Abs(Size::_32)],
            WasmOperator::F32Neg => smallvec![Operator::Neg(Size::_32)],
            WasmOperator::F32Ceil => smallvec![Operator::Ceil(Size::_32)],
            WasmOperator::F32Floor => smallvec![Operator::Floor(Size::_32)],
            WasmOperator::F32Trunc => smallvec![Operator::Trunc(Size::_32)],
            WasmOperator::F32Nearest => smallvec![Operator::Nearest(Size::_32)],
            WasmOperator::F32Sqrt => smallvec![Operator::Sqrt(Size::_32)],
            WasmOperator::F32Add => smallvec![Operator::Add(F32)],
            WasmOperator::F32Sub => smallvec![Operator::Sub(F32)],
            WasmOperator::F32Mul => smallvec![Operator::Mul(F32)],
            WasmOperator::F32Div => smallvec![Operator::Div(SF32)],
            WasmOperator::F32Min => smallvec![Operator::Min(Size::_32)],
            WasmOperator::F32Max => smallvec![Operator::Max(Size::_32)],
            WasmOperator::F32Copysign => smallvec![Operator::Copysign(Size::_32)],
            WasmOperator::F64Abs => smallvec![Operator::Abs(Size::_64)],
            WasmOperator::F64Neg => smallvec![Operator::Neg(Size::_64)],
            WasmOperator::F64Ceil => smallvec![Operator::Ceil(Size::_64)],
            WasmOperator::F64Floor => smallvec![Operator::Floor(Size::_64)],
            WasmOperator::F64Trunc => smallvec![Operator::Trunc(Size::_64)],
            WasmOperator::F64Nearest => smallvec![Operator::Nearest(Size::_64)],
            WasmOperator::F64Sqrt => smallvec![Operator::Sqrt(Size::_64)],
            WasmOperator::F64Add => smallvec![Operator::Add(F64)],
            WasmOperator::F64Sub => smallvec![Operator::Sub(F64)],
            WasmOperator::F64Mul => smallvec![Operator::Mul(F64)],
            WasmOperator::F64Div => smallvec![Operator::Div(SF64)],
            WasmOperator::F64Min => smallvec![Operator::Min(Size::_64)],
            WasmOperator::F64Max => smallvec![Operator::Max(Size::_64)],
            WasmOperator::F64Copysign => smallvec![Operator::Copysign(Size::_64)],
            WasmOperator::I32WrapI64 => smallvec![Operator::I32WrapFromI64],
            WasmOperator::I32TruncF32S => smallvec![Operator::ITruncFromF {
                input_ty: Size::_32,
                output_ty: sint::I32
            }],
            WasmOperator::I32TruncF32U => smallvec![Operator::ITruncFromF {
                input_ty: Size::_32,
                output_ty: sint::U32
            }],
            WasmOperator::I32TruncF64S => smallvec![Operator::ITruncFromF {
                input_ty: Size::_64,
                output_ty: sint::I32
            }],
            WasmOperator::I32TruncF64U => smallvec![Operator::ITruncFromF {
                input_ty: Size::_64,
                output_ty: sint::U32
            }],
            WasmOperator::I64ExtendI32S => smallvec![Operator::Extend {
                sign: Signedness::Signed
            }],
            WasmOperator::I64ExtendI32U => smallvec![Operator::Extend {
                sign: Signedness::Unsigned
            }],
            WasmOperator::I64TruncF32S => smallvec![Operator::ITruncFromF {
                input_ty: Size::_32,
                output_ty: sint::I64,
            }],
            WasmOperator::I64TruncF32U => smallvec![Operator::ITruncFromF {
                input_ty: Size::_32,
                output_ty: sint::U64,
            }],
            WasmOperator::I64TruncF64S => smallvec![Operator::ITruncFromF {
                input_ty: Size::_64,
                output_ty: sint::I64,
            }],
            WasmOperator::I64TruncF64U => smallvec![Operator::ITruncFromF {
                input_ty: Size::_64,
                output_ty: sint::U64,
            }],
            WasmOperator::F32ConvertI32S => smallvec![Operator::FConvertFromI {
                input_ty: sint::I32,
                output_ty: Size::_32
            }],
            WasmOperator::F32ConvertI32U => smallvec![Operator::FConvertFromI {
                input_ty: sint::U32,
                output_ty: Size::_32
            }],
            WasmOperator::F32ConvertI64S => smallvec![Operator::FConvertFromI {
                input_ty: sint::I64,
                output_ty: Size::_32
            }],
            WasmOperator::F32ConvertI64U => smallvec![Operator::FConvertFromI {
                input_ty: sint::U64,
                output_ty: Size::_32
            }],
            WasmOperator::F64ConvertI32S => smallvec![Operator::FConvertFromI {
                input_ty: sint::I32,
                output_ty: Size::_64
            }],
            WasmOperator::F64ConvertI32U => smallvec![Operator::FConvertFromI {
                input_ty: sint::U32,
                output_ty: Size::_64
            }],
            WasmOperator::F64ConvertI64S => smallvec![Operator::FConvertFromI {
                input_ty: sint::I64,
                output_ty: Size::_64
            }],
            WasmOperator::F64ConvertI64U => smallvec![Operator::FConvertFromI {
                input_ty: sint::U64,
                output_ty: Size::_64
            }],
            WasmOperator::F32DemoteF64 => smallvec![Operator::F32DemoteFromF64],
            WasmOperator::F64PromoteF32 => smallvec![Operator::F64PromoteFromF32],
            WasmOperator::I32ReinterpretF32 => smallvec![Operator::I32ReinterpretFromF32],
            WasmOperator::I64ReinterpretF64 => smallvec![Operator::I64ReinterpretFromF64],
            WasmOperator::F32ReinterpretI32 => smallvec![Operator::F32ReinterpretFromI32],
            WasmOperator::F64ReinterpretI64 => smallvec![Operator::F64ReinterpretFromI64],
            WasmOperator::I32Extend8S => {
                return Some(Err(BinaryReaderError {
                    message: "I32Extend8S unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I32Extend16S => {
                return Some(Err(BinaryReaderError {
                    message: "I32Extend16S unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I64Extend8S => {
                return Some(Err(BinaryReaderError {
                    message: "I64Extend8S unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I64Extend16S => {
                return Some(Err(BinaryReaderError {
                    message: "I64Extend16S unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I64Extend32S => {
                return Some(Err(BinaryReaderError {
                    message: "I64Extend32S unimplemented",
                    offset: -1isize as usize,
                }))
            }

            // 0xFC operators
            // Non-trapping Float-to-int Conversions
            WasmOperator::I32TruncSatF32S => {
                return Some(Err(BinaryReaderError {
                    message: "I32TruncSSatF32 unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I32TruncSatF32U => {
                return Some(Err(BinaryReaderError {
                    message: "I32TruncUSatF32 unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I32TruncSatF64S => {
                return Some(Err(BinaryReaderError {
                    message: "I32TruncSSatF64 unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I32TruncSatF64U => {
                return Some(Err(BinaryReaderError {
                    message: "I32TruncUSatF64 unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I64TruncSatF32S => {
                return Some(Err(BinaryReaderError {
                    message: "I64TruncSSatF32 unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I64TruncSatF32U => {
                return Some(Err(BinaryReaderError {
                    message: "I64TruncUSatF32 unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I64TruncSatF64S => {
                return Some(Err(BinaryReaderError {
                    message: "I64TruncSSatF64 unimplemented",
                    offset: -1isize as usize,
                }))
            }
            WasmOperator::I64TruncSatF64U => {
                return Some(Err(BinaryReaderError {
                    message: "I64TruncUSatF64 unimplemented",
                    offset: -1isize as usize,
                }))
            }

            _other => {
                return Some(Err(BinaryReaderError {
                    message: "Opcode unimplemented",
                    offset: -1isize as usize,
                }))
            }
        }))
    }
}

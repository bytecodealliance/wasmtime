
//! Representation of Cretonne IL functions.

use types::{Type, FunctionName, Signature};
use immediates::*;
use std::fmt::{self, Display, Formatter, Write};
use std::ops::Index;
use std::u32;

// ====--------------------------------------------------------------------------------------====//
//
// Public data types.
//
// ====--------------------------------------------------------------------------------------====//

/// An opaque reference to an extended basic block in a function.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Ebb(u32);

/// A guaranteed invalid EBB reference.
pub const NO_EBB: Ebb = Ebb(u32::MAX);

/// An opaque reference to an instruction in a function.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Inst(u32);

/// A guaranteed invalid instruction reference.
pub const NO_INST: Inst = Inst(u32::MAX);

/// An opaque reference to an SSA value.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Value(u32);

/// A guaranteed invalid value reference.
pub const NO_VALUE: Value = Value(u32::MAX);

/// An opaque reference to a stack slot.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct StackSlot(u32);

/// A guaranteed invalid stack slot reference.
pub const NO_STACK_SLOT: StackSlot = StackSlot(u32::MAX);

/// A function.
///
/// The `Function` struct owns all of its instructions and extended basic blocks, and it works as a
/// container for those objects by implementing both `Index<Inst>` and `Index<Ebb>`.
///
#[derive(Debug)]
pub struct Function {
    /// Name of this function. Mostly used by `.cton` files.
    pub name: FunctionName,

    /// Signature of this function.
    signature: Signature,

    /// Stack slots allocated in this function.
    stack_slots: Vec<StackSlotData>,

    /// Data about all of the instructions in the function. The instructions in this vector is not
    /// necessarily in program order. The `Inst` reference indexes into this vector.
    instructions: Vec<InstructionData>,

    /// Extended basic blocks in the function, not necessarily in program order. The `Ebb`
    /// reference indexes into this vector.
    extended_basic_blocks: Vec<EbbData>,

    /// Extended value table. Most `Value` references refer directly to their defining instruction.
    /// Others index into this table.
    extended_values: Vec<ValueData>,

    /// Return type(s). A function may return zero or more values.
    pub return_types: Vec<Type>,
}

/// Contents of a stack slot.
#[derive(Debug)]
pub struct StackSlotData {
    /// Size of stack slot in bytes.
    pub size: u32,
}

/// Contents of an extended basic block.
#[derive(Debug)]
pub struct EbbData {
    /// Arguments for this extended basic block. These values dominate everything in the EBB.
    /// All branches to this EBB must provide matching arguments, and the arguments to the entry
    /// EBB must match the function arguments.
    pub arguments: Vec<Value>,
}

/// Contents on an instruction.
///
/// Every variant must contain `opcode` and `ty` fields. An instruction that doesn't produce a
/// value should have its `ty` field set to `VOID`. The size of `InstructionData` should be kept at
/// 16 bytes on 64-bit architectures. If more space is needed to represent an instruction, use a
/// `Box<AuxData>` to store the additional information out of line.
#[derive(Debug)]
pub enum InstructionData {
    Nullary {
        opcode: Opcode,
        ty: Type,
    },
    Unary {
        opcode: Opcode,
        ty: Type,
        arg: Value,
    },
    UnaryImm {
        opcode: Opcode,
        ty: Type,
        imm: Imm64,
    },
    Binary {
        opcode: Opcode,
        ty: Type,
        args: [Value; 2],
    },
    BinaryImm {
        opcode: Opcode,
        ty: Type,
        arg: Value,
        imm: Imm64,
    },
    Call {
        opcode: Opcode,
        ty: Type,
        data: Box<CallData>,
    },
}

/// Payload of a call instruction.
#[derive(Debug)]
pub struct CallData {
    // Number of result values.
    results: u8,

    // Dynamically sized array containing `results-1` result values (not including the first value)
    // followed by the argument values.
    values: Vec<Value>,
}


// ====--------------------------------------------------------------------------------------====//
//
// Stack slot implementation.
//
// ====--------------------------------------------------------------------------------------====//

impl StackSlot {
    fn new(index: usize) -> StackSlot {
        assert!(index < (u32::MAX as usize));
        StackSlot(index as u32)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Display a `StackSlot` reference as "ss12".
impl Display for StackSlot {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "ss{}", self.0)
    }
}

impl StackSlotData {
    /// Create a stack slot with the specified byte size.
    pub fn new(size: u32) -> StackSlotData {
        StackSlotData { size: size }
    }
}

impl Display for StackSlotData {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "stack_slot {}", self.size)
    }
}

/// Allow immutable access to stack slots via function indexing.
impl Index<StackSlot> for Function {
    type Output = StackSlotData;

    fn index<'a>(&'a self, ss: StackSlot) -> &'a StackSlotData {
        &self.stack_slots[ss.index()]
    }
}

/// Stack slot iterator visits all stack slots in a function, returning `StackSlot` references.
pub struct StackSlotIter {
    cur: usize,
    end: usize,
}

impl Iterator for StackSlotIter {
    type Item = StackSlot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.end {
            let ss = StackSlot::new(self.cur);
            self.cur += 1;
            Some(ss)
        } else {
            None
        }
    }
}

// ====--------------------------------------------------------------------------------------====//
//
// Extended basic block implementation.
//
// ====--------------------------------------------------------------------------------------====//

impl Ebb {
    fn new(index: usize) -> Ebb {
        assert!(index < (u32::MAX as usize));
        Ebb(index as u32)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Display an `Ebb` reference as "ebb12".
impl Display for Ebb {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "ebb{}", self.0)
    }
}

impl EbbData {
    fn new() -> EbbData {
        EbbData { arguments: Vec::new() }
    }
}

// ====--------------------------------------------------------------------------------------====//
//
// Instruction implementation.
//
// ====--------------------------------------------------------------------------------------====//

impl Inst {
    fn new(index: usize) -> Inst {
        assert!(index < (u32::MAX as usize));
        Inst(index as u32)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Display an `Inst` reference as "inst7".
impl Display for Inst {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "inst{}", self.0)
    }
}

/// Allow immutable access to instructions via function indexing.
impl Index<Inst> for Function {
    type Output = InstructionData;

    fn index<'a>(&'a self, inst: Inst) -> &'a InstructionData {
        &self.instructions[inst.index()]
    }
}

// ====--------------------------------------------------------------------------------------====//
//
// Value implementation.
//
// ====--------------------------------------------------------------------------------------====//

// Value references can either reference an instruction directly, or they can refer to the
// extended value table.
enum ExpandedValue {
    // This is the first value produced by the referenced instruction.
    Direct(Inst),

    // This value is described in the extended value table.
    Table(usize),
}

impl Value {
    fn new_direct(i: Inst) -> Value {
        let encoding = i.index() * 2;
        assert!(encoding < u32::MAX as usize);
        Value(encoding as u32)
    }

    fn new_table(index: usize) -> Value {
        let encoding = index * 2 + 1;
        assert!(encoding < u32::MAX as usize);
        Value(encoding as u32)
    }

    // Expand the internal representation into something useful.
    fn expand(&self) -> ExpandedValue {
        use self::ExpandedValue::*;
        let index = (self.0 / 2) as usize;
        if self.0 % 2 == 0 {
            Direct(Inst::new(index))
        } else {
            Table(index)
        }
    }
}

/// Display a `Value` reference as "v7" or "v2x".
impl Display for Value {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        use self::ExpandedValue::*;
        match self.expand() {
            Direct(i) => write!(fmt, "v{}", i.0),
            Table(i) => write!(fmt, "v{}x", i),
        }
    }
}

// Most values are simply the first value produced by an instruction.
// Other values have an entry in the value table.
#[derive(Debug)]
enum ValueData {
    // An unused entry in the value table. No instruction should be defining or using this value.
    Unused,

    // Value is defined by an instruction, but it is not the first result.
    Def {
        ty: Type,
        num: u8,
        def: Inst,
    },

    // Value is an EBB argument.
    Argument {
        ty: Type,
        num: u8,
        ebb: Ebb,
    },
}

impl InstructionData {
    /// Get the opcode of this instruction.
    pub fn opcode(&self) -> Opcode {
        use self::InstructionData::*;
        match *self {
            Nullary { opcode, .. } => opcode,
            Unary { opcode, .. } => opcode,
            UnaryImm { opcode, .. } => opcode,
            Binary { opcode, .. } => opcode,
            BinaryImm { opcode, .. } => opcode,
            Call { opcode, .. } => opcode,
        }
    }

    /// Type of the first result.
    pub fn first_type(&self) -> Type {
        use self::InstructionData::*;
        match *self {
            Nullary { ty, .. } => ty,
            Unary { ty, .. } => ty,
            UnaryImm { ty, .. } => ty,
            Binary { ty, .. } => ty,
            BinaryImm { ty, .. } => ty,
            Call { ty, .. } => ty,
        }
    }
}

impl Function {
    /// Create a function with the given name and signature.
    pub fn with_name_signature(name: FunctionName, sig: Signature) -> Function {
        Function {
            name: name,
            signature: sig,
            stack_slots: Vec::new(),
            instructions: Vec::new(),
            extended_basic_blocks: Vec::new(),
            extended_values: Vec::new(),
            return_types: Vec::new(),
        }
    }

    /// Create a new empty, anomymous function.
    pub fn new() -> Function {
        Self::with_name_signature(FunctionName::new(), Signature::new())
    }

    /// Get the signature of this function.
    pub fn own_signature(&self) -> &Signature {
        &self.signature
    }

    /// Allocate a new stack slot.
    pub fn make_stack_slot(&mut self, data: StackSlotData) -> StackSlot {
        let ss = StackSlot::new(self.stack_slots.len());
        self.stack_slots.push(data);
        ss
    }

    /// Iterate over all stack slots in function.
    pub fn stack_slot_iter(&self) -> StackSlotIter {
        StackSlotIter {
            cur: 0,
            end: self.stack_slots.len(),
        }
    }

    /// Create a new instruction.
    pub fn make_inst(&mut self, data: InstructionData) -> Inst {
        let iref = Inst::new(self.instructions.len());
        self.instructions.push(data);
        // FIXME: Allocate extended value table entries if needed.
        iref
    }

    /// Create a new basic block.
    pub fn make_ebb(&mut self) -> Ebb {
        let ebb = Ebb::new(self.extended_basic_blocks.len());
        self.extended_basic_blocks.push(EbbData::new());
        ebb
    }

    /// Get the type of a value.
    pub fn value_type(&self, v: Value) -> Type {
        use self::ExpandedValue::*;
        use self::ValueData::*;
        match v.expand() {
            Direct(i) => self[i].first_type(),
            Table(i) => {
                match self.extended_values[i] {
                    Unused => panic!("Can't get type of Unused value {}", v),
                    Def { ty, .. } => ty,
                    Argument { ty, .. } => ty,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types;
    use immediates::*;

    #[test]
    fn make_inst() {
        let mut func = Function::new();

        let idata = InstructionData::Nullary {
            opcode: Opcode::Iconst,
            ty: types::I32,
        };
        let inst = func.make_inst(idata);
        assert_eq!(inst.to_string(), "inst0");

        // Immutable reference resolution.
        let ins = &func[inst];
        assert_eq!(ins.opcode(), Opcode::Iconst);
        assert_eq!(ins.first_type(), types::I32);
    }

    #[test]
    fn stack_slot() {
        let mut func = Function::new();

        let ss0 = func.make_stack_slot(StackSlotData::new(4));
        let ss1 = func.make_stack_slot(StackSlotData::new(8));
        assert_eq!(ss0.to_string(), "ss0");
        assert_eq!(ss1.to_string(), "ss1");

        assert_eq!(func[ss0].size, 4);
        assert_eq!(func[ss1].size, 8);
    }

}

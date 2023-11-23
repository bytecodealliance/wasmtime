use crate::{isa::reg::Reg, masm::StackSlot};
use std::collections::VecDeque;
use wasmparser::{Ieee32, Ieee64};
use wasmtime_environ::WasmType;

/// A typed register value used to track register values in the value
/// stack.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct TypedReg {
    /// The physical register.
    pub reg: Reg,
    /// The type associated to the physical register.
    pub ty: WasmType,
}

impl TypedReg {
    /// Create a new [`TypedReg`].
    pub fn new(ty: WasmType, reg: Reg) -> Self {
        Self { ty, reg }
    }

    /// Create an i64 [`TypedReg`].
    pub fn i64(reg: Reg) -> Self {
        Self {
            ty: WasmType::I64,
            reg,
        }
    }

    /// Create an i32 [`TypedReg`].
    pub fn i32(reg: Reg) -> Self {
        Self {
            ty: WasmType::I32,
            reg,
        }
    }
}

impl From<TypedReg> for Reg {
    fn from(tr: TypedReg) -> Self {
        tr.reg
    }
}

/// A local value.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Local {
    /// The index of the local.
    pub index: u32,
    /// The type of the local.
    pub ty: WasmType,
}

/// A memory value.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Memory {
    /// The type associated with the memory offset.
    pub ty: WasmType,
    /// The stack slot corresponding to the memory value.
    pub slot: StackSlot,
}

/// Value definition to be used within the shadow stack.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub(crate) enum Val {
    /// I32 Constant.
    I32(i32),
    /// I64 Constant.
    I64(i64),
    /// F32 Constant.
    F32(Ieee32),
    /// F64 Constant.
    F64(Ieee64),
    /// A register value.
    Reg(TypedReg),
    /// A local slot.
    Local(Local),
    /// Offset to a memory location.
    Memory(Memory),
}

impl From<TypedReg> for Val {
    fn from(tr: TypedReg) -> Self {
        Val::Reg(tr)
    }
}

impl From<Local> for Val {
    fn from(local: Local) -> Self {
        Val::Local(local)
    }
}

impl From<Memory> for Val {
    fn from(mem: Memory) -> Self {
        Val::Memory(mem)
    }
}

impl TryFrom<u32> for Val {
    type Error = anyhow::Error;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        i32::try_from(value).map(Val::i32).map_err(Into::into)
    }
}

impl Val {
    /// Create a new I32 constant value.
    pub fn i32(v: i32) -> Self {
        Self::I32(v)
    }

    /// Create a new I64 constant value.
    pub fn i64(v: i64) -> Self {
        Self::I64(v)
    }

    /// Create a new F32 constant value.
    pub fn f32(v: Ieee32) -> Self {
        Self::F32(v)
    }

    pub fn f64(v: Ieee64) -> Self {
        Self::F64(v)
    }

    /// Create a new Reg value.
    pub fn reg(reg: Reg, ty: WasmType) -> Self {
        Self::Reg(TypedReg { reg, ty })
    }

    /// Create a new Local value.
    pub fn local(index: u32, ty: WasmType) -> Self {
        Self::Local(Local { index, ty })
    }

    /// Create a Memory value.
    pub fn mem(ty: WasmType, slot: StackSlot) -> Self {
        Self::Memory(Memory { ty, slot })
    }

    /// Check whether the value is a register.
    pub fn is_reg(&self) -> bool {
        match *self {
            Self::Reg(_) => true,
            _ => false,
        }
    }

    /// Check wheter the value is a memory offset.
    pub fn is_mem(&self) -> bool {
        match *self {
            Self::Memory(_) => true,
            _ => false,
        }
    }

    /// Check whether the value is a constant.
    pub fn is_const(&self) -> bool {
        match *self {
            Val::I32(_) | Val::I64(_) | Val::F32(_) | Val::F64(_) => true,
            _ => false,
        }
    }

    /// Check whether the value is local with a particular index.
    pub fn is_local_at_index(&self, index: u32) -> bool {
        match *self {
            Self::Local(Local { index: i, .. }) if i == index => true,
            _ => false,
        }
    }

    /// Get the register representation of the value.
    ///
    /// # Panics
    /// This method will panic if the value is not a register.
    pub fn unwrap_reg(&self) -> TypedReg {
        match self {
            Self::Reg(tr) => *tr,
            v => panic!("expected value {:?} to be a register", v),
        }
    }

    /// Get the integer representation of the value.
    ///
    /// # Panics
    /// This method will panic if the value is not an i32.
    pub fn unwrap_i32(&self) -> i32 {
        match self {
            Self::I32(v) => *v,
            v => panic!("expected value {:?} to be i32", v),
        }
    }

    /// Get the integer representation of the value.
    ///
    /// # Panics
    /// This method will panic if the value is not an i64.
    pub fn unwrap_i64(&self) -> i64 {
        match self {
            Self::I64(v) => *v,
            v => panic!("expected value {:?} to be i64", v),
        }
    }

    /// Returns the underlying memory value if it is one, panics otherwise.
    pub fn unwrap_mem(&self) -> Memory {
        match self {
            Self::Memory(m) => *m,
            v => panic!("expected value {:?} to be a Memory", v),
        }
    }

    /// Check whether the value is an i32 constant.
    pub fn is_i32_const(&self) -> bool {
        match *self {
            Self::I32(_) => true,
            _ => false,
        }
    }

    /// Check whether the value is an i64 constant.
    pub fn is_i64_const(&self) -> bool {
        match *self {
            Self::I64(_) => true,
            _ => false,
        }
    }

    /// Get the type of the value.
    pub fn ty(&self) -> WasmType {
        match self {
            Val::I32(_) => WasmType::I32,
            Val::I64(_) => WasmType::I64,
            Val::F32(_) => WasmType::F32,
            Val::F64(_) => WasmType::F64,
            Val::Reg(r) => r.ty,
            Val::Memory(m) => m.ty,
            Val::Local(l) => l.ty,
        }
    }
}

/// The shadow stack used for compilation.
#[derive(Default, Debug)]
pub(crate) struct Stack {
    inner: VecDeque<Val>,
}

impl Stack {
    /// Allocate a new stack.
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    /// Returns true if the stack contains a local with the provided index
    /// except if the only time the local appears is the top element.
    pub fn contains_latent_local(&self, index: u32) -> bool {
        self.inner
            .iter()
            // Iterate top-to-bottom so we can skip the top element and stop
            // when we see a memory element.
            .rev()
            // The local is not latent if it's the top element because the top
            // element will be popped next which materializes the local.
            .skip(1)
            // Stop when we see a memory element because that marks where we
            // spilled up to so there will not be any locals past this point.
            .take_while(|v| !v.is_mem())
            .any(|v| v.is_local_at_index(index))
    }

    /// Extend the stack with the given elements.
    pub fn extend(&mut self, values: impl IntoIterator<Item = Val>) {
        self.inner.extend(values);
    }

    /// Inserts many values at the given index.
    pub fn insert_many(&mut self, at: usize, values: impl IntoIterator<Item = Val>) {
        debug_assert!(at <= self.len());
        // If last, simply extend.
        if at == self.inner.len() {
            self.inner.extend(values);
        } else {
            let mut tail = self.inner.split_off(at);
            self.inner.extend(values);
            self.inner.append(&mut tail);
        }
    }

    /// Get the length of the stack.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Push a value to the stack.
    pub fn push(&mut self, val: Val) {
        self.inner.push_back(val);
    }

    /// Peek into the top in the stack.
    pub fn peek(&self) -> Option<&Val> {
        self.inner.back()
    }

    /// Returns an iterator referencing the last n items of the stack,
    /// in bottom-most to top-most order.
    pub fn peekn(&self, n: usize) -> impl Iterator<Item = &Val> + '_ {
        let len = self.len();
        assert!(n <= len);

        let partition = len - n;
        self.inner.range(partition..)
    }

    /// Duplicates the top `n` elements of the stack.
    // Will be needed for control flow, it's just not integrated yet.
    #[allow(dead_code)]
    pub fn dup(&mut self, n: usize) {
        let len = self.len();
        assert!(n <= len);
        let partition = len - n;

        if n > 0 {
            for e in partition..len {
                if let Some(v) = self.inner.get(e) {
                    self.push(*v)
                }
            }
        }
    }

    /// Pops the top element of the stack, if any.
    pub fn pop(&mut self) -> Option<Val> {
        self.inner.pop_back()
    }

    /// Pops the element at the top of the stack if it is an i32 const;
    /// returns `None` otherwise.
    pub fn pop_i32_const(&mut self) -> Option<i32> {
        match self.peek() {
            Some(v) => v.is_i32_const().then(|| self.pop().unwrap().unwrap_i32()),
            _ => None,
        }
    }

    /// Pops the element at the top of the stack if it is an i64 const;
    /// returns `None` otherwise.
    pub fn pop_i64_const(&mut self) -> Option<i64> {
        match self.peek() {
            Some(v) => v.is_i64_const().then(|| self.pop().unwrap().unwrap_i64()),
            _ => None,
        }
    }

    /// Pops the element at the top of the stack if it is a register;
    /// returns `None` otherwise.
    pub fn pop_reg(&mut self) -> Option<TypedReg> {
        match self.peek() {
            Some(v) => v.is_reg().then(|| self.pop().unwrap().unwrap_reg()),
            _ => None,
        }
    }

    /// Pops the given register if it is at the top of the stack;
    /// returns `None` otherwise.
    pub fn pop_named_reg(&mut self, reg: Reg) -> Option<TypedReg> {
        match self.peek() {
            Some(v) => {
                (v.is_reg() && v.unwrap_reg().reg == reg).then(|| self.pop().unwrap().unwrap_reg())
            }
            _ => None,
        }
    }

    /// Get a mutable reference to the inner stack representation.
    pub fn inner_mut(&mut self) -> &mut VecDeque<Val> {
        &mut self.inner
    }

    /// Calculates the size of, in bytes, of the top n [Memory] entries
    /// in the value stack.
    pub fn sizeof(&self, top: usize) -> u32 {
        self.peekn(top).fold(0, |acc, v| {
            if v.is_mem() {
                acc + v.unwrap_mem().slot.size
            } else {
                acc
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Stack, Val};
    use crate::isa::reg::Reg;
    use wasmtime_environ::WasmType;

    #[test]
    fn test_pop_i32_const() {
        let mut stack = Stack::new();
        stack.push(Val::i32(33i32));
        assert_eq!(33, stack.pop_i32_const().unwrap());

        stack.push(Val::local(10, WasmType::I32));
        assert!(stack.pop_i32_const().is_none());
    }

    #[test]
    fn test_pop_reg() {
        let mut stack = Stack::new();
        let reg = Reg::int(2usize);
        stack.push(Val::reg(reg, WasmType::I32));
        stack.push(Val::i32(4));

        assert_eq!(None, stack.pop_reg());
        let _ = stack.pop().unwrap();
        assert_eq!(reg, stack.pop_reg().unwrap().reg);
    }

    #[test]
    fn test_pop_named_reg() {
        let mut stack = Stack::new();
        let reg = Reg::int(2usize);
        stack.push(Val::reg(reg, WasmType::I32));
        stack.push(Val::reg(Reg::int(4), WasmType::I32));

        assert_eq!(None, stack.pop_named_reg(reg));
        let _ = stack.pop().unwrap();
        assert_eq!(reg, stack.pop_named_reg(reg).unwrap().reg);
    }
}

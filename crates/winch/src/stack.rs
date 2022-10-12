use crate::isa::reg::Reg;

/// Value definition to be used within the shadow stack
#[derive(Debug)]
pub(crate) enum Val {
    /// I32 Constant
    I32(i32),
    /// A register
    Reg(Reg),
}

impl Val {
    /// Create a new I32 constant value
    pub fn i32(v: i32) -> Self {
        Self::I32(v)
    }

    /// Create a new Reg value
    pub fn reg(r: Reg) -> Self {
        Self::Reg(r)
    }

    /// Check whether the value is a register
    pub fn is_reg(&self) -> bool {
        match *self {
            Self::Reg(_) => true,
            _ => false,
        }
    }

    pub fn get_reg(&self) -> Reg {
        match self {
            Self::Reg(r) => *r,
            v => panic!("expected value {:?} to be a register", v),
        }
    }

    pub fn get_i32(&self) -> i32 {
        match self {
            Self::I32(v) => *v,
            v => panic!("expected value {:?} to be i32", v),
        }
    }

    /// Check whether the value is a constant
    pub fn is_i32_const(&self) -> bool {
        match *self {
            Self::I32(_) => true,
            _ => false,
        }
    }
}

/// The shadow stack used for compilation
#[derive(Default)]
pub(crate) struct Stack {
    inner: std::collections::VecDeque<Val>,
}

impl Stack {
    /// Allocate a new stack
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    /// Push a value to the stack
    pub fn push(&mut self, val: Val) {
        self.inner.push_back(val);
    }

    /// Peek into the last item in the stack
    pub fn peek(&mut self) -> Option<&Val> {
        self.inner.back()
    }

    /// Pops the element at the top of the stack, if any
    pub fn pop(&mut self) -> Option<Val> {
        self.inner.pop_back()
    }

    /// Pops the element at the top of the stack if it is a const
    pub fn pop_i32_const(&mut self) -> Option<i32> {
        match self.peek() {
            Some(v) => v.is_i32_const().then(|| self.pop().unwrap().get_i32()),
            _ => None,
        }
    }

    /// Pops the element at the top of the stack if it is a register
    pub fn pop_reg(&mut self) -> Option<Reg> {
        match self.peek() {
            Some(v) => v.is_reg().then(|| self.pop().unwrap().get_reg()),
            _ => None,
        }
    }

    /// Pops the given register if it is at the top of the stack
    pub fn pop_named_reg(&mut self, reg: Reg) -> Option<Reg> {
        match self.peek() {
            Some(v) => (v.is_reg() && v.get_reg() == reg).then(|| self.pop().unwrap().get_reg()),
            _ => None,
        }
    }
}

//! WebAssembly function translation state.
//!
//! The `TranslationState` struct defined in this module is used to keep track of the WebAssembly
//! value and control stacks during the translation of a single function.

use cretonne::ir::{self, Ebb, Inst, Type, Value};

/// A control stack frame can be an `if`, a `block` or a `loop`, each one having the following
/// fields:
///
/// - `destination`: reference to the `Ebb` that will hold the code after the control block;
/// - `return_values`: types of the values returned by the control block;
/// - `original_stack_size`: size of the value stack at the beginning of the control block.
///
/// Moreover, the `if` frame has the `branch_inst` field that points to the `brz` instruction
/// separating the `true` and `false` branch. The `loop` frame has a `header` field that references
/// the `Ebb` that contains the beginning of the body of the loop.
#[derive(Debug)]
pub enum ControlStackFrame {
    If {
        destination: Ebb,
        branch_inst: Inst,
        return_values: Vec<Type>,
        original_stack_size: usize,
        reachable: bool,
    },
    Block {
        destination: Ebb,
        return_values: Vec<Type>,
        original_stack_size: usize,
        reachable: bool,
    },
    Loop {
        destination: Ebb,
        header: Ebb,
        return_values: Vec<Type>,
        original_stack_size: usize,
        reachable: bool,
    },
}

/// Helper methods for the control stack objects.
impl ControlStackFrame {
    pub fn return_values(&self) -> &[Type] {
        match *self {
            ControlStackFrame::If { ref return_values, .. } |
            ControlStackFrame::Block { ref return_values, .. } |
            ControlStackFrame::Loop { ref return_values, .. } => &return_values,
        }
    }
    pub fn following_code(&self) -> Ebb {
        match *self {
            ControlStackFrame::If { destination, .. } |
            ControlStackFrame::Block { destination, .. } |
            ControlStackFrame::Loop { destination, .. } => destination,
        }
    }
    pub fn br_destination(&self) -> Ebb {
        match *self {
            ControlStackFrame::If { destination, .. } |
            ControlStackFrame::Block { destination, .. } => destination,
            ControlStackFrame::Loop { header, .. } => header,
        }
    }
    pub fn original_stack_size(&self) -> usize {
        match *self {
            ControlStackFrame::If { original_stack_size, .. } |
            ControlStackFrame::Block { original_stack_size, .. } |
            ControlStackFrame::Loop { original_stack_size, .. } => original_stack_size,
        }
    }
    pub fn is_loop(&self) -> bool {
        match *self {
            ControlStackFrame::If { .. } |
            ControlStackFrame::Block { .. } => false,
            ControlStackFrame::Loop { .. } => true,
        }
    }

    pub fn is_reachable(&self) -> bool {
        match *self {
            ControlStackFrame::If { reachable, .. } |
            ControlStackFrame::Block { reachable, .. } |
            ControlStackFrame::Loop { reachable, .. } => reachable,
        }
    }

    pub fn set_reachable(&mut self) {
        match *self {
            ControlStackFrame::If { ref mut reachable, .. } |
            ControlStackFrame::Block { ref mut reachable, .. } |
            ControlStackFrame::Loop { ref mut reachable, .. } => *reachable = true,
        }
    }
}

/// Contains information passed along during the translation and that records:
///
/// - The current value and control stacks.
/// - The depth of the two unreachable control blocks stacks, that are manipulated when translating
///   unreachable code;
pub struct TranslationState {
    pub stack: Vec<Value>,
    pub control_stack: Vec<ControlStackFrame>,
    pub phantom_unreachable_stack_depth: usize,
    pub real_unreachable_stack_depth: usize,
}

impl TranslationState {
    pub fn new() -> TranslationState {
        TranslationState {
            stack: Vec::new(),
            control_stack: Vec::new(),
            phantom_unreachable_stack_depth: 0,
            real_unreachable_stack_depth: 0,
        }
    }

    fn clear(&mut self) {
        self.stack.clear();
        self.control_stack.clear();
        self.phantom_unreachable_stack_depth = 0;
        self.real_unreachable_stack_depth = 0;
    }

    /// Initialize the state for compiling a function with the given signature.
    ///
    /// This resets the state to containing only a single block representing the whole function.
    /// The exit block is the last block in the function which will contain the return instruction.
    pub fn initialize(&mut self, sig: &ir::Signature, exit_block: Ebb) {
        self.clear();
        self.push_block(
            exit_block,
            sig.return_types
                .iter()
                .filter(|arg| arg.purpose == ir::ArgumentPurpose::Normal)
                .map(|argty| argty.value_type)
                .collect(),
        );
    }

    /// Push a value.
    pub fn push1(&mut self, val: Value) {
        self.stack.push(val);
    }

    /// Pop one value.
    pub fn pop1(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    /// Peek at the top of the stack without popping it.
    pub fn peek1(&self) -> Value {
        *self.stack.last().unwrap()
    }

    /// Pop two values. Return them in the order they were pushed.
    pub fn pop2(&mut self) -> (Value, Value) {
        let v2 = self.stack.pop().unwrap();
        let v1 = self.stack.pop().unwrap();
        (v1, v2)
    }

    /// Pop three values. Return them in the order they were pushed.
    pub fn pop3(&mut self) -> (Value, Value, Value) {
        let v3 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        let v1 = self.stack.pop().unwrap();
        (v1, v2, v3)
    }

    // Push a block on the control stack.
    pub fn push_block(&mut self, following_code: Ebb, result_types: Vec<Type>) {
        self.control_stack.push(ControlStackFrame::Block {
            destination: following_code,
            original_stack_size: self.stack.len(),
            return_values: result_types,
            reachable: false,
        });
    }

    // Push a loop on the control stack.
    pub fn push_loop(&mut self, header: Ebb, following_code: Ebb, result_types: Vec<Type>) {
        self.control_stack.push(ControlStackFrame::Loop {
            header,
            destination: following_code,
            original_stack_size: self.stack.len(),
            return_values: result_types,
            reachable: false,
        });
    }

    // Push an if on the control stack.
    pub fn push_if(&mut self, branch_inst: Inst, following_code: Ebb, result_types: Vec<Type>) {
        self.control_stack.push(ControlStackFrame::If {
            branch_inst,
            destination: following_code,
            original_stack_size: self.stack.len(),
            return_values: result_types,
            reachable: false,
        });
    }
}

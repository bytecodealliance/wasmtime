//! WebAssembly function translation state.
//!
//! The `TranslationState` struct defined in this module is used to keep track of the WebAssembly
//! value and control stacks during the translation of a single function.

use cranelift_codegen::ir::{self, Ebb, Inst, Value};
use cranelift_entity::EntityRef;
use environ::{FuncEnvironment, GlobalVariable};
use std::collections::HashMap;
use std::vec::Vec;
use translation_utils::{FuncIndex, GlobalIndex, MemoryIndex, SignatureIndex, TableIndex};

/// A control stack frame can be an `if`, a `block` or a `loop`, each one having the following
/// fields:
///
/// - `destination`: reference to the `Ebb` that will hold the code after the control block;
/// - `num_return_values`: number of values returned by the control block;
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
        num_return_values: usize,
        original_stack_size: usize,
        exit_is_branched_to: bool,
        reachable_from_top: bool,
    },
    Block {
        destination: Ebb,
        num_return_values: usize,
        original_stack_size: usize,
        exit_is_branched_to: bool,
    },
    Loop {
        destination: Ebb,
        header: Ebb,
        num_return_values: usize,
        original_stack_size: usize,
    },
}

/// Helper methods for the control stack objects.
impl ControlStackFrame {
    pub fn num_return_values(&self) -> usize {
        match *self {
            ControlStackFrame::If {
                num_return_values, ..
            }
            | ControlStackFrame::Block {
                num_return_values, ..
            }
            | ControlStackFrame::Loop {
                num_return_values, ..
            } => num_return_values,
        }
    }
    pub fn following_code(&self) -> Ebb {
        match *self {
            ControlStackFrame::If { destination, .. }
            | ControlStackFrame::Block { destination, .. }
            | ControlStackFrame::Loop { destination, .. } => destination,
        }
    }
    pub fn br_destination(&self) -> Ebb {
        match *self {
            ControlStackFrame::If { destination, .. }
            | ControlStackFrame::Block { destination, .. } => destination,
            ControlStackFrame::Loop { header, .. } => header,
        }
    }
    pub fn original_stack_size(&self) -> usize {
        match *self {
            ControlStackFrame::If {
                original_stack_size,
                ..
            }
            | ControlStackFrame::Block {
                original_stack_size,
                ..
            }
            | ControlStackFrame::Loop {
                original_stack_size,
                ..
            } => original_stack_size,
        }
    }
    pub fn is_loop(&self) -> bool {
        match *self {
            ControlStackFrame::If { .. } | ControlStackFrame::Block { .. } => false,
            ControlStackFrame::Loop { .. } => true,
        }
    }

    pub fn exit_is_branched_to(&self) -> bool {
        match *self {
            ControlStackFrame::If {
                exit_is_branched_to,
                ..
            }
            | ControlStackFrame::Block {
                exit_is_branched_to,
                ..
            } => exit_is_branched_to,
            ControlStackFrame::Loop { .. } => false,
        }
    }

    pub fn set_branched_to_exit(&mut self) {
        match *self {
            ControlStackFrame::If {
                ref mut exit_is_branched_to,
                ..
            }
            | ControlStackFrame::Block {
                ref mut exit_is_branched_to,
                ..
            } => *exit_is_branched_to = true,
            ControlStackFrame::Loop { .. } => {}
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
    pub reachable: bool,

    // Map of global variables that have already been created by `FuncEnvironment::make_global`.
    globals: HashMap<GlobalIndex, GlobalVariable>,

    // Map of heaps that have been created by `FuncEnvironment::make_heap`.
    heaps: HashMap<MemoryIndex, ir::Heap>,

    // Map of tables that have been created by `FuncEnvironment::make_table`.
    tables: HashMap<TableIndex, ir::Table>,

    // Map of indirect call signatures that have been created by
    // `FuncEnvironment::make_indirect_sig()`.
    // Stores both the signature reference and the number of WebAssembly arguments
    signatures: HashMap<SignatureIndex, (ir::SigRef, usize)>,

    // Imported and local functions that have been created by
    // `FuncEnvironment::make_direct_func()`.
    // Stores both the function reference and the number of WebAssembly arguments
    functions: HashMap<FuncIndex, (ir::FuncRef, usize)>,
}

impl TranslationState {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            control_stack: Vec::new(),
            reachable: true,
            globals: HashMap::new(),
            heaps: HashMap::new(),
            tables: HashMap::new(),
            signatures: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        debug_assert!(self.stack.is_empty());
        debug_assert!(self.control_stack.is_empty());
        self.reachable = true;
        self.globals.clear();
        self.heaps.clear();
        self.signatures.clear();
        self.functions.clear();
    }

    /// Initialize the state for compiling a function with the given signature.
    ///
    /// This resets the state to containing only a single block representing the whole function.
    /// The exit block is the last block in the function which will contain the return instruction.
    pub fn initialize(&mut self, sig: &ir::Signature, exit_block: Ebb) {
        self.clear();
        self.push_block(
            exit_block,
            sig.returns
                .iter()
                .filter(|arg| arg.purpose == ir::ArgumentPurpose::Normal)
                .count(),
        );
    }

    /// Push a value.
    pub fn push1(&mut self, val: Value) {
        self.stack.push(val);
    }

    /// Push multiple values.
    pub fn pushn(&mut self, vals: &[Value]) {
        self.stack.extend_from_slice(vals);
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

    /// Pop the top `n` values on the stack.
    ///
    /// The popped values are not returned. Use `peekn` to look at them before popping.
    pub fn popn(&mut self, n: usize) {
        let new_len = self.stack.len() - n;
        self.stack.truncate(new_len);
    }

    /// Peek at the top `n` values on the stack in the order they were pushed.
    pub fn peekn(&self, n: usize) -> &[Value] {
        &self.stack[self.stack.len() - n..]
    }

    // Push a block on the control stack.
    pub fn push_block(&mut self, following_code: Ebb, num_result_types: usize) {
        self.control_stack.push(ControlStackFrame::Block {
            destination: following_code,
            original_stack_size: self.stack.len(),
            num_return_values: num_result_types,
            exit_is_branched_to: false,
        });
    }

    // Push a loop on the control stack.
    pub fn push_loop(&mut self, header: Ebb, following_code: Ebb, num_result_types: usize) {
        self.control_stack.push(ControlStackFrame::Loop {
            header,
            destination: following_code,
            original_stack_size: self.stack.len(),
            num_return_values: num_result_types,
        });
    }

    // Push an if on the control stack.
    pub fn push_if(&mut self, branch_inst: Inst, following_code: Ebb, num_result_types: usize) {
        self.control_stack.push(ControlStackFrame::If {
            branch_inst,
            destination: following_code,
            original_stack_size: self.stack.len(),
            num_return_values: num_result_types,
            exit_is_branched_to: false,
            reachable_from_top: self.reachable,
        });
    }
}

/// Methods for handling entity references.
impl TranslationState {
    /// Get the `GlobalVariable` reference that should be used to access the global variable
    /// `index`. Create the reference if necessary.
    /// Also return the WebAssembly type of the global.
    pub fn get_global<FE: FuncEnvironment + ?Sized>(
        &mut self,
        func: &mut ir::Function,
        index: u32,
        environ: &mut FE,
    ) -> GlobalVariable {
        let index = index as GlobalIndex;
        *self
            .globals
            .entry(index)
            .or_insert_with(|| environ.make_global(func, index))
    }

    /// Get the `Heap` reference that should be used to access linear memory `index`.
    /// Create the reference if necessary.
    pub fn get_heap<FE: FuncEnvironment + ?Sized>(
        &mut self,
        func: &mut ir::Function,
        index: u32,
        environ: &mut FE,
    ) -> ir::Heap {
        let index = index as MemoryIndex;
        *self
            .heaps
            .entry(index)
            .or_insert_with(|| environ.make_heap(func, index))
    }

    /// Get the `Table` reference that should be used to access table `index`.
    /// Create the reference if necessary.
    pub fn get_table<FE: FuncEnvironment + ?Sized>(
        &mut self,
        func: &mut ir::Function,
        index: u32,
        environ: &mut FE,
    ) -> ir::Table {
        let index = index as TableIndex;
        *self
            .tables
            .entry(index)
            .or_insert_with(|| environ.make_table(func, index))
    }

    /// Get the `SigRef` reference that should be used to make an indirect call with signature
    /// `index`. Also return the number of WebAssembly arguments in the signature.
    ///
    /// Create the signature if necessary.
    pub fn get_indirect_sig<FE: FuncEnvironment + ?Sized>(
        &mut self,
        func: &mut ir::Function,
        index: u32,
        environ: &mut FE,
    ) -> (ir::SigRef, usize) {
        let index = index as SignatureIndex;
        *self.signatures.entry(index).or_insert_with(|| {
            let sig = environ.make_indirect_sig(func, index);
            (sig, normal_args(&func.dfg.signatures[sig]))
        })
    }

    /// Get the `FuncRef` reference that should be used to make a direct call to function
    /// `index`. Also return the number of WebAssembly arguments in the signature.
    ///
    /// Create the function reference if necessary.
    pub fn get_direct_func<FE: FuncEnvironment + ?Sized>(
        &mut self,
        func: &mut ir::Function,
        index: u32,
        environ: &mut FE,
    ) -> (ir::FuncRef, usize) {
        let index = FuncIndex::new(index as usize);
        *self.functions.entry(index).or_insert_with(|| {
            let fref = environ.make_direct_func(func, index);
            let sig = func.dfg.ext_funcs[fref].signature;
            (fref, normal_args(&func.dfg.signatures[sig]))
        })
    }
}

/// Count the number of normal parameters in a signature.
/// Exclude special-purpose parameters that represent runtime stuff and not WebAssembly arguments.
fn normal_args(sig: &ir::Signature) -> usize {
    sig.params
        .iter()
        .filter(|arg| arg.purpose == ir::ArgumentPurpose::Normal)
        .count()
}

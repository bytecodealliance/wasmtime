//! State of the Wasm stack for translation into CLIF.
//!
//! The `FuncTranslationStacks` struct defined in this module is used to keep
//! track of the WebAssembly value and control stacks during the translation of
//! a single function.

use cranelift_codegen::ir::{self, Block, ExceptionTag, Inst, Value};
use cranelift_frontend::FunctionBuilder;
use std::vec::Vec;
use wasmtime_environ::FrameStackShape;

/// Information about the presence of an associated `else` for an `if`, or the
/// lack thereof.
#[derive(Debug)]
pub enum ElseData {
    /// The `if` does not already have an `else` block.
    ///
    /// This doesn't mean that it will never have an `else`, just that we
    /// haven't seen it yet.
    NoElse {
        /// If we discover that we need an `else` block, this is the jump
        /// instruction that needs to be fixed up to point to the new `else`
        /// block rather than the destination block after the `if...end`.
        branch_inst: Inst,

        /// The placeholder block we're replacing.
        placeholder: Block,
    },

    /// We have already allocated an `else` block.
    ///
    /// Usually we don't know whether we will hit an `if .. end` or an `if
    /// .. else .. end`, but sometimes we can tell based on the block's type
    /// signature that the signature is not valid if there isn't an `else`. In
    /// these cases, we pre-allocate the `else` block.
    WithElse {
        /// This is the `else` block.
        else_block: Block,
    },
}

/// A control stack frame can be an `if`, a `block` or a `loop`, each one having the following
/// fields:
///
/// - `destination`: reference to the `Block` that will hold the code after the control block;
/// - `num_return_values`: number of values returned by the control block;
/// - `original_stack_size`: size of the value stack at the beginning of the control block.
///
/// The `loop` frame has a `header` field that references the `Block` that contains the beginning
/// of the body of the loop.
#[derive(Debug)]
pub enum ControlStackFrame {
    If {
        destination: Block,
        else_data: ElseData,
        num_param_values: usize,
        num_return_values: usize,
        original_stack_size: usize,
        exit_is_branched_to: bool,
        blocktype: wasmparser::BlockType,
        /// Was the head of the `if` reachable?
        head_is_reachable: bool,
        /// What was the reachability at the end of the consequent?
        ///
        /// This is `None` until we're finished translating the consequent, and
        /// is set to `Some` either by hitting an `else` when we will begin
        /// translating the alternative, or by hitting an `end` in which case
        /// there is no alternative.
        consequent_ends_reachable: Option<bool>,
        // Note: no need for `alternative_ends_reachable` because that is just
        // `state.reachable` when we hit the `end` in the `if .. else .. end`.
    },
    Block {
        destination: Block,
        num_param_values: usize,
        num_return_values: usize,
        original_stack_size: usize,
        exit_is_branched_to: bool,
        /// If this block is a try-table block, the handler state
        /// checkpoint to rewind to when we leave the block, and the
        /// list of catch blocks to seal when done.
        try_table_info: Option<(HandlerStateCheckpoint, Vec<Block>)>,
    },
    Loop {
        destination: Block,
        header: Block,
        num_param_values: usize,
        num_return_values: usize,
        original_stack_size: usize,
    },
}

/// Helper methods for the control stack objects.
impl ControlStackFrame {
    pub fn num_return_values(&self) -> usize {
        match *self {
            Self::If {
                num_return_values, ..
            }
            | Self::Block {
                num_return_values, ..
            }
            | Self::Loop {
                num_return_values, ..
            } => num_return_values,
        }
    }

    pub fn num_param_values(&self) -> usize {
        match *self {
            Self::If {
                num_param_values, ..
            }
            | Self::Block {
                num_param_values, ..
            }
            | Self::Loop {
                num_param_values, ..
            } => num_param_values,
        }
    }

    pub fn following_code(&self) -> Block {
        match *self {
            Self::If { destination, .. }
            | Self::Block { destination, .. }
            | Self::Loop { destination, .. } => destination,
        }
    }

    pub fn br_destination(&self) -> Block {
        match *self {
            Self::If { destination, .. } | Self::Block { destination, .. } => destination,
            Self::Loop { header, .. } => header,
        }
    }

    /// Private helper. Use `truncate_value_stack_to_else_params()` or
    /// `truncate_value_stack_to_original_size()` to restore value-stack state.
    fn original_stack_size(&self) -> usize {
        match *self {
            Self::If {
                original_stack_size,
                ..
            }
            | Self::Block {
                original_stack_size,
                ..
            }
            | Self::Loop {
                original_stack_size,
                ..
            } => original_stack_size,
        }
    }

    pub fn is_loop(&self) -> bool {
        match *self {
            Self::If { .. } | Self::Block { .. } => false,
            Self::Loop { .. } => true,
        }
    }

    pub fn exit_is_branched_to(&self) -> bool {
        match *self {
            Self::If {
                exit_is_branched_to,
                ..
            }
            | Self::Block {
                exit_is_branched_to,
                ..
            } => exit_is_branched_to,
            Self::Loop { .. } => false,
        }
    }

    pub fn set_branched_to_exit(&mut self) {
        match *self {
            Self::If {
                ref mut exit_is_branched_to,
                ..
            }
            | Self::Block {
                ref mut exit_is_branched_to,
                ..
            } => *exit_is_branched_to = true,
            Self::Loop { .. } => {}
        }
    }

    /// Pop values from the value stack so that it is left at the
    /// input-parameters to an else-block.
    pub fn truncate_value_stack_to_else_params(
        &self,
        stack: &mut Vec<Value>,
        stack_shape: &mut Vec<FrameStackShape>,
    ) {
        debug_assert!(matches!(self, &ControlStackFrame::If { .. }));
        stack.truncate(self.original_stack_size());
        stack_shape.truncate(self.original_stack_size());
    }

    /// Pop values from the value stack so that it is left at the state it was
    /// before this control-flow frame.
    pub fn truncate_value_stack_to_original_size(
        &self,
        stack: &mut Vec<Value>,
        stack_shape: &mut Vec<FrameStackShape>,
    ) {
        // The "If" frame pushes its parameters twice, so they're available to the else block
        // (see also `FuncTranslationStacks::push_if`).
        // Yet, the original_stack_size member accounts for them only once, so that the else
        // block can see the same number of parameters as the consequent block. As a matter of
        // fact, we need to subtract an extra number of parameter values for if blocks.
        let num_duplicated_params = match self {
            &ControlStackFrame::If {
                num_param_values, ..
            } => {
                debug_assert!(num_param_values <= self.original_stack_size());
                num_param_values
            }
            _ => 0,
        };

        let new_len = self.original_stack_size() - num_duplicated_params;
        stack.truncate(new_len);
        stack_shape.truncate(new_len);
    }

    /// Restore the catch-handlers as they were outside of this block.
    pub fn restore_catch_handlers(
        &self,
        handlers: &mut HandlerState,
        builder: &mut FunctionBuilder,
    ) {
        match self {
            ControlStackFrame::Block {
                try_table_info: Some((ckpt, catch_blocks)),
                ..
            } => {
                handlers.restore_checkpoint(*ckpt);
                for block in catch_blocks {
                    builder.seal_block(*block);
                }
            }
            _ => {}
        }
    }
}

/// Keeps track of Wasm's operand and control stacks, as well as reachability
/// for each control frame.
pub struct FuncTranslationStacks {
    /// A stack of values corresponding to the active values in the input wasm function at this
    /// point.
    pub(crate) stack: Vec<Value>,
    /// "Shape" of stack at each index, if emitting debug instrumentation.
    ///
    /// When we pop `stack`, we automatically pop `stack_shape` as
    /// well, but we never push automatically; this enables us to
    /// determine which values are new and need to be flushed to
    /// memory after translating an operator.
    pub(crate) stack_shape: Vec<FrameStackShape>,
    /// A stack of active control flow operations at this point in the input wasm function.
    pub(crate) control_stack: Vec<ControlStackFrame>,
    /// Exception handler state, updated as we enter and exit
    /// `try_table` scopes and attached to each call that we make.
    pub(crate) handlers: HandlerState,
    /// Is the current translation state still reachable? This is false when translating operators
    /// like End, Return, or Unreachable.
    pub(crate) reachable: bool,
}

// Public methods that are exposed to non- API consumers.
impl FuncTranslationStacks {
    /// True if the current translation state expresses reachable code, false if it is unreachable.
    #[inline]
    pub fn reachable(&self) -> bool {
        self.reachable
    }
}

impl FuncTranslationStacks {
    /// Construct a new, empty, `FuncTranslationStacks`
    pub(crate) fn new() -> Self {
        Self {
            stack: Vec::new(),
            stack_shape: Vec::new(),
            control_stack: Vec::new(),
            handlers: HandlerState::default(),
            reachable: true,
        }
    }

    fn clear(&mut self) {
        debug_assert!(self.stack.is_empty());
        debug_assert!(self.stack_shape.is_empty());
        debug_assert!(self.control_stack.is_empty());
        debug_assert!(self.handlers.is_empty());
        self.reachable = true;
    }

    /// Initialize the state for compiling a function with the given signature.
    ///
    /// This resets the state to containing only a single block representing the whole function.
    /// The exit block is the last block in the function which will contain the return instruction.
    pub(crate) fn initialize(&mut self, sig: &ir::Signature, exit_block: Block) {
        self.clear();
        self.push_block(
            exit_block,
            0,
            sig.returns
                .iter()
                .filter(|arg| arg.purpose == ir::ArgumentPurpose::Normal)
                .count(),
        );
    }

    /// Push a value.
    pub(crate) fn push1(&mut self, val: Value) {
        self.stack.push(val);
    }

    /// Push two values.
    pub(crate) fn push2(&mut self, val1: Value, val2: Value) {
        self.stack.push(val1);
        self.stack.push(val2);
    }

    /// Push multiple values.
    pub(crate) fn pushn(&mut self, vals: &[Value]) {
        self.stack.extend_from_slice(vals);
    }

    /// Pop one value.
    pub(crate) fn pop1(&mut self) -> Value {
        self.pop_stack_shape(1);
        self.stack
            .pop()
            .expect("attempted to pop a value from an empty stack")
    }

    /// Peek at the top of the stack without popping it.
    pub(crate) fn peek1(&self) -> Value {
        *self
            .stack
            .last()
            .expect("attempted to peek at a value on an empty stack")
    }

    /// Pop two values. Return them in the order they were pushed.
    pub(crate) fn pop2(&mut self) -> (Value, Value) {
        self.pop_stack_shape(2);
        let v2 = self.stack.pop().unwrap();
        let v1 = self.stack.pop().unwrap();
        (v1, v2)
    }

    /// Pop three values. Return them in the order they were pushed.
    pub(crate) fn pop3(&mut self) -> (Value, Value, Value) {
        self.pop_stack_shape(3);
        let v3 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        let v1 = self.stack.pop().unwrap();
        (v1, v2, v3)
    }

    /// Pop four values. Return them in the order they were pushed.
    pub(crate) fn pop4(&mut self) -> (Value, Value, Value, Value) {
        self.pop_stack_shape(4);
        let v4 = self.stack.pop().unwrap();
        let v3 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        let v1 = self.stack.pop().unwrap();
        (v1, v2, v3, v4)
    }

    /// Pop five values. Return them in the order they were pushed.
    pub(crate) fn pop5(&mut self) -> (Value, Value, Value, Value, Value) {
        self.pop_stack_shape(5);
        let v5 = self.stack.pop().unwrap();
        let v4 = self.stack.pop().unwrap();
        let v3 = self.stack.pop().unwrap();
        let v2 = self.stack.pop().unwrap();
        let v1 = self.stack.pop().unwrap();
        (v1, v2, v3, v4, v5)
    }

    /// Helper to ensure the stack size is at least as big as `n`; note that due to
    /// `debug_assert` this will not execute in non-optimized builds.
    #[inline]
    fn ensure_length_is_at_least(&self, n: usize) {
        debug_assert!(
            n <= self.stack.len(),
            "attempted to access {} values but stack only has {} values",
            n,
            self.stack.len()
        )
    }

    /// Pop the top `n` values on the stack.
    ///
    /// The popped values are not returned. Use `peekn` to look at them before popping.
    pub(crate) fn popn(&mut self, n: usize) {
        self.ensure_length_is_at_least(n);
        let new_len = self.stack.len() - n;
        self.stack.truncate(new_len);
        self.stack_shape.truncate(new_len);
    }

    fn pop_stack_shape(&mut self, n: usize) {
        // The `stack_shape` vec represents the *clean* slots (already
        // flushed to memory); its length is always less than or equal
        // to `stack`, but indices always correspond between the
        // two. Thus a pop on `stack` may or may not pop something on
        // `stack_shape`; but if `stack` is truncated down to a length
        // L by some number of pops, truncating `stack_shape` to that
        // same length L will pop exactly the right shapes and will
        // ensure that any new pushes that are "dirty" will be
        // correctly represented as such.
        let new_len = self.stack.len() - n;
        self.stack_shape.truncate(new_len);
    }

    /// Peek at the top `n` values on the stack in the order they were pushed.
    pub(crate) fn peekn(&self, n: usize) -> &[Value] {
        self.ensure_length_is_at_least(n);
        &self.stack[self.stack.len() - n..]
    }

    /// Peek at the top `n` values on the stack in the order they were pushed.
    pub(crate) fn peekn_mut(&mut self, n: usize) -> &mut [Value] {
        self.ensure_length_is_at_least(n);
        let len = self.stack.len();
        &mut self.stack[len - n..]
    }

    fn push_block_impl(
        &mut self,
        following_code: Block,
        num_param_types: usize,
        num_result_types: usize,
        try_table_info: Option<(HandlerStateCheckpoint, Vec<Block>)>,
    ) {
        debug_assert!(num_param_types <= self.stack.len());
        self.control_stack.push(ControlStackFrame::Block {
            destination: following_code,
            original_stack_size: self.stack.len() - num_param_types,
            num_param_values: num_param_types,
            num_return_values: num_result_types,
            exit_is_branched_to: false,
            try_table_info,
        });
    }

    /// Push a block on the control stack.
    pub(crate) fn push_block(
        &mut self,
        following_code: Block,
        num_param_types: usize,
        num_result_types: usize,
    ) {
        self.push_block_impl(following_code, num_param_types, num_result_types, None);
    }

    /// Push a try-table block on the control stack.
    pub(crate) fn push_try_table_block(
        &mut self,
        following_code: Block,
        catch_blocks: Vec<Block>,
        num_param_types: usize,
        num_result_types: usize,
        checkpoint: HandlerStateCheckpoint,
    ) {
        self.push_block_impl(
            following_code,
            num_param_types,
            num_result_types,
            Some((checkpoint, catch_blocks)),
        );
    }

    /// Push a loop on the control stack.
    pub(crate) fn push_loop(
        &mut self,
        header: Block,
        following_code: Block,
        num_param_types: usize,
        num_result_types: usize,
    ) {
        debug_assert!(num_param_types <= self.stack.len());
        self.control_stack.push(ControlStackFrame::Loop {
            header,
            destination: following_code,
            original_stack_size: self.stack.len() - num_param_types,
            num_param_values: num_param_types,
            num_return_values: num_result_types,
        });
    }

    /// Push an if on the control stack.
    pub(crate) fn push_if(
        &mut self,
        destination: Block,
        else_data: ElseData,
        num_param_types: usize,
        num_result_types: usize,
        blocktype: wasmparser::BlockType,
    ) {
        debug_assert!(num_param_types <= self.stack.len());
        self.assert_debug_stack_is_synced();

        // Push a second copy of our `if`'s parameters on the stack. This lets
        // us avoid saving them on the side in the `ControlStackFrame` for our
        // `else` block (if it exists), which would require a second heap
        // allocation. See also the comment in `translate_operator` for
        // `Operator::Else`.
        self.stack.reserve(num_param_types);
        for i in (self.stack.len() - num_param_types)..self.stack.len() {
            let val = self.stack[i];
            self.stack.push(val);
            // Duplicate the stack-shape as well, if we're doing debug
            // instrumentation. Note that we must have flushed
            // everything before processing an `if`, so (as per the
            // assert above) we can rely on either no shapes (if no
            // instrumentation) or all shapes being present.
            if !self.stack_shape.is_empty() {
                let shape = self.stack_shape[i];
                self.stack_shape.push(shape);
            }
        }

        self.control_stack.push(ControlStackFrame::If {
            destination,
            else_data,
            original_stack_size: self.stack.len() - num_param_types,
            num_param_values: num_param_types,
            num_return_values: num_result_types,
            exit_is_branched_to: false,
            head_is_reachable: self.reachable,
            consequent_ends_reachable: None,
            blocktype,
        });
    }

    pub(crate) fn assert_debug_stack_is_synced(&self) {
        debug_assert!(self.stack_shape.is_empty() || self.stack_shape.len() == self.stack.len());
    }
}

/// Exception handler state.
///
/// We update this state as we enter and exit `try_table` scopes. When
/// we visit a call, we use this state to attach handler info to a
/// `try_call` CLIF instruction.
///
/// Note that although handlers are lexically-scoped, and we could
/// optimize away shadowing, this is fairly subtle, because handler
/// order also matters (two *distinct* tag indices in our module are
/// not necessarily distinct: tag imports can create aliasing). Rather
/// than attempt to keep an ordered map and also remove shadowing, we
/// follow the Wasm spec more closely: handlers are on "the stack" and
/// inner handlers win over outer handlers. Within a single
/// `try_table`, we push handlers *in reverse*, because the semantics
/// of handler matching in `try_table` are left-to-right; this allows
/// us to *flatten* the LIFO stack of `try_table`s with left-to-right
/// scans within a table into a single stack we scan backward from the
/// end.
pub struct HandlerState {
    /// List of pairs mapping from CLIF-level exception tag to
    /// CLIF-level block. We will have already filled in these blocks
    /// with the appropriate branch implementation when we start the
    /// `try_table` scope.
    pub(crate) handlers: Vec<(Option<ExceptionTag>, Block)>,
}

impl core::default::Default for HandlerState {
    fn default() -> Self {
        HandlerState { handlers: vec![] }
    }
}

/// A checkpoint in the handler state. Can be restored in LIFO order
/// only: the last-taken checkpoint can be restored first, then the
/// one before it, etc.
#[derive(Clone, Copy, Debug)]
pub struct HandlerStateCheckpoint(usize);

impl HandlerState {
    /// Set a given tag's handler to a given CLIF block.
    pub fn add_handler(&mut self, tag: Option<ExceptionTag>, block: Block) {
        self.handlers.push((tag, block));
    }

    /// Take a checkpoint.
    pub fn take_checkpoint(&self) -> HandlerStateCheckpoint {
        HandlerStateCheckpoint(self.handlers.len())
    }

    /// Restore to a checkpoint.
    pub fn restore_checkpoint(&mut self, ckpt: HandlerStateCheckpoint) {
        assert!(ckpt.0 <= self.handlers.len());
        self.handlers.truncate(ckpt.0);
    }

    /// Get an iterator over handlers. The exception-matching
    /// semantics are to take the *first* match in this sequence; that
    /// is, this returns the sequence of handlers latest-first (top of
    /// stack first).
    pub fn handlers(&self) -> impl Iterator<Item = (Option<ExceptionTag>, Block)> + '_ {
        self.handlers
            .iter()
            .map(|(tag, block)| (*tag, *block))
            .rev()
    }

    /// Are there no handlers registered?
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

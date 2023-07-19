//! Data structures for control flow emission.
//!
//! As of the current implementation, Winch doesn't offer support for the
//! multi-value proposal, which in the context of control flow constructs it
//! means that blocks don't take any params and produce 0 or 1 return. The
//! intention is to implement support for multi-value across the compiler, when
//! that time comes, here are some general changes that will be needed for
//! control flow:
//!
//! * Consider having a copy of the block params on the side, and push them when
//! encountering an else or duplicate the block params upfront. If no else is
//! present, clean the extra copies from the stack.
//!
//! * Eagerly load the block params. Params can flow "downward" as the block
//! results in the case of an empty then or else block:
//!   (module
//!     (func (export "params") (param i32) (result i32)
//!       (i32.const 2)
//!       (if (param i32) (result i32) (local.get 0)
//!       (then))
//!     (i32.const 3)
//!     (i32.add)
//!   )
//!
//! As a future optimization, we could perform a look ahead to the next
//! instruction when reaching any of the comparison instructions. If the next
//! instruction is a control instruction, we could avoid emitting
//! a [`crate::masm::MacroAssembler::cmp_with_set`] and instead emit
//! a conditional jump inline when emitting the control flow instruction.

use super::{CodeGenContext, MacroAssembler, OperandSize};
use crate::{
    abi::{ABIResult, ABI},
    masm::CmpKind,
    CallingConvention,
};
use cranelift_codegen::MachLabel;
use wasmtime_environ::WasmType;

/// Holds the necessary metdata to support the emission
/// of control flow instructions.
#[derive(Debug)]
pub(crate) enum ControlStackFrame {
    If {
        /// The if continuation label.
        cont: MachLabel,
        /// The exit label of the block.
        exit: MachLabel,
        /// The return values of the block.
        result: ABIResult,
        /// The size of the value stack at the beginning of the If.
        original_stack_len: usize,
        /// The stack pointer offset at the beginning of the If.
        original_sp_offset: u32,
        /// Local reachability state when entering the block.
        reachable: bool,
    },
    Else {
        /// The exit label of the block.
        exit: MachLabel,
        /// The return values of the block.
        result: ABIResult,
        /// The size of the value stack at the beginning of the Else.
        original_stack_len: usize,
        /// The stack pointer offset at the beginning of the Else.
        original_sp_offset: u32,
        /// Local reachability state when entering the block.
        reachable: bool,
    },
    Block {
        /// The block exit label.
        exit: MachLabel,
        /// The size of the value stack at the beginning of the block.
        original_stack_len: usize,
        /// The return values of the block.
        result: ABIResult,
        /// The stack pointer offset at the beginning of the Block.
        original_sp_offset: u32,
        /// Exit state of the block.
        ///
        /// This flag is used to dertermine if a block is a branch
        /// target. By default, this is false, and it's updated when
        /// emitting a `br` or `br_if`.
        is_branch_target: bool,
    },
    Loop {
        /// The start of the loop.
        head: MachLabel,
        /// The size of the value stack at the beginning of the block.
        original_stack_len: usize,
        /// The stack pointer offset at the beginning of the Block.
        original_sp_offset: u32,
        /// The return values of the block.
        result: ABIResult,
    },
}

impl ControlStackFrame {
    /// Returns [`ControlStackFrame`] for an if.
    pub fn if_<M: MacroAssembler>(
        returns: &[WasmType],
        masm: &mut M,
        context: &mut CodeGenContext,
    ) -> Self {
        let result = <M::ABI as ABI>::result(&returns, &CallingConvention::Default);
        let mut control = Self::If {
            cont: masm.get_label(),
            exit: masm.get_label(),
            result,
            reachable: context.reachable,
            original_stack_len: 0,
            original_sp_offset: 0,
        };

        control.emit(masm, context);
        control
    }

    /// Creates a block that represents the base
    /// block for the function body.
    pub fn function_body_block<M: MacroAssembler>(
        result: ABIResult,
        masm: &mut M,
        context: &mut CodeGenContext,
    ) -> Self {
        Self::Block {
            original_stack_len: context.stack.len(),
            result,
            is_branch_target: false,
            exit: masm.get_label(),
            original_sp_offset: masm.sp_offset(),
        }
    }

    /// Returns [`ControlStackFrame`] for a block.
    pub fn block<M: MacroAssembler>(
        returns: &[WasmType],
        masm: &mut M,
        context: &mut CodeGenContext,
    ) -> Self {
        let result = <M::ABI as ABI>::result(&returns, &CallingConvention::Default);
        let mut control = Self::Block {
            original_stack_len: 0,
            result,
            is_branch_target: false,
            exit: masm.get_label(),
            original_sp_offset: 0,
        };

        control.emit(masm, context);
        control
    }

    /// Returns [`ControlStackFrame`] for a loop.
    pub fn loop_<M: MacroAssembler>(
        returns: &[WasmType],
        masm: &mut M,
        context: &mut CodeGenContext,
    ) -> Self {
        let result = <M::ABI as ABI>::result(&returns, &CallingConvention::Default);
        let mut control = Self::Loop {
            original_stack_len: 0,
            result,
            head: masm.get_label(),
            original_sp_offset: 0,
        };

        control.emit(masm, context);
        control
    }

    fn emit<M: MacroAssembler>(&mut self, masm: &mut M, context: &mut CodeGenContext) {
        use ControlStackFrame::*;

        // Do not perform any emissions if we are in an unreachable state.
        if !context.reachable {
            return;
        }

        match self {
            If {
                cont,
                original_stack_len,
                original_sp_offset,
                ..
            } => {
                // Pop the condition value.
                let top = context.pop_to_reg(masm, None, OperandSize::S32);

                // Unconditionall spill before emitting control flow.
                context.spill(masm);

                *original_stack_len = context.stack.len();
                *original_sp_offset = masm.sp_offset();
                masm.branch(CmpKind::Eq, top.into(), top.into(), *cont, OperandSize::S32);
                context.free_gpr(top);
            }
            Block {
                original_stack_len,
                original_sp_offset,
                ..
            } => {
                // Unconditional spill before entering the block.
                // We assume that there are no live registers when
                // exiting the block.
                context.spill(masm);
                *original_stack_len = context.stack.len();
                *original_sp_offset = masm.sp_offset();
            }
            Loop {
                original_stack_len,
                original_sp_offset,
                head,
                ..
            } => {
                // Unconditional spill before entering the loop block.
                context.spill(masm);
                *original_stack_len = context.stack.len();
                *original_sp_offset = masm.sp_offset();
                masm.bind(*head);
            }
            _ => unreachable!(),
        }
    }

    /// Handles the else branch if the current control stack frame is
    /// [`ControlStackFrame::If`].
    pub fn emit_else<M: MacroAssembler>(&mut self, masm: &mut M, context: &mut CodeGenContext) {
        use ControlStackFrame::*;
        match self {
            If {
                result,
                original_stack_len,
                exit,
                ..
            } => {
                assert!((*original_stack_len + result.len()) == context.stack.len());
                // Before emitting an unconditional jump to the exit branch,
                // we handle the result of the if-then block.
                context.pop_abi_results(&result, masm);
                // Before binding the else branch, we emit the jump to the end
                // label.
                masm.jmp(*exit);
                // Bind the else branch.
                self.bind_else(masm, context.reachable);
            }
            _ => unreachable!(),
        }
    }

    /// Binds the else branch label and converts `self` to
    /// [`ControlStackFrame::Else`].
    pub fn bind_else<M: MacroAssembler>(&mut self, masm: &mut M, reachable: bool) {
        use ControlStackFrame::*;
        match self {
            If {
                cont,
                result,
                original_stack_len,
                original_sp_offset,
                exit,
                ..
            } => {
                // Bind the else branch.
                masm.bind(*cont);

                // Update the stack control frame with an else control frame.
                *self = ControlStackFrame::Else {
                    exit: *exit,
                    original_stack_len: *original_stack_len,
                    result: *result,
                    reachable,
                    original_sp_offset: *original_sp_offset,
                };
            }
            _ => unreachable!(),
        }
    }

    /// Handles the end of a control stack frame.
    pub fn emit_end<M: MacroAssembler>(&mut self, masm: &mut M, context: &mut CodeGenContext) {
        use ControlStackFrame::*;
        match self {
            If {
                result,
                original_stack_len,
                ..
            }
            | Else {
                result,
                original_stack_len,
                ..
            } => {
                assert!((*original_stack_len + result.len()) == context.stack.len());
                // Before binding the exit label, we handle the block results.
                context.pop_abi_results(&result, masm);
                self.bind_end(masm, context);
            }
            Block {
                original_stack_len,
                result,
                ..
            } => {
                assert!((*original_stack_len + result.len()) == context.stack.len());
                context.pop_abi_results(&result, masm);
                self.bind_end(masm, context);
            }
            Loop {
                result,
                original_stack_len,
                ..
            } => {
                assert!((*original_stack_len + result.len()) == context.stack.len());
            }
        }
    }

    /// Binds the exit label of the current control stack frame and pushes the
    /// ABI results to the value stack.
    pub fn bind_end<M: MacroAssembler>(&self, masm: &mut M, context: &mut CodeGenContext) {
        // Push the results to the value stack.
        context.push_abi_results(self.result(), masm);
        self.bind_exit_label(masm);
    }

    /// Binds the exit label of the control stack frame.
    pub fn bind_exit_label<M: MacroAssembler>(&self, masm: &mut M) {
        use ControlStackFrame::*;
        match self {
            // We use an explicit label to track the exit of an if block. In case there's no
            // else, we bind the if's continuation block to make sure that any jumps from the if
            // condition are reachable and we bind the explicit exit label as well to ensure that any
            // branching instructions are able to correctly reach the block's end.
            If { cont, .. } => masm.bind(*cont),
            _ => {}
        }
        if let Some(label) = self.exit_label() {
            masm.bind(*label);
        }
    }

    /// Returns the continuation label of the current control stack frame.
    pub fn label(&self) -> &MachLabel {
        use ControlStackFrame::*;

        match self {
            If { exit, .. } | Else { exit, .. } | Block { exit, .. } => exit,
            Loop { head, .. } => head,
        }
    }

    /// Returns the exit label of the current control stack frame. Note that
    /// this is similar to [`ControlStackFrame::label`], with the only difference that it
    /// returns `None` for `Loop` since its label doesn't represent an exit.
    pub fn exit_label(&self) -> Option<&MachLabel> {
        use ControlStackFrame::*;

        match self {
            If { exit, .. } | Else { exit, .. } | Block { exit, .. } => Some(exit),
            Loop { .. } => None,
        }
    }

    /// Set the current control stack frame as a branch target.
    pub fn set_as_target(&mut self) {
        match self {
            ControlStackFrame::Block {
                is_branch_target, ..
            } => {
                *is_branch_target = true;
            }
            _ => {}
        }
    }

    /// Returns [`crate::abi::ABIResult`] of the control stack frame
    /// block.
    pub fn result(&self) -> &ABIResult {
        use ControlStackFrame::*;

        match self {
            If { result, .. }
            | Else { result, .. }
            | Block { result, .. }
            | Loop { result, .. } => result,
        }
    }

    /// This function is used at the end of unreachable code handling
    /// to determine if the reachability status should be updated.
    pub fn is_next_sequence_reachable(&self) -> bool {
        use ControlStackFrame::*;

        match self {
            // For if/else, the reachability of the next sequence is determined
            // by the reachability state at the start of the block. An else
            // block will be reachable if the if block is also reachable at
            // entry.
            If { reachable, .. } | Else { reachable, .. } => *reachable,
            // For blocks, the reachability of the next sequence is determined
            // if they're a branch target.
            Block {
                is_branch_target, ..
            } => *is_branch_target,
            // Loops are not used for reachability analysis,
            // given that they don't have exit branches.
            Loop { .. } => false,
        }
    }

    // TODO document.
    pub fn original_stack_len_and_sp_offset(&self) -> (usize, u32) {
        use ControlStackFrame::*;
        match self {
            If {
                original_stack_len,
                original_sp_offset,
                ..
            }
            | Else {
                original_stack_len,
                original_sp_offset,
                ..
            }
            | Block {
                original_stack_len,
                original_sp_offset,
                ..
            }
            | Loop {
                original_stack_len,
                original_sp_offset,
                ..
            } => (*original_stack_len, *original_sp_offset),
        }
    }
}

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
    abi::ABIResultsData,
    codegen::env::BlockTypeInfo,
    masm::{IntCmpKind, SPOffset},
};
use cranelift_codegen::MachLabel;

/// Holds the all the metdata to support the emission
/// of control flow instructions.
#[derive(Debug)]
pub(crate) enum ControlStackFrame {
    If {
        /// The if continuation label.
        cont: MachLabel,
        /// The exit label of the block.
        exit: MachLabel,
        /// Data about the block's results.
        results_data: ABIResultsData,
        /// Information about the parameters and returns of the block.
        block_type_info: BlockTypeInfo,
        /// The length of the value stack at the beginning of the If.
        base_stack_len: usize,
        /// The stack pointer offset at the beginning of the If.
        base_sp: SPOffset,
        /// Local reachability state when entering the block.
        reachable: bool,
    },
    Else {
        /// The exit label of the block.
        exit: MachLabel,
        /// Data about the block's results.
        results_data: ABIResultsData,
        /// Information about the parameters and returns of the block.
        block_type_info: BlockTypeInfo,
        /// The length of the value stack at the beginning of the Else.
        base_stack_len: usize,
        /// The stack pointer offset at the beginning of the Else.
        base_sp: SPOffset,
        /// Local reachability state when entering the block.
        reachable: bool,
    },
    Block {
        /// The block exit label.
        exit: MachLabel,
        /// The length of the value stack at the beginning of the block.
        base_stack_len: usize,
        /// Data about the block's results.
        results_data: ABIResultsData,
        /// Information about the parameters and returns of the block.
        block_type_info: BlockTypeInfo,
        /// The stack pointer offset at the beginning of the Block.
        base_sp: SPOffset,
        /// Exit state of the block.
        ///
        /// This flag is used to dertermine if a block is a branch
        /// target. By default, this is false, and it's updated when
        /// emitting a `br` or `br_if`.
        is_branch_target: bool,
    },
    Loop {
        /// The start of the Loop.
        head: MachLabel,
        /// The length of the value stack at the beginning of the Loop.
        base_stack_len: usize,
        /// The stack pointer offset at the beginning of the Loop.
        base_sp: SPOffset,
        /// Information about the parameters and returns of the block.
        block_type_info: BlockTypeInfo,
    },
}

impl ControlStackFrame {
    /// Returns [`ControlStackFrame`] for an if.
    pub fn r#if<M: MacroAssembler>(
        results_data: ABIResultsData,
        block_type_info: BlockTypeInfo,
        masm: &mut M,
        context: &mut CodeGenContext,
    ) -> Self {
        let mut control = Self::If {
            cont: masm.get_label(),
            exit: masm.get_label(),
            results_data,
            block_type_info,
            reachable: context.reachable,
            base_stack_len: 0,
            base_sp: SPOffset::from_u32(0),
        };

        control.emit(masm, context);
        control
    }

    /// Creates a block that represents the base
    /// block for the function body.
    pub fn function_body_block<M: MacroAssembler>(
        results_data: ABIResultsData,
        block_type_info: BlockTypeInfo,
        masm: &mut M,
        context: &mut CodeGenContext,
    ) -> Self {
        Self::Block {
            base_stack_len: context.stack.len(),
            results_data,
            block_type_info,
            is_branch_target: false,
            exit: masm.get_label(),
            base_sp: masm.sp_offset(),
        }
    }

    /// Returns [`ControlStackFrame`] for a block.
    pub fn block<M: MacroAssembler>(
        results_data: ABIResultsData,
        block_type_info: BlockTypeInfo,
        masm: &mut M,
        context: &mut CodeGenContext,
    ) -> Self {
        let mut control = Self::Block {
            base_stack_len: 0,
            results_data,
            block_type_info,
            is_branch_target: false,
            exit: masm.get_label(),
            base_sp: SPOffset::from_u32(0),
        };

        control.emit(masm, context);
        control
    }

    /// Returns [`ControlStackFrame`] for a loop.
    pub fn r#loop<M: MacroAssembler>(
        block_type_info: BlockTypeInfo,
        masm: &mut M,
        context: &mut CodeGenContext,
    ) -> Self {
        let mut control = Self::Loop {
            base_stack_len: 0,
            block_type_info,
            head: masm.get_label(),
            base_sp: SPOffset::from_u32(0),
        };

        control.emit(masm, context);
        control
    }

    fn init<M: MacroAssembler>(&mut self, masm: &mut M, context: &mut CodeGenContext) {
        assert!(self.block_type_info().param_count == 0);
        assert!(self.block_type_info().result_count < 2);
        // Save any live registers and locals.
        context.spill(masm);
        self.set_base_stack_len(context.stack.len());
        self.set_base_sp(masm.sp_offset());
    }

    fn set_base_stack_len(&mut self, len: usize) {
        use ControlStackFrame::*;

        match self {
            If { base_stack_len, .. }
            | Block { base_stack_len, .. }
            | Loop { base_stack_len, .. } => *base_stack_len = len,
            _ => {}
        }
    }

    fn set_base_sp(&mut self, base: SPOffset) {
        use ControlStackFrame::*;

        match self {
            If { base_sp, .. } | Block { base_sp, .. } | Loop { base_sp, .. } => *base_sp = base,
            _ => {}
        }
    }

    fn block_type_info(&mut self) -> &BlockTypeInfo {
        use ControlStackFrame::*;
        match self {
            If {
                block_type_info, ..
            }
            | Else {
                block_type_info, ..
            }
            | Loop {
                block_type_info, ..
            }
            | Block {
                block_type_info, ..
            } => block_type_info,
        }
    }

    fn emit<M: MacroAssembler>(&mut self, masm: &mut M, context: &mut CodeGenContext) {
        use ControlStackFrame::*;

        // Do not perform any emissions if we are in an unreachable state.
        if !context.reachable {
            return;
        }

        match *self {
            If { cont, .. } => {
                // Pop the condition value.
                let top = context.pop_to_reg(masm, None);
                self.init(masm, context);
                masm.branch(
                    IntCmpKind::Eq,
                    top.reg.into(),
                    top.reg.into(),
                    cont,
                    OperandSize::S32,
                );
                context.free_reg(top);
            }
            Block { .. } => self.init(masm, context),
            Loop { head, .. } => {
                self.init(masm, context);
                masm.bind(head);
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
                results_data,
                base_stack_len,
                exit,
                block_type_info,
                ..
            } => {
                assert!(
                    (*base_stack_len + block_type_info.result_count - block_type_info.param_count)
                        == context.stack.len()
                );
                // Before emitting an unconditional jump to the exit branch,
                // we handle the result of the if-then block.
                context.pop_abi_results(results_data, masm);
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
                results_data,
                block_type_info,
                base_stack_len,
                base_sp,
                exit,
                ..
            } => {
                // Bind the else branch.
                masm.bind(*cont);

                // Update the stack control frame with an else control frame.
                *self = ControlStackFrame::Else {
                    exit: *exit,
                    base_stack_len: *base_stack_len,
                    reachable,
                    base_sp: *base_sp,
                    results_data: results_data.clone(),
                    block_type_info: *block_type_info,
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
                results_data,
                base_stack_len,
                block_type_info,
                ..
            }
            | Else {
                results_data,
                base_stack_len,
                block_type_info,
                ..
            }
            | Block {
                results_data,
                base_stack_len,
                block_type_info,
                ..
            } => {
                assert!(
                    (*base_stack_len + block_type_info.result_count - block_type_info.param_count)
                        == context.stack.len()
                );
                // Before binding the exit label, we handle the block results.
                context.pop_abi_results(results_data, masm);
                self.bind_end(masm, context);
            }
            Loop {
                block_type_info,
                base_stack_len,
                ..
            } => {
                assert!(
                    (*base_stack_len + block_type_info.result_count - block_type_info.param_count)
                        == context.stack.len()
                );
            }
        }
    }

    /// Binds the exit label of the current control stack frame and pushes the
    /// ABI results to the value stack.
    pub fn bind_end<M: MacroAssembler>(&self, masm: &mut M, context: &mut CodeGenContext) {
        // Push the results to the value stack.
        if let Some(data) = self.results() {
            context.push_abi_results(data, masm);
        }
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

    /// Returns [`crate::abi::ABIResults`] of the control stack frame
    /// block.
    pub fn results(&self) -> Option<&ABIResultsData> {
        use ControlStackFrame::*;

        match self {
            If { results_data, .. } | Else { results_data, .. } | Block { results_data, .. } => {
                Some(results_data)
            }
            Loop { .. } => None,
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

    /// Returns the value stack length and stack pointer offset of the
    /// control frame registered at entry.
    pub fn base_stack_len_and_sp(&self) -> (usize, SPOffset) {
        use ControlStackFrame::*;
        match self {
            If {
                base_sp,
                base_stack_len,
                ..
            }
            | Else {
                base_sp,
                base_stack_len,
                ..
            }
            | Block {
                base_sp,
                base_stack_len,
                ..
            }
            | Loop {
                base_sp,
                base_stack_len,
                ..
            } => (*base_stack_len, *base_sp),
        }
    }

    /// Resolves how to handle results when the current frame is a
    /// jump target Notably in the case of loops we don't take into
    /// account the frame's results.
    pub fn as_target_results(&self) -> Option<&ABIResultsData> {
        self.results()
    }

    /// Returns true if the current frame is [ControlStackFrame::If].
    pub fn is_if(&self) -> bool {
        match self {
            Self::If { .. } => true,
            _ => false,
        }
    }
}

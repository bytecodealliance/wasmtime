//! Data structures for control flow emission.
//!
//! As of the current implementation, Winch doesnt offer support for the
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
pub(crate) enum ControlStackFrame {
    If {
        /// The continuation label.
        ///
        /// This could be either the else branch if
        /// the if instruction has an else or the end
        /// label otherwise.
        cont: MachLabel,
        /// The exit label, only present if there's an else
        /// branch.
        end: Option<MachLabel>,
        /// The return values of the block.
        result: ABIResult,
        /// The size of the value stack at the beginning of the If.
        original_stack_size: usize,
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
            end: None,
            result,
            original_stack_size: 0,
        };

        control.emit(masm, context);
        control
    }

    fn emit<M: MacroAssembler>(&mut self, masm: &mut M, context: &mut CodeGenContext) {
        match self {
            ControlStackFrame::If {
                cont,
                original_stack_size,
                ..
            } => {
                // Pop the condition value.
                let top = context.pop_to_reg(masm, None, OperandSize::S32);

                // Unconditionall spill before emitting control flow.
                context.spill(masm);

                *original_stack_size = context.stack.len();
                masm.branch(CmpKind::Eq, top.into(), top.into(), *cont, OperandSize::S32);
                context.free_gpr(top);
            }
        }
    }

    /// Handles the else branch if the current control stack frame is
    /// [`ControlStackFrame::If`].
    pub fn emit_else<M: MacroAssembler>(&mut self, masm: &mut M, context: &mut CodeGenContext) {
        match self {
            ControlStackFrame::If {
                cont,
                end,
                result,
                original_stack_size,
                ..
            } => {
                assert!((*original_stack_size + result.len()) == context.stack.len());
                // Before emitting an unconditional jump to the end branch,
                // we handle the result of the if-then block.
                context.pop_abi_results(&result, masm);
                // Before binding the else branch, we emit the jump to the end
                // label.
                let end_label = masm.get_label();
                masm.jmp(end_label);
                // Update the stack control frame with the
                // end branch.
                *end = Some(end_label);
                // Bind the continuation branch.
                masm.bind(*cont);
            }
        }
    }

    /// Handles the end of a control stack frame.
    pub fn emit_end<M: MacroAssembler>(&mut self, masm: &mut M, context: &mut CodeGenContext) {
        match self {
            ControlStackFrame::If {
                cont,
                end,
                result,
                original_stack_size,
                ..
            } => {
                assert!((*original_stack_size + result.len()) == context.stack.len());
                // Before binding the exit label, we handle the block results.
                context.pop_abi_results(&result, masm);
                // Then we push the block results ino the value stack.
                context.push_abi_results(&result, masm);
                // If an end label is present, it means that
                // there's an else branch present, and we bind the end
                // label; else no else branch is present and
                // we bind the if's exit label.
                if let Some(end) = end {
                    masm.bind(*end);
                } else {
                    masm.bind(*cont);
                }
            }
        }
    }
}

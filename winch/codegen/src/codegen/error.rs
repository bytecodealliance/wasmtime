//! Classification of code generation errors.

use thiserror::Error;

/// A code generation error.
#[derive(Error, Debug)]
pub(crate) enum CodeGenError {
    /// 32-bit platform support.
    #[error("32-bit platforms are not supported")]
    Unsupported32BitPlatform,
    /// Unsupported WebAssembly type.
    #[error("Unsupported Wasm type")]
    UnsupportedWasmType,
    /// Missing implementation for a current instruction.
    #[error("Unimplemented Wasm instruction")]
    UnimplementedWasmInstruction,
    /// Unsupported eager initialization of tables.
    #[error("Unsupported eager initialization of tables")]
    UnsupportedTableEagerInit,
    /// An internal error.
    ///
    /// This error means that an internal invariant was not met and usually
    /// implies a compiler bug.
    #[error("Winch internal error")]
    Internal(InternalError),
}

/// An internal error.
#[derive(Error, Debug)]
pub(crate) enum InternalError {
    /// Register allocation error.
    #[error("Expected register to be available")]
    ExpectedRegisterToBeAvailable,
    /// Control frame expected.
    #[error("Expected control frame")]
    ControlFrameExpected,
    /// Control frame for if expected.
    #[error("Control frame for if expected")]
    IfControlFrameExpected,
    /// Not enough values in the value stack.
    #[error("Not enough values in the value stack")]
    MissingValuesInStack,
    /// Unexpected operand size. 32 or 64 bits are supported.
    #[error("Unexpected operand size for operation")]
    UnexpectedOperandSize,
    /// Accessing the value stack with an invalid index.
    #[error("Unexpected value stack index")]
    UnexpectedValueStackIndex,
    /// Expects a specific state in the value stack.
    #[error("Unexpected value in value stack")]
    UnexpectedValueInValueStack,
    /// A mismatch occured in the control frame state.
    #[error("Mismatch in control frame state")]
    ControlFrameStateMismatch,
    /// Expected a specific table element value.
    #[error("Table element value expected")]
    TableElementValueExpected,
    /// Illegal fuel tracking state.
    #[error("Illegal fuel state")]
    IllegalFuelState,
    /// Missing special function argument.
    #[error("Argument for `VMContext` expected")]
    VMContextArgumentExpected,
    /// Expected memory location to be addressed via the stack pointer.
    #[error("Expected stack pointer addressing")]
    SPAddressingExpected,
    /// Stack pointer offset is illegal.
    #[error("Invalid stack pointer offset")]
    InvalidSPOffset,
    /// Unexpected function call at location.
    #[error("Unexpected function call in current context")]
    UnexpectedFunctionCall,
}

impl CodeGenError {
    pub(crate) const fn unsupported_wasm_type() -> Self {
        Self::UnsupportedWasmType
    }

    pub(crate) const fn unsupported_table_eager_init() -> Self {
        Self::UnsupportedTableEagerInit
    }

    pub(crate) const fn unimplemented_wasm_instruction() -> Self {
        Self::UnimplementedWasmInstruction
    }

    pub(crate) const fn unsupported_32_bit_platform() -> Self {
        Self::Unsupported32BitPlatform
    }

    pub(crate) const fn unexpected_function_call() -> Self {
        Self::Internal(InternalError::UnexpectedFunctionCall)
    }

    pub(crate) const fn sp_addressing_expected() -> Self {
        Self::Internal(InternalError::SPAddressingExpected)
    }

    pub(crate) const fn invalid_sp_offset() -> Self {
        Self::Internal(InternalError::InvalidSPOffset)
    }

    pub(crate) const fn expected_register_to_be_available() -> Self {
        Self::Internal(InternalError::ExpectedRegisterToBeAvailable)
    }

    pub(crate) fn vmcontext_arg_expected() -> Self {
        Self::Internal(InternalError::VMContextArgumentExpected)
    }

    pub(crate) const fn control_frame_expected() -> Self {
        Self::Internal(InternalError::ControlFrameExpected)
    }

    pub(crate) const fn if_control_frame_expected() -> Self {
        Self::Internal(InternalError::IfControlFrameExpected)
    }

    pub(crate) const fn missing_values_in_stack() -> Self {
        Self::Internal(InternalError::MissingValuesInStack)
    }

    pub(crate) const fn unexpected_operand_size() -> Self {
        Self::Internal(InternalError::UnexpectedOperandSize)
    }

    pub(crate) const fn unexpected_value_stack_index() -> Self {
        Self::Internal(InternalError::UnexpectedValueStackIndex)
    }

    pub(crate) const fn unexpected_value_in_value_stack() -> Self {
        Self::Internal(InternalError::UnexpectedValueInValueStack)
    }

    pub(crate) const fn control_frame_state_mismatch() -> Self {
        Self::Internal(InternalError::ControlFrameStateMismatch)
    }

    pub(crate) const fn table_element_value_expected() -> Self {
        Self::Internal(InternalError::TableElementValueExpected)
    }

    pub(crate) const fn illegal_fuel_state() -> Self {
        Self::Internal(InternalError::IllegalFuelState)
    }
}

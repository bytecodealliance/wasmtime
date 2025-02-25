//! This module contains basic type definitions used by the implementation of
//! the stack switching proposal.

/// FIXME(frank-emrich) Will remove in the final upstreamed version
#[allow(dead_code, reason = "Only accessed in debug builds")]
pub const ENABLE_DEBUG_PRINTING: bool = false;

/// FIXME(frank-emrich) Will remove in the final upstreamed version
#[macro_export]
macro_rules! debug_println {
    ($( $args:expr ),+ ) => {
        #[cfg(debug_assertions)]
        if ENABLE_DEBUG_PRINTING {
            #[cfg(feature = "std")]
            println!($($args),*);
        }
    }
}

/// Runtime configuration options for stack switching that can be set
/// via the command line.
///
/// Part of wasmtime::config::Config type (which is not in scope in this crate).
#[derive(Debug, Clone)]
pub struct StackSwitchingConfig {
    /// The (fixed) size of a continuation stack.
    pub stack_size: usize,
}

impl Default for StackSwitchingConfig {
    fn default() -> Self {
        /// Default size for continuation stacks
        const DEFAULT_FIBER_SIZE: usize = 2097152; // 2MB = 512 pages of 4k

        Self {
            stack_size: DEFAULT_FIBER_SIZE,
        }
    }
}

/// Discriminant of variant `Absent` in
/// `wasmtime::runtime::vm::stack_switching::VMStackChain`.
pub const STACK_CHAIN_ABSENT_DISCRIMINANT: usize = 0;
/// Discriminant of variant `InitialStack` in
/// `wasmtime::runtime::vm::stack_switching::VMStackChain`.
pub const STACK_CHAIN_INITIAL_STACK_DISCRIMINANT: usize = 1;
/// Discriminant of variant `Continiation` in
/// `wasmtime::runtime::vm::stack_switching::VMStackChain`.
pub const STACK_CHAIN_CONTINUATION_DISCRIMINANT: usize = 2;

/// Discriminant of variant `Fresh` in
/// `runtime::vm::stack_switching::VMStackState`.
pub const STACK_STATE_FRESH_DISCRIMINANT: u32 = 0;
/// Discriminant of variant `Running` in
/// `runtime::vm::stack_switching::VMStackState`.
pub const STACK_STATE_RUNNING_DISCRIMINANT: u32 = 1;
/// Discriminant of variant `Parent` in
/// `runtime::vm::stack_switching::VMStackState`.
pub const STACK_STATE_PARENT_DISCRIMINANT: u32 = 2;
/// Discriminant of variant `Suspended` in
/// `runtime::vm::stack_switching::VMStackState`.
pub const STACK_STATE_SUSPENDED_DISCRIMINANT: u32 = 3;
/// Discriminant of variant `Returned` in
/// `runtime::vm::stack_switching::VMStackState`.
pub const STACK_STATE_RETURNED_DISCRIMINANT: u32 = 4;

/// Discriminant of variant `Return` in
/// `runtime::vm::stack_switching::ControlEffect`.
pub const CONTROL_EFFECT_RETURN_DISCRIMINANT: u32 = 0;
/// Discriminant of variant `Resume` in
/// `runtime::vm::stack_switching::ControlEffect`.
pub const CONTROL_EFFECT_RESUME_DISCRIMINANT: u32 = 1;
/// Discriminant of variant `Suspend` in
/// `runtime::vm::stack_switching::ControlEffect`.
pub const CONTROL_EFFECT_SUSPEND_DISCRIMINANT: u32 = 2;
/// Discriminant of variant `Switch` in
/// `runtime::vm::stack_switching::ControlEffect`.
pub const CONTROL_EFFECT_SWITCH_DISCRIMINANT: u32 = 3;

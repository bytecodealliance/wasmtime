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

/// Encodes the life cycle of a `VMContRef`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum VMStackState {
    /// The `VMContRef` has been created, but neither `resume` or `switch` has ever been
    /// called on it. During this stage, we may add arguments using `cont.bind`.
    Fresh,
    /// The continuation is running, meaning that it is the one currently
    /// executing code.
    Running,
    /// The continuation is suspended because it executed a resume instruction
    /// that has not finished yet. In other words, it became the parent of
    /// another continuation (which may itself be `Running`, a `Parent`, or
    /// `Suspended`).
    Parent,
    /// The continuation was suspended by a `suspend` or `switch` instruction.
    Suspended,
    /// The function originally passed to `cont.new` has returned normally.
    /// Note that there is no guarantee that a VMContRef will ever
    /// reach this status, as it may stay suspended until being dropped.
    Returned,
}

impl VMStackState {
    /// Returns i32 discriminant of this variant.
    pub fn discriminant(&self) -> i32 {
        // This is well-defined for an enum with repr(i32).
        unsafe { *(self as *const Self as *const i32) }
    }
}

impl From<VMStackState> for i32 {
    fn from(st: VMStackState) -> Self {
        st.discriminant()
    }
}

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

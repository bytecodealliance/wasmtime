use crate::frame_info::{GlobalFrameInfo, FRAME_INFO};
use crate::FrameInfo;
use backtrace::Backtrace;
use std::fmt;
use std::sync::Arc;
use wasmtime_environ::ir;

/// A struct representing an aborted instruction execution, with a message
/// indicating the cause.
#[derive(Clone)]
pub struct Trap {
    inner: Arc<TrapInner>,
}

/// State describing the occasion which evoked a trap.
#[derive(Debug)]
enum TrapReason {
    /// An error message describing a trap.
    Message(String),

    /// An `i32` exit status describing an explicit program exit.
    I32Exit(i32),

    /// A structured error describing a trap.
    Error(Box<dyn std::error::Error + Send + Sync>),

    /// A specific code for a trap triggered while executing WASM.
    InstructionTrap(TrapCode),
}

impl fmt::Display for TrapReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrapReason::Message(s) => write!(f, "{}", s),
            TrapReason::I32Exit(status) => write!(f, "Exited with i32 exit status {}", status),
            TrapReason::Error(e) => write!(f, "{}", e),
            TrapReason::InstructionTrap(code) => write!(f, "wasm trap: {}", code),
        }
    }
}

/// A trap code describing the reason for a trap.
///
/// All trap instructions have an explicit trap code.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TrapCode {
    /// The current stack space was exhausted.
    StackOverflow,

    /// An out-of-bounds memory access.
    MemoryOutOfBounds,

    /// A wasm atomic operation was presented with a not-naturally-aligned linear-memory address.
    HeapMisaligned,

    /// An out-of-bounds access to a table.
    TableOutOfBounds,

    /// Indirect call to a null table entry.
    IndirectCallToNull,

    /// Signature mismatch on indirect call.
    BadSignature,

    /// An integer arithmetic operation caused an overflow.
    IntegerOverflow,

    /// An integer division by zero.
    IntegerDivisionByZero,

    /// Failed float-to-int conversion.
    BadConversionToInteger,

    /// Code that was supposed to have been unreachable was reached.
    UnreachableCodeReached,

    /// Execution has potentially run too long and may be interrupted.
    Interrupt,
}

impl TrapCode {
    /// Panics if `code` is `ir::TrapCode::User`.
    fn from_non_user(code: ir::TrapCode) -> Self {
        match code {
            ir::TrapCode::StackOverflow => TrapCode::StackOverflow,
            ir::TrapCode::HeapOutOfBounds => TrapCode::MemoryOutOfBounds,
            ir::TrapCode::HeapMisaligned => TrapCode::HeapMisaligned,
            ir::TrapCode::TableOutOfBounds => TrapCode::TableOutOfBounds,
            ir::TrapCode::IndirectCallToNull => TrapCode::IndirectCallToNull,
            ir::TrapCode::BadSignature => TrapCode::BadSignature,
            ir::TrapCode::IntegerOverflow => TrapCode::IntegerOverflow,
            ir::TrapCode::IntegerDivisionByZero => TrapCode::IntegerDivisionByZero,
            ir::TrapCode::BadConversionToInteger => TrapCode::BadConversionToInteger,
            ir::TrapCode::UnreachableCodeReached => TrapCode::UnreachableCodeReached,
            ir::TrapCode::Interrupt => TrapCode::Interrupt,
            ir::TrapCode::User(_) => panic!("Called `TrapCode::from_non_user` with user code"),
        }
    }
}

impl fmt::Display for TrapCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TrapCode::*;
        let desc = match self {
            StackOverflow => "call stack exhausted",
            MemoryOutOfBounds => "out of bounds memory access",
            HeapMisaligned => "misaligned memory access",
            TableOutOfBounds => "undefined element: out of bounds table access",
            IndirectCallToNull => "uninitialized element",
            BadSignature => "indirect call type mismatch",
            IntegerOverflow => "integer overflow",
            IntegerDivisionByZero => "integer divide by zero",
            BadConversionToInteger => "invalid conversion to integer",
            UnreachableCodeReached => "unreachable",
            Interrupt => "interrupt",
        };
        write!(f, "{}", desc)
    }
}

struct TrapInner {
    reason: TrapReason,
    wasm_trace: Vec<FrameInfo>,
    native_trace: Backtrace,
}

fn _assert_trap_is_sync_and_send(t: &Trap) -> (&dyn Sync, &dyn Send) {
    (t, t)
}

impl Trap {
    /// Creates a new `Trap` with `message`.
    /// # Example
    /// ```
    /// let trap = wasmtime::Trap::new("unexpected error");
    /// assert!(trap.to_string().contains("unexpected error"));
    /// ```
    pub fn new<I: Into<String>>(message: I) -> Self {
        let info = FRAME_INFO.read().unwrap();
        let reason = TrapReason::Message(message.into());
        Trap::new_with_trace(&info, None, reason, Backtrace::new_unresolved())
    }

    /// Creates a new `Trap` representing an explicit program exit with a classic `i32`
    /// exit status value.
    pub fn i32_exit(status: i32) -> Self {
        Trap {
            inner: Arc::new(TrapInner {
                reason: TrapReason::I32Exit(status),
                wasm_trace: Vec::new(),
                native_trace: Backtrace::from(Vec::new()),
            }),
        }
    }

    pub(crate) fn from_runtime(runtime_trap: wasmtime_runtime::Trap) -> Self {
        let info = FRAME_INFO.read().unwrap();
        match runtime_trap {
            wasmtime_runtime::Trap::User(error) => Trap::from(error),
            wasmtime_runtime::Trap::Jit {
                pc,
                backtrace,
                maybe_interrupted,
            } => {
                let mut code = info
                    .lookup_trap_info(pc)
                    .map(|info| info.trap_code)
                    .unwrap_or(ir::TrapCode::StackOverflow);
                if maybe_interrupted && code == ir::TrapCode::StackOverflow {
                    code = ir::TrapCode::Interrupt;
                }
                Trap::new_wasm(&info, Some(pc), code, backtrace)
            }
            wasmtime_runtime::Trap::Wasm {
                trap_code,
                backtrace,
            } => Trap::new_wasm(&info, None, trap_code, backtrace),
            wasmtime_runtime::Trap::OOM { backtrace } => {
                let reason = TrapReason::Message("out of memory".to_string());
                Trap::new_with_trace(&info, None, reason, backtrace)
            }
        }
    }

    fn new_wasm(
        info: &GlobalFrameInfo,
        trap_pc: Option<usize>,
        code: ir::TrapCode,
        backtrace: Backtrace,
    ) -> Self {
        let code = TrapCode::from_non_user(code);
        Trap::new_with_trace(info, trap_pc, TrapReason::InstructionTrap(code), backtrace)
    }

    fn new_with_trace(
        info: &GlobalFrameInfo,
        trap_pc: Option<usize>,
        reason: TrapReason,
        native_trace: Backtrace,
    ) -> Self {
        let mut wasm_trace = Vec::new();
        for frame in native_trace.frames() {
            let pc = frame.ip() as usize;
            if pc == 0 {
                continue;
            }
            // Note that we need to be careful about the pc we pass in here to
            // lookup frame information. This program counter is used to
            // translate back to an original source location in the origin wasm
            // module. If this pc is the exact pc that the trap happened at,
            // then we look up that pc precisely. Otherwise backtrace
            // information typically points at the pc *after* the call
            // instruction (because otherwise it's likely a call instruction on
            // the stack). In that case we want to lookup information for the
            // previous instruction (the call instruction) so we subtract one as
            // the lookup.
            let pc_to_lookup = if Some(pc) == trap_pc { pc } else { pc - 1 };
            if let Some(info) = info.lookup_frame_info(pc_to_lookup) {
                wasm_trace.push(info);
            }
        }
        Trap {
            inner: Arc::new(TrapInner {
                reason,
                wasm_trace,
                native_trace,
            }),
        }
    }

    /// If the trap was the result of an explicit program exit with a classic
    /// `i32` exit status value, return the value, otherwise return `None`.
    pub fn i32_exit_status(&self) -> Option<i32> {
        match self.inner.reason {
            TrapReason::I32Exit(status) => Some(status),
            _ => None,
        }
    }

    /// Returns a list of function frames in WebAssembly code that led to this
    /// trap happening.
    pub fn trace(&self) -> &[FrameInfo] {
        &self.inner.wasm_trace
    }

    /// Code of a trap that happened while executing a WASM instruction.
    /// If the trap was triggered by a host export this will be `None`.
    pub fn trap_code(&self) -> Option<TrapCode> {
        match self.inner.reason {
            TrapReason::InstructionTrap(code) => Some(code),
            _ => None,
        }
    }
}

impl fmt::Debug for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Trap")
            .field("reason", &self.inner.reason)
            .field("wasm_trace", &self.inner.wasm_trace)
            .field("native_trace", &self.inner.native_trace)
            .finish()
    }
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.reason)?;
        let trace = self.trace();
        if trace.is_empty() {
            return Ok(());
        }
        writeln!(f, "\nwasm backtrace:")?;
        for (i, frame) in self.trace().iter().enumerate() {
            let name = frame.module_name().unwrap_or("<unknown>");
            write!(f, "  {}: {:#6x} - {}!", i, frame.module_offset(), name)?;
            match frame.func_name() {
                Some(name) => match rustc_demangle::try_demangle(name) {
                    Ok(name) => write!(f, "{}", name)?,
                    Err(_) => write!(f, "{}", name)?,
                },
                None => write!(f, "<wasm function {}>", frame.func_index())?,
            }
            writeln!(f, "")?;
        }
        Ok(())
    }
}

impl std::error::Error for Trap {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner.reason {
            TrapReason::Error(e) => e.source(),
            TrapReason::I32Exit(_) | TrapReason::Message(_) | TrapReason::InstructionTrap(_) => {
                None
            }
        }
    }
}

impl From<anyhow::Error> for Trap {
    fn from(e: anyhow::Error) -> Trap {
        Box::<dyn std::error::Error + Send + Sync>::from(e).into()
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Trap {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Trap {
        // If the top-level error is already a trap, don't be redundant and just return it.
        if let Some(trap) = e.downcast_ref::<Trap>() {
            trap.clone()
        } else {
            let info = FRAME_INFO.read().unwrap();
            let reason = TrapReason::Error(e.into());
            Trap::new_with_trace(&info, None, reason, Backtrace::new_unresolved())
        }
    }
}

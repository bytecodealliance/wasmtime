use crate::frame_info::{GlobalFrameInfo, FRAME_INFO};
use crate::FrameInfo;
use backtrace::Backtrace;
use std::fmt;
use std::sync::Arc;
use wasmtime_environ::ir::TrapCode;

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
}

impl fmt::Display for TrapReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrapReason::Message(s) => write!(f, "{}", s),
            TrapReason::I32Exit(status) => write!(f, "Exited with i32 exit status {}", status),
            TrapReason::Error(e) => write!(f, "{}", e),
        }
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
                    .unwrap_or(TrapCode::StackOverflow);
                if maybe_interrupted && code == TrapCode::StackOverflow {
                    code = TrapCode::Interrupt;
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
        code: TrapCode,
        backtrace: Backtrace,
    ) -> Self {
        use wasmtime_environ::ir::TrapCode::*;
        let desc = match code {
            StackOverflow => "call stack exhausted",
            HeapOutOfBounds => "out of bounds memory access",
            HeapMisaligned => "misaligned memory access",
            TableOutOfBounds => "undefined element: out of bounds table access",
            IndirectCallToNull => "uninitialized element",
            BadSignature => "indirect call type mismatch",
            IntegerOverflow => "integer overflow",
            IntegerDivisionByZero => "integer divide by zero",
            BadConversionToInteger => "invalid conversion to integer",
            UnreachableCodeReached => "unreachable",
            Interrupt => "interrupt",
            User(_) => unreachable!(),
        };
        let msg = TrapReason::Message(format!("wasm trap: {}", desc));
        Trap::new_with_trace(info, trap_pc, msg, backtrace)
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
            TrapReason::I32Exit(_) | TrapReason::Message(_) => None,
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

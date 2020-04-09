use crate::frame_info::{GlobalFrameInfo, FRAME_INFO};
use crate::FrameInfo;
use backtrace::Backtrace;
use std::fmt;
use std::sync::Arc;
use wasmtime_environ::ir::{SourceLoc, TrapCode};

/// A struct representing an aborted instruction execution, with a message
/// indicating the cause.
#[derive(Clone)]
pub struct Trap {
    inner: Arc<TrapInner>,
}

struct TrapInner {
    message: String,
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
    /// assert_eq!("unexpected error", trap.message());
    /// ```
    pub fn new<I: Into<String>>(message: I) -> Self {
        let info = FRAME_INFO.read().unwrap();
        Trap::new_with_trace(&info, message.into(), Backtrace::new_unresolved())
    }

    pub(crate) fn from_jit(jit: wasmtime_runtime::Trap) -> Self {
        let info = FRAME_INFO.read().unwrap();
        match jit {
            wasmtime_runtime::Trap::User(error) => {
                // Since we're the only one using the wasmtime internals (in
                // theory) we should only see user errors which were originally
                // created from our own `Trap` type (see the trampoline module
                // with functions).
                //
                // If this unwrap trips for someone we'll need to tweak the
                // return type of this function to probably be `anyhow::Error`
                // or something like that.
                *error
                    .downcast()
                    .expect("only `Trap` user errors are supported")
            }
            wasmtime_runtime::Trap::Jit { pc, backtrace } => {
                let (code, loc) = info
                    .lookup_trap_info(pc)
                    .map(|info| (info.trap_code, info.source_loc))
                    .unwrap_or((TrapCode::StackOverflow, SourceLoc::default()));
                Trap::new_wasm(&info, code, loc, backtrace)
            }
            wasmtime_runtime::Trap::Wasm {
                trap_code,
                source_loc,
                backtrace,
            } => Trap::new_wasm(&info, trap_code, source_loc, backtrace),
            wasmtime_runtime::Trap::OOM { backtrace } => {
                Trap::new_with_trace(&info, "out of memory".to_string(), backtrace)
            }
        }
    }

    fn new_wasm(
        info: &GlobalFrameInfo,
        code: TrapCode,
        loc: SourceLoc,
        backtrace: Backtrace,
    ) -> Self {
        use wasmtime_environ::ir::TrapCode::*;
        let desc = match code {
            StackOverflow => "call stack exhausted",
            HeapOutOfBounds => "out of bounds memory access",
            TableOutOfBounds => "undefined element: out of bounds table access",
            OutOfBounds => "out of bounds",
            IndirectCallToNull => "uninitialized element",
            BadSignature => "indirect call type mismatch",
            IntegerOverflow => "integer overflow",
            IntegerDivisionByZero => "integer divide by zero",
            BadConversionToInteger => "invalid conversion to integer",
            UnreachableCodeReached => "unreachable",
            Interrupt => "interrupt",
            User(_) => unreachable!(),
        };
        let msg = if loc != SourceLoc::default() {
            format!("wasm trap: {}, source location: {}", desc, loc)
        } else {
            format!("wasm trap: {}", desc)
        };
        Trap::new_with_trace(info, msg, backtrace)
    }

    fn new_with_trace(info: &GlobalFrameInfo, message: String, native_trace: Backtrace) -> Self {
        let mut wasm_trace = Vec::new();
        for frame in native_trace.frames() {
            let pc = frame.ip() as usize;
            if let Some(info) = info.lookup_frame_info(pc) {
                wasm_trace.push(info);
            }
        }
        Trap {
            inner: Arc::new(TrapInner {
                message,
                wasm_trace,
                native_trace,
            }),
        }
    }

    /// Returns a reference the `message` stored in `Trap`.
    pub fn message(&self) -> &str {
        &self.inner.message
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
            .field("message", &self.inner.message)
            .field("wasm_trace", &self.inner.wasm_trace)
            .field("native_trace", &self.inner.native_trace)
            .finish()
    }
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.message)?;
        let trace = self.trace();
        if trace.is_empty() {
            return Ok(());
        }
        writeln!(f, "\nwasm backtrace:")?;
        for (i, frame) in self.trace().iter().enumerate() {
            let name = frame.module_name().unwrap_or("<unknown>");
            write!(f, "  {}: {}!", i, name)?;
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

impl std::error::Error for Trap {}

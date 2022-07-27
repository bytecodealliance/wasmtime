use crate::store::StoreOpaque;
use crate::Module;
use once_cell::sync::OnceCell;
use std::fmt;
use std::sync::Arc;
use wasmtime_environ::{EntityRef, FilePos, TrapCode as EnvTrapCode};
use wasmtime_jit::{demangle_function_name, demangle_function_name_or_index};
use wasmtime_runtime::Backtrace;

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
///
/// The code can be accessed from the c-api, where the possible values are translated
/// into enum values defined there:
///
/// * `wasm_trap_code` in c-api/src/trap.rs, and
/// * `wasmtime_trap_code_enum` in c-api/include/wasmtime/trap.h.
///
/// These need to be kept in sync.
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

    /// When the `component-model` feature is enabled this trap represents a
    /// function that was `canon lift`'d, then `canon lower`'d, then called.
    /// This combination of creation of a function in the component model
    /// generates a function that always traps and, when called, produces this
    /// flavor of trap.
    AlwaysTrapAdapter,
}

impl TrapCode {
    /// Panics if `code` is `EnvTrapCode::User`.
    fn from_non_user(code: EnvTrapCode) -> Self {
        match code {
            EnvTrapCode::StackOverflow => TrapCode::StackOverflow,
            EnvTrapCode::HeapOutOfBounds => TrapCode::MemoryOutOfBounds,
            EnvTrapCode::HeapMisaligned => TrapCode::HeapMisaligned,
            EnvTrapCode::TableOutOfBounds => TrapCode::TableOutOfBounds,
            EnvTrapCode::IndirectCallToNull => TrapCode::IndirectCallToNull,
            EnvTrapCode::BadSignature => TrapCode::BadSignature,
            EnvTrapCode::IntegerOverflow => TrapCode::IntegerOverflow,
            EnvTrapCode::IntegerDivisionByZero => TrapCode::IntegerDivisionByZero,
            EnvTrapCode::BadConversionToInteger => TrapCode::BadConversionToInteger,
            EnvTrapCode::UnreachableCodeReached => TrapCode::UnreachableCodeReached,
            EnvTrapCode::Interrupt => TrapCode::Interrupt,
            EnvTrapCode::AlwaysTrapAdapter => TrapCode::AlwaysTrapAdapter,
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
            UnreachableCodeReached => "wasm `unreachable` instruction executed",
            Interrupt => "interrupt",
            AlwaysTrapAdapter => "degenerate component adapter called",
        };
        write!(f, "{}", desc)
    }
}

#[derive(Debug)]
pub(crate) struct TrapBacktrace {
    wasm_trace: Vec<FrameInfo>,
    native_trace: Backtrace,
    hint_wasm_backtrace_details_env: bool,
}

impl TrapBacktrace {
    pub fn new(store: &StoreOpaque, native_trace: Backtrace, trap_pc: Option<usize>) -> Self {
        let mut wasm_trace = Vec::<FrameInfo>::new();
        let mut hint_wasm_backtrace_details_env = false;
        let wasm_backtrace_details_env_used =
            store.engine().config().wasm_backtrace_details_env_used;

        for frame in native_trace.frames() {
            let pc = frame.ip() as usize;
            if pc == 0 {
                continue;
            }
            // Note that we need to be careful about the pc we pass in
            // here to lookup frame information. This program counter is
            // used to translate back to an original source location in
            // the origin wasm module. If this pc is the exact pc that
            // the trap happened at, then we look up that pc precisely.
            // Otherwise backtrace information typically points at the
            // pc *after* the call instruction (because otherwise it's
            // likely a call instruction on the stack). In that case we
            // want to lookup information for the previous instruction
            // (the call instruction) so we subtract one as the lookup.
            let pc_to_lookup = if Some(pc) == trap_pc { pc } else { pc - 1 };
            if let Some((info, module)) = store.modules().lookup_frame_info(pc_to_lookup) {
                wasm_trace.push(info);

                // If this frame has unparsed debug information and the
                // store's configuration indicates that we were
                // respecting the environment variable of whether to
                // do this then we will print out a helpful note in
                // `Display` to indicate that more detailed information
                // in a trap may be available.
                let has_unparsed_debuginfo = module.compiled_module().has_unparsed_debuginfo();
                if has_unparsed_debuginfo && wasm_backtrace_details_env_used {
                    hint_wasm_backtrace_details_env = true;
                }
            }
        }

        Self {
            wasm_trace,
            native_trace,
            hint_wasm_backtrace_details_env,
        }
    }
}

struct TrapInner {
    reason: TrapReason,
    backtrace: OnceCell<TrapBacktrace>,
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
    #[cold] // traps are exceptional, this helps move handling off the main path
    pub fn new<I: Into<String>>(message: I) -> Self {
        let reason = TrapReason::Message(message.into());
        Trap::new_with_trace(reason, None)
    }

    /// Creates a new `Trap` representing an explicit program exit with a classic `i32`
    /// exit status value.
    #[cold] // see Trap::new
    pub fn i32_exit(status: i32) -> Self {
        Trap::new_with_trace(TrapReason::I32Exit(status), None)
    }

    #[cold] // see Trap::new
    pub(crate) fn from_runtime_box(
        store: &StoreOpaque,
        runtime_trap: Box<wasmtime_runtime::Trap>,
    ) -> Self {
        Self::from_runtime(store, *runtime_trap)
    }

    #[cold] // see Trap::new
    pub(crate) fn from_runtime(store: &StoreOpaque, runtime_trap: wasmtime_runtime::Trap) -> Self {
        let wasmtime_runtime::Trap { reason, backtrace } = runtime_trap;
        match reason {
            wasmtime_runtime::TrapReason::User(error) => {
                let trap = Trap::from(error);
                if let Some(backtrace) = backtrace {
                    trap.record_backtrace(TrapBacktrace::new(store, backtrace, None));
                }
                trap
            }
            wasmtime_runtime::TrapReason::Jit(pc) => {
                let code = store
                    .modules()
                    .lookup_trap_code(pc)
                    .unwrap_or(EnvTrapCode::StackOverflow);
                let backtrace = backtrace.map(|bt| TrapBacktrace::new(store, bt, Some(pc)));
                Trap::new_wasm(code, backtrace)
            }
            wasmtime_runtime::TrapReason::Wasm(trap_code) => {
                let backtrace = backtrace.map(|bt| TrapBacktrace::new(store, bt, None));
                Trap::new_wasm(trap_code, backtrace)
            }
        }
    }

    #[cold] // see Trap::new
    pub(crate) fn new_wasm(code: EnvTrapCode, backtrace: Option<TrapBacktrace>) -> Self {
        let code = TrapCode::from_non_user(code);
        Trap::new_with_trace(TrapReason::InstructionTrap(code), backtrace)
    }

    /// Creates a new `Trap`.
    /// * `reason` - this is the wasmtime-internal reason for why this trap is
    ///   being created.
    ///
    /// * `backtrace` - this is a captured backtrace from when the trap
    ///   occurred. Contains the native backtrace, and the backtrace of
    ///   WebAssembly frames.
    fn new_with_trace(reason: TrapReason, backtrace: Option<TrapBacktrace>) -> Self {
        let backtrace = if let Some(bt) = backtrace {
            OnceCell::with_value(bt)
        } else {
            OnceCell::new()
        };
        Trap {
            inner: Arc::new(TrapInner { reason, backtrace }),
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

    /// Displays the error reason for this trap.
    ///
    /// In particular, it differs from this struct's `Display` by *only*
    /// showing the reason, and not the full backtrace. This is useful to
    /// customize the way the trap is reported, for instance to display a short
    /// message for user-facing errors.
    pub fn display_reason<'a>(&'a self) -> impl fmt::Display + 'a {
        &self.inner.reason
    }

    /// Returns a list of function frames in WebAssembly code that led to this
    /// trap happening.
    ///
    /// This function return an `Option` of a list of frames to indicate that
    /// wasm frames are not always available. Frames will never be available if
    /// backtraces are disabled via
    /// [`Config::wasm_backtrace`](crate::Config::wasm_backtrace). Frames will
    /// also not be available for freshly-created traps. WebAssembly frames are
    /// currently only captured when the trap reaches wasm itself to get raised
    /// across a wasm boundary.
    pub fn trace(&self) -> Option<&[FrameInfo]> {
        self.inner
            .backtrace
            .get()
            .as_ref()
            .map(|bt| bt.wasm_trace.as_slice())
    }

    /// Code of a trap that happened while executing a WASM instruction.
    /// If the trap was triggered by a host export this will be `None`.
    pub fn trap_code(&self) -> Option<TrapCode> {
        match self.inner.reason {
            TrapReason::InstructionTrap(code) => Some(code),
            _ => None,
        }
    }

    fn record_backtrace(&self, backtrace: TrapBacktrace) {
        // When a trap is created on top of the wasm stack, the trampoline will
        // re-raise it via
        // `wasmtime_runtime::raise_user_trap(trap.into::<Box<dyn Error>>())`
        // after panic::catch_unwind. We don't want to overwrite the first
        // backtrace recorded, as it is most precise.
        // FIXME: make sure backtraces are only created once per trap! they are
        // actually kinda expensive to create.
        let _ = self.inner.backtrace.try_insert(backtrace);
    }
}

impl fmt::Debug for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Trap");
        f.field("reason", &self.inner.reason);
        if let Some(backtrace) = self.inner.backtrace.get() {
            f.field("wasm_trace", &backtrace.wasm_trace)
                .field("native_trace", &backtrace.native_trace);
        }
        f.finish()
    }
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.reason)?;

        if let Some(trace) = self.trace() {
            if trace.is_empty() {
                return Ok(());
            }
            writeln!(f, "\nwasm backtrace:")?;

            for (i, frame) in trace.iter().enumerate() {
                let name = frame.module_name().unwrap_or("<unknown>");
                write!(f, "  {:>3}: ", i)?;

                if let Some(offset) = frame.module_offset() {
                    write!(f, "{:#6x} - ", offset)?;
                }

                let write_raw_func_name = |f: &mut fmt::Formatter<'_>| {
                    demangle_function_name_or_index(
                        f,
                        frame.func_name(),
                        frame.func_index() as usize,
                    )
                };
                if frame.symbols().is_empty() {
                    write!(f, "{}!", name)?;
                    write_raw_func_name(f)?;
                    writeln!(f, "")?;
                } else {
                    for (i, symbol) in frame.symbols().iter().enumerate() {
                        if i > 0 {
                            write!(f, "              - ")?;
                        } else {
                            // ...
                        }
                        match symbol.name() {
                            Some(name) => demangle_function_name(f, name)?,
                            None if i == 0 => write_raw_func_name(f)?,
                            None => write!(f, "<inlined function>")?,
                        }
                        writeln!(f, "")?;
                        if let Some(file) = symbol.file() {
                            write!(f, "                    at {}", file)?;
                            if let Some(line) = symbol.line() {
                                write!(f, ":{}", line)?;
                                if let Some(col) = symbol.column() {
                                    write!(f, ":{}", col)?;
                                }
                            }
                        }
                        writeln!(f, "")?;
                    }
                }
            }
            if self
                .inner
                .backtrace
                .get()
                .map(|t| t.hint_wasm_backtrace_details_env)
                .unwrap_or(false)
            {
                writeln!(f, "note: using the `WASMTIME_BACKTRACE_DETAILS=1` environment variable to may show more debugging information")?;
            }
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
        match e.downcast::<Trap>() {
            Ok(trap) => trap,
            Err(e) => Box::<dyn std::error::Error + Send + Sync>::from(e).into(),
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Trap {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Trap {
        // If the top-level error is already a trap, don't be redundant and just return it.
        if let Some(trap) = e.downcast_ref::<Trap>() {
            trap.clone()
        } else {
            let reason = TrapReason::Error(e.into());
            Trap::new_with_trace(reason, None)
        }
    }
}

/// Description of a frame in a backtrace for a [`Trap`].
///
/// Whenever a WebAssembly trap occurs an instance of [`Trap`] is created. Each
/// [`Trap`] has a backtrace of the WebAssembly frames that led to the trap, and
/// each frame is described by this structure.
///
/// [`Trap`]: crate::Trap
#[derive(Debug)]
pub struct FrameInfo {
    module_name: Option<String>,
    func_index: u32,
    func_name: Option<String>,
    func_start: FilePos,
    instr: Option<FilePos>,
    symbols: Vec<FrameSymbol>,
}

impl FrameInfo {
    /// Fetches frame information about a program counter in a backtrace.
    ///
    /// Returns an object if this `pc` is known to this module, or returns `None`
    /// if no information can be found.
    pub(crate) fn new(module: &Module, text_offset: usize) -> Option<FrameInfo> {
        let module = module.compiled_module();
        let (index, _func_offset) = module.func_by_text_offset(text_offset)?;
        let info = module.func_info(index);
        let instr = wasmtime_environ::lookup_file_pos(module.address_map_data(), text_offset);

        // In debug mode for now assert that we found a mapping for `pc` within
        // the function, because otherwise something is buggy along the way and
        // not accounting for all the instructions. This isn't super critical
        // though so we can omit this check in release mode.
        //
        // Note that if the module doesn't even have an address map due to
        // compilation settings then it's expected that `instr` is `None`.
        debug_assert!(
            instr.is_some() || !module.has_address_map(),
            "failed to find instruction for {:#x}",
            text_offset
        );

        // Use our wasm-relative pc to symbolize this frame. If there's a
        // symbolication context (dwarf debug info) available then we can try to
        // look this up there.
        //
        // Note that dwarf pcs are code-section-relative, hence the subtraction
        // from the location of `instr`. Also note that all errors are ignored
        // here for now since technically wasm modules can always have any
        // custom section contents.
        let mut symbols = Vec::new();

        if let Some(s) = &module.symbolize_context().ok().and_then(|c| c) {
            if let Some(offset) = instr.and_then(|i| i.file_offset()) {
                let to_lookup = u64::from(offset) - s.code_section_offset();
                if let Ok(mut frames) = s.addr2line().find_frames(to_lookup) {
                    while let Ok(Some(frame)) = frames.next() {
                        symbols.push(FrameSymbol {
                            name: frame
                                .function
                                .as_ref()
                                .and_then(|l| l.raw_name().ok())
                                .map(|s| s.to_string()),
                            file: frame
                                .location
                                .as_ref()
                                .and_then(|l| l.file)
                                .map(|s| s.to_string()),
                            line: frame.location.as_ref().and_then(|l| l.line),
                            column: frame.location.as_ref().and_then(|l| l.column),
                        });
                    }
                }
            }
        }

        let index = module.module().func_index(index);

        Some(FrameInfo {
            module_name: module.module().name.clone(),
            func_index: index.index() as u32,
            func_name: module.func_name(index).map(|s| s.to_string()),
            instr,
            func_start: info.start_srcloc,
            symbols,
        })
    }

    /// Returns the WebAssembly function index for this frame.
    ///
    /// This function index is the index in the function index space of the
    /// WebAssembly module that this frame comes from.
    pub fn func_index(&self) -> u32 {
        self.func_index
    }

    /// Returns the identifer of the module that this frame is for.
    ///
    /// Module identifiers are present in the `name` section of a WebAssembly
    /// binary, but this may not return the exact item in the `name` section.
    /// Module names can be overwritten at construction time or perhaps inferred
    /// from file names. The primary purpose of this function is to assist in
    /// debugging and therefore may be tweaked over time.
    ///
    /// This function returns `None` when no name can be found or inferred.
    pub fn module_name(&self) -> Option<&str> {
        self.module_name.as_deref()
    }

    /// Returns a descriptive name of the function for this frame, if one is
    /// available.
    ///
    /// The name of this function may come from the `name` section of the
    /// WebAssembly binary, or wasmtime may try to infer a better name for it if
    /// not available, for example the name of the export if it's exported.
    ///
    /// This return value is primarily used for debugging and human-readable
    /// purposes for things like traps. Note that the exact return value may be
    /// tweaked over time here and isn't guaranteed to be something in
    /// particular about a wasm module due to its primary purpose of assisting
    /// in debugging.
    ///
    /// This function returns `None` when no name could be inferred.
    pub fn func_name(&self) -> Option<&str> {
        self.func_name.as_deref()
    }

    /// Returns the offset within the original wasm module this frame's program
    /// counter was at.
    ///
    /// The offset here is the offset from the beginning of the original wasm
    /// module to the instruction that this frame points to.
    ///
    /// Note that `None` may be returned if the original module was not
    /// compiled with mapping information to yield this information. This is
    /// controlled by the
    /// [`Config::generate_address_map`](crate::Config::generate_address_map)
    /// configuration option.
    pub fn module_offset(&self) -> Option<usize> {
        Some(self.instr?.file_offset()? as usize)
    }

    /// Returns the offset from the original wasm module's function to this
    /// frame's program counter.
    ///
    /// The offset here is the offset from the beginning of the defining
    /// function of this frame (within the wasm module) to the instruction this
    /// frame points to.
    ///
    /// Note that `None` may be returned if the original module was not
    /// compiled with mapping information to yield this information. This is
    /// controlled by the
    /// [`Config::generate_address_map`](crate::Config::generate_address_map)
    /// configuration option.
    pub fn func_offset(&self) -> Option<usize> {
        let instr_offset = self.instr?.file_offset()?;
        Some((instr_offset - self.func_start.file_offset()?) as usize)
    }

    /// Returns the debug symbols found, if any, for this function frame.
    ///
    /// When a wasm program is compiled with DWARF debug information then this
    /// function may be populated to return symbols which contain extra debug
    /// information about a frame including the filename and line number. If no
    /// debug information was found or if it was malformed then this will return
    /// an empty array.
    pub fn symbols(&self) -> &[FrameSymbol] {
        &self.symbols
    }
}

/// Debug information for a symbol that is attached to a [`FrameInfo`].
///
/// When DWARF debug information is present in a wasm file then this structure
/// can be found on a [`FrameInfo`] and can be used to learn about filenames,
/// line numbers, etc, which are the origin of a function in a stack trace.
#[derive(Debug)]
pub struct FrameSymbol {
    name: Option<String>,
    file: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
}

impl FrameSymbol {
    /// Returns the function name associated with this symbol.
    ///
    /// Note that this may not be present with malformed debug information, or
    /// the debug information may not include it. Also note that the symbol is
    /// frequently mangled, so you might need to run some form of demangling
    /// over it.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the source code filename this symbol was defined in.
    ///
    /// Note that this may not be present with malformed debug information, or
    /// the debug information may not include it.
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }

    /// Returns the 1-indexed source code line number this symbol was defined
    /// on.
    ///
    /// Note that this may not be present with malformed debug information, or
    /// the debug information may not include it.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// Returns the 1-indexed source code column number this symbol was defined
    /// on.
    ///
    /// Note that this may not be present with malformed debug information, or
    /// the debug information may not include it.
    pub fn column(&self) -> Option<u32> {
        self.column
    }
}

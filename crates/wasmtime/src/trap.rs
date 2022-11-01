use crate::store::StoreOpaque;
use crate::Module;
use anyhow::Error;
use std::fmt;
use wasmtime_environ::{EntityRef, FilePos, TrapCode};
use wasmtime_jit::{demangle_function_name, demangle_function_name_or_index};

/// Description of a trap which can happen in WebAssembly, halting wasm
/// execution.
///
/// This enumeration is a list of all possible traps that can happen in wasm, in
/// addition to some Wasmtime-specific trap codes listed here as well. This type
/// will be returned as an error when a wasm function hits one of these trap
/// conditions through the [`anyhow::Error`] wrapper.
//
// The code can be accessed from the c-api, where the possible values are translated
// into enum values defined there:
//
// * `wasm_trap_code` in c-api/src/trap.rs, and
// * `wasmtime_trap_code_enum` in c-api/include/wasmtime/trap.h.
//
// These need to be kept in sync.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Trap {
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

    /// When wasm code is configured to consume fuel and it runs out of fuel
    /// then this trap will be raised.
    ///
    /// For more information see
    /// [`Config::consume_fuel`](crate::Config::consume_fuel).
    OutOfFuel,
}

impl Trap {
    // Same safety requirements and caveats as
    // `wasmtime_runtime::raise_user_trap`.
    pub(crate) unsafe fn raise(error: anyhow::Error) -> ! {
        let needs_backtrace = error.downcast_ref::<WasmBacktrace>().is_none();
        wasmtime_runtime::raise_user_trap(error, needs_backtrace)
    }

    #[cold] // traps are exceptional, this helps move handling off the main path
    pub(crate) fn from_runtime_box(
        store: &StoreOpaque,
        runtime_trap: Box<wasmtime_runtime::Trap>,
    ) -> Error {
        let wasmtime_runtime::Trap { reason, backtrace } = *runtime_trap;
        let (error, pc) = match reason {
            // For user-defined errors they're already an `anyhow::Error` so no
            // conversion is really necessary here, but a `backtrace` may have
            // been captured so it's attempted to get inserted here.
            //
            // If the error is actually a `Trap` then the backtrace is inserted
            // directly into the `Trap` since there's storage there for it.
            // Otherwise though this represents a host-defined error which isn't
            // using a `Trap` but instead some other condition that was fatal to
            // wasm itself. In that situation the backtrace is inserted as
            // contextual information on error using `error.context(...)` to
            // provide useful information to debug with for the embedder/caller,
            // otherwise the information about what the wasm was doing when the
            // error was generated would be lost.
            wasmtime_runtime::TrapReason::User {
                error,
                needs_backtrace,
            } => {
                debug_assert!(needs_backtrace == backtrace.is_some());
                (error, None)
            }
            wasmtime_runtime::TrapReason::Jit(pc) => {
                let code = store
                    .modules()
                    .lookup_trap_code(pc)
                    .unwrap_or(TrapCode::StackOverflow);
                (Trap::from_env(code).into(), Some(pc))
            }
            wasmtime_runtime::TrapReason::Wasm(trap_code) => {
                (Trap::from_env(trap_code).into(), None)
            }
        };
        match backtrace {
            Some(bt) => {
                let bt = WasmBacktrace::new(store, bt, pc);
                if bt.wasm_trace.is_empty() {
                    error
                } else {
                    error.context(bt)
                }
            }
            None => error,
        }
    }

    /// Panics if `code` is `TrapCode::User`.
    pub(crate) fn from_env(code: TrapCode) -> Self {
        match code {
            TrapCode::StackOverflow => Trap::StackOverflow,
            TrapCode::HeapOutOfBounds => Trap::MemoryOutOfBounds,
            TrapCode::HeapMisaligned => Trap::HeapMisaligned,
            TrapCode::TableOutOfBounds => Trap::TableOutOfBounds,
            TrapCode::IndirectCallToNull => Trap::IndirectCallToNull,
            TrapCode::BadSignature => Trap::BadSignature,
            TrapCode::IntegerOverflow => Trap::IntegerOverflow,
            TrapCode::IntegerDivisionByZero => Trap::IntegerDivisionByZero,
            TrapCode::BadConversionToInteger => Trap::BadConversionToInteger,
            TrapCode::UnreachableCodeReached => Trap::UnreachableCodeReached,
            TrapCode::Interrupt => Trap::Interrupt,
            TrapCode::AlwaysTrapAdapter => Trap::AlwaysTrapAdapter,
        }
    }
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Trap::*;

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
            OutOfFuel => "all fuel consumed by WebAssembly",
        };
        write!(f, "wasm trap: {desc}")
    }
}

impl std::error::Error for Trap {}

/// todo
#[derive(Debug)]
pub struct WasmBacktrace {
    wasm_trace: Vec<FrameInfo>,
    #[allow(dead_code)]
    runtime_trace: wasmtime_runtime::Backtrace,
    hint_wasm_backtrace_details_env: bool,
}

impl WasmBacktrace {
    fn new(
        store: &StoreOpaque,
        runtime_trace: wasmtime_runtime::Backtrace,
        trap_pc: Option<usize>,
    ) -> Self {
        let mut wasm_trace = Vec::<FrameInfo>::with_capacity(runtime_trace.frames().len());
        let mut hint_wasm_backtrace_details_env = false;
        let wasm_backtrace_details_env_used =
            store.engine().config().wasm_backtrace_details_env_used;

        for frame in runtime_trace.frames() {
            debug_assert!(frame.pc() != 0);

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
            let pc_to_lookup = if Some(frame.pc()) == trap_pc {
                frame.pc()
            } else {
                frame.pc() - 1
            };

            // NB: The PC we are looking up _must_ be a Wasm PC since
            // `wasmtime_runtime::Backtrace` only contains Wasm frames.
            //
            // However, consider the case where we have multiple, nested calls
            // across stores (with host code in between, by necessity, since
            // only things in the same store can be linked directly together):
            //
            //     | ...             |
            //     | Host            |  |
            //     +-----------------+  | stack
            //     | Wasm in store A |  | grows
            //     +-----------------+  | down
            //     | Host            |  |
            //     +-----------------+  |
            //     | Wasm in store B |  V
            //     +-----------------+
            //
            // In this scenario, the `wasmtime_runtime::Backtrace` will contain
            // two frames: Wasm in store B followed by Wasm in store A. But
            // `store.modules()` will only have the module information for
            // modules instantiated within this store. Therefore, we use `if let
            // Some(..)` instead of the `unwrap` you might otherwise expect and
            // we ignore frames from modules that were not registered in this
            // store's module registry.
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
            runtime_trace,
            hint_wasm_backtrace_details_env,
        }
    }

    /// Returns a list of function frames in WebAssembly this backtrace
    /// represents.
    pub fn frames(&self) -> &[FrameInfo] {
        self.wasm_trace.as_slice()
    }
}

impl fmt::Display for WasmBacktrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "error while executing at wasm backtrace:")?;

        let mut needs_newline = false;
        for (i, frame) in self.wasm_trace.iter().enumerate() {
            // Avoid putting a trailing newline on the output
            if needs_newline {
                writeln!(f, "")?;
            } else {
                needs_newline = true;
            }
            let name = frame.module_name().unwrap_or("<unknown>");
            write!(f, "  {:>3}: ", i)?;

            if let Some(offset) = frame.module_offset() {
                write!(f, "{:#6x} - ", offset)?;
            }

            let write_raw_func_name = |f: &mut fmt::Formatter<'_>| {
                demangle_function_name_or_index(f, frame.func_name(), frame.func_index() as usize)
            };
            if frame.symbols().is_empty() {
                write!(f, "{}!", name)?;
                write_raw_func_name(f)?;
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
                    if let Some(file) = symbol.file() {
                        writeln!(f, "")?;
                        write!(f, "                    at {}", file)?;
                        if let Some(line) = symbol.line() {
                            write!(f, ":{}", line)?;
                            if let Some(col) = symbol.column() {
                                write!(f, ":{}", col)?;
                            }
                        }
                    }
                }
            }
        }
        if self.hint_wasm_backtrace_details_env {
            write!(f, "\nnote: using the `WASMTIME_BACKTRACE_DETAILS=1` environment variable to may show more debugging information")?;
        }
        Ok(())
    }
}

/// Description of a frame in a backtrace for a [`Trap`] or [`BacktraceContext`].
///
/// Whenever a WebAssembly trap occurs an instance of [`Trap`] is created. Each
/// [`Trap`] has a backtrace of the WebAssembly frames that led to the trap, and
/// each frame is described by this structure.
///
/// [`Trap`]: crate::Trap
/// [`BacktraceContext`]: crate::BacktraceContext
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

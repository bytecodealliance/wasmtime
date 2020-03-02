//! All the runtime support necessary for the wasm to cranelift translation is formalized by the
//! traits `FunctionEnvironment` and `ModuleEnvironment`.
//!
//! There are skeleton implementations of these traits in the `dummy` module, and complete
//! implementations in [Wasmtime].
//!
//! [Wasmtime]: https://github.com/bytecodealliance/wasmtime

use crate::state::{FuncTranslationState, ModuleTranslationState};
use crate::translation_utils::{
    DataIndex, ElemIndex, FuncIndex, Global, GlobalIndex, Memory, MemoryIndex, SignatureIndex,
    Table, TableIndex,
};
use core::convert::From;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_frontend::FunctionBuilder;
use std::boxed::Box;
use thiserror::Error;
use wasmparser::BinaryReaderError;
use wasmparser::Operator;

/// The value of a WebAssembly global variable.
#[derive(Clone, Copy)]
pub enum GlobalVariable {
    /// This is a constant global with a value known at compile time.
    Const(ir::Value),

    /// This is a variable in memory that should be referenced through a `GlobalValue`.
    Memory {
        /// The address of the global variable storage.
        gv: ir::GlobalValue,
        /// An offset to add to the address.
        offset: Offset32,
        /// The global variable's type.
        ty: ir::Type,
    },

    /// This is a global variable that needs to be handled by the environment.
    Custom,
}

/// A WebAssembly translation error.
///
/// When a WebAssembly function can't be translated, one of these error codes will be returned
/// to describe the failure.
#[derive(Error, Debug)]
pub enum WasmError {
    /// The input WebAssembly code is invalid.
    ///
    /// This error code is used by a WebAssembly translator when it encounters invalid WebAssembly
    /// code. This should never happen for validated WebAssembly code.
    #[error("Invalid input WebAssembly code at offset {offset}: {message}")]
    InvalidWebAssembly {
        /// A string describing the validation error.
        message: std::string::String,
        /// The bytecode offset where the error occurred.
        offset: usize,
    },

    /// A feature used by the WebAssembly code is not supported by the embedding environment.
    ///
    /// Embedding environments may have their own limitations and feature restrictions.
    #[error("Unsupported feature: {0}")]
    Unsupported(std::string::String),

    /// An implementation limit was exceeded.
    ///
    /// Cranelift can compile very large and complicated functions, but the [implementation has
    /// limits][limits] that cause compilation to fail when they are exceeded.
    ///
    /// [limits]: https://cranelift.readthedocs.io/en/latest/ir.html#implementation-limits
    #[error("Implementation limit exceeded")]
    ImplLimitExceeded,

    /// Any user-defined error.
    #[error("User error: {0}")]
    User(std::string::String),
}

/// Return an `Err(WasmError::Unsupported(msg))` where `msg` the string built by calling `format!`
/// on the arguments to this macro.
#[macro_export]
macro_rules! wasm_unsupported {
    ($($arg:tt)*) => { $crate::environ::WasmError::Unsupported(format!($($arg)*)) }
}

impl From<BinaryReaderError> for WasmError {
    /// Convert from a `BinaryReaderError` to a `WasmError`.
    fn from(e: BinaryReaderError) -> Self {
        Self::InvalidWebAssembly {
            message: e.message().into(),
            offset: e.offset(),
        }
    }
}

/// A convenient alias for a `Result` that uses `WasmError` as the error type.
pub type WasmResult<T> = Result<T, WasmError>;

/// How to return from functions.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ReturnMode {
    /// Use normal return instructions as needed.
    NormalReturns,
    /// Use a single fallthrough return at the end of the function.
    FallthroughReturn,
}

/// Environment affecting the translation of a WebAssembly.
pub trait TargetEnvironment {
    /// Get the information needed to produce Cranelift IR for the given target.
    fn target_config(&self) -> TargetFrontendConfig;

    /// Get the Cranelift integer type to use for native pointers.
    ///
    /// This returns `I64` for 64-bit architectures and `I32` for 32-bit architectures.
    fn pointer_type(&self) -> ir::Type {
        ir::Type::int(u16::from(self.target_config().pointer_bits())).unwrap()
    }

    /// Get the size of a native pointer, in bytes.
    fn pointer_bytes(&self) -> u8 {
        self.target_config().pointer_bytes()
    }

    /// Get the Cranelift reference type to use for native references.
    ///
    /// This returns `R64` for 64-bit architectures and `R32` for 32-bit architectures.
    fn reference_type(&self) -> ir::Type {
        match self.pointer_type() {
            ir::types::I32 => ir::types::R32,
            ir::types::I64 => ir::types::R64,
            _ => panic!("unsupported pointer type"),
        }
    }
}

/// Environment affecting the translation of a single WebAssembly function.
///
/// A `FuncEnvironment` trait object is required to translate a WebAssembly function to Cranelift
/// IR. The function environment provides information about the WebAssembly module as well as the
/// runtime environment.
pub trait FuncEnvironment: TargetEnvironment {
    /// Is the given parameter of the given function a wasm-level parameter, as opposed to a hidden
    /// parameter added for use by the implementation?
    fn is_wasm_parameter(&self, signature: &ir::Signature, index: usize) -> bool {
        signature.params[index].purpose == ir::ArgumentPurpose::Normal
    }

    /// Is the given return of the given function a wasm-level parameter, as
    /// opposed to a hidden parameter added for use by the implementation?
    fn is_wasm_return(&self, signature: &ir::Signature, index: usize) -> bool {
        signature.returns[index].purpose == ir::ArgumentPurpose::Normal
    }

    /// Should the code be structured to use a single `fallthrough_return` instruction at the end
    /// of the function body, rather than `return` instructions as needed? This is used by VMs
    /// to append custom epilogues.
    fn return_mode(&self) -> ReturnMode {
        ReturnMode::NormalReturns
    }

    /// Set up the necessary preamble definitions in `func` to access the global variable
    /// identified by `index`.
    ///
    /// The index space covers both imported globals and globals defined by the module.
    ///
    /// Return the global variable reference that should be used to access the global and the
    /// WebAssembly type of the global.
    fn make_global(
        &mut self,
        func: &mut ir::Function,
        index: GlobalIndex,
    ) -> WasmResult<GlobalVariable>;

    /// Set up the necessary preamble definitions in `func` to access the linear memory identified
    /// by `index`.
    ///
    /// The index space covers both imported and locally declared memories.
    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> WasmResult<ir::Heap>;

    /// Set up the necessary preamble definitions in `func` to access the table identified
    /// by `index`.
    ///
    /// The index space covers both imported and locally declared tables.
    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> WasmResult<ir::Table>;

    /// Set up a signature definition in the preamble of `func` that can be used for an indirect
    /// call with signature `index`.
    ///
    /// The signature may contain additional arguments needed for an indirect call, but the
    /// arguments marked as `ArgumentPurpose::Normal` must correspond to the WebAssembly signature
    /// arguments.
    ///
    /// The signature will only be used for indirect calls, even if the module has direct function
    /// calls with the same WebAssembly type.
    fn make_indirect_sig(
        &mut self,
        func: &mut ir::Function,
        index: SignatureIndex,
    ) -> WasmResult<ir::SigRef>;

    /// Set up an external function definition in the preamble of `func` that can be used to
    /// directly call the function `index`.
    ///
    /// The index space covers both imported functions and functions defined in the current module.
    ///
    /// The function's signature may contain additional arguments needed for a direct call, but the
    /// arguments marked as `ArgumentPurpose::Normal` must correspond to the WebAssembly signature
    /// arguments.
    ///
    /// The function's signature will only be used for direct calls, even if the module has
    /// indirect calls with the same WebAssembly type.
    fn make_direct_func(
        &mut self,
        func: &mut ir::Function,
        index: FuncIndex,
    ) -> WasmResult<ir::FuncRef>;

    /// Translate a `call_indirect` WebAssembly instruction at `pos`.
    ///
    /// Insert instructions at `pos` for an indirect call to the function `callee` in the table
    /// `table_index` with WebAssembly signature `sig_index`. The `callee` value will have type
    /// `i32`.
    ///
    /// The signature `sig_ref` was previously created by `make_indirect_sig()`.
    ///
    /// Return the call instruction whose results are the WebAssembly return values.
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]
    fn translate_call_indirect(
        &mut self,
        pos: FuncCursor,
        table_index: TableIndex,
        table: ir::Table,
        sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst>;

    /// Translate a `call` WebAssembly instruction at `pos`.
    ///
    /// Insert instructions at `pos` for a direct call to the function `callee_index`.
    ///
    /// The function reference `callee` was previously created by `make_direct_func()`.
    ///
    /// Return the call instruction whose results are the WebAssembly return values.
    fn translate_call(
        &mut self,
        mut pos: FuncCursor,
        _callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        Ok(pos.ins().call(callee, call_args))
    }

    /// Translate a `memory.grow` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to grow, and `heap` is the heap reference
    /// returned by `make_heap` for the same index.
    ///
    /// The `val` value is the requested memory size in pages.
    ///
    /// Returns the old size (in pages) of the memory.
    fn translate_memory_grow(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
        val: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translates a `memory.size` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to query, and `heap` is the heap reference
    /// returned by `make_heap` for the same index.
    ///
    /// Returns the size in pages of the memory.
    fn translate_memory_size(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
    ) -> WasmResult<ir::Value>;

    /// Translate a `memory.copy` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to query, and `heap` is the heap reference
    /// returned by `make_heap` for the same index.
    fn translate_memory_copy(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `memory.fill` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to query, and `heap` is the heap reference
    /// returned by `make_heap` for the same index.
    fn translate_memory_fill(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `memory.init` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to query, and `heap` is the heap reference
    /// returned by `make_heap` for the same index. `seg_index` is the index of the segment to copy
    /// from.
    #[allow(clippy::too_many_arguments)]
    fn translate_memory_init(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
        seg_index: u32,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `data.drop` WebAssembly instruction.
    fn translate_data_drop(&mut self, pos: FuncCursor, seg_index: u32) -> WasmResult<()>;

    /// Translate a `table.size` WebAssembly instruction.
    fn translate_table_size(
        &mut self,
        pos: FuncCursor,
        index: TableIndex,
        table: ir::Table,
    ) -> WasmResult<ir::Value>;

    /// Translate a `table.grow` WebAssembly instruction.
    fn translate_table_grow(
        &mut self,
        pos: FuncCursor,
        table_index: u32,
        delta: ir::Value,
        init_value: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate a `table.get` WebAssembly instruction.
    fn translate_table_get(
        &mut self,
        pos: FuncCursor,
        table_index: u32,
        index: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate a `table.set` WebAssembly instruction.
    fn translate_table_set(
        &mut self,
        pos: FuncCursor,
        table_index: u32,
        value: ir::Value,
        index: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `table.copy` WebAssembly instruction.
    #[allow(clippy::too_many_arguments)]
    fn translate_table_copy(
        &mut self,
        pos: FuncCursor,
        dst_table_index: TableIndex,
        dst_table: ir::Table,
        src_table_index: TableIndex,
        src_table: ir::Table,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `table.fill` WebAssembly instruction.
    fn translate_table_fill(
        &mut self,
        pos: FuncCursor,
        table_index: u32,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `table.init` WebAssembly instruction.
    #[allow(clippy::too_many_arguments)]
    fn translate_table_init(
        &mut self,
        pos: FuncCursor,
        seg_index: u32,
        table_index: TableIndex,
        table: ir::Table,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `elem.drop` WebAssembly instruction.
    fn translate_elem_drop(&mut self, pos: FuncCursor, seg_index: u32) -> WasmResult<()>;

    /// Translate a `ref.func` WebAssembly instruction.
    fn translate_ref_func(&mut self, pos: FuncCursor, func_index: u32) -> WasmResult<ir::Value>;

    /// Translate a `global.get` WebAssembly instruction at `pos` for a global
    /// that is custom.
    fn translate_custom_global_get(
        &mut self,
        pos: FuncCursor,
        global_index: GlobalIndex,
    ) -> WasmResult<ir::Value>;

    /// Translate a `global.set` WebAssembly instruction at `pos` for a global
    /// that is custom.
    fn translate_custom_global_set(
        &mut self,
        pos: FuncCursor,
        global_index: GlobalIndex,
        val: ir::Value,
    ) -> WasmResult<()>;

    /// Emit code at the beginning of every wasm loop.
    ///
    /// This can be used to insert explicit interrupt or safepoint checking at
    /// the beginnings of loops.
    fn translate_loop_header(&mut self, _pos: FuncCursor) -> WasmResult<()> {
        // By default, don't emit anything.
        Ok(())
    }

    /// Optional callback for the `FunctionEnvironment` performing this translation to maintain
    /// internal state or prepare custom state for the operator to translate
    fn before_translate_operator(
        &mut self,
        _op: &Operator,
        _builder: &mut FunctionBuilder,
        _state: &FuncTranslationState,
    ) -> WasmResult<()> {
        Ok(())
    }

    /// Optional callback for the `FunctionEnvironment` performing this translation to maintain
    /// internal state or finalize custom state for the operator that was translated
    fn after_translate_operator(
        &mut self,
        _op: &Operator,
        _builder: &mut FunctionBuilder,
        _state: &FuncTranslationState,
    ) -> WasmResult<()> {
        Ok(())
    }
}

/// An object satisfying the `ModuleEnvironment` trait can be passed as argument to the
/// [`translate_module`](fn.translate_module.html) function. These methods should not be called
/// by the user, they are only for `cranelift-wasm` internal use.
pub trait ModuleEnvironment<'data>: TargetEnvironment {
    /// Provides the number of signatures up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_signatures(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a function signature to the environment.
    fn declare_signature(&mut self, sig: ir::Signature) -> WasmResult<()>;

    /// Provides the number of imports up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_imports(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a function import to the environment.
    fn declare_func_import(
        &mut self,
        sig_index: SignatureIndex,
        module: &'data str,
        field: &'data str,
    ) -> WasmResult<()>;

    /// Declares a table import to the environment.
    fn declare_table_import(
        &mut self,
        table: Table,
        module: &'data str,
        field: &'data str,
    ) -> WasmResult<()>;

    /// Declares a memory import to the environment.
    fn declare_memory_import(
        &mut self,
        memory: Memory,
        module: &'data str,
        field: &'data str,
    ) -> WasmResult<()>;

    /// Declares a global import to the environment.
    fn declare_global_import(
        &mut self,
        global: Global,
        module: &'data str,
        field: &'data str,
    ) -> WasmResult<()>;

    /// Notifies the implementation that all imports have been declared.
    fn finish_imports(&mut self) -> WasmResult<()> {
        Ok(())
    }

    /// Provides the number of defined functions up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_func_types(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares the type (signature) of a local function in the module.
    fn declare_func_type(&mut self, sig_index: SignatureIndex) -> WasmResult<()>;

    /// Provides the number of defined tables up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_tables(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a table to the environment.
    fn declare_table(&mut self, table: Table) -> WasmResult<()>;

    /// Provides the number of defined memories up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_memories(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a memory to the environment
    fn declare_memory(&mut self, memory: Memory) -> WasmResult<()>;

    /// Provides the number of defined globals up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_globals(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a global to the environment.
    fn declare_global(&mut self, global: Global) -> WasmResult<()>;

    /// Provides the number of exports up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_exports(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a function export to the environment.
    fn declare_func_export(&mut self, func_index: FuncIndex, name: &'data str) -> WasmResult<()>;

    /// Declares a table export to the environment.
    fn declare_table_export(&mut self, table_index: TableIndex, name: &'data str)
        -> WasmResult<()>;

    /// Declares a memory export to the environment.
    fn declare_memory_export(
        &mut self,
        memory_index: MemoryIndex,
        name: &'data str,
    ) -> WasmResult<()>;

    /// Declares a global export to the environment.
    fn declare_global_export(
        &mut self,
        global_index: GlobalIndex,
        name: &'data str,
    ) -> WasmResult<()>;

    /// Notifies the implementation that all exports have been declared.
    fn finish_exports(&mut self) -> WasmResult<()> {
        Ok(())
    }

    /// Declares the optional start function.
    fn declare_start_func(&mut self, index: FuncIndex) -> WasmResult<()>;

    /// Provides the number of element initializers up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_table_elements(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Fills a declared table with references to functions in the module.
    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        elements: Box<[FuncIndex]>,
    ) -> WasmResult<()>;

    /// Declare a passive element segment.
    fn declare_passive_element(
        &mut self,
        index: ElemIndex,
        elements: Box<[FuncIndex]>,
    ) -> WasmResult<()>;

    /// Provides the number of passive data segments up front.
    ///
    /// By default this does nothing, but implementations may use this to
    /// pre-allocate memory if desired.
    fn reserve_passive_data(&mut self, count: u32) -> WasmResult<()> {
        let _ = count;
        Ok(())
    }

    /// Declare a passive data segment.
    fn declare_passive_data(&mut self, data_index: DataIndex, data: &'data [u8]) -> WasmResult<()>;

    /// Provides the contents of a function body.
    ///
    /// Note there's no `reserve_function_bodies` function because the number of
    /// functions is already provided by `reserve_func_types`.
    fn define_function_body(
        &mut self,
        module_translation_state: &ModuleTranslationState,
        body_bytes: &'data [u8],
        body_offset: usize,
    ) -> WasmResult<()>;

    /// Provides the number of data initializers up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_data_initializers(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Fills a declared memory with bytes at module instantiation.
    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        data: &'data [u8],
    ) -> WasmResult<()>;

    /// Declares the name of a function to the environment.
    ///
    /// By default this does nothing, but implementations can use this to read
    /// the function name subsection of the custom name section if desired.
    fn declare_func_name(&mut self, _func_index: FuncIndex, _name: &'data str) -> WasmResult<()> {
        Ok(())
    }

    /// Indicates that a custom section has been found in the wasm file
    fn custom_section(&mut self, _name: &'data str, _data: &'data [u8]) -> WasmResult<()> {
        Ok(())
    }
}

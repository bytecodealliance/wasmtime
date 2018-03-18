//! All the runtime support necessary for the wasm to cretonne translation is formalized by the
//! traits `FunctionEnvironment` and `ModuleEnvironment`.
use cretonne::ir::{self, InstBuilder};
use cretonne::cursor::FuncCursor;
use cretonne::settings::Flags;
use translation_utils::{SignatureIndex, FunctionIndex, TableIndex, GlobalIndex, MemoryIndex,
                        Global, Table, Memory};

/// The value of a WebAssembly global variable.
#[derive(Clone, Copy)]
pub enum GlobalValue {
    /// This is a constant global with a value known at compile time.
    Const(ir::Value),

    /// This is a variable in memory that should be referenced as a `GlobalVar`.
    Memory {
        /// Which global variable should be referenced.
        gv: ir::GlobalVar,
        /// The global variable's type.
        ty: ir::Type,
    },
}

/// Environment affecting the translation of a single WebAssembly function.
///
/// A `FuncEnvironment` trait object is required to translate a WebAssembly function to Cretonne
/// IL. The function environment provides information about the WebAssembly module as well as the
/// runtime environment.
pub trait FuncEnvironment {
    /// Get the flags for the current compilation.
    fn flags(&self) -> &Flags;

    /// Get the Cretonne integer type to use for native pointers.
    ///
    /// This returns `I64` for 64-bit architectures and `I32` for 32-bit architectures.
    fn native_pointer(&self) -> ir::Type {
        if self.flags().is_64bit() {
            ir::types::I64
        } else {
            ir::types::I32
        }
    }

    /// Set up the necessary preamble definitions in `func` to access the global variable
    /// identified by `index`.
    ///
    /// The index space covers both imported globals and globals defined by the module.
    ///
    /// Return the global variable reference that should be used to access the global and the
    /// WebAssembly type of the global.
    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalValue;

    /// Set up the necessary preamble definitions in `func` to access the linear memory identified
    /// by `index`.
    ///
    /// The index space covers both imported and locally declared memories.
    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> ir::Heap;

    /// Set up a signature definition in the preamble of `func` that can be used for an indirect
    /// call with signature `index`.
    ///
    /// The signature may contain additional arguments needed for an indirect call, but the
    /// arguments marked as `ArgumentPurpose::Normal` must correspond to the WebAssembly signature
    /// arguments.
    ///
    /// The signature will only be used for indirect calls, even if the module has direct function
    /// calls with the same WebAssembly type.
    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef;

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
    fn make_direct_func(&mut self, func: &mut ir::Function, index: FunctionIndex) -> ir::FuncRef;

    /// Translate a `call_indirect` WebAssembly instruction at `pos`.
    ///
    /// Insert instructions at `pos` for an indirect call to the function `callee` in the table
    /// `table_index` with WebAssembly signature `sig_index`. The `callee` value will have type
    /// `i32`.
    ///
    /// The signature `sig_ref` was previously created by `make_indirect_sig()`.
    ///
    /// Return the call instruction whose results are the WebAssembly return values.
    fn translate_call_indirect(
        &mut self,
        pos: FuncCursor,
        table_index: TableIndex,
        sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> ir::Inst;

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
        _callee_index: FunctionIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> ir::Inst {
        pos.ins().call(callee, call_args)
    }

    /// Translate a `grow_memory` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to grow, and `heap` is the heap reference
    /// returned by `make_heap` for the same index.
    ///
    /// The `val` value is the requested memory size in pages.
    ///
    /// Returns the old size (in pages) of the memory.
    fn translate_grow_memory(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
        val: ir::Value,
    ) -> ir::Value;

    /// Translates a `current_memory` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to query, and `heap` is the heap reference
    /// returned by `make_heap` for the same index.
    ///
    /// Returns the size in pages of the memory.
    fn translate_current_memory(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
    ) -> ir::Value;

    /// Emit code at the beginning of every wasm loop.
    ///
    /// This can be used to insert explicit interrupt or safepoint checking at
    /// the beginnings of loops.
    fn translate_loop_header(&mut self, _pos: FuncCursor) {
        // By default, don't emit anything.
    }
}

/// An object satisfying the `ModuleEnvironment` trait can be passed as argument to the
/// [`translate_module`](fn.translate_module.html) function. These methods should not be called
/// by the user, they are only for `cretonne-wasm` internal use.
pub trait ModuleEnvironment<'data> {
    /// Return the name for the given function index.
    fn get_func_name(&self, func_index: FunctionIndex) -> ir::ExternalName;

    /// Declares a function signature to the environment.
    fn declare_signature(&mut self, sig: &ir::Signature);

    /// Return the signature with the given index.
    fn get_signature(&self, sig_index: SignatureIndex) -> &ir::Signature;

    /// Declares a function import to the environment.
    fn declare_func_import(
        &mut self,
        sig_index: SignatureIndex,
        module: &'data str,
        field: &'data str,
    );

    /// Return the number of imported funcs.
    fn get_num_func_imports(&self) -> usize;

    /// Declares the type (signature) of a local function in the module.
    fn declare_func_type(&mut self, sig_index: SignatureIndex);

    /// Return the signature index for the given function index.
    fn get_func_type(&self, func_index: FunctionIndex) -> SignatureIndex;

    /// Declares a global to the environment.
    fn declare_global(&mut self, global: Global);

    /// Return the global for the given global index.
    fn get_global(&self, global_index: GlobalIndex) -> &Global;

    /// Declares a table to the environment.
    fn declare_table(&mut self, table: Table);
    /// Fills a declared table with references to functions in the module.
    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        elements: Vec<FunctionIndex>,
    );
    /// Declares a memory to the environment
    fn declare_memory(&mut self, memory: Memory);
    /// Fills a declared memory with bytes at module instantiation.
    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        data: &'data [u8],
    );

    /// Declares a function export to the environment.
    fn declare_func_export(&mut self, func_index: FunctionIndex, name: &'data str);
    /// Declares a table export to the environment.
    fn declare_table_export(&mut self, table_index: TableIndex, name: &'data str);
    /// Declares a memory export to the environment.
    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &'data str);
    /// Declares a global export to the environment.
    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &'data str);

    /// Declares a start function.
    fn declare_start_func(&mut self, index: FunctionIndex);

    /// Provides the contents of a function body.
    fn define_function_body(&mut self, body_bytes: &'data [u8]) -> Result<(), String>;
}

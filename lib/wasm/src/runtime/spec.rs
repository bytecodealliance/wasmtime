//! All the runtime support necessary for the wasm to cretonne translation is formalized by the
//! trait `WasmRuntime`.
use cton_frontend::FunctionBuilder;
use cretonne::ir::{self, Value, SigRef};
use translation_utils::{Local, FunctionIndex, TableIndex, GlobalIndex, MemoryIndex, Global, Table,
                        Memory};

/// The value of a WebAssembly global variable.
#[derive(Clone, Copy)]
pub enum GlobalValue {
    /// This is a constant global with a value known at compile time.
    Const(ir::Value),

    /// This is a variable in memory that should be referenced as a `GlobalVar`.
    Memory { gv: ir::GlobalVar, ty: ir::Type },
}

/// Environment affecting the translation of a single WebAssembly function.
///
/// A `FuncEnvironment` trait object is required to translate a WebAssembly function to Cretonne
/// IL. The function environment provides information about the WebAssembly module as well as the
/// runtime environment.
pub trait FuncEnvironment {
    /// Get the Cretonne integer type to use for native pointers.
    ///
    /// This should be `I64` for 64-bit architectures and `I32` for 32-bit architectures.
    fn native_pointer(&self) -> ir::Type;

    /// Set up the necessary preamble definitions in `func` to access the global variable
    /// identified by `index`.
    ///
    /// The index space covers both imported globals and globals defined by the module.
    ///
    /// Return the global variable reference that should be used to access the global and the
    /// WebAssembly type of the global.
    fn make_global(&self, func: &mut ir::Function, index: GlobalIndex) -> GlobalValue;

    /// Set up the necessary preamble definitions in `func` to access the linear memory identified
    /// by `index`.
    ///
    /// The index space covers both imported and locally declared memories.
    fn make_heap(&self, func: &mut ir::Function, index: MemoryIndex) -> ir::Heap;
}

/// An object satisfyng the `WasmRuntime` trait can be passed as argument to the
/// [`translate_module`](fn.translate_module.html) function. These methods should not be called
/// by the user, they are only for the `wasm2cretonne` internal use.
pub trait WasmRuntime: FuncEnvironment {
    /// Declares a global to the runtime.
    fn declare_global(&mut self, global: Global);
    /// Declares a table to the runtime.
    fn declare_table(&mut self, table: Table);
    /// Fills a declared table with references to functions in the module.
    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        offset: usize,
        elements: &[FunctionIndex],
    );
    /// Declares a memory to the runtime
    fn declare_memory(&mut self, memory: Memory);
    /// Fills a declared memory with bytes at module instantiation.
    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        offset: usize,
        data: &[u8],
    ) -> Result<(), String>;
    /// Call this function after having declared all the runtime elements but prior to the
    /// function body translation.
    fn begin_translation(&mut self);
    /// Call this function between each function body translation.
    fn next_function(&mut self);
    /// Translates a `grow_memory` wasm instruction. Returns the old size (in pages) of the memory.
    fn translate_grow_memory(&mut self, builder: &mut FunctionBuilder<Local>, val: Value) -> Value;
    /// Translates a `current_memory` wasm instruction. Returns the size in pages of the memory.
    fn translate_current_memory(&mut self, builder: &mut FunctionBuilder<Local>) -> Value;
    /// Translates a `call_indirect` wasm instruction. It involves looking up the value contained
    /// it the table at location `index_val` and calling the corresponding function.
    fn translate_call_indirect<'a>(
        &self,
        builder: &'a mut FunctionBuilder<Local>,
        sig_ref: SigRef,
        index_val: Value,
        call_args: &[Value],
    ) -> &'a [Value];
}

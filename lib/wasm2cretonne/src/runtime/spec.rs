//! All the runtime support necessary for the wasm to cretonne translation is formalized by the
//! trait `WasmRuntime`.
use cton_frontend::FunctionBuilder;
use cretonne::ir::{Value, SigRef};
use translation_utils::{Local, FunctionIndex, TableIndex, GlobalIndex, MemoryIndex, Global, Table,
                        Memory};

/// An object satisfyng the `WasmRuntime` trait can be passed as argument to the
/// [`translate_module`](fn.translate_module.html) function. These methods should not be called
/// by the user, they are only for the `wasm2cretonne` internal use.
pub trait WasmRuntime {
    /// Declares a global to the runtime.
    fn declare_global(&mut self, global: Global);
    /// Declares a table to the runtime.
    fn declare_table(&mut self, table: Table);
    /// Fills a declared table with references to functions in the module.
    fn declare_table_elements(&mut self,
                              table_index: TableIndex,
                              offset: usize,
                              elements: &[FunctionIndex]);
    /// Declares a memory to the runtime
    fn declare_memory(&mut self, memory: Memory);
    /// Fills a declared memory with bytes at module instantiation.
    fn declare_data_initialization(&mut self,
                                   memory_index: MemoryIndex,
                                   offset: usize,
                                   data: &[u8])
                                   -> Result<(), String>;
    /// Call this function after having declared all the runtime elements but prior to the
    /// function body translation.
    fn begin_translation(&mut self);
    /// Call this function between each function body translation.
    fn next_function(&mut self);
    /// Translates a `get_global` wasm instruction.
    fn translate_get_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex)
                            -> Value;
    /// Translates a `set_global` wasm instruction.
    fn translate_set_global(&self,
                            builder: &mut FunctionBuilder<Local>,
                            global_index: GlobalIndex,
                            val: Value);
    /// Translates a `grow_memory` wasm instruction. Returns the old size (in pages) of the memory.
    fn translate_grow_memory(&mut self, builder: &mut FunctionBuilder<Local>, val: Value) -> Value;
    /// Translates a `current_memory` wasm instruction. Returns the size in pages of the memory.
    fn translate_current_memory(&mut self, builder: &mut FunctionBuilder<Local>) -> Value;
    /// Returns the base address of a wasm memory as a Cretonne `Value`.
    fn translate_memory_base_address(&self,
                                     builder: &mut FunctionBuilder<Local>,
                                     index: MemoryIndex)
                                     -> Value;
    /// Translates a `call_indirect` wasm instruction. It involves looking up the value contained
    /// it the table at location `index_val` and calling the corresponding function.
    fn translate_call_indirect<'a>(&self,
                                   builder: &'a mut FunctionBuilder<Local>,
                                   sig_ref: SigRef,
                                   index_val: Value,
                                   call_args: &[Value])
                                   -> &'a [Value];
}

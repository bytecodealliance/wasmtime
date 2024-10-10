//! All the runtime support necessary for the wasm to cranelift translation is formalized by the
//! traits `FunctionEnvironment` and `ModuleEnvironment`.
//!
//! There are skeleton implementations of these traits in the `dummy` module, and complete
//! implementations in [Wasmtime].
//!
//! [Wasmtime]: https://github.com/bytecodealliance/wasmtime

use crate::translate::state::FuncTranslationState;
use crate::translate::{Heap, HeapData};
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, Type};
use cranelift_codegen::isa::{TargetFrontendConfig, TargetIsa};
use cranelift_entity::PrimaryMap;
use cranelift_frontend::FunctionBuilder;
use smallvec::SmallVec;
use wasmparser::{Operator, WasmFeatures};
use wasmtime_environ::{
    DataIndex, ElemIndex, FuncIndex, GlobalIndex, MemoryIndex, TableIndex, TypeConvert, TypeIndex,
    WasmHeapType, WasmRefType, WasmResult,
};

/// The value of a WebAssembly global variable.
#[derive(Clone, Copy)]
pub enum GlobalVariable {
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

/// Environment affecting the translation of a WebAssembly.
pub trait TargetEnvironment: TypeConvert {
    /// Get the information needed to produce Cranelift IR for the given target.
    fn target_config(&self) -> TargetFrontendConfig;

    /// Whether to enable Spectre mitigations for heap accesses.
    fn heap_access_spectre_mitigation(&self) -> bool;

    /// Whether to add proof-carrying-code facts to verify memory accesses.
    fn proof_carrying_code(&self) -> bool;

    /// Get the Cranelift integer type to use for native pointers.
    ///
    /// This returns `I64` for 64-bit architectures and `I32` for 32-bit architectures.
    fn pointer_type(&self) -> ir::Type {
        ir::Type::int(u16::from(self.target_config().pointer_bits())).unwrap()
    }

    /// Get the Cranelift reference type to use for the given Wasm reference
    /// type.
    ///
    /// Returns a pair of the CLIF reference type to use and a boolean that
    /// describes whether the value should be included in GC stack maps or not.
    fn reference_type(&self, ty: WasmHeapType) -> (ir::Type, bool);
}

/// A smallvec that holds the IR values for a struct's fields.
pub type StructFieldsVec = SmallVec<[ir::Value; 4]>;

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

    /// Does the given parameter require inclusion in stack maps?
    fn param_needs_stack_map(&self, signature: &ir::Signature, index: usize) -> bool;

    /// Does the given result require inclusion in stack maps?
    fn sig_ref_result_needs_stack_map(&self, sig_ref: ir::SigRef, index: usize) -> bool;

    /// Does the given result require inclusion in stack maps?
    fn func_ref_result_needs_stack_map(
        &self,
        func: &ir::Function,
        func_ref: ir::FuncRef,
        index: usize,
    ) -> bool;

    /// Is the given return of the given function a wasm-level parameter, as
    /// opposed to a hidden parameter added for use by the implementation?
    fn is_wasm_return(&self, signature: &ir::Signature, index: usize) -> bool {
        signature.returns[index].purpose == ir::ArgumentPurpose::Normal
    }

    /// Called after the locals for a function have been parsed, and the number
    /// of variables defined by this function is provided.
    fn after_locals(&mut self, num_locals_defined: usize) {
        let _ = num_locals_defined;
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

    /// Get the heaps for this function environment.
    ///
    /// The returned map should provide heap format details (encoded in
    /// `HeapData`) for each `Heap` that was previously returned by
    /// `make_heap()`. The translator will first call make_heap for each Wasm
    /// memory, and then later when translating code, will invoke `heaps()` to
    /// learn how to access the environment's implementation of each memory.
    fn heaps(&self) -> &PrimaryMap<Heap, HeapData>;

    /// Set up the necessary preamble definitions in `func` to access the linear memory identified
    /// by `index`.
    ///
    /// The index space covers both imported and locally declared memories.
    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> WasmResult<Heap>;

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
        index: TypeIndex,
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

    /// Translate a `call` WebAssembly instruction at `pos`.
    ///
    /// Insert instructions at `pos` for a direct call to the function `callee_index`.
    ///
    /// The function reference `callee` was previously created by `make_direct_func()`.
    ///
    /// Return the call instruction whose results are the WebAssembly return values.
    fn translate_call(
        &mut self,
        builder: &mut FunctionBuilder,
        _callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        Ok(builder.ins().call(callee, call_args))
    }

    /// Translate a `call_indirect` WebAssembly instruction at `pos`.
    ///
    /// Insert instructions at `pos` for an indirect call to the function `callee` in the table
    /// `table_index` with WebAssembly signature `sig_index`. The `callee` value will have type
    /// `i32`.
    ///
    /// The signature `sig_ref` was previously created by `make_indirect_sig()`.
    ///
    /// Return the call instruction whose results are the WebAssembly return values.
    /// Returns `None` if this statically traps instead of creating a call
    /// instruction.
    fn translate_call_indirect(
        &mut self,
        builder: &mut FunctionBuilder,
        features: &WasmFeatures,
        table_index: TableIndex,
        sig_index: TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<Option<ir::Inst>>;

    /// Translate a `return_call` WebAssembly instruction at the builder's
    /// current position.
    ///
    /// Insert instructions at the builder's current position for a direct tail
    /// call to the function `callee_index`.
    ///
    /// The function reference `callee` was previously created by `make_direct_func()`.
    ///
    /// Return the call instruction whose results are the WebAssembly return values.
    fn translate_return_call(
        &mut self,
        builder: &mut FunctionBuilder,
        _callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<()> {
        builder.ins().return_call(callee, call_args);
        Ok(())
    }

    /// Translate a `return_call_indirect` WebAssembly instruction at the
    /// builder's current position.
    ///
    /// Insert instructions at the builder's current position for an indirect
    /// tail call to the function `callee` in the table `table_index` with
    /// WebAssembly signature `sig_index`. The `callee` value will have type
    /// `i32`.
    ///
    /// The signature `sig_ref` was previously created by `make_indirect_sig()`.
    fn translate_return_call_indirect(
        &mut self,
        builder: &mut FunctionBuilder,
        features: &WasmFeatures,
        table_index: TableIndex,
        sig_index: TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<()>;

    /// Translate a `return_call_ref` WebAssembly instruction at the builder's
    /// given position.
    ///
    /// Insert instructions at the builder's current position for an indirect
    /// tail call to the function `callee`. The `callee` value will be a Wasm
    /// funcref that may need to be translated to a native function address
    /// depending on your implementation of this trait.
    ///
    /// The signature `sig_ref` was previously created by `make_indirect_sig()`.
    fn translate_return_call_ref(
        &mut self,
        builder: &mut FunctionBuilder,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<()>;

    /// Translate a `call_ref` WebAssembly instruction at the builder's current
    /// position.
    ///
    /// Insert instructions at the builder's current position for an indirect
    /// call to the function `callee`. The `callee` value will be a Wasm funcref
    /// that may need to be translated to a native function address depending on
    /// your implementation of this trait.
    ///
    /// The signature `sig_ref` was previously created by `make_indirect_sig()`.
    ///
    /// Return the call instruction whose results are the WebAssembly return values.
    fn translate_call_ref(
        &mut self,
        builder: &mut FunctionBuilder,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst>;

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
        heap: Heap,
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
        heap: Heap,
    ) -> WasmResult<ir::Value>;

    /// Translate a `memory.copy` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to query, and `heap` is the heap reference
    /// returned by `make_heap` for the same index.
    fn translate_memory_copy(
        &mut self,
        pos: FuncCursor,
        src_index: MemoryIndex,
        src_heap: Heap,
        dst_index: MemoryIndex,
        dst_heap: Heap,
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
        heap: Heap,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `memory.init` WebAssembly instruction.
    ///
    /// The `index` provided identifies the linear memory to query, and `heap` is the heap reference
    /// returned by `make_heap` for the same index. `seg_index` is the index of the segment to copy
    /// from.
    fn translate_memory_init(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: Heap,
        seg_index: u32,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `data.drop` WebAssembly instruction.
    fn translate_data_drop(&mut self, pos: FuncCursor, seg_index: u32) -> WasmResult<()>;

    /// Translate a `table.size` WebAssembly instruction.
    fn translate_table_size(&mut self, pos: FuncCursor, index: TableIndex)
        -> WasmResult<ir::Value>;

    /// Translate a `table.grow` WebAssembly instruction.
    fn translate_table_grow(
        &mut self,
        pos: FuncCursor,
        table_index: TableIndex,
        delta: ir::Value,
        init_value: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate a `table.get` WebAssembly instruction.
    fn translate_table_get(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        index: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate a `table.set` WebAssembly instruction.
    fn translate_table_set(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        value: ir::Value,
        index: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `table.copy` WebAssembly instruction.
    fn translate_table_copy(
        &mut self,
        pos: FuncCursor,
        dst_table_index: TableIndex,
        src_table_index: TableIndex,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `table.fill` WebAssembly instruction.
    fn translate_table_fill(
        &mut self,
        pos: FuncCursor,
        table_index: TableIndex,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `table.init` WebAssembly instruction.
    fn translate_table_init(
        &mut self,
        pos: FuncCursor,
        seg_index: u32,
        table_index: TableIndex,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `elem.drop` WebAssembly instruction.
    fn translate_elem_drop(&mut self, pos: FuncCursor, seg_index: u32) -> WasmResult<()>;

    /// Translate a `ref.null T` WebAssembly instruction.
    fn translate_ref_null(&mut self, pos: FuncCursor, ty: WasmHeapType) -> WasmResult<ir::Value>;

    /// Translate a `ref.is_null` WebAssembly instruction.
    fn translate_ref_is_null(&mut self, pos: FuncCursor, value: ir::Value)
        -> WasmResult<ir::Value>;

    /// Translate a `ref.func` WebAssembly instruction.
    fn translate_ref_func(
        &mut self,
        pos: FuncCursor,
        func_index: FuncIndex,
    ) -> WasmResult<ir::Value>;

    /// Translate a `global.get` WebAssembly instruction at `pos` for a global
    /// that is custom.
    fn translate_custom_global_get(
        &mut self,
        builder: &mut FunctionBuilder,
        global_index: GlobalIndex,
    ) -> WasmResult<ir::Value>;

    /// Translate a `global.set` WebAssembly instruction at `pos` for a global
    /// that is custom.
    fn translate_custom_global_set(
        &mut self,
        builder: &mut FunctionBuilder,
        global_index: GlobalIndex,
        val: ir::Value,
    ) -> WasmResult<()>;

    /// Translate an `i32.atomic.wait` or `i64.atomic.wait` WebAssembly instruction.
    /// The `index` provided identifies the linear memory containing the value
    /// to wait on, and `heap` is the heap reference returned by `make_heap`
    /// for the same index.  Whether the waited-on value is 32- or 64-bit can be
    /// determined by examining the type of `expected`, which must be only I32 or I64.
    ///
    /// Note that the `addr` here is the host linear memory address rather
    /// than a relative wasm linear memory address. The type of this value is
    /// the same as the host's pointer.
    ///
    /// Returns an i32, which is negative if the helper call failed.
    fn translate_atomic_wait(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: Heap,
        addr: ir::Value,
        expected: ir::Value,
        timeout: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `atomic.notify` WebAssembly instruction.
    /// The `index` provided identifies the linear memory containing the value
    /// to wait on, and `heap` is the heap reference returned by `make_heap`
    /// for the same index.
    ///
    /// Note that the `addr` here is the host linear memory address rather
    /// than a relative wasm linear memory address. The type of this value is
    /// the same as the host's pointer.
    ///
    /// Returns an i64, which is negative if the helper call failed.
    fn translate_atomic_notify(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: Heap,
        addr: ir::Value,
        count: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `i32` value into an `i31ref`.
    fn translate_ref_i31(&mut self, pos: FuncCursor, val: ir::Value) -> WasmResult<ir::Value>;

    /// Sign-extend an `i31ref` into an `i32`.
    fn translate_i31_get_s(
        &mut self,
        pos: &mut FunctionBuilder,
        i31ref: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Zero-extend an `i31ref` into an `i32`.
    fn translate_i31_get_u(
        &mut self,
        pos: &mut FunctionBuilder,
        i31ref: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Get the number of fields in a struct type.
    fn struct_fields_len(&mut self, struct_type_index: TypeIndex) -> WasmResult<usize>;

    /// Translate a `struct.new` instruction.
    fn translate_struct_new(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
        fields: StructFieldsVec,
    ) -> WasmResult<ir::Value>;

    /// Translate a `struct.new_default` instruction.
    fn translate_struct_new_default(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
    ) -> WasmResult<ir::Value>;

    /// Translate a `struct.set` instruction.
    fn translate_struct_set(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
        field_index: u32,
        struct_ref: ir::Value,
        value: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `struct.get` instruction.
    fn translate_struct_get(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
        field_index: u32,
        struct_ref: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate a `struct.get_s` instruction.
    fn translate_struct_get_s(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
        field_index: u32,
        struct_ref: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate a `struct.get_u` instruction.
    fn translate_struct_get_u(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
        field_index: u32,
        struct_ref: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.new` instruction.
    fn translate_array_new(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        elem: ir::Value,
        len: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.new_default` instruction.
    fn translate_array_new_default(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        len: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.new_fixed` instruction.
    fn translate_array_new_fixed(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        elems: &[ir::Value],
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.new_data` instruction.
    fn translate_array_new_data(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        data_index: DataIndex,
        data_offset: ir::Value,
        len: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.new_elem` instruction.
    fn translate_array_new_elem(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        elem_index: ElemIndex,
        elem_offset: ir::Value,
        len: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.copy` instruction.
    fn translate_array_copy(
        &mut self,
        builder: &mut FunctionBuilder,
        dst_array_type_index: TypeIndex,
        dst_array: ir::Value,
        dst_index: ir::Value,
        src_array_type_index: TypeIndex,
        src_array: ir::Value,
        src_index: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate an `array.fill` instruction.
    fn translate_array_fill(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        index: ir::Value,
        value: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate an `array.init_data` instruction.
    fn translate_array_init_data(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        dst_index: ir::Value,
        data_index: DataIndex,
        data_offset: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate an `array.init_elem` instruction.
    fn translate_array_init_elem(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        dst_index: ir::Value,
        elem_index: ElemIndex,
        elem_offset: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()>;

    /// Translate an `array.len` instruction.
    fn translate_array_len(
        &mut self,
        builder: &mut FunctionBuilder,
        array: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.get` instruction.
    fn translate_array_get(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        index: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.get_s` instruction.
    fn translate_array_get_s(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        index: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.get_u` instruction.
    fn translate_array_get_u(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        index: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Translate an `array.set` instruction.
    fn translate_array_set(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        index: ir::Value,
        value: ir::Value,
    ) -> WasmResult<()>;

    /// Translate a `ref.test` instruction.
    fn translate_ref_test(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        ref_ty: WasmRefType,
        gc_ref: ir::Value,
    ) -> WasmResult<ir::Value>;

    /// Emit code at the beginning of every wasm loop.
    ///
    /// This can be used to insert explicit interrupt or safepoint checking at
    /// the beginnings of loops.
    fn translate_loop_header(&mut self, _builder: &mut FunctionBuilder) -> WasmResult<()> {
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

    /// Optional callback for the `FuncEnvironment` performing this translation
    /// to maintain, prepare, or finalize custom, internal state when we
    /// statically determine that a Wasm memory access will unconditionally
    /// trap, rendering the rest of the block unreachable. Called just before
    /// the unconditional trap is emitted.
    fn before_unconditionally_trapping_memory_access(
        &mut self,
        _builder: &mut FunctionBuilder,
    ) -> WasmResult<()> {
        Ok(())
    }

    /// Optional callback for the `FunctionEnvironment` performing this translation to perform work
    /// before the function body is translated.
    fn before_translate_function(
        &mut self,
        _builder: &mut FunctionBuilder,
        _state: &FuncTranslationState,
    ) -> WasmResult<()> {
        Ok(())
    }

    /// Optional callback for the `FunctionEnvironment` performing this translation to perform work
    /// after the function body is translated.
    fn after_translate_function(
        &mut self,
        _builder: &mut FunctionBuilder,
        _state: &FuncTranslationState,
    ) -> WasmResult<()> {
        Ok(())
    }

    /// Whether or not to force relaxed simd instructions to have deterministic
    /// lowerings meaning they will produce the same results across all hosts,
    /// regardless of the cost to performance.
    fn relaxed_simd_deterministic(&self) -> bool {
        true
    }

    /// Whether or not the target being translated for has a native fma
    /// instruction. If it does not then when relaxed simd isn't deterministic
    /// the translation of the `f32x4.relaxed_fma` instruction, for example,
    /// will do a multiplication and then an add instead of the fused version.
    fn has_native_fma(&self) -> bool {
        false
    }

    /// Returns whether this is an x86 target, which may alter lowerings of
    /// relaxed simd instructions.
    fn is_x86(&self) -> bool {
        false
    }

    /// Returns whether the CLIF `x86_blendv` instruction should be used for the
    /// relaxed simd `*.relaxed_laneselect` instruction for the specified type.
    fn use_x86_blendv_for_relaxed_laneselect(&self, ty: Type) -> bool {
        let _ = ty;
        false
    }

    /// Returns whether the CLIF `x86_pshufb` instruction should be used for the
    /// `i8x16.relaxed_swizzle` instruction.
    fn use_x86_pshufb_for_relaxed_swizzle(&self) -> bool {
        false
    }

    /// Returns whether the CLIF `x86_pmulhrsw` instruction should be used for
    /// the `i8x16.relaxed_q15mulr_s` instruction.
    fn use_x86_pmulhrsw_for_relaxed_q15mul(&self) -> bool {
        false
    }

    /// Returns whether the CLIF `x86_pmaddubsw` instruction should be used for
    /// the relaxed-simd dot-product instructions instruction.
    fn use_x86_pmaddubsw_for_dot(&self) -> bool {
        false
    }

    /// Inserts code before a function return.
    fn handle_before_return(&mut self, _retvals: &[ir::Value], _builder: &mut FunctionBuilder) {}

    /// Inserts code before a load.
    fn before_load(
        &mut self,
        _builder: &mut FunctionBuilder,
        _val_size: u8,
        _addr: ir::Value,
        _offset: u64,
    ) {
    }

    /// Inserts code before a store.
    fn before_store(
        &mut self,
        _builder: &mut FunctionBuilder,
        _val_size: u8,
        _addr: ir::Value,
        _offset: u64,
    ) {
    }

    /// Inserts code before updating a global.
    fn update_global(
        &mut self,
        _builder: &mut FunctionBuilder,
        _global_index: u32,
        _value: ir::Value,
    ) {
    }

    /// Inserts code before memory.grow.
    fn before_memory_grow(
        &mut self,
        _builder: &mut FunctionBuilder,
        _num_bytes: ir::Value,
        _mem_index: MemoryIndex,
    ) {
    }

    /// Optional hook for customizing how `trap` is lowered.
    fn trap(&mut self, builder: &mut FunctionBuilder, code: ir::TrapCode) {
        builder.ins().trap(code);
    }

    /// Optional hook for customizing how `trapz` is lowered.
    fn trapz(&mut self, builder: &mut FunctionBuilder, value: ir::Value, code: ir::TrapCode) {
        builder.ins().trapz(value, code);
    }

    /// Optional hook for customizing how `trapnz` is lowered.
    fn trapnz(&mut self, builder: &mut FunctionBuilder, value: ir::Value, code: ir::TrapCode) {
        builder.ins().trapnz(value, code);
    }

    /// Optional hook for customizing how `uadd_overflow_trap` is lowered.
    fn uadd_overflow_trap(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
        code: ir::TrapCode,
    ) -> ir::Value {
        builder.ins().uadd_overflow_trap(lhs, rhs, code)
    }

    /// Accesses the ISA that is being compiled for.
    fn isa(&self) -> &dyn TargetIsa;

    /// Embedder-defined hook for indicating whether signals can be used to
    /// indicate traps.
    fn signals_based_traps(&self) -> bool {
        true
    }

    /// Optional hook for customizing `sdiv` instruction lowering.
    fn translate_sdiv(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) -> ir::Value {
        builder.ins().sdiv(lhs, rhs)
    }

    /// Optional hook for customizing `udiv` instruction lowering.
    fn translate_udiv(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) -> ir::Value {
        builder.ins().udiv(lhs, rhs)
    }

    /// Optional hook for customizing `srem` instruction lowering.
    fn translate_srem(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) -> ir::Value {
        builder.ins().srem(lhs, rhs)
    }

    /// Optional hook for customizing `urem` instruction lowering.
    fn translate_urem(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) -> ir::Value {
        builder.ins().urem(lhs, rhs)
    }

    /// Optional hook for customizing `fcvt_to_sint` instruction lowering.
    fn translate_fcvt_to_sint(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: ir::Type,
        val: ir::Value,
    ) -> ir::Value {
        builder.ins().fcvt_to_sint(ty, val)
    }

    /// Optional hook for customizing `fcvt_to_uint` instruction lowering.
    fn translate_fcvt_to_uint(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: ir::Type,
        val: ir::Value,
    ) -> ir::Value {
        builder.ins().fcvt_to_uint(ty, val)
    }
}

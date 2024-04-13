//! All the runtime support necessary for the wasm to cranelift translation is formalized by the
//! traits `FunctionEnvironment` and `ModuleEnvironment`.
//!
//! There are skeleton implementations of these traits in the `dummy` module, and complete
//! implementations in [Wasmtime].
//!
//! [Wasmtime]: https://github.com/bytecodealliance/wasmtime

use crate::state::FuncTranslationState;
use crate::{
    DataIndex, ElemIndex, FuncIndex, Global, GlobalIndex, GlobalInit, Heap, HeapData, Memory,
    MemoryIndex, Table, TableIndex, Tag, TagIndex, TypeConvert, TypeIndex, WasmError, WasmFuncType,
    WasmHeapType, WasmResult,
};
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, Type};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::PrimaryMap;
use cranelift_frontend::FunctionBuilder;
use std::boxed::Box;
use std::string::ToString;
use wasmparser::{FuncValidator, FunctionBody, Operator, ValidatorResources, WasmFeatures};
use wasmtime_types::ModuleInternedTypeIndex;

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

    /// Get the size of a native pointer, in bytes.
    fn pointer_bytes(&self) -> u8 {
        self.target_config().pointer_bytes()
    }

    /// Get the Cranelift reference type to use for the given Wasm reference
    /// type.
    ///
    /// By default, this returns `R64` for 64-bit architectures and `R32` for
    /// 32-bit architectures. If you override this, then you should also
    /// override `FuncEnvironment::{translate_ref_null, translate_ref_is_null}`
    /// as well.
    fn reference_type(&self, ty: WasmHeapType) -> ir::Type {
        let _ = ty;
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
    ///
    /// By default, translates into a null reference type.
    ///
    /// Override this if you don't use Cranelift reference types for all Wasm
    /// reference types (e.g. you use a raw pointer for `funcref`s) or if the
    /// null sentinel is not a null reference type pointer for your type. If you
    /// override this method, then you should also override
    /// `translate_ref_is_null` as well.
    fn translate_ref_null(
        &mut self,
        mut pos: FuncCursor,
        ty: WasmHeapType,
    ) -> WasmResult<ir::Value> {
        let _ = ty;
        Ok(pos.ins().null(self.reference_type(ty)))
    }

    /// Translate a `ref.is_null` WebAssembly instruction.
    ///
    /// By default, assumes that `value` is a Cranelift reference type, and that
    /// a null Cranelift reference type is the null value for all Wasm reference
    /// types.
    ///
    /// If you override this method, you probably also want to override
    /// `translate_ref_null` as well.
    fn translate_ref_is_null(
        &mut self,
        mut pos: FuncCursor,
        value: ir::Value,
    ) -> WasmResult<ir::Value> {
        let is_null = pos.ins().is_null(value);
        Ok(pos.ins().uextend(ir::types::I32, is_null))
    }

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
    fn translate_i31_get_s(&mut self, pos: FuncCursor, i31ref: ir::Value) -> WasmResult<ir::Value>;

    /// Zero-extend an `i31ref` into an `i32`.
    fn translate_i31_get_u(&mut self, pos: FuncCursor, i31ref: ir::Value) -> WasmResult<ir::Value>;

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
}

/// An object satisfying the `ModuleEnvironment` trait can be passed as argument to the
/// [`translate_module`](fn.translate_module.html) function. These methods should not be called
/// by the user, they are only for `cranelift-wasm` internal use.
pub trait ModuleEnvironment<'data>: TypeConvert {
    /// Provides the number of types up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_types(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a function signature to the environment.
    fn declare_type_func(&mut self, wasm_func_type: WasmFuncType) -> WasmResult<()>;

    /// Translates a type index to its signature index, only called for type
    /// indices which point to functions.
    fn type_to_signature(&self, index: TypeIndex) -> WasmResult<ModuleInternedTypeIndex> {
        let _ = index;
        Err(WasmError::Unsupported("module linking".to_string()))
    }

    /// Provides the number of imports up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_imports(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a function import to the environment.
    fn declare_func_import(
        &mut self,
        index: TypeIndex,
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

    /// Declares an tag import to the environment.
    fn declare_tag_import(
        &mut self,
        tag: Tag,
        module: &'data str,
        field: &'data str,
    ) -> WasmResult<()> {
        let _ = (tag, module, field);
        Err(WasmError::Unsupported("wasm tags".to_string()))
    }

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
    fn declare_func_type(&mut self, index: TypeIndex) -> WasmResult<()>;

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

    /// Provides the number of defined tags up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_tags(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares an tag to the environment
    fn declare_tag(&mut self, tag: Tag) -> WasmResult<()> {
        let _ = tag;
        Err(WasmError::Unsupported("wasm tags".to_string()))
    }

    /// Provides the number of defined globals up front. By default this does nothing, but
    /// implementations can use this to preallocate memory if desired.
    fn reserve_globals(&mut self, _num: u32) -> WasmResult<()> {
        Ok(())
    }

    /// Declares a global to the environment.
    fn declare_global(&mut self, global: Global, init: GlobalInit) -> WasmResult<()>;

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

    /// Declares an tag export to the environment.
    fn declare_tag_export(&mut self, tag_index: TagIndex, name: &'data str) -> WasmResult<()> {
        let _ = (tag_index, name);
        Err(WasmError::Unsupported("wasm tags".to_string()))
    }

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
        offset: u32,
        elements: Box<[FuncIndex]>,
    ) -> WasmResult<()>;

    /// Declare a passive element segment.
    fn declare_passive_element(
        &mut self,
        index: ElemIndex,
        elements: Box<[FuncIndex]>,
    ) -> WasmResult<()>;

    /// Indicates that a declarative element segment was seen in the wasm
    /// module.
    fn declare_elements(&mut self, elements: Box<[FuncIndex]>) -> WasmResult<()> {
        let _ = elements;
        Ok(())
    }

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

    /// Indicates how many functions the code section reports and the byte
    /// offset of where the code sections starts.
    fn reserve_function_bodies(&mut self, bodies: u32, code_section_offset: u64) {
        let _ = (bodies, code_section_offset);
    }

    /// Provides the contents of a function body.
    fn define_function_body(
        &mut self,
        validator: FuncValidator<ValidatorResources>,
        body: FunctionBody<'data>,
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
        offset: u64,
        data: &'data [u8],
    ) -> WasmResult<()>;

    /// Declares the name of a module to the environment.
    ///
    /// By default this does nothing, but implementations can use this to read
    /// the module name subsection of the custom name section if desired.
    fn declare_module_name(&mut self, _name: &'data str) {}

    /// Declares the name of a function to the environment.
    ///
    /// By default this does nothing, but implementations can use this to read
    /// the function name subsection of the custom name section if desired.
    fn declare_func_name(&mut self, _func_index: FuncIndex, _name: &'data str) {}

    /// Declares the name of a function's local to the environment.
    ///
    /// By default this does nothing, but implementations can use this to read
    /// the local name subsection of the custom name section if desired.
    fn declare_local_name(&mut self, _func_index: FuncIndex, _local_index: u32, _name: &'data str) {
    }

    /// Indicates that a custom section has been found in the wasm file
    fn custom_section(&mut self, _name: &'data str, _data: &'data [u8]) -> WasmResult<()> {
        Ok(())
    }

    /// Returns the list of enabled wasm features this translation will be using.
    fn wasm_features(&self) -> WasmFeatures {
        WasmFeatures::default()
    }
}

mod gc;
pub(crate) mod stack_switching;

use crate::compiler::Compiler;
use crate::translate::{
    FuncTranslationStacks, GlobalVariable, Heap, HeapData, MemoryKind, StructFieldsVec, TableData,
    TableSize, TargetEnvironment,
};
use crate::trap::TranslateTrap;
use crate::{BuiltinFunctionSignatures, TRAP_ARRAY_OUT_OF_BOUNDS, TRAP_TABLE_OUT_OF_BOUNDS};
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::immediates::{Imm64, Offset32, V128Imm};
use cranelift_codegen::ir::{
    self, BlockArg, Endianness, ExceptionTableData, ExceptionTableItem, types,
};
use cranelift_codegen::ir::{ArgumentPurpose, ConstantData, Function, InstBuilder, MemFlagsData};
use cranelift_codegen::ir::{Block, types::*};
use cranelift_codegen::isa::{CallConv, TargetFrontendConfig, TargetIsa};
use cranelift_entity::packed_option::{PackedOption, ReservedValue};
use cranelift_entity::{EntityRef, PrimaryMap, SecondaryMap};
use cranelift_frontend::Variable;
use cranelift_frontend::{FuncInstBuilder, FunctionBuilder};
use smallvec::{SmallVec, smallvec};
use std::mem;
use wasmparser::{FuncValidator, Operator, WasmFeatures, WasmModuleResources};
use wasmtime_core::math::f64_cvt_to_int_bounds;
use wasmtime_environ::{
    BuiltinFunctionIndex, ComponentPC, DataIndex, DefinedFuncIndex, ElemIndex,
    EngineOrModuleTypeIndex, FrameStateSlotBuilder, FrameValType, FuncIndex, FuncKey,
    GlobalConstValue, GlobalIndex, IndexType, Memory, MemoryIndex, MemoryTunables, Module,
    ModuleInternedTypeIndex, ModuleTranslation, ModuleTypesBuilder, PtrSize, Table, TableIndex,
    TagIndex, Tunables, TypeConvert, TypeIndex, VMOffsets, WasmCompositeInnerType, WasmFuncType,
    WasmHeapTopType, WasmHeapType, WasmRefType, WasmResult, WasmStorageType, WasmValType,
};
use wasmtime_environ::{FUNCREF_INIT_BIT, FUNCREF_MASK};

#[derive(Copy, Clone, Debug)]
pub(crate) enum Extension {
    Sign,
    Zero,
}

/// A struct with an `Option<ir::FuncRef>` member for every builtin
/// function, to de-duplicate constructing/getting its function.
pub(crate) struct BuiltinFunctions {
    types: BuiltinFunctionSignatures,

    builtins: [Option<ir::FuncRef>; BuiltinFunctionIndex::len() as usize],
    breakpoint_trampoline: Option<ir::FuncRef>,
}

impl BuiltinFunctions {
    pub(crate) fn new(compiler: &Compiler) -> Self {
        Self {
            types: BuiltinFunctionSignatures::new(compiler),
            builtins: [None; BuiltinFunctionIndex::len() as usize],
            breakpoint_trampoline: None,
        }
    }

    pub(crate) fn load_builtin(
        &mut self,
        func: &mut Function,
        builtin: BuiltinFunctionIndex,
    ) -> ir::FuncRef {
        let cache = &mut self.builtins[builtin.index() as usize];
        if let Some(f) = cache {
            return *f;
        }
        let signature = func.import_signature(self.types.wasm_signature(builtin));
        let key = FuncKey::WasmToBuiltinTrampoline(builtin);
        let (namespace, index) = key.into_raw_parts();
        let name = ir::ExternalName::User(
            func.declare_imported_user_function(ir::UserExternalName { namespace, index }),
        );
        let f = func.import_function(ir::ExtFuncData {
            name,
            signature,
            colocated: true,
            patchable: false,
        });
        *cache = Some(f);
        f
    }

    pub(crate) fn patchable_breakpoint(&mut self, func: &mut Function) -> ir::FuncRef {
        *self.breakpoint_trampoline.get_or_insert_with(|| {
            let mut signature = ir::Signature::new(CallConv::PreserveAll);
            signature
                .params
                .push(ir::AbiParam::new(self.types.pointer_type));
            let signature = func.import_signature(signature);
            let key = FuncKey::PatchableToBuiltinTrampoline(BuiltinFunctionIndex::breakpoint());
            let (namespace, index) = key.into_raw_parts();
            let name = ir::ExternalName::User(
                func.declare_imported_user_function(ir::UserExternalName { namespace, index }),
            );
            func.import_function(ir::ExtFuncData {
                name,
                signature,
                colocated: true,
                patchable: true,
            })
        })
    }
}

// Generate helper methods on `BuiltinFunctions` above for each named builtin
// as well.
macro_rules! declare_function_signatures {
    ($(
        $( #[$attr:meta] )*
        $name:ident( $( $pname:ident: $param:ident ),* ) $( -> $result:ident )?;
    )*) => {
        $(impl BuiltinFunctions {
            $( #[$attr] )*
            #[allow(dead_code, reason = "debug breakpoint libcall not used in host ABI, only patchable ABI")]
            pub(crate) fn $name(&mut self, func: &mut Function) -> ir::FuncRef {
                self.load_builtin(func, BuiltinFunctionIndex::$name())
            }
        })*
    };
}
wasmtime_environ::foreach_builtin_function!(declare_function_signatures);

/// The `FuncEnvironment` implementation for use by the `ModuleEnvironment`.
pub struct FuncEnvironment<'module_environment> {
    compiler: &'module_environment Compiler,
    isa: &'module_environment (dyn TargetIsa + 'static),
    key: FuncKey,
    pub(crate) module: &'module_environment Module,
    types: &'module_environment ModuleTypesBuilder,
    wasm_func_ty: &'module_environment WasmFuncType,
    sig_ref_to_ty: SecondaryMap<ir::SigRef, Option<&'module_environment WasmFuncType>>,
    needs_gc_heap: bool,
    entities: WasmEntities,

    /// The byte offset of the module's wasm binary within the outer
    /// binary (e.g. a component). Used to make source locations in
    /// guest-debug frame tables module-relative.
    pub(crate) wasm_module_offset: u64,

    /// Translation state at the given point.
    pub(crate) stacks: FuncTranslationStacks,

    #[cfg(feature = "gc")]
    ty_to_gc_layout: std::collections::HashMap<
        wasmtime_environ::ModuleInternedTypeIndex,
        wasmtime_environ::GcLayout,
    >,

    #[cfg(feature = "gc")]
    gc_heap: Option<Heap>,

    /// The Cranelift global holding the GC heap's base address.
    #[cfg(feature = "gc")]
    gc_heap_base: Option<ir::GlobalValue>,

    /// The Cranelift global holding the GC heap's base address.
    #[cfg(feature = "gc")]
    gc_heap_bound: Option<ir::GlobalValue>,

    translation: &'module_environment ModuleTranslation<'module_environment>,

    /// Heaps implementing WebAssembly linear memories.
    heaps: PrimaryMap<Heap, HeapData>,

    /// The Cranelift global holding the vmctx address.
    vmctx: Option<ir::GlobalValue>,

    /// The Cranelift global for our vmctx's `*mut VMStoreContext`.
    vm_store_context: Option<ir::GlobalValue>,

    /// Caches of signatures for builtin functions.
    builtin_functions: BuiltinFunctions,

    /// Offsets to struct fields accessed by JIT code.
    pub(crate) offsets: VMOffsets<u8>,

    tunables: &'module_environment Tunables,

    /// A function-local variable which stores the cached value of the amount of
    /// fuel remaining to execute. If used this is modified frequently so it's
    /// stored locally as a variable instead of always referenced from the field
    /// in `*const VMStoreContext`
    fuel_var: cranelift_frontend::Variable,

    /// A cached epoch deadline value, when performing epoch-based
    /// interruption. Loaded from `VMStoreContext` and reloaded after
    /// any yield.
    epoch_deadline_var: cranelift_frontend::Variable,

    /// A cached pointer to the per-Engine epoch counter, when
    /// performing epoch-based interruption. Initialized in the
    /// function prologue. We prefer to use a variable here rather
    /// than reload on each check because it's better to let the
    /// regalloc keep it in a register if able; if not, it can always
    /// spill, and this isn't any worse than reloading each time.
    epoch_ptr_var: cranelift_frontend::Variable,

    fuel_consumed: i64,

    /// A `GlobalValue` in CLIF which represents the stack limit.
    ///
    /// Typically this resides in the `stack_limit` value of `ir::Function` but
    /// that requires signal handlers on the host and when that's disabled this
    /// is here with an explicit check instead. Note that the explicit check is
    /// always present even if this is a "leaf" function, as we have to call
    /// into the host to trap when signal handlers are disabled.
    pub(crate) stack_limit_at_function_entry: Option<ir::GlobalValue>,

    /// Used by the stack switching feature. If set, we have a allocated a
    /// slot on this function's stack to be used for the
    /// current stack's `handler_list` field.
    stack_switching_handler_list_buffer: Option<ir::StackSlot>,

    /// Used by the stack switching feature. If set, we have a allocated a
    /// slot on this function's stack to be used for the
    /// current continuation's `values` field.
    stack_switching_values_buffer: Option<ir::StackSlot>,

    /// The stack-slot used for exposing Wasm state via debug
    /// instrumentation, if any, and the builder containing its metadata.
    pub(crate) state_slot: Option<(ir::StackSlot, FrameStateSlotBuilder)>,

    /// The next-srcloc: the location of the operator *after* this one
    /// (in original bytecode order, i.e., not accounting for
    /// nonlinear control flow). This is useful in cases where we need
    /// to e.g. record the return-address of a callsite for debuginfo.
    pub(crate) next_srcloc: ir::SourceLoc,
}

impl<'module_environment> FuncEnvironment<'module_environment> {
    pub fn new(
        compiler: &'module_environment Compiler,
        translation: &'module_environment ModuleTranslation<'module_environment>,
        types: &'module_environment ModuleTypesBuilder,
        wasm_func_ty: &'module_environment WasmFuncType,
        key: FuncKey,
    ) -> Self {
        let tunables = compiler.tunables();
        let builtin_functions = BuiltinFunctions::new(compiler);

        // This isn't used during translation, so squash the warning about this
        // being unused from the compiler.
        let _ = BuiltinFunctions::raise;

        Self {
            key,
            isa: compiler.isa(),
            module: &translation.module,
            compiler,
            types,
            wasm_func_ty,
            sig_ref_to_ty: SecondaryMap::default(),
            needs_gc_heap: false,
            entities: WasmEntities::default(),
            stacks: FuncTranslationStacks::new(),

            #[cfg(feature = "gc")]
            ty_to_gc_layout: std::collections::HashMap::new(),
            #[cfg(feature = "gc")]
            gc_heap: None,
            #[cfg(feature = "gc")]
            gc_heap_base: None,
            #[cfg(feature = "gc")]
            gc_heap_bound: None,

            heaps: PrimaryMap::default(),
            vmctx: None,
            vm_store_context: None,
            builtin_functions,
            offsets: VMOffsets::new(compiler.isa().pointer_bytes(), &translation.module),
            tunables,
            fuel_var: Variable::reserved_value(),
            epoch_deadline_var: Variable::reserved_value(),
            epoch_ptr_var: Variable::reserved_value(),

            // Start with at least one fuel being consumed because even empty
            // functions should consume at least some fuel.
            fuel_consumed: 1,

            translation,

            stack_limit_at_function_entry: None,

            stack_switching_handler_list_buffer: None,
            stack_switching_values_buffer: None,

            state_slot: None,
            next_srcloc: ir::SourceLoc::default(),
            wasm_module_offset: translation.wasm_module_offset,
        }
    }

    pub(crate) fn pointer_type(&self) -> ir::Type {
        self.isa.pointer_type()
    }

    pub(crate) fn vmctx(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.vmctx.unwrap_or_else(|| {
            let vmctx = func.create_global_value(ir::GlobalValueData::VMContext);
            self.vmctx = Some(vmctx);
            vmctx
        })
    }

    #[cfg(feature = "threads")]
    fn get_memory_atomic_wait(&mut self, func: &mut Function, ty: ir::Type) -> ir::FuncRef {
        match ty {
            I32 => self.builtin_functions.memory_atomic_wait32(func),
            I64 => self.builtin_functions.memory_atomic_wait64(func),
            x => panic!("get_memory_atomic_wait unsupported type: {x:?}"),
        }
    }

    fn get_global_location(
        &mut self,
        func: &mut ir::Function,
        index: GlobalIndex,
    ) -> (ir::GlobalValue, i32) {
        let pointer_type = self.pointer_type();
        let vmctx = self.vmctx(func);
        if let Some(def_index) = self.module.defined_global_index(index) {
            let offset = i32::try_from(self.offsets.vmctx_vmglobal_definition(def_index)).unwrap();
            (vmctx, offset)
        } else {
            let from_offset = self.offsets.vmctx_vmglobal_import_from(index);
            let global = func.create_global_value(ir::GlobalValueData::Load {
                base: vmctx,
                offset: Offset32::new(i32::try_from(from_offset).unwrap()),
                global_type: pointer_type,
                flags: MemFlagsData::trusted().with_readonly().with_can_move(),
            });
            (global, 0)
        }
    }

    /// Get or create the `ir::Global` for the `*mut VMStoreContext` in our
    /// `VMContext`.
    fn get_vmstore_context_ptr_global(&mut self, func: &mut ir::Function) -> ir::GlobalValue {
        if let Some(ptr) = self.vm_store_context {
            return ptr;
        }

        let offset = self.offsets.ptr.vmctx_store_context();
        let base = self.vmctx(func);
        let ptr = func.create_global_value(ir::GlobalValueData::Load {
            base,
            offset: Offset32::new(offset.into()),
            global_type: self.pointer_type(),
            flags: ir::MemFlagsData::trusted().with_readonly().with_can_move(),
        });
        self.vm_store_context = Some(ptr);
        ptr
    }

    /// Get the `*mut VMStoreContext` value for our `VMContext`.
    fn get_vmstore_context_ptr(&mut self, builder: &mut FunctionBuilder) -> ir::Value {
        let global = self.get_vmstore_context_ptr_global(&mut builder.func);
        builder.ins().global_value(self.pointer_type(), global)
    }

    fn fuel_function_entry(&mut self, builder: &mut FunctionBuilder<'_>) {
        // On function entry we load the amount of fuel into a function-local
        // `self.fuel_var` to make fuel modifications fast locally. This cache
        // is then periodically flushed to the Store-defined location in
        // `VMStoreContext` later.
        debug_assert!(self.fuel_var.is_reserved_value());
        self.fuel_var = builder.declare_var(ir::types::I64);
        self.fuel_load_into_var(builder);
        self.fuel_check(builder);
    }

    fn fuel_function_exit(&mut self, builder: &mut FunctionBuilder<'_>) {
        // On exiting the function we need to be sure to save the fuel we have
        // cached locally in `self.fuel_var` back into the Store-defined
        // location.
        self.fuel_save_from_var(builder);
    }

    fn fuel_before_op(
        &mut self,
        op: &Operator<'_>,
        builder: &mut FunctionBuilder<'_>,
        reachable: bool,
    ) {
        if !reachable {
            // In unreachable code we shouldn't have any leftover fuel we
            // haven't accounted for since the reason for us to become
            // unreachable should have already added it to `self.fuel_var`.
            debug_assert_eq!(self.fuel_consumed, 0);
            return;
        }

        self.fuel_consumed += self.tunables.operator_cost.cost(op);

        match op {
            // Exiting a function (via a return or unreachable) or otherwise
            // entering a different function (via a call) means that we need to
            // update the fuel consumption in `VMStoreContext` because we're
            // about to move control out of this function itself and the fuel
            // may need to be read.
            //
            // Before this we need to update the fuel counter from our own cost
            // leading up to this function call, and then we can store
            // `self.fuel_var` into `VMStoreContext`.
            Operator::Unreachable
            | Operator::Return
            | Operator::CallIndirect { .. }
            | Operator::Call { .. }
            | Operator::ReturnCall { .. }
            | Operator::ReturnCallRef { .. }
            | Operator::ReturnCallIndirect { .. }
            | Operator::Throw { .. } | Operator::ThrowRef => {
                self.fuel_increment_var(builder);
                self.fuel_save_from_var(builder);
            }

            // To ensure all code preceding a loop is only counted once we
            // update the fuel variable on entry.
            Operator::Loop { .. }

            // Entering into an `if` block means that the edge we take isn't
            // known until runtime, so we need to update our fuel consumption
            // before we take the branch.
            | Operator::If { .. }

            // Control-flow instructions mean that we're moving to the end/exit
            // of a block somewhere else. That means we need to update the fuel
            // counter since we're effectively terminating our basic block.
            | Operator::Br { .. }
            | Operator::BrIf { .. }
            | Operator::BrTable { .. }
            | Operator::BrOnNull { .. }
            | Operator::BrOnNonNull { .. }
            | Operator::BrOnCast { .. }
            | Operator::BrOnCastFail { .. }

            // Exiting a scope means that we need to update the fuel
            // consumption because there are multiple ways to exit a scope and
            // this is the only time we have to account for instructions
            // executed so far.
            | Operator::End

            // This is similar to `end`, except that it's only the terminator
            // for an `if` block. The same reasoning applies though in that we
            // are terminating a basic block and need to update the fuel
            // variable.
            | Operator::Else => self.fuel_increment_var(builder),

            // This is a normal instruction where the fuel is buffered to later
            // get added to `self.fuel_var`.
            //
            // Note that we generally ignore instructions which may trap and
            // therefore result in exiting a block early. Current usage of fuel
            // means that it's not too important to account for a precise amount
            // of fuel consumed but rather "close to the actual amount" is good
            // enough. For 100% precise counting, however, we'd probably need to
            // not only increment but also save the fuel amount more often
            // around trapping instructions. (see the `unreachable` instruction
            // case above)
            //
            // Note that `Block` is specifically omitted from incrementing the
            // fuel variable. Control flow entering a `block` is unconditional
            // which means it's effectively executing straight-line code. We'll
            // update the counter when exiting a block, but we shouldn't need to
            // do so upon entering a block.
            _ => {}
        }
    }

    fn fuel_after_op(&mut self, op: &Operator<'_>, builder: &mut FunctionBuilder<'_>) {
        // After a function call we need to reload our fuel value since the
        // function may have changed it.
        match op {
            Operator::Call { .. } | Operator::CallIndirect { .. } => {
                self.fuel_load_into_var(builder);
            }
            _ => {}
        }
    }

    /// Adds `self.fuel_consumed` to the `fuel_var`, zero-ing out the amount of
    /// fuel consumed at that point.
    fn fuel_increment_var(&mut self, builder: &mut FunctionBuilder<'_>) {
        let consumption = mem::replace(&mut self.fuel_consumed, 0);
        if consumption == 0 {
            return;
        }

        let fuel = builder.use_var(self.fuel_var);
        let fuel = builder.ins().iadd_imm(fuel, consumption);
        builder.def_var(self.fuel_var, fuel);
    }

    /// Loads the fuel consumption value from `VMStoreContext` into `self.fuel_var`
    fn fuel_load_into_var(&mut self, builder: &mut FunctionBuilder<'_>) {
        let (addr, offset) = self.fuel_addr_offset(builder);
        let fuel = builder
            .ins()
            .load(ir::types::I64, ir::MemFlagsData::trusted(), addr, offset);
        builder.def_var(self.fuel_var, fuel);
    }

    /// Stores the fuel consumption value from `self.fuel_var` into
    /// `VMStoreContext`.
    fn fuel_save_from_var(&mut self, builder: &mut FunctionBuilder<'_>) {
        let (addr, offset) = self.fuel_addr_offset(builder);
        let fuel_consumed = builder.use_var(self.fuel_var);
        builder
            .ins()
            .store(ir::MemFlagsData::trusted(), fuel_consumed, addr, offset);
    }

    /// Returns the `(address, offset)` of the fuel consumption within
    /// `VMStoreContext`, used to perform loads/stores later.
    fn fuel_addr_offset(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
    ) -> (ir::Value, ir::immediates::Offset32) {
        let vmstore_ctx = self.get_vmstore_context_ptr(builder);
        (
            vmstore_ctx,
            i32::from(self.offsets.ptr.vmstore_context_fuel_consumed()).into(),
        )
    }

    /// Checks the amount of remaining, and if we've run out of fuel we call
    /// the out-of-fuel function.
    fn fuel_check(&mut self, builder: &mut FunctionBuilder) {
        self.fuel_increment_var(builder);
        let out_of_gas_block = builder.create_block();
        let continuation_block = builder.create_block();

        // Note that our fuel is encoded as adding positive values to a
        // negative number. Whenever the negative number goes positive that
        // means we ran out of fuel.
        //
        // Compare to see if our fuel is positive, and if so we ran out of gas.
        // Otherwise we can continue on like usual.
        let zero = builder.ins().iconst(ir::types::I64, 0);
        let fuel = builder.use_var(self.fuel_var);
        let cmp = builder
            .ins()
            .icmp(IntCC::SignedGreaterThanOrEqual, fuel, zero);
        builder
            .ins()
            .brif(cmp, out_of_gas_block, &[], continuation_block, &[]);
        builder.seal_block(out_of_gas_block);

        // If we ran out of gas then we call our out-of-gas intrinsic and it
        // figures out what to do. Note that this may raise a trap, or do
        // something like yield to an async runtime. In either case we don't
        // assume what happens and handle the case the intrinsic returns.
        //
        // Note that we save/reload fuel around this since the out-of-gas
        // intrinsic may alter how much fuel is in the system.
        builder.switch_to_block(out_of_gas_block);
        self.fuel_save_from_var(builder);
        let out_of_gas = self.builtin_functions.out_of_gas(builder.func);
        let vmctx = self.vmctx_val(&mut builder.cursor());
        builder.ins().call(out_of_gas, &[vmctx]);
        self.fuel_load_into_var(builder);
        builder.ins().jump(continuation_block, &[]);
        builder.seal_block(continuation_block);

        builder.switch_to_block(continuation_block);
    }

    /// Manually insert a fuel check, as opposed to what already happens around
    /// normal loops headers and function entries.
    ///
    /// This can be used for expensive opcodes, such as `array.copy`, where the
    /// operation's runtime is a function of the runtime state.
    fn manual_fuel_check(&mut self, builder: &mut FunctionBuilder<'_>, fuel_to_consume: ir::Value) {
        self.fuel_increment_var(builder);

        let fuel = builder.use_var(self.fuel_var);
        let fuel = builder.ins().iadd(fuel, fuel_to_consume);
        builder.def_var(self.fuel_var, fuel);

        self.fuel_check(builder);
    }

    fn epoch_function_entry(&mut self, builder: &mut FunctionBuilder<'_>) {
        debug_assert!(self.epoch_deadline_var.is_reserved_value());
        self.epoch_deadline_var = builder.declare_var(ir::types::I64);
        // Let epoch_check_full load the current deadline and call def_var

        debug_assert!(self.epoch_ptr_var.is_reserved_value());
        self.epoch_ptr_var = builder.declare_var(self.pointer_type());
        let epoch_ptr = self.epoch_ptr(builder);
        builder.def_var(self.epoch_ptr_var, epoch_ptr);

        // We must check for an epoch change when entering a
        // function. Why? Why aren't checks at loops sufficient to
        // bound runtime to O(|static program size|)?
        //
        // The reason is that one can construct a "zip-bomb-like"
        // program with exponential-in-program-size runtime, with no
        // backedges (loops), by building a tree of function calls: f0
        // calls f1 ten times, f1 calls f2 ten times, etc. E.g., nine
        // levels of this yields a billion function calls with no
        // backedges. So we can't do checks only at backedges.
        //
        // In this "call-tree" scenario, and in fact in any program
        // that uses calls as a sort of control flow to try to evade
        // backedge checks, a check at every function entry is
        // sufficient. Then, combined with checks at every backedge
        // (loop) the longest runtime between checks is bounded by the
        // straightline length of any function body.
        let continuation_block = builder.create_block();
        let cur_epoch_value = self.epoch_load_current(builder);
        self.epoch_check_full(builder, cur_epoch_value, continuation_block);
    }

    #[cfg(feature = "wmemcheck")]
    fn hook_malloc_exit(&mut self, builder: &mut FunctionBuilder, retvals: &[ir::Value]) {
        let check_malloc = self.builtin_functions.check_malloc(builder.func);
        let vmctx = self.vmctx_val(&mut builder.cursor());
        let func_args = builder
            .func
            .dfg
            .block_params(builder.func.layout.entry_block().unwrap());
        let len = if func_args.len() < 3 {
            return;
        } else {
            // If a function named `malloc` has at least one argument, we assume the
            // first argument is the requested allocation size.
            func_args[2]
        };
        let retval = if retvals.len() < 1 {
            return;
        } else {
            retvals[0]
        };
        builder.ins().call(check_malloc, &[vmctx, retval, len]);
    }

    #[cfg(feature = "wmemcheck")]
    fn hook_free_exit(&mut self, builder: &mut FunctionBuilder) {
        let check_free = self.builtin_functions.check_free(builder.func);
        let vmctx = self.vmctx_val(&mut builder.cursor());
        let func_args = builder
            .func
            .dfg
            .block_params(builder.func.layout.entry_block().unwrap());
        let ptr = if func_args.len() < 3 {
            return;
        } else {
            // If a function named `free` has at least one argument, we assume the
            // first argument is a pointer to memory.
            func_args[2]
        };
        builder.ins().call(check_free, &[vmctx, ptr]);
    }

    fn epoch_ptr(&mut self, builder: &mut FunctionBuilder<'_>) -> ir::Value {
        let vmctx = self.vmctx(builder.func);
        let pointer_type = self.pointer_type();
        let base = builder.ins().global_value(pointer_type, vmctx);
        let offset = i32::from(self.offsets.ptr.vmctx_epoch_ptr());
        let epoch_ptr = builder
            .ins()
            .load(pointer_type, ir::MemFlagsData::trusted(), base, offset);
        epoch_ptr
    }

    fn epoch_load_current(&mut self, builder: &mut FunctionBuilder<'_>) -> ir::Value {
        let addr = builder.use_var(self.epoch_ptr_var);
        builder.ins().load(
            ir::types::I64,
            ir::MemFlagsData::trusted(),
            addr,
            ir::immediates::Offset32::new(0),
        )
    }

    fn epoch_check(&mut self, builder: &mut FunctionBuilder<'_>) {
        let continuation_block = builder.create_block();

        // Load new epoch and check against the cached deadline.
        let cur_epoch_value = self.epoch_load_current(builder);
        self.epoch_check_cached(builder, cur_epoch_value, continuation_block);

        // At this point we've noticed that the epoch has exceeded our
        // cached deadline. However the real deadline may have been
        // updated (within another yield) during some function that we
        // called in the meantime, so reload the cache and check again.
        self.epoch_check_full(builder, cur_epoch_value, continuation_block);
    }

    fn epoch_check_cached(
        &mut self,
        builder: &mut FunctionBuilder,
        cur_epoch_value: ir::Value,
        continuation_block: ir::Block,
    ) {
        let new_epoch_block = builder.create_block();
        builder.set_cold_block(new_epoch_block);

        let epoch_deadline = builder.use_var(self.epoch_deadline_var);
        let cmp = builder.ins().icmp(
            IntCC::UnsignedGreaterThanOrEqual,
            cur_epoch_value,
            epoch_deadline,
        );
        builder
            .ins()
            .brif(cmp, new_epoch_block, &[], continuation_block, &[]);
        builder.seal_block(new_epoch_block);

        builder.switch_to_block(new_epoch_block);
    }

    fn epoch_check_full(
        &mut self,
        builder: &mut FunctionBuilder,
        cur_epoch_value: ir::Value,
        continuation_block: ir::Block,
    ) {
        // We keep the deadline cached in a register to speed the checks
        // in the common case (between epoch ticks) but we want to do a
        // precise check here by reloading the cache first.
        let vmstore_ctx = self.get_vmstore_context_ptr(builder);
        let deadline = builder.ins().load(
            ir::types::I64,
            ir::MemFlagsData::trusted(),
            vmstore_ctx,
            ir::immediates::Offset32::new(self.offsets.ptr.vmstore_context_epoch_deadline() as i32),
        );
        builder.def_var(self.epoch_deadline_var, deadline);
        self.epoch_check_cached(builder, cur_epoch_value, continuation_block);

        let new_epoch = self.builtin_functions.new_epoch(builder.func);
        let vmctx = self.vmctx_val(&mut builder.cursor());
        // new_epoch() returns the new deadline, so we don't have to
        // reload it.
        let call = builder.ins().call(new_epoch, &[vmctx]);
        let new_deadline = *builder.func.dfg.inst_results(call).first().unwrap();
        builder.def_var(self.epoch_deadline_var, new_deadline);
        builder.ins().jump(continuation_block, &[]);
        builder.seal_block(continuation_block);

        builder.switch_to_block(continuation_block);
    }

    /// Get the Memory for the given index.
    fn memory(&self, index: MemoryIndex) -> Memory {
        self.module.memories[index]
    }

    /// Get the Table for the given index.
    fn table(&self, index: TableIndex) -> Table {
        self.module.tables[index]
    }

    /// Cast the value to I64 and sign extend if necessary.
    ///
    /// Returns the value casted to I64.
    fn cast_index_to_i64(
        &self,
        pos: &mut FuncCursor<'_>,
        val: ir::Value,
        index_type: IndexType,
    ) -> ir::Value {
        match index_type {
            IndexType::I32 => pos.ins().uextend(I64, val),
            IndexType::I64 => val,
        }
    }

    /// Cast the wasm pointer `val`, with type `index_type`, to a host pointer
    /// type.
    ///
    /// Does not perform any in-bounds checks, so `val` must already be
    /// validated to be in-bounds.
    fn unchecked_cast_wasm_addr_to_native_addr(
        &self,
        pos: &mut FuncCursor<'_>,
        val: ir::Value,
        index_type: IndexType,
    ) -> ir::Value {
        match (self.pointer_type(), index_type) {
            (I32, IndexType::I32) | (I64, IndexType::I64) => val,
            (I32, IndexType::I64) => pos.ins().ireduce(I32, val),
            (I64, IndexType::I32) => pos.ins().uextend(I64, val),
            _ => unreachable!(),
        }
    }

    /// Convert the target pointer-sized integer `val` into the memory/table's index type.
    ///
    /// For memory, `val` is holding a memory length (or the `-1` `memory.grow`-failed sentinel).
    /// For table, `val` is holding a table length.
    ///
    /// This might involve extending or truncating it depending on the memory/table's
    /// index type and the target's pointer type.
    fn convert_pointer_to_index_type(
        &self,
        mut pos: FuncCursor<'_>,
        val: ir::Value,
        index_type: IndexType,
        // When it is a memory and the memory is using single-byte pages,
        // we need to handle the truncation differently. See comments below.
        //
        // When it is a table, this should be set to false.
        single_byte_pages: bool,
    ) -> ir::Value {
        let desired_type = index_type_to_ir_type(index_type);
        let pointer_type = self.pointer_type();
        assert_eq!(pos.func.dfg.value_type(val), pointer_type);

        // The current length is of type `pointer_type` but we need to fit it
        // into `desired_type`. We are guaranteed that the result will always
        // fit, so we just need to do the right ireduce/sextend here.
        if pointer_type == desired_type {
            val
        } else if pointer_type.bits() > desired_type.bits() {
            pos.ins().ireduce(desired_type, val)
        } else {
            // We have a 64-bit memory/table on a 32-bit host -- this combo doesn't
            // really make a whole lot of sense to do from a user perspective
            // but that is neither here nor there. We want to logically do an
            // unsigned extend *except* when we are given the `-1` sentinel,
            // which we must preserve as `-1` in the wider type.
            match single_byte_pages {
                false => {
                    // In the case that we have default page sizes, we can
                    // always sign extend, since valid memory lengths (in pages)
                    // never have their sign bit set, and so if the sign bit is
                    // set then this must be the `-1` sentinel, which we want to
                    // preserve through the extension.
                    //
                    // When it comes to table, `single_byte_pages` should have always been set to false.
                    // Then we simply do a signed extension.
                    pos.ins().sextend(desired_type, val)
                }
                true => {
                    // For single-byte pages, we have to explicitly check for
                    // `-1` and choose whether to do an unsigned extension or
                    // return a larger `-1` because there are valid memory
                    // lengths (in pages) that have the sign bit set.
                    let extended = pos.ins().uextend(desired_type, val);
                    let neg_one = pos.ins().iconst(desired_type, -1);
                    let is_failure = pos.ins().icmp_imm(IntCC::Equal, val, -1);
                    pos.ins().select(is_failure, neg_one, extended)
                }
            }
        }
    }

    fn table_get_funcref(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        index: ir::Value,
        cold_blocks: bool,
    ) -> ir::Value {
        let pointer_type = self.pointer_type();
        let table_data = self.get_or_create_table(builder.func, table_index);

        // To support lazy initialization of table
        // contents, we check for a null entry here, and
        // if null, we take a slow-path that invokes a
        // libcall.
        let (table_entry_addr, flags) = table_data.prepare_table_addr(self, builder, index);
        let value = builder.ins().load(pointer_type, flags, table_entry_addr, 0);

        if !self.tunables.table_lazy_init {
            return value;
        }
        let pointer_type = self.pointer_type();

        // Mask off the "initialized bit". See documentation on
        // FUNCREF_INIT_BIT in crates/environ/src/ref_bits.rs for more
        // details. Note that `FUNCREF_MASK` has type `usize` which may not be
        // appropriate for the target architecture. Right now its value is
        // always -2 so assert that part doesn't change and then thread through
        // -2 as the immediate.
        assert_eq!(FUNCREF_MASK as isize, -2);
        let value_masked = builder.ins().band_imm(value, Imm64::from(-2));

        let null_block = builder.create_block();
        let continuation_block = builder.create_block();
        if cold_blocks {
            builder.set_cold_block(null_block);
            builder.set_cold_block(continuation_block);
        }
        let result_param = builder.append_block_param(continuation_block, pointer_type);
        builder.set_cold_block(null_block);

        builder.ins().brif(
            value,
            continuation_block,
            &[value_masked.into()],
            null_block,
            &[],
        );
        builder.seal_block(null_block);

        builder.switch_to_block(null_block);
        let index_type = self.table(table_index).idx_type;
        let table_index = builder.ins().iconst(I32, table_index.index() as i64);
        let lazy_init = self
            .builtin_functions
            .table_get_lazy_init_func_ref(builder.func);
        let vmctx = self.vmctx_val(&mut builder.cursor());
        let index = self.cast_index_to_i64(&mut builder.cursor(), index, index_type);
        let call_inst = builder.ins().call(lazy_init, &[vmctx, table_index, index]);
        let returned_entry = builder.func.dfg.inst_results(call_inst)[0];
        builder
            .ins()
            .jump(continuation_block, &[returned_entry.into()]);
        builder.seal_block(continuation_block);

        builder.switch_to_block(continuation_block);
        result_param
    }

    #[cfg(feature = "wmemcheck")]
    fn check_malloc_start(&mut self, builder: &mut FunctionBuilder) {
        let malloc_start = self.builtin_functions.malloc_start(builder.func);
        let vmctx = self.vmctx_val(&mut builder.cursor());
        builder.ins().call(malloc_start, &[vmctx]);
    }

    #[cfg(feature = "wmemcheck")]
    fn check_free_start(&mut self, builder: &mut FunctionBuilder) {
        let free_start = self.builtin_functions.free_start(builder.func);
        let vmctx = self.vmctx_val(&mut builder.cursor());
        builder.ins().call(free_start, &[vmctx]);
    }

    #[cfg(feature = "wmemcheck")]
    fn current_func_name(&self, builder: &mut FunctionBuilder) -> Option<&str> {
        let func_index = match &builder.func.name {
            ir::UserFuncName::User(user) => FuncIndex::from_u32(user.index),
            _ => {
                panic!("function name not a UserFuncName::User as expected")
            }
        };
        self.translation
            .debuginfo
            .name_section
            .func_names
            .get(&func_index)
            .copied()
    }

    /// Create an `ir::Global` that does `load(ptr + offset)`.
    pub(crate) fn global_load(
        &mut self,
        func: &mut ir::Function,
        ptr: ir::GlobalValue,
        offset: u32,
        flags: ir::MemFlagsData,
    ) -> ir::GlobalValue {
        func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(i32::try_from(offset).unwrap()),
            global_type: self.pointer_type(),
            flags,
        })
    }

    /// Like `global_load` but specialized for loads out of the
    /// `vmctx`.
    pub(crate) fn global_load_from_vmctx(
        &mut self,
        func: &mut ir::Function,
        offset: u32,
        flags: ir::MemFlagsData,
    ) -> ir::GlobalValue {
        let vmctx = self.vmctx(func);
        self.global_load(func, vmctx, offset, flags)
    }

    /// Helper used when `!self.clif_instruction_traps_enabled()` is enabled to
    /// test whether the divisor is zero.
    fn guard_zero_divisor(&mut self, builder: &mut FunctionBuilder, rhs: ir::Value) {
        if self.clif_instruction_traps_enabled() {
            return;
        }
        self.trapz(builder, rhs, ir::TrapCode::INTEGER_DIVISION_BY_ZERO);
    }

    /// Helper used when `!self.clif_instruction_traps_enabled()` is enabled to
    /// test whether a signed division operation will raise a trap.
    fn guard_signed_divide(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) {
        if self.clif_instruction_traps_enabled() {
            return;
        }
        self.trapz(builder, rhs, ir::TrapCode::INTEGER_DIVISION_BY_ZERO);

        let ty = builder.func.dfg.value_type(rhs);
        let minus_one = builder.ins().iconst(ty, -1);
        let rhs_is_minus_one = builder.ins().icmp(IntCC::Equal, rhs, minus_one);
        let int_min = builder.ins().iconst(
            ty,
            match ty {
                I32 => i64::from(i32::MIN),
                I64 => i64::MIN,
                _ => unreachable!(),
            },
        );
        let lhs_is_int_min = builder.ins().icmp(IntCC::Equal, lhs, int_min);
        let is_integer_overflow = builder.ins().band(rhs_is_minus_one, lhs_is_int_min);
        self.conditionally_trap(builder, is_integer_overflow, ir::TrapCode::INTEGER_OVERFLOW);
    }

    /// Helper used when `!self.clif_instruction_traps_enabled()` is enabled to
    /// guard the traps from float-to-int conversions.
    fn guard_fcvt_to_int(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: ir::Type,
        val: ir::Value,
        signed: bool,
    ) {
        assert!(!self.clif_instruction_traps_enabled());
        let val_ty = builder.func.dfg.value_type(val);
        let val = if val_ty == F64 {
            val
        } else {
            builder.ins().fpromote(F64, val)
        };
        let isnan = builder.ins().fcmp(FloatCC::NotEqual, val, val);
        self.trapnz(builder, isnan, ir::TrapCode::BAD_CONVERSION_TO_INTEGER);
        let val = self.trunc_f64(builder, val);
        let (lower_bound, upper_bound) = f64_cvt_to_int_bounds(signed, ty.bits());
        let lower_bound = builder.ins().f64const(lower_bound);
        let too_small = builder
            .ins()
            .fcmp(FloatCC::LessThanOrEqual, val, lower_bound);
        self.trapnz(builder, too_small, ir::TrapCode::INTEGER_OVERFLOW);
        let upper_bound = builder.ins().f64const(upper_bound);
        let too_large = builder
            .ins()
            .fcmp(FloatCC::GreaterThanOrEqual, val, upper_bound);
        self.trapnz(builder, too_large, ir::TrapCode::INTEGER_OVERFLOW);
    }

    /// Get the `ir::Type` for a `VMSharedTypeIndex`.
    pub(crate) fn vmshared_type_index_ty(&self) -> Type {
        Type::int_with_byte_size(self.offsets.size_of_vmshared_type_index().into()).unwrap()
    }

    /// Given a `ModuleInternedTypeIndex`, emit code to get the corresponding
    /// `VMSharedTypeIndex` at runtime.
    pub(crate) fn module_interned_to_shared_ty(
        &mut self,
        pos: &mut FuncCursor,
        interned_ty: ModuleInternedTypeIndex,
    ) -> ir::Value {
        let vmctx = self.vmctx_val(pos);
        let pointer_type = self.pointer_type();
        let mem_flags = ir::MemFlagsData::trusted().with_readonly().with_can_move();

        // Load the base pointer of the array of `VMSharedTypeIndex`es.
        let shared_indices = pos.ins().load(
            pointer_type,
            mem_flags,
            vmctx,
            i32::from(self.offsets.ptr.vmctx_type_ids_array()),
        );

        // Calculate the offset in that array for this type's entry.
        let ty = self.vmshared_type_index_ty();
        let offset = i32::try_from(interned_ty.as_u32().checked_mul(ty.bytes()).unwrap()).unwrap();

        // Load the`VMSharedTypeIndex` that this `ModuleInternedTypeIndex` is
        // associated with at runtime from the array.
        pos.ins().load(ty, mem_flags, shared_indices, offset)
    }

    /// Load the associated `VMSharedTypeIndex` from inside a `*const VMFuncRef`.
    ///
    /// Does not check for null; just assumes that the `funcref` is a valid
    /// pointer.
    pub(crate) fn load_funcref_type_index(
        &mut self,
        pos: &mut FuncCursor,
        mem_flags: ir::MemFlagsData,
        funcref: ir::Value,
    ) -> ir::Value {
        let ty = self.vmshared_type_index_ty();
        pos.ins().load(
            ty,
            mem_flags,
            funcref,
            i32::from(self.offsets.ptr.vm_func_ref_type_index()),
        )
    }

    /// Does this function need a GC heap?
    pub fn needs_gc_heap(&self) -> bool {
        self.needs_gc_heap
    }

    /// Get the number of Wasm parameters for the given function.
    pub(crate) fn num_params_for_func(&self, function_index: FuncIndex) -> usize {
        let ty = self.module.functions[function_index]
            .signature
            .unwrap_module_type_index();
        self.types[ty].unwrap_func().params().len()
    }

    /// Get the number of Wasm parameters for the given function type.
    ///
    /// Panics on non-function types.
    pub(crate) fn num_params_for_function_type(&self, type_index: TypeIndex) -> usize {
        let ty = self.module.types[type_index].unwrap_module_type_index();
        self.types[ty].unwrap_func().params().len()
    }

    /// Initialize the state slot with an empty layout.
    pub(crate) fn create_state_slot(&mut self, builder: &mut FunctionBuilder) {
        if self.tunables.debug_guest {
            let frame_builder = FrameStateSlotBuilder::new(self.key, self.pointer_type().bytes());

            // Initially zero-size and with no descriptor; we will fill in
            // this info once we're done with the function body.
            let slot = builder
                .func
                .create_sized_stack_slot(ir::StackSlotData::new_with_key(
                    ir::StackSlotKind::ExplicitSlot,
                    0,
                    0,
                    ir::StackSlotKey::new(self.key.into_raw_u64()),
                ));

            self.state_slot = Some((slot, frame_builder));
        }
    }

    fn memflags_for_debug_slot_value_wasm_ty(&self, ty: WasmValType) -> MemFlagsData {
        // Store vectors in little-endian format: this is
        // universally supported, while native or
        // big-endian formats may not be in all cases
        // (e.g. Pulley on s390x).
        let mut flags = MemFlagsData::trusted();
        if ty == WasmValType::V128 {
            flags.set_endianness(Endianness::Little);
        }
        flags
    }

    fn memflags_for_debug_slot_value_clif_ty(&self, ty: ir::Type) -> MemFlagsData {
        let mut flags = MemFlagsData::trusted();
        if ty.is_vector() {
            flags.set_endianness(Endianness::Little);
        }
        flags
    }

    /// Update the state slot layout with a new layout given a local.
    pub(crate) fn add_state_slot_local(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: WasmValType,
        init: Option<ir::Value>,
    ) {
        if let Some((slot, b)) = &mut self.state_slot {
            let offset = b.add_local(FrameValType::from(ty));
            if let Some(init) = init {
                let slot = *slot;
                let address = builder
                    .ins()
                    .stack_addr(self.pointer_type(), slot, offset.offset());
                builder.ins().store(
                    self.memflags_for_debug_slot_value_wasm_ty(ty),
                    init,
                    address,
                    0,
                );
            }
        }
    }

    fn update_state_slot_stack(
        &mut self,
        validator: &FuncValidator<impl WasmModuleResources>,
        builder: &mut FunctionBuilder,
    ) -> WasmResult<()> {
        // Take ownership of the state-slot builder temporarily rather
        // than mutably borrowing so we can invoke a method below.
        if let Some((slot, mut b)) = self.state_slot.take() {
            // If the stack-shape stack is shorter than the value
            // stack, that means that values were popped and then new
            // values were pushed; hence, these operand-stack values
            // are "dirty" and need to be flushed to the stackslot.
            //
            // N.B.: note that we don't re-sync GC-rooted values, and
            // we don't root the instrumentation slots
            // explicitly. This is safe as long as we don't have a
            // moving GC, because the value that we're observing in
            // the main program dataflow is already rooted in the main
            // program (we are only storing an extra copy of it). But
            // if/when we do build a moving GC, we will need to handle
            // this, probably by invalidating the "freshness" of all
            // ref-typed values after a safepoint and re-writing them
            // to the instrumentation slot; or alternately, extending
            // the debug instrumentation mechanism to be able to
            // directly refer to the user stack-slot.
            for i in self.stacks.stack_shape.len()..self.stacks.stack.len() {
                let parent_shape = i
                    .checked_sub(1)
                    .map(|parent_idx| self.stacks.stack_shape[parent_idx]);
                if let Some(this_ty) = validator
                    .get_operand_type(self.stacks.stack.len() - i - 1)
                    .expect("Index should not be out of range")
                {
                    let wasm_ty = self.convert_valtype(this_ty)?;
                    let (this_shape, offset) =
                        b.push_stack(parent_shape, FrameValType::from(wasm_ty));
                    self.stacks.stack_shape.push(this_shape);

                    let value = self.stacks.stack[i];
                    let address =
                        builder
                            .ins()
                            .stack_addr(self.pointer_type(), slot, offset.offset());
                    builder.ins().store(
                        self.memflags_for_debug_slot_value_wasm_ty(wasm_ty),
                        value,
                        address,
                        0,
                    );
                } else {
                    // Unreachable code with unknown type -- no
                    // flushes for this or later-pushed values.
                    break;
                }
            }

            self.state_slot = Some((slot, b));
        }

        Ok(())
    }

    pub(crate) fn debug_tags(&self, srcloc: ir::SourceLoc) -> Vec<ir::DebugTag> {
        if let Some((slot, _b)) = &self.state_slot {
            self.stacks.assert_debug_stack_is_synced();
            let stack_shape = self
                .stacks
                .stack_shape
                .last()
                .map(|s| s.raw())
                .unwrap_or(u32::MAX);
            // Convert component-relative srcloc to module-relative
            // Wasm PC for the frame table. The srcloc on the builder
            // remains component-relative for native DWARF and other
            // purposes, but the frame table must be module-relative
            // because the guest-debug API presents a purely core-Wasm
            // view of the world where components are deconstructed
            // into core Wasm modules.
            let component_pc = ComponentPC::new(srcloc.bits());
            let module_pc = component_pc.to_module_pc(self.wasm_module_offset);
            vec![
                ir::DebugTag::StackSlot(*slot),
                ir::DebugTag::User(module_pc.raw()),
                ir::DebugTag::User(stack_shape),
            ]
        } else {
            vec![]
        }
    }

    fn finish_debug_metadata(&self, builder: &mut FunctionBuilder) {
        if let Some((slot, b)) = &self.state_slot {
            builder.func.sized_stack_slots[*slot].size = b.size();
        }
    }

    /// Store a new value for a local in the state slot, if present.
    pub(crate) fn state_slot_local_set(
        &self,
        builder: &mut FunctionBuilder,
        local: u32,
        value: ir::Value,
    ) {
        if let Some((slot, b)) = &self.state_slot {
            let offset = b.local_offset(local);
            let address = builder
                .ins()
                .stack_addr(self.pointer_type(), *slot, offset.offset());
            let ty = builder.func.dfg.value_type(value);
            builder.ins().store(
                self.memflags_for_debug_slot_value_clif_ty(ty),
                value,
                address,
                0,
            );
        }
    }

    fn update_state_slot_vmctx(&mut self, builder: &mut FunctionBuilder) {
        if let &Some((slot, _)) = &self.state_slot {
            let vmctx = self.vmctx_val(&mut builder.cursor());
            // N.B.: we always store vmctx at offset 0 in the
            // slot. This is relied upon in
            // crates/wasmtime/src/runtime/debug.rs in
            // `raw_instance()`. See also the slot layout computation in crates/environ/src/
            //
            // This is a native-endian store (the only mode for
            // `stack_store`) because it is read by host code directly
            // as a pointer.
            builder.ins().stack_store(vmctx, slot, 0);
        }
    }

    pub(crate) fn val_ty_needs_stack_map(&self, ty: WasmValType) -> bool {
        match ty {
            WasmValType::Ref(r) => self.heap_ty_needs_stack_map(r.heap_type),
            _ => false,
        }
    }

    pub(crate) fn heap_ty_needs_stack_map(&self, ty: WasmHeapType) -> bool {
        ty.is_vmgcref_type_and_not_i31() && !ty.is_bottom()
    }
}

impl TranslateTrap for FuncEnvironment<'_> {
    fn compiler(&self) -> &Compiler {
        &self.compiler
    }

    fn vmctx_val(&mut self, pos: &mut FuncCursor<'_>) -> ir::Value {
        let pointer_type = self.pointer_type();
        let vmctx = self.vmctx(&mut pos.func);
        pos.ins().global_value(pointer_type, vmctx)
    }

    fn builtin_funcref(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        index: BuiltinFunctionIndex,
    ) -> ir::FuncRef {
        self.builtin_functions.load_builtin(builder.func, index)
    }

    fn debug_tags(&self, srcloc: ir::SourceLoc) -> Vec<ir::DebugTag> {
        FuncEnvironment::debug_tags(self, srcloc)
    }
}

#[derive(Default)]
pub(crate) struct WasmEntities {
    /// Map from a Wasm global index from this module to its implementation in
    /// the Cranelift function we are building.
    pub(crate) globals: SecondaryMap<GlobalIndex, Option<GlobalVariable>>,

    /// Map from a Wasm memory index to its `Heap` implementation in the
    /// Cranelift function we are building.
    pub(crate) memories: SecondaryMap<MemoryIndex, PackedOption<Heap>>,

    /// Map from an (interned) Wasm type index from this module to its
    /// `ir::SigRef` in the Cranelift function we are building.
    pub(crate) sig_refs: SecondaryMap<ModuleInternedTypeIndex, PackedOption<ir::SigRef>>,

    /// Map from a defined Wasm function index to its associated function
    /// reference in the Cranelift function we are building.
    pub(crate) defined_func_refs: SecondaryMap<DefinedFuncIndex, PackedOption<ir::FuncRef>>,

    /// Map from an imported Wasm function index for which we statically know
    /// which function will always be used to satisfy that import to its
    /// associated function reference in the Cranelift function we are building.
    pub(crate) imported_func_refs: SecondaryMap<FuncIndex, PackedOption<ir::FuncRef>>,

    /// Map from a Wasm table index to its associated implementation in the
    /// Cranelift function we are building.
    pub(crate) tables: SecondaryMap<TableIndex, Option<TableData>>,
}

macro_rules! define_get_or_create_methods {
    ( $( $name:ident ( $map:ident ) : $create:ident : $key:ty => $val:ty ; )* ) => {
        $(
            pub(crate) fn $name(&mut self, func: &mut ir::Function, key: $key) -> $val {
                match self.entities.$map[key].clone().into() {
                    Some(val) => val,
                    None => {
                        let val = self.$create(func, key);
                        self.entities.$map[key] = Some(val.clone()).into();
                        val
                    }
                }
            }
        )*
    };
}

impl FuncEnvironment<'_> {
    define_get_or_create_methods! {
        get_or_create_global(globals) : make_global : GlobalIndex => GlobalVariable;
        get_or_create_heap(memories) : make_heap : MemoryIndex => Heap;
        get_or_create_interned_sig_ref(sig_refs) : make_sig_ref : ModuleInternedTypeIndex => ir::SigRef;
        get_or_create_defined_func_ref(defined_func_refs) : make_defined_func_ref : DefinedFuncIndex => ir::FuncRef;
        get_or_create_imported_func_ref(imported_func_refs) : make_imported_func_ref : FuncIndex => ir::FuncRef;
        get_or_create_table(tables) : make_table : TableIndex => TableData;
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalVariable {
        let ty = self.module.globals[index].wasm_ty;

        if ty.is_vmgcref_type() {
            // Although reference-typed globals live at the same memory location as
            // any other type of global at the same index would, getting or
            // setting them requires ref counting barriers. Therefore, we need
            // to use `GlobalVariable::Custom`, as that is the only kind of
            // `GlobalVariable` for which translation supports custom
            // access translation.
            return GlobalVariable::Custom;
        }

        if !self.module.globals[index].mutability {
            if let Some(index) = self.module.defined_global_index(index) {
                let init = &self.module.global_initializers[index];
                if let Some(value) = init.const_eval() {
                    return GlobalVariable::Constant { value };
                }
            }
        }

        let (gv, offset) = self.get_global_location(func, index);
        GlobalVariable::Memory {
            gv,
            offset: offset.into(),
            ty: super::value_type(self.isa, ty),
        }
    }

    pub(crate) fn get_or_create_sig_ref(
        &mut self,
        func: &mut ir::Function,
        ty: TypeIndex,
    ) -> ir::SigRef {
        let ty = self.module.types[ty].unwrap_module_type_index();
        self.get_or_create_interned_sig_ref(func, ty)
    }

    fn make_sig_ref(
        &mut self,
        func: &mut ir::Function,
        index: ModuleInternedTypeIndex,
    ) -> ir::SigRef {
        let wasm_func_ty = self.types[index].unwrap_func();
        let sig = crate::wasm_call_signature(self.isa, wasm_func_ty, &self.tunables);
        let sig_ref = func.import_signature(sig);
        self.sig_ref_to_ty[sig_ref] = Some(wasm_func_ty);
        sig_ref
    }

    fn make_defined_func_ref(
        &mut self,
        func: &mut ir::Function,
        def_func_index: DefinedFuncIndex,
    ) -> ir::FuncRef {
        let func_index = self.module.func_index(def_func_index);

        let ty = self.module.functions[func_index]
            .signature
            .unwrap_module_type_index();
        let signature = self.get_or_create_interned_sig_ref(func, ty);

        let key = FuncKey::DefinedWasmFunction(self.translation.module_index(), def_func_index);
        let (namespace, index) = key.into_raw_parts();
        let name = ir::ExternalName::User(
            func.declare_imported_user_function(ir::UserExternalName { namespace, index }),
        );

        func.import_function(ir::ExtFuncData {
            name,
            signature,
            colocated: true,
            patchable: false,
        })
    }

    fn make_imported_func_ref(
        &mut self,
        func: &mut ir::Function,
        func_index: FuncIndex,
    ) -> ir::FuncRef {
        assert!(self.module.is_imported_function(func_index));
        assert!(self.translation.known_imported_functions[func_index].is_some());

        let ty = self.module.functions[func_index]
            .signature
            .unwrap_module_type_index();
        let signature = self.get_or_create_interned_sig_ref(func, ty);

        let key = match self.translation.known_imported_functions[func_index] {
            Some(key @ FuncKey::DefinedWasmFunction(..)) => key,

            #[cfg(feature = "component-model")]
            Some(key @ FuncKey::UnsafeIntrinsic(..)) => key,

            Some(key) => {
                panic!("unexpected kind of known-import function: {key:?}")
            }

            None => panic!(
                "cannot make an `ir::FuncRef` for a function import that is not statically known"
            ),
        };

        let (namespace, index) = key.into_raw_parts();
        let name = ir::ExternalName::User(
            func.declare_imported_user_function(ir::UserExternalName { namespace, index }),
        );

        func.import_function(ir::ExtFuncData {
            name,
            signature,
            colocated: true,
            patchable: false,
        })
    }

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> Heap {
        let pointer_type = self.pointer_type();
        let memory = self.module.memories[index];
        let is_shared = memory.shared;

        let (base_ptr, base_offset, current_length_offset) = {
            let vmctx = self.vmctx(func);
            if let Some(def_index) = self.module.defined_memory_index(index) {
                if is_shared {
                    // As with imported memory, the `VMMemoryDefinition` for a
                    // shared memory is stored elsewhere. We store a `*mut
                    // VMMemoryDefinition` to it and dereference that when
                    // atomically growing it.
                    let from_offset = self.offsets.vmctx_vmmemory_pointer(def_index);
                    let memory = self.global_load_from_vmctx(
                        func,
                        from_offset,
                        ir::MemFlagsData::trusted().with_readonly().with_can_move(),
                    );
                    let base_offset = i32::from(self.offsets.ptr.vmmemory_definition_base());
                    let current_length_offset =
                        i32::from(self.offsets.ptr.vmmemory_definition_current_length());
                    (memory, base_offset, current_length_offset)
                } else {
                    let owned_index = self.module.owned_memory_index(def_index);
                    let owned_base_offset =
                        self.offsets.vmctx_vmmemory_definition_base(owned_index);
                    let owned_length_offset = self
                        .offsets
                        .vmctx_vmmemory_definition_current_length(owned_index);
                    let current_base_offset = i32::try_from(owned_base_offset).unwrap();
                    let current_length_offset = i32::try_from(owned_length_offset).unwrap();
                    (vmctx, current_base_offset, current_length_offset)
                }
            } else {
                let from_offset = self.offsets.vmctx_vmmemory_import_from(index);
                let memory = self.global_load_from_vmctx(
                    func,
                    from_offset,
                    ir::MemFlagsData::trusted().with_readonly().with_can_move(),
                );
                let base_offset = i32::from(self.offsets.ptr.vmmemory_definition_base());
                let current_length_offset =
                    i32::from(self.offsets.ptr.vmmemory_definition_current_length());
                (memory, base_offset, current_length_offset)
            }
        };

        let bound = func.create_global_value(ir::GlobalValueData::Load {
            base: base_ptr,
            offset: Offset32::new(current_length_offset),
            global_type: pointer_type,
            flags: MemFlagsData::trusted(),
        });

        let base = self.make_heap_base(func, memory, base_ptr, base_offset);

        self.heaps.push(HeapData {
            base,
            bound,
            kind: MemoryKind::LinearMemory,
            memory,
        })
    }

    pub(crate) fn make_heap_base(
        &self,
        func: &mut Function,
        memory: Memory,
        ptr: ir::GlobalValue,
        offset: i32,
    ) -> ir::GlobalValue {
        let pointer_type = self.pointer_type();
        let memory_tunables = MemoryTunables::new(self.tunables, MemoryKind::LinearMemory);

        let mut flags = ir::MemFlagsData::trusted().with_can_move();
        if !memory.memory_may_move(&memory_tunables) {
            flags.set_readonly();
        }

        let heap_base = func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(offset),
            global_type: pointer_type,
            flags,
        });
        heap_base
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> TableData {
        let pointer_type = self.pointer_type();

        let (ptr, base_offset, current_elements_offset) = {
            let vmctx = self.vmctx(func);
            if let Some(def_index) = self.module.defined_table_index(index) {
                let base_offset =
                    i32::try_from(self.offsets.vmctx_vmtable_definition_base(def_index)).unwrap();
                let current_elements_offset = i32::try_from(
                    self.offsets
                        .vmctx_vmtable_definition_current_elements(def_index),
                )
                .unwrap();
                (vmctx, base_offset, current_elements_offset)
            } else {
                let from_offset = self.offsets.vmctx_vmtable_from(index);
                let table = func.create_global_value(ir::GlobalValueData::Load {
                    base: vmctx,
                    offset: Offset32::new(i32::try_from(from_offset).unwrap()),
                    global_type: pointer_type,
                    flags: MemFlagsData::trusted().with_readonly().with_can_move(),
                });
                let base_offset = i32::from(self.offsets.vmtable_definition_base());
                let current_elements_offset =
                    i32::from(self.offsets.vmtable_definition_current_elements());
                (table, base_offset, current_elements_offset)
            }
        };

        let table = &self.module.tables[index];
        let element_size = if table.ref_type.is_vmgcref_type() {
            // For GC-managed references, tables store `Option<VMGcRef>`s.
            ir::types::I32.bytes()
        } else {
            self.reference_type(table.ref_type.heap_type).0.bytes()
        };

        let base_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(base_offset),
            global_type: pointer_type,
            flags: if Some(table.limits.min) == table.limits.max {
                // A fixed-size table can't be resized so its base address won't
                // change.
                MemFlagsData::trusted().with_readonly().with_can_move()
            } else {
                MemFlagsData::trusted()
            },
        });

        let bound = if Some(table.limits.min) == table.limits.max {
            TableSize::Static {
                bound: table.limits.min,
            }
        } else {
            TableSize::Dynamic {
                bound_gv: func.create_global_value(ir::GlobalValueData::Load {
                    base: ptr,
                    offset: Offset32::new(current_elements_offset),
                    global_type: ir::Type::int(
                        u16::from(self.offsets.size_of_vmtable_definition_current_elements()) * 8,
                    )
                    .unwrap(),
                    flags: MemFlagsData::trusted(),
                }),
            }
        };

        TableData {
            base_gv,
            bound,
            element_size,
        }
    }

    /// Get the type index associated with an exception object.
    #[cfg(feature = "gc")]
    pub(crate) fn exception_type_from_tag(&self, tag: TagIndex) -> EngineOrModuleTypeIndex {
        self.module.tags[tag].exception
    }

    /// Get the parameter arity of the associated function type for the given tag.
    pub(crate) fn tag_param_arity(&self, tag: TagIndex) -> usize {
        let func_ty = self.module.tags[tag].signature.unwrap_module_type_index();
        let func_ty = self
            .types
            .unwrap_func(func_ty)
            .expect("already validated to refer to a function type");
        func_ty.params().len()
    }

    /// Get the runtime instance ID and defined-tag ID in that
    /// instance for a particular static tag ID.
    #[cfg(feature = "gc")]
    pub(crate) fn get_instance_and_tag(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        tag_index: TagIndex,
    ) -> (ir::Value, ir::Value) {
        if let Some(defined_tag_index) = self.module.defined_tag_index(tag_index) {
            // Our own tag -- we only need to get our instance ID.
            let builtin = self.builtin_functions.get_instance_id(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let call = builder.ins().call(builtin, &[vmctx]);
            let instance_id = builder.func.dfg.inst_results(call)[0];
            let tag_id = builder
                .ins()
                .iconst(I32, i64::from(defined_tag_index.as_u32()));
            (instance_id, tag_id)
        } else {
            // An imported tag -- we need to load the VMTagImport struct.
            let vmctx_tag_vmctx_offset = self.offsets.vmctx_vmtag_import_vmctx(tag_index);
            let vmctx_tag_index_offset = self.offsets.vmctx_vmtag_import_index(tag_index);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let pointer_type = self.pointer_type();
            let from_vmctx = builder.ins().load(
                pointer_type,
                MemFlagsData::trusted().with_readonly(),
                vmctx,
                i32::try_from(vmctx_tag_vmctx_offset).unwrap(),
            );
            let index = builder.ins().load(
                I32,
                MemFlagsData::trusted().with_readonly(),
                vmctx,
                i32::try_from(vmctx_tag_index_offset).unwrap(),
            );
            let builtin = self.builtin_functions.get_instance_id(builder.func);
            let call = builder.ins().call(builtin, &[from_vmctx]);
            let from_instance_id = builder.func.dfg.inst_results(call)[0];
            (from_instance_id, index)
        }
    }
}

struct Call<'a, 'func, 'module_env> {
    builder: &'a mut FunctionBuilder<'func>,
    env: &'a mut FuncEnvironment<'module_env>,
    srcloc: ir::SourceLoc,
    tail: bool,
}

enum CheckIndirectCallTypeSignature {
    Runtime,
    StaticMatch {
        /// Whether or not the funcref may be null or if it's statically known
        /// to not be null.
        may_be_null: bool,
    },
    StaticTrap,
}

type CallRets = SmallVec<[ir::Value; 4]>;

impl<'a, 'func, 'module_env> Call<'a, 'func, 'module_env> {
    /// Create a new `Call` site that will do regular, non-tail calls.
    pub fn new(
        builder: &'a mut FunctionBuilder<'func>,
        env: &'a mut FuncEnvironment<'module_env>,
        srcloc: ir::SourceLoc,
    ) -> Self {
        Call {
            builder,
            env,
            srcloc,
            tail: false,
        }
    }

    /// Create a new `Call` site that will perform tail calls.
    pub fn new_tail(
        builder: &'a mut FunctionBuilder<'func>,
        env: &'a mut FuncEnvironment<'module_env>,
        srcloc: ir::SourceLoc,
    ) -> Self {
        Call {
            builder,
            env,
            srcloc,
            tail: true,
        }
    }

    /// Do a Wasm-level direct call to the given callee function.
    pub fn direct_call(
        mut self,
        callee_index: FuncIndex,
        sig_ref: ir::SigRef,
        wasm_call_args: &[ir::Value],
    ) -> WasmResult<CallRets> {
        let mut real_call_args = Vec::with_capacity(wasm_call_args.len() + 2);
        let caller_vmctx = self
            .builder
            .func
            .special_param(ArgumentPurpose::VMContext)
            .unwrap();

        // Handle direct calls to locally-defined functions.
        if let Some(def_func_index) = self.env.module.defined_func_index(callee_index) {
            // First append the callee vmctx address, which is the same as the caller vmctx in
            // this case.
            real_call_args.push(caller_vmctx);

            // Then append the caller vmctx address.
            real_call_args.push(caller_vmctx);

            // Then append the regular call arguments.
            real_call_args.extend_from_slice(wasm_call_args);

            // Finally, make the direct call!
            let callee = self
                .env
                .get_or_create_defined_func_ref(self.builder.func, def_func_index);
            return Ok(self.direct_call_inst(callee, &real_call_args));
        }

        // Handle direct calls to imported functions. We use an indirect call
        // so that we don't have to patch the code at runtime.
        let pointer_type = self.env.pointer_type();
        let vmctx = self.env.vmctx(self.builder.func);
        let base = self.builder.ins().global_value(pointer_type, vmctx);

        let mem_flags = ir::MemFlagsData::trusted().with_readonly().with_can_move();

        // Load the callee address.
        let body_offset = i32::try_from(
            self.env
                .offsets
                .vmctx_vmfunction_import_wasm_call(callee_index),
        )
        .unwrap();

        // First append the callee vmctx address.
        let vmctx_offset =
            i32::try_from(self.env.offsets.vmctx_vmfunction_import_vmctx(callee_index)).unwrap();
        let callee_vmctx = self
            .builder
            .ins()
            .load(pointer_type, mem_flags, base, vmctx_offset);
        real_call_args.push(callee_vmctx);
        real_call_args.push(caller_vmctx);

        // Then append the Wasm call arguments.
        real_call_args.extend_from_slice(wasm_call_args);

        // If we statically know the imported function (e.g. this is a
        // component-to-component call where we statically know both components)
        // then we can avoid doing an indirect call.
        match self.env.translation.known_imported_functions[callee_index].as_ref() {
            // The import is always a compile-time builtin intrinsic. Make a
            // direct call to that function (presumably it will eventually be
            // inlined).
            #[cfg(feature = "component-model")]
            Some(FuncKey::UnsafeIntrinsic(abi, intrinsic)) => {
                let callee = self
                    .env
                    .get_or_create_imported_func_ref(self.builder.func, callee_index);
                if self.can_directly_inline_unsafe_intrinsic(*abi) {
                    let result = super::compiler::component::UnsafeIntrinsicCompiler {
                        cursor: self.builder.cursor(),
                        isa: self.env.isa,
                        ptr: &self.env.offsets.ptr,
                    }
                    .translate(*intrinsic, &real_call_args)
                    .unwrap();
                    Ok(result.into_iter().collect())
                } else {
                    Ok(self.direct_call_inst(callee, &real_call_args))
                }
            }

            // The import is always satisfied with the given defined Wasm
            // function, so do a direct call to that function! (Although we take
            // care to still pass its `funcref`'s `vmctx` as the callee `vmctx`
            // in `real_call_args` and not the caller's.)
            Some(FuncKey::DefinedWasmFunction(..)) => {
                let callee = self
                    .env
                    .get_or_create_imported_func_ref(self.builder.func, callee_index);
                Ok(self.direct_call_inst(callee, &real_call_args))
            }

            Some(key) => panic!("unexpected kind of known-import function: {key:?}"),

            // Unknown import function or this module is instantiated many times
            // and with different functions. Either way, we have to do the
            // indirect call.
            None => {
                let func_addr = self
                    .builder
                    .ins()
                    .load(pointer_type, mem_flags, base, body_offset);
                Ok(self.indirect_call_inst(sig_ref, func_addr, &real_call_args))
            }
        }
    }

    /// Determines if a direct inline-during-translation is possible for a call
    /// made to an `UnsafeIntrinsic`.
    ///
    /// This only happens in "normal" circumstances where it's considered safe
    /// to bypass the otherwise off-by-default Cranelift inliner that Wasmtime
    /// has. This is a performance optimization to avoid needing to turn on all
    /// of inlining to get the performance benefit of inlining unsafe
    /// intrinsics. The fallback of issuing a `call` to the intrinsic is always
    /// suitable to do and is used in situations where the call instruction may
    /// have extra context.
    #[cfg(feature = "component-model")]
    fn can_directly_inline_unsafe_intrinsic(&self, abi: wasmtime_environ::Abi) -> bool {
        abi == wasmtime_environ::Abi::Wasm && !self.tail && !self.env.tunables.debug_guest
    }

    /// Do a Wasm-level indirect call through the given funcref table.
    pub fn indirect_call(
        mut self,
        features: &WasmFeatures,
        table_index: TableIndex,
        ty_index: TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<Option<CallRets>> {
        let (code_ptr, callee_vmctx) = match self.check_and_load_code_and_callee_vmctx(
            features,
            table_index,
            ty_index,
            callee,
            false,
        )? {
            Some(pair) => pair,
            None => return Ok(None),
        };

        self.unchecked_call_impl(sig_ref, code_ptr, callee_vmctx, call_args)
            .map(Some)
    }

    fn check_and_load_code_and_callee_vmctx(
        &mut self,
        features: &WasmFeatures,
        table_index: TableIndex,
        ty_index: TypeIndex,
        callee: ir::Value,
        cold_blocks: bool,
    ) -> WasmResult<Option<(ir::Value, ir::Value)>> {
        // Get the funcref pointer from the table.
        let funcref_ptr =
            self.env
                .table_get_funcref(self.builder, table_index, callee, cold_blocks);

        // If necessary, check the signature.
        let check =
            self.check_indirect_call_type_signature(features, table_index, ty_index, funcref_ptr);

        let trap_code = match check {
            // `funcref_ptr` is checked at runtime that its type matches,
            // meaning that if code gets this far it's guaranteed to not be
            // null. That means nothing in `unchecked_call` can fail.
            CheckIndirectCallTypeSignature::Runtime => None,

            // No type check was performed on `funcref_ptr` because it's
            // statically known to have the right type. Note that whether or
            // not the function is null is not necessarily tested so far since
            // no type information was inspected.
            //
            // If the table may hold null functions, then further loads in
            // `unchecked_call` may fail. If the table only holds non-null
            // functions, though, then there's no possibility of a trap.
            CheckIndirectCallTypeSignature::StaticMatch { may_be_null } => {
                if may_be_null {
                    Some(crate::TRAP_INDIRECT_CALL_TO_NULL)
                } else {
                    None
                }
            }

            // Code has already trapped, so return nothing indicating that this
            // is now unreachable code.
            CheckIndirectCallTypeSignature::StaticTrap => return Ok(None),
        };

        Ok(Some(self.load_code_and_vmctx(funcref_ptr, trap_code)))
    }

    fn check_indirect_call_type_signature(
        &mut self,
        features: &WasmFeatures,
        table_index: TableIndex,
        ty_index: TypeIndex,
        funcref_ptr: ir::Value,
    ) -> CheckIndirectCallTypeSignature {
        let table = &self.env.module.tables[table_index];
        let sig_id_size = self.env.offsets.size_of_vmshared_type_index();
        let sig_id_type = Type::int(u16::from(sig_id_size) * 8).unwrap();

        // Test if a type check is necessary for this table. If this table is a
        // table of typed functions and that type matches `ty_index`, then
        // there's no need to perform a typecheck.
        match table.ref_type.heap_type {
            // Functions do not have a statically known type in the table, a
            // typecheck is required. Fall through to below to perform the
            // actual typecheck.
            WasmHeapType::Func => {}

            // Functions that have a statically known type are either going to
            // always succeed or always fail. Figure out by inspecting the types
            // further.
            WasmHeapType::ConcreteFunc(EngineOrModuleTypeIndex::Module(table_ty)) => {
                // If `ty_index` matches `table_ty`, then this call is
                // statically known to have the right type, so no checks are
                // necessary.
                let specified_ty = self.env.module.types[ty_index].unwrap_module_type_index();
                if specified_ty == table_ty {
                    return CheckIndirectCallTypeSignature::StaticMatch {
                        may_be_null: table.ref_type.nullable,
                    };
                }

                if features.gc() {
                    // If we are in the Wasm GC world, then we need to perform
                    // an actual subtype check at runtime. Fall through to below
                    // to do that.
                } else {
                    // Otherwise if the types don't match then either (a) this
                    // is a null pointer or (b) it's a pointer with the wrong
                    // type. Figure out which and trap here.
                    //
                    // If it's possible to have a null here then try to load the
                    // type information. If that fails due to the function being
                    // a null pointer, then this was a call to null. Otherwise
                    // if it succeeds then we know it won't match, so trap
                    // anyway.
                    if table.ref_type.nullable {
                        if self.env.clif_memory_traps_enabled() {
                            self.builder.ins().load(
                                sig_id_type,
                                ir::MemFlagsData::trusted()
                                    .with_readonly()
                                    .with_trap_code(Some(crate::TRAP_INDIRECT_CALL_TO_NULL)),
                                funcref_ptr,
                                i32::from(self.env.offsets.ptr.vm_func_ref_type_index()),
                            );
                        } else {
                            self.env.trapz(
                                self.builder,
                                funcref_ptr,
                                crate::TRAP_INDIRECT_CALL_TO_NULL,
                            );
                        }
                    }
                    self.env.trap(self.builder, crate::TRAP_BAD_SIGNATURE);
                    return CheckIndirectCallTypeSignature::StaticTrap;
                }
            }

            // Tables of `nofunc` can only be inhabited by null, so go ahead and
            // trap with that.
            WasmHeapType::NoFunc => {
                assert!(table.ref_type.nullable);
                self.env
                    .trap(self.builder, crate::TRAP_INDIRECT_CALL_TO_NULL);
                return CheckIndirectCallTypeSignature::StaticTrap;
            }

            // Engine-indexed types don't show up until runtime and it's a Wasm
            // validation error to perform a call through a non-function table,
            // so these cases are dynamically not reachable.
            WasmHeapType::ConcreteFunc(EngineOrModuleTypeIndex::Engine(_))
            | WasmHeapType::ConcreteFunc(EngineOrModuleTypeIndex::RecGroup(_))
            | WasmHeapType::Extern
            | WasmHeapType::NoExtern
            | WasmHeapType::Any
            | WasmHeapType::Eq
            | WasmHeapType::I31
            | WasmHeapType::Array
            | WasmHeapType::ConcreteArray(_)
            | WasmHeapType::Struct
            | WasmHeapType::ConcreteStruct(_)
            | WasmHeapType::Exn
            | WasmHeapType::ConcreteExn(_)
            | WasmHeapType::NoExn
            | WasmHeapType::Cont
            | WasmHeapType::ConcreteCont(_)
            | WasmHeapType::NoCont
            | WasmHeapType::None => {
                unreachable!()
            }
        }

        // Load the caller's `VMSharedTypeIndex.
        let interned_ty = self.env.module.types[ty_index].unwrap_module_type_index();
        let caller_sig_id = self
            .env
            .module_interned_to_shared_ty(&mut self.builder.cursor(), interned_ty);

        // Load the callee's `VMSharedTypeIndex`.
        //
        // Note that the callee may be null in which case this load may
        // trap. If so use the `TRAP_INDIRECT_CALL_TO_NULL` trap code.
        let mut mem_flags = ir::MemFlagsData::trusted().with_readonly();
        if self.env.clif_memory_traps_enabled() {
            mem_flags = mem_flags.with_trap_code(Some(crate::TRAP_INDIRECT_CALL_TO_NULL));
        } else {
            self.env
                .trapz(self.builder, funcref_ptr, crate::TRAP_INDIRECT_CALL_TO_NULL);
        }
        let callee_sig_id =
            self.env
                .load_funcref_type_index(&mut self.builder.cursor(), mem_flags, funcref_ptr);

        // Check that they match: in the case of Wasm GC, this means doing a
        // full subtype check. Otherwise, we do a simple equality check.
        let matches = if features.gc() {
            #[cfg(feature = "gc")]
            {
                self.env
                    .is_subtype(self.builder, callee_sig_id, caller_sig_id)
            }
            #[cfg(not(feature = "gc"))]
            {
                unreachable!()
            }
        } else {
            self.builder
                .ins()
                .icmp(IntCC::Equal, callee_sig_id, caller_sig_id)
        };
        self.env
            .trapz(self.builder, matches, crate::TRAP_BAD_SIGNATURE);
        CheckIndirectCallTypeSignature::Runtime
    }

    /// Call a typed function reference.
    pub fn call_ref(
        self,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        args: &[ir::Value],
    ) -> WasmResult<CallRets> {
        // FIXME: the wasm type system tracks enough information to know whether
        // `callee` is a null reference or not. In some situations it can be
        // statically known here that `callee` cannot be null in which case this
        // can be `None` instead. This requires feeding type information from
        // wasmparser's validator into this function, however, which is not
        // easily done at this time.
        let callee_load_trap_code = Some(crate::TRAP_NULL_REFERENCE);

        self.unchecked_call(sig_ref, callee, callee_load_trap_code, args)
    }

    /// This calls a function by reference without checking the signature.
    ///
    /// It gets the function address, sets relevant flags, and passes the
    /// special callee/caller vmctxs. It is used by both call_indirect (which
    /// checks the signature) and call_ref (which doesn't).
    fn unchecked_call(
        mut self,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        callee_load_trap_code: Option<ir::TrapCode>,
        call_args: &[ir::Value],
    ) -> WasmResult<CallRets> {
        let (func_addr, callee_vmctx) = self.load_code_and_vmctx(callee, callee_load_trap_code);
        self.unchecked_call_impl(sig_ref, func_addr, callee_vmctx, call_args)
    }

    fn load_code_and_vmctx(
        &mut self,
        callee: ir::Value,
        callee_load_trap_code: Option<ir::TrapCode>,
    ) -> (ir::Value, ir::Value) {
        let pointer_type = self.env.pointer_type();

        // Dereference callee pointer to get the function address.
        //
        // Note that this may trap if `callee` hasn't previously been verified
        // to be non-null. This means that this load is annotated with an
        // optional trap code provided by the caller of `unchecked_call` which
        // will handle the case where this is either already known to be
        // non-null or may trap.
        let mem_flags = ir::MemFlagsData::trusted().with_readonly();
        let mut callee_flags = mem_flags;
        if self.env.clif_memory_traps_enabled() {
            callee_flags = callee_flags.with_trap_code(callee_load_trap_code);
        } else {
            if let Some(trap) = callee_load_trap_code {
                self.env.trapz(self.builder, callee, trap);
            }
        }
        let func_addr = self.builder.ins().load(
            pointer_type,
            callee_flags,
            callee,
            i32::from(self.env.offsets.ptr.vm_func_ref_wasm_call()),
        );
        let callee_vmctx = self.builder.ins().load(
            pointer_type,
            mem_flags,
            callee,
            i32::from(self.env.offsets.ptr.vm_func_ref_vmctx()),
        );

        (func_addr, callee_vmctx)
    }

    fn caller_vmctx(&self) -> ir::Value {
        self.builder
            .func
            .special_param(ArgumentPurpose::VMContext)
            .unwrap()
    }

    /// This calls a function by reference without checking the
    /// signature, given the raw code pointer to the
    /// Wasm-calling-convention entry point and the callee vmctx.
    fn unchecked_call_impl(
        mut self,
        sig_ref: ir::SigRef,
        func_addr: ir::Value,
        callee_vmctx: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<CallRets> {
        let mut real_call_args = Vec::with_capacity(call_args.len() + 2);
        let caller_vmctx = self.caller_vmctx();

        // First append the callee and caller vmctx addresses.
        real_call_args.push(callee_vmctx);
        real_call_args.push(caller_vmctx);

        // Then append the regular call arguments.
        real_call_args.extend_from_slice(call_args);

        Ok(self.indirect_call_inst(sig_ref, func_addr, &real_call_args))
    }

    fn exception_table(
        &mut self,
        sig: ir::SigRef,
    ) -> Option<(ir::ExceptionTable, Block, CallRets)> {
        if !self.tail && !self.env.stacks.handlers.is_empty() {
            let continuation_block = self.builder.create_block();
            let mut args = vec![];
            let mut results = smallvec![];
            for i in 0..self.builder.func.dfg.signatures[sig].returns.len() {
                let ty = self.builder.func.dfg.signatures[sig].returns[i].value_type;
                results.push(
                    self.builder
                        .func
                        .dfg
                        .append_block_param(continuation_block, ty),
                );
                args.push(BlockArg::TryCallRet(u32::try_from(i).unwrap()));
            }

            let continuation = self
                .builder
                .func
                .dfg
                .block_call(continuation_block, args.iter());
            let mut handlers = vec![ExceptionTableItem::Context(self.caller_vmctx())];
            for (tag, block) in self.env.stacks.handlers.handlers() {
                let block_call = self
                    .builder
                    .func
                    .dfg
                    .block_call(block, &[BlockArg::TryCallExn(0)]);
                handlers.push(match tag {
                    Some(tag) => ExceptionTableItem::Tag(tag, block_call),
                    None => ExceptionTableItem::Default(block_call),
                });
            }
            let etd = ExceptionTableData::new(sig, continuation, handlers);
            let et = self.builder.func.dfg.exception_tables.push(etd);
            Some((et, continuation_block, results))
        } else {
            None
        }
    }

    fn results_from_call_inst(&self, inst: ir::Inst) -> CallRets {
        self.builder
            .func
            .dfg
            .inst_results(inst)
            .iter()
            .copied()
            .collect()
    }

    fn handle_call_result_stackmap(&mut self, results: &[ir::Value], sig_ref: ir::SigRef) {
        for (i, &val) in results.iter().enumerate() {
            if self.env.sig_ref_result_needs_stack_map(sig_ref, i) {
                self.builder.declare_value_needs_stack_map(val);
            }
        }
    }

    fn direct_call_inst(&mut self, callee: ir::FuncRef, args: &[ir::Value]) -> CallRets {
        let sig_ref = self.builder.func.dfg.ext_funcs[callee].signature;
        if self.tail {
            self.builder.ins().return_call(callee, args);
            smallvec![]
        } else if let Some((exception_table, continuation_block, results)) =
            self.exception_table(sig_ref)
        {
            let inst = self.builder.ins().try_call(callee, args, exception_table);
            self.handle_call_result_stackmap(&results, sig_ref);
            self.builder.switch_to_block(continuation_block);
            self.builder.seal_block(continuation_block);
            self.attach_tags(inst);
            results
        } else {
            let inst = self.builder.ins().call(callee, args);
            let results = self.results_from_call_inst(inst);
            self.handle_call_result_stackmap(&results, sig_ref);
            self.attach_tags(inst);
            results
        }
    }

    fn indirect_call_inst(
        &mut self,
        sig_ref: ir::SigRef,
        func_addr: ir::Value,
        args: &[ir::Value],
    ) -> CallRets {
        if self.tail {
            self.builder
                .ins()
                .return_call_indirect(sig_ref, func_addr, args);
            smallvec![]
        } else if let Some((exception_table, continuation_block, results)) =
            self.exception_table(sig_ref)
        {
            let inst = self
                .builder
                .ins()
                .try_call_indirect(func_addr, args, exception_table);
            self.handle_call_result_stackmap(&results, sig_ref);
            self.builder.switch_to_block(continuation_block);
            self.builder.seal_block(continuation_block);
            self.attach_tags(inst);
            results
        } else {
            let inst = self.builder.ins().call_indirect(sig_ref, func_addr, args);
            let results = self.results_from_call_inst(inst);
            self.handle_call_result_stackmap(&results, sig_ref);
            self.attach_tags(inst);
            results
        }
    }

    fn attach_tags(&mut self, inst: ir::Inst) {
        let tags = self.env.debug_tags(self.srcloc);
        if !tags.is_empty() {
            self.builder.func.debug_tags.set(inst, tags);
        }
    }
}

impl TypeConvert for FuncEnvironment<'_> {
    fn lookup_heap_type(&self, ty: wasmparser::UnpackedIndex) -> WasmHeapType {
        wasmtime_environ::WasmparserTypeConverter::new(self.types, |idx| {
            self.module.types[idx].unwrap_module_type_index()
        })
        .lookup_heap_type(ty)
    }

    fn lookup_type_index(&self, index: wasmparser::UnpackedIndex) -> EngineOrModuleTypeIndex {
        wasmtime_environ::WasmparserTypeConverter::new(self.types, |idx| {
            self.module.types[idx].unwrap_module_type_index()
        })
        .lookup_type_index(index)
    }
}

impl<'module_environment> TargetEnvironment for FuncEnvironment<'module_environment> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.isa.frontend_config()
    }

    fn reference_type(&self, wasm_ty: WasmHeapType) -> (ir::Type, bool) {
        let ty = crate::reference_type(wasm_ty, self.pointer_type());
        let needs_stack_map = self.heap_ty_needs_stack_map(wasm_ty);
        (ty, needs_stack_map)
    }

    fn heap_access_spectre_mitigation(&self) -> bool {
        self.isa.flags().enable_heap_access_spectre_mitigation()
    }

    fn tunables(&self) -> &Tunables {
        self.compiler.tunables()
    }
}

impl FuncEnvironment<'_> {
    pub fn heaps(&self) -> &PrimaryMap<Heap, HeapData> {
        &self.heaps
    }

    pub fn is_wasm_parameter(&self, index: usize) -> bool {
        // The first two parameters are the vmctx and caller vmctx. The rest are
        // the wasm parameters.
        index >= 2
    }

    pub fn clif_param_as_wasm_param(&self, index: usize) -> Option<WasmValType> {
        if index >= 2 {
            Some(self.wasm_func_ty.params()[index - 2])
        } else {
            None
        }
    }

    pub fn param_needs_stack_map(&self, _signature: &ir::Signature, index: usize) -> bool {
        // Skip the caller and callee vmctx.
        if index < 2 {
            return false;
        }

        self.wasm_func_ty.params()[index - 2].is_vmgcref_type_and_not_i31()
    }

    pub fn sig_ref_result_needs_stack_map(&self, sig_ref: ir::SigRef, index: usize) -> bool {
        let wasm_func_ty = self.sig_ref_to_ty[sig_ref].as_ref().unwrap();
        wasm_func_ty.results()[index].is_vmgcref_type_and_not_i31()
    }

    pub fn translate_table_grow(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        table_index: TableIndex,
        delta: ir::Value,
        init_value: ir::Value,
    ) -> WasmResult<ir::Value> {
        let mut pos = builder.cursor();
        let table = self.table(table_index);
        let ty = table.ref_type.heap_type;
        let (table_vmctx, defined_table_index) =
            self.table_vmctx_and_defined_index(&mut pos, table_index);
        let index_type = table.idx_type;
        let delta = self.cast_index_to_i64(&mut pos, delta, index_type);

        let mut args: SmallVec<[_; 6]> = smallvec![table_vmctx, defined_table_index, delta];
        let grow = match ty.top() {
            WasmHeapTopType::Extern | WasmHeapTopType::Any | WasmHeapTopType::Exn => {
                args.push(init_value);
                gc::builtins::table_grow_gc_ref(self, pos.func)?
            }
            WasmHeapTopType::Func => {
                args.push(init_value);
                self.builtin_functions.table_grow_func_ref(pos.func)
            }
            WasmHeapTopType::Cont => {
                let (revision, contref) =
                    stack_switching::fatpointer::deconstruct(self, &mut pos, init_value);
                args.extend_from_slice(&[contref, revision]);
                stack_switching::builtins::table_grow_cont_obj(self, pos.func)?
            }
        };

        let call_inst = pos.ins().call(grow, &args);
        let result = builder.func.dfg.first_result(call_inst);

        Ok(self.convert_pointer_to_index_type(builder.cursor(), result, index_type, false))
    }

    pub fn translate_table_get(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        index: ir::Value,
    ) -> WasmResult<ir::Value> {
        let table = self.module.tables[table_index];
        let table_data = self.get_or_create_table(builder.func, table_index);
        let heap_ty = table.ref_type.heap_type;
        match heap_ty.top() {
            // GC-managed types.
            WasmHeapTopType::Any | WasmHeapTopType::Extern | WasmHeapTopType::Exn => {
                let (src, flags) = table_data.prepare_table_addr(self, builder, index);
                gc::gc_compiler(self)?.translate_read_gc_reference(
                    self,
                    builder,
                    table.ref_type,
                    src,
                    flags,
                )
            }

            // Function types.
            WasmHeapTopType::Func => Ok(self.table_get_funcref(builder, table_index, index, false)),

            // Continuation types.
            WasmHeapTopType::Cont => {
                let (elem_addr, flags) = table_data.prepare_table_addr(self, builder, index);
                Ok(builder.ins().load(
                    stack_switching::fatpointer::fatpointer_type(self),
                    flags,
                    elem_addr,
                    0,
                ))
            }
        }
    }

    pub fn translate_table_set(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        value: ir::Value,
        index: ir::Value,
    ) -> WasmResult<()> {
        let table_data = self.get_or_create_table(builder.func, table_index);
        let (dst, flags) = table_data.prepare_table_addr(self, builder, index);
        self.emit_table_set(builder, table_index, dst, flags, value)
    }

    /// Helper to store `value` into the table address at `addr` using `flags`.
    ///
    /// This assumes that `addr` is a native address and is already
    /// bounds-checked. Additionally `value` must be appropriately typed.
    fn emit_table_set(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        addr: ir::Value,
        flags: ir::MemFlagsData,
        value: ir::Value,
    ) -> WasmResult<()> {
        let table = self.module.tables[table_index];
        match table.ref_type.heap_type.top() {
            // GC-managed types.
            WasmHeapTopType::Any | WasmHeapTopType::Extern | WasmHeapTopType::Exn => {
                gc::gc_compiler(self)?.translate_write_gc_reference(
                    self,
                    builder,
                    table.ref_type,
                    addr,
                    value,
                    flags,
                )
            }

            // Function types.
            WasmHeapTopType::Func => {
                self.table_set_funcref(builder, value, addr, flags);
                Ok(())
            }

            // Continuation types.
            WasmHeapTopType::Cont => {
                builder.ins().store(flags, value, addr, 0);
                Ok(())
            }
        }
    }

    /// Helper to store the funcref `value` at the raw native address
    /// `elem_addr` using the `flags` specified.
    fn table_set_funcref(
        &mut self,
        builder: &mut FunctionBuilder,
        value: ir::Value,
        elem_addr: ir::Value,
        flags: ir::MemFlagsData,
    ) {
        // Set the "initialized bit". See doc-comment on
        // `FUNCREF_INIT_BIT` in
        // crates/environ/src/ref_bits.rs for details.
        let value_with_init_bit = if self.tunables.table_lazy_init {
            builder
                .ins()
                .bor_imm(value, Imm64::from(FUNCREF_INIT_BIT as i64))
        } else {
            value
        };
        builder
            .ins()
            .store(flags, value_with_init_bit, elem_addr, 0);
    }

    pub fn translate_table_fill(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        table_index: TableIndex,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let mut pos = builder.cursor();
        let table = self.table(table_index);
        let ty = table.ref_type.heap_type;
        let dst = self.cast_index_to_i64(&mut pos, dst, table.idx_type);
        let len = self.cast_index_to_i64(&mut pos, len, table.idx_type);
        let (table_vmctx, table_index) = self.table_vmctx_and_defined_index(&mut pos, table_index);

        let mut args: SmallVec<[_; 6]> = smallvec![table_vmctx, table_index, dst];
        let libcall = match ty.top() {
            WasmHeapTopType::Any | WasmHeapTopType::Extern | WasmHeapTopType::Exn => {
                args.push(val);
                gc::builtins::table_fill_gc_ref(self, &mut pos.func)?
            }
            WasmHeapTopType::Func => {
                args.push(val);
                self.builtin_functions.table_fill_func_ref(&mut pos.func)
            }
            WasmHeapTopType::Cont => {
                let (revision, contref) =
                    stack_switching::fatpointer::deconstruct(self, &mut pos, val);
                args.extend_from_slice(&[contref, revision]);
                stack_switching::builtins::table_fill_cont_obj(self, &mut pos.func)?
            }
        };

        args.push(len);
        builder.ins().call(libcall, &args);

        Ok(())
    }

    pub fn translate_ref_i31(
        &mut self,
        mut pos: FuncCursor,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        debug_assert_eq!(pos.func.dfg.value_type(val), ir::types::I32);
        let shifted = pos.ins().ishl_imm(val, 1);
        let tagged = pos
            .ins()
            .bor_imm(shifted, i64::from(crate::I31_REF_DISCRIMINANT));
        let (ref_ty, _needs_stack_map) = self.reference_type(WasmHeapType::I31);
        debug_assert_eq!(ref_ty, ir::types::I32);
        Ok(tagged)
    }

    pub fn translate_i31_get_s(
        &mut self,
        builder: &mut FunctionBuilder,
        i31ref: ir::Value,
    ) -> WasmResult<ir::Value> {
        // TODO: If we knew we have a `(ref i31)` here, instead of maybe a `(ref
        // null i31)`, we could omit the `trapz`. But plumbing that type info
        // from `wasmparser` and through to here is a bit funky.
        self.trapz(builder, i31ref, crate::TRAP_NULL_REFERENCE);
        Ok(builder.ins().sshr_imm(i31ref, 1))
    }

    pub fn translate_i31_get_u(
        &mut self,
        builder: &mut FunctionBuilder,
        i31ref: ir::Value,
    ) -> WasmResult<ir::Value> {
        // TODO: If we knew we have a `(ref i31)` here, instead of maybe a `(ref
        // null i31)`, we could omit the `trapz`. But plumbing that type info
        // from `wasmparser` and through to here is a bit funky.
        self.trapz(builder, i31ref, crate::TRAP_NULL_REFERENCE);
        Ok(builder.ins().ushr_imm(i31ref, 1))
    }

    pub fn struct_fields_len(&mut self, struct_type_index: TypeIndex) -> WasmResult<usize> {
        let ty = self.module.types[struct_type_index].unwrap_module_type_index();
        match &self.types[ty].composite_type.inner {
            WasmCompositeInnerType::Struct(s) => Ok(s.fields.len()),
            _ => unreachable!(),
        }
    }

    pub fn translate_struct_new(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
        fields: StructFieldsVec,
    ) -> WasmResult<ir::Value> {
        gc::translate_struct_new(self, builder, struct_type_index, &fields)
    }

    pub fn translate_struct_new_default(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
    ) -> WasmResult<ir::Value> {
        gc::translate_struct_new_default(self, builder, struct_type_index)
    }

    pub fn translate_struct_get(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
        field_index: u32,
        struct_ref: ir::Value,
        extension: Option<Extension>,
    ) -> WasmResult<ir::Value> {
        gc::translate_struct_get(
            self,
            builder,
            struct_type_index,
            field_index,
            struct_ref,
            extension,
        )
    }

    pub fn translate_struct_set(
        &mut self,
        builder: &mut FunctionBuilder,
        struct_type_index: TypeIndex,
        field_index: u32,
        struct_ref: ir::Value,
        value: ir::Value,
    ) -> WasmResult<()> {
        gc::translate_struct_set(
            self,
            builder,
            struct_type_index,
            field_index,
            struct_ref,
            value,
        )
    }

    pub fn translate_exn_unbox(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        tag_index: TagIndex,
        exn_ref: ir::Value,
    ) -> WasmResult<SmallVec<[ir::Value; 4]>> {
        gc::translate_exn_unbox(self, builder, tag_index, exn_ref)
    }

    pub fn translate_exn_throw(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        tag_index: TagIndex,
        args: &[ir::Value],
    ) -> WasmResult<()> {
        gc::translate_exn_throw(self, builder, tag_index, args)
    }

    pub fn translate_exn_throw_ref(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        exnref: ir::Value,
    ) -> WasmResult<()> {
        gc::translate_exn_throw_ref(self, builder, exnref)
    }

    pub fn translate_array_new(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        elem: ir::Value,
        len: ir::Value,
    ) -> WasmResult<ir::Value> {
        gc::translate_array_new(self, builder, array_type_index, elem, len)
    }

    pub fn translate_array_new_default(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        len: ir::Value,
    ) -> WasmResult<ir::Value> {
        gc::translate_array_new_default(self, builder, array_type_index, len)
    }

    pub fn translate_array_new_fixed(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        elems: &[ir::Value],
    ) -> WasmResult<ir::Value> {
        gc::translate_array_new_fixed(self, builder, array_type_index, elems)
    }

    pub fn translate_array_new_data(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        data_index: DataIndex,
        data_offset: ir::Value,
        len: ir::Value,
    ) -> WasmResult<ir::Value> {
        gc::translate_array_new_data(
            self,
            builder,
            array_type_index,
            data_index,
            data_offset,
            len,
        )
    }

    pub fn translate_array_new_elem(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        elem_index: ElemIndex,
        elem_offset: ir::Value,
        len: ir::Value,
    ) -> WasmResult<ir::Value> {
        let libcall = gc::builtins::array_new_elem(self, builder.func)?;
        let vmctx = self.vmctx_val(&mut builder.cursor());
        let interned_type_index = self.module.types[array_type_index].unwrap_module_type_index();
        let interned_type_index = builder
            .ins()
            .iconst(I32, i64::from(interned_type_index.as_u32()));
        let elem_index = builder.ins().iconst(I32, i64::from(elem_index.as_u32()));
        let call_inst = builder.ins().call(
            libcall,
            &[vmctx, interned_type_index, elem_index, elem_offset, len],
        );
        let array_ref = builder.func.dfg.first_result(call_inst);
        builder.declare_value_needs_stack_map(array_ref);
        Ok(array_ref)
    }

    pub fn translate_array_copy(
        &mut self,
        builder: &mut FunctionBuilder,
        dst_array_type_index: TypeIndex,
        dst_array: ir::Value,
        dst_index: ir::Value,
        src_array_type_index: TypeIndex,
        src_array: ir::Value,
        src_index: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        gc::translate_array_copy(
            self,
            builder,
            dst_array_type_index,
            dst_array,
            dst_index,
            src_array_type_index,
            src_array,
            src_index,
            len,
        )
    }

    pub fn translate_array_fill(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        index: ir::Value,
        value: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        gc::translate_array_fill(self, builder, array_type_index, array, index, value, len)
    }

    pub fn translate_array_init_data(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        dst_index: ir::Value,
        data_index: DataIndex,
        data_offset: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        gc::translate_array_init_data(
            self,
            builder,
            array_type_index,
            array,
            dst_index,
            data_index,
            data_offset,
            len,
        )
    }

    pub fn translate_array_init_elem(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        dst_index: ir::Value,
        elem_index: ElemIndex,
        elem_offset: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let libcall = gc::builtins::array_init_elem(self, builder.func)?;
        let vmctx = self.vmctx_val(&mut builder.cursor());
        let interned_type_index = self.module.types[array_type_index].unwrap_module_type_index();
        let interned_type_index = builder
            .ins()
            .iconst(I32, i64::from(interned_type_index.as_u32()));
        let elem_index = builder.ins().iconst(I32, i64::from(elem_index.as_u32()));
        builder.ins().call(
            libcall,
            &[
                vmctx,
                interned_type_index,
                array,
                dst_index,
                elem_index,
                elem_offset,
                len,
            ],
        );
        Ok(())
    }

    pub fn translate_array_len(
        &mut self,
        builder: &mut FunctionBuilder,
        array: ir::Value,
    ) -> WasmResult<ir::Value> {
        gc::translate_array_len(self, builder, array)
    }

    pub fn translate_array_get(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        index: ir::Value,
        extension: Option<Extension>,
    ) -> WasmResult<ir::Value> {
        gc::translate_array_get(self, builder, array_type_index, array, index, extension)
    }

    pub fn translate_array_set(
        &mut self,
        builder: &mut FunctionBuilder,
        array_type_index: TypeIndex,
        array: ir::Value,
        index: ir::Value,
        value: ir::Value,
    ) -> WasmResult<()> {
        gc::translate_array_set(self, builder, array_type_index, array, index, value)
    }

    pub fn translate_ref_test(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        test_ty: WasmRefType,
        gc_ref: ir::Value,
        gc_ref_ty: WasmRefType,
    ) -> WasmResult<ir::Value> {
        gc::translate_ref_test(self, builder, test_ty, gc_ref, gc_ref_ty)
    }

    pub fn translate_ref_null(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor,
        ht: WasmHeapType,
    ) -> WasmResult<ir::Value> {
        Ok(match ht.top() {
            WasmHeapTopType::Func => pos.ins().iconst(self.pointer_type(), 0),
            // NB: null GC references don't need to be in stack maps.
            WasmHeapTopType::Any | WasmHeapTopType::Extern | WasmHeapTopType::Exn => {
                pos.ins().iconst(types::I32, 0)
            }
            WasmHeapTopType::Cont => {
                let zero = pos.ins().iconst(self.pointer_type(), 0);
                stack_switching::fatpointer::construct(self, &mut pos, zero, zero)
            }
        })
    }

    pub fn translate_ref_is_null(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor,
        value: ir::Value,
        ty: WasmRefType,
    ) -> WasmResult<ir::Value> {
        // If we know the type is not nullable, then we don't actually need to
        // check for null.
        if !ty.nullable {
            return Ok(pos.ins().iconst(ir::types::I32, 0));
        }

        let byte_is_null = match ty.heap_type.top() {
            WasmHeapTopType::Cont => {
                let (_revision, contref) =
                    stack_switching::fatpointer::deconstruct(self, &mut pos, value);
                pos.ins()
                    .icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, contref, 0)
            }
            _ => pos
                .ins()
                .icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, value, 0),
        };

        Ok(pos.ins().uextend(ir::types::I32, byte_is_null))
    }

    pub fn translate_ref_func(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor<'_>,
        func_index: FuncIndex,
    ) -> WasmResult<ir::Value> {
        let func_index = pos.ins().iconst(I32, func_index.as_u32() as i64);
        let ref_func = self.builtin_functions.ref_func(&mut pos.func);
        let vmctx = self.vmctx_val(&mut pos);

        let call_inst = pos.ins().call(ref_func, &[vmctx, func_index]);
        Ok(pos.func.dfg.first_result(call_inst))
    }

    pub(crate) fn translate_global_get(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        global_index: GlobalIndex,
    ) -> WasmResult<ir::Value> {
        match self.get_or_create_global(builder.func, global_index) {
            GlobalVariable::Constant { value } => match value {
                GlobalConstValue::I32(x) => Ok(builder.ins().iconst(ir::types::I32, i64::from(x))),
                GlobalConstValue::I64(x) => Ok(builder.ins().iconst(ir::types::I64, x)),
                GlobalConstValue::F32(x) => {
                    Ok(builder.ins().f32const(ir::immediates::Ieee32::with_bits(x)))
                }
                GlobalConstValue::F64(x) => {
                    Ok(builder.ins().f64const(ir::immediates::Ieee64::with_bits(x)))
                }
                GlobalConstValue::V128(x) => {
                    let data = x.to_le_bytes().to_vec().into();
                    let handle = builder.func.dfg.constants.insert(data);
                    Ok(builder.ins().vconst(ir::types::I8X16, handle))
                }
            },
            GlobalVariable::Memory { gv, offset, ty } => {
                let addr = builder.ins().global_value(self.pointer_type(), gv);
                let mut flags = ir::MemFlagsData::trusted();
                // Store vector globals in little-endian format to avoid
                // byte swaps on big-endian platforms since at-rest vectors
                // should already be in little-endian format anyway.
                if ty.is_vector() {
                    flags.set_endianness(ir::Endianness::Little);
                }
                // Put globals in the "table" abstract heap category as well.
                flags.set_alias_region(Some(ir::AliasRegion::Table));
                Ok(builder.ins().load(ty, flags, addr, offset))
            }
            GlobalVariable::Custom => {
                let global_ty = self.module.globals[global_index];
                let wasm_ty = global_ty.wasm_ty;
                debug_assert!(
                    wasm_ty.is_vmgcref_type(),
                    "We only use GlobalVariable::Custom for VMGcRef types"
                );
                let WasmValType::Ref(ref_ty) = wasm_ty else {
                    unreachable!()
                };

                let (gv, offset) = self.get_global_location(builder.func, global_index);
                let gv = builder.ins().global_value(self.pointer_type(), gv);
                let src = builder.ins().iadd_imm(gv, i64::from(offset));

                let flags = if global_ty.mutability || gc::gc_compiler(self)?.is_moving_collector()
                {
                    ir::MemFlagsData::trusted()
                } else {
                    ir::MemFlagsData::trusted().with_readonly().with_can_move()
                };
                gc::gc_compiler(self)?
                    .translate_read_gc_reference(self, builder, ref_ty, src, flags)
            }
        }
    }

    pub(crate) fn translate_global_set(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        global_index: GlobalIndex,
        val: ir::Value,
    ) -> WasmResult<()> {
        match self.get_or_create_global(builder.func, global_index) {
            GlobalVariable::Constant { .. } => {
                unreachable!("validation checks that Wasm cannot `global.set` constant globals")
            }
            GlobalVariable::Memory { gv, offset, ty } => {
                let addr = builder.ins().global_value(self.pointer_type(), gv);
                let mut flags = ir::MemFlagsData::trusted();
                // Like `global.get`, store globals in little-endian format.
                if ty.is_vector() {
                    flags.set_endianness(ir::Endianness::Little);
                }
                // Put globals in the "table" abstract heap category as well.
                flags.set_alias_region(Some(ir::AliasRegion::Table));
                debug_assert_eq!(ty, builder.func.dfg.value_type(val));
                builder.ins().store(flags, val, addr, offset);
                self.update_global(builder, global_index, val);
            }
            GlobalVariable::Custom => {
                let ty = self.module.globals[global_index].wasm_ty;
                debug_assert!(
                    ty.is_vmgcref_type(),
                    "We only use GlobalVariable::Custom for VMGcRef types"
                );
                let WasmValType::Ref(ty) = ty else {
                    unreachable!()
                };

                let (gv, offset) = self.get_global_location(builder.func, global_index);
                let gv = builder.ins().global_value(self.pointer_type(), gv);
                let src = builder.ins().iadd_imm(gv, i64::from(offset));

                gc::gc_compiler(self)?.translate_write_gc_reference(
                    self,
                    builder,
                    ty,
                    src,
                    val,
                    ir::MemFlagsData::trusted(),
                )?
            }
        }
        Ok(())
    }

    pub fn translate_call_indirect<'a>(
        &mut self,
        builder: &'a mut FunctionBuilder,
        srcloc: ir::SourceLoc,
        features: &WasmFeatures,
        table_index: TableIndex,
        ty_index: TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<Option<CallRets>> {
        Call::new(builder, self, srcloc).indirect_call(
            features,
            table_index,
            ty_index,
            sig_ref,
            callee,
            call_args,
        )
    }

    pub fn translate_call<'a>(
        &mut self,
        builder: &'a mut FunctionBuilder,
        srcloc: ir::SourceLoc,
        callee_index: FuncIndex,
        sig_ref: ir::SigRef,
        call_args: &[ir::Value],
    ) -> WasmResult<CallRets> {
        Call::new(builder, self, srcloc).direct_call(callee_index, sig_ref, call_args)
    }

    pub fn translate_call_ref<'a>(
        &mut self,
        builder: &'a mut FunctionBuilder,
        srcloc: ir::SourceLoc,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<CallRets> {
        Call::new(builder, self, srcloc).call_ref(sig_ref, callee, call_args)
    }

    pub fn translate_return_call(
        &mut self,
        builder: &mut FunctionBuilder,
        srcloc: ir::SourceLoc,
        callee_index: FuncIndex,
        sig_ref: ir::SigRef,
        call_args: &[ir::Value],
    ) -> WasmResult<()> {
        Call::new_tail(builder, self, srcloc).direct_call(callee_index, sig_ref, call_args)?;
        Ok(())
    }

    pub fn translate_return_call_indirect(
        &mut self,
        builder: &mut FunctionBuilder,
        srcloc: ir::SourceLoc,
        features: &WasmFeatures,
        table_index: TableIndex,
        ty_index: TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<()> {
        Call::new_tail(builder, self, srcloc).indirect_call(
            features,
            table_index,
            ty_index,
            sig_ref,
            callee,
            call_args,
        )?;
        Ok(())
    }

    pub fn translate_return_call_ref(
        &mut self,
        builder: &mut FunctionBuilder,
        srcloc: ir::SourceLoc,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<()> {
        Call::new_tail(builder, self, srcloc).call_ref(sig_ref, callee, call_args)?;
        Ok(())
    }

    /// Returns two `ir::Value`s, the first of which is the vmctx for the memory
    /// `index` and the second of which is the `DefinedMemoryIndex` for `index`.
    ///
    /// Handles internally whether `index` is an imported memory or not.
    fn memory_vmctx_and_defined_index(
        &mut self,
        pos: &mut FuncCursor,
        index: MemoryIndex,
    ) -> (ir::Value, ir::Value) {
        let cur_vmctx = self.vmctx_val(pos);
        match self.module.defined_memory_index(index) {
            // This is a defined memory, so the vmctx is our own and the defined
            // index is `index` here.
            Some(index) => (cur_vmctx, pos.ins().iconst(I32, i64::from(index.as_u32()))),

            // This is an imported memory, so load the vmctx/defined index from
            // the import definition itself.
            None => {
                let vmimport = self.offsets.vmctx_vmmemory_import(index);

                let vmctx = pos.ins().load(
                    self.isa.pointer_type(),
                    ir::MemFlagsData::trusted(),
                    cur_vmctx,
                    i32::try_from(vmimport + u32::from(self.offsets.vmmemory_import_vmctx()))
                        .unwrap(),
                );
                let index = pos.ins().load(
                    ir::types::I32,
                    ir::MemFlagsData::trusted(),
                    cur_vmctx,
                    i32::try_from(vmimport + u32::from(self.offsets.vmmemory_import_index()))
                        .unwrap(),
                );
                (vmctx, index)
            }
        }
    }

    /// Returns two `ir::Value`s, the first of which is the vmctx for the table
    /// `index` and the second of which is the `DefinedTableIndex` for `index`.
    ///
    /// Handles internally whether `index` is an imported table or not.
    fn table_vmctx_and_defined_index(
        &mut self,
        pos: &mut FuncCursor,
        index: TableIndex,
    ) -> (ir::Value, ir::Value) {
        // NB: the body of this method is similar to
        // `memory_vmctx_and_defined_index` above.
        let cur_vmctx = self.vmctx_val(pos);
        match self.module.defined_table_index(index) {
            Some(index) => (cur_vmctx, pos.ins().iconst(I32, i64::from(index.as_u32()))),
            None => {
                let vmimport = self.offsets.vmctx_vmtable_import(index);

                let vmctx = pos.ins().load(
                    self.isa.pointer_type(),
                    ir::MemFlagsData::trusted(),
                    cur_vmctx,
                    i32::try_from(vmimport + u32::from(self.offsets.vmtable_import_vmctx()))
                        .unwrap(),
                );
                let index = pos.ins().load(
                    ir::types::I32,
                    ir::MemFlagsData::trusted(),
                    cur_vmctx,
                    i32::try_from(vmimport + u32::from(self.offsets.vmtable_import_index()))
                        .unwrap(),
                );
                (vmctx, index)
            }
        }
    }

    pub fn translate_memory_grow(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        index: MemoryIndex,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        let mut pos = builder.cursor();
        let memory_grow = self.builtin_functions.memory_grow(&mut pos.func);

        let (memory_vmctx, defined_memory_index) =
            self.memory_vmctx_and_defined_index(&mut pos, index);

        let index_type = self.memory(index).idx_type;
        let val = self.cast_index_to_i64(&mut pos, val, index_type);
        let call_inst = pos
            .ins()
            .call(memory_grow, &[memory_vmctx, val, defined_memory_index]);
        let result = *pos.func.dfg.inst_results(call_inst).first().unwrap();
        let single_byte_pages = match self.memory(index).page_size_log2 {
            16 => false,
            0 => true,
            _ => unreachable!("only page sizes 2**0 and 2**16 are currently valid"),
        };
        Ok(self.convert_pointer_to_index_type(
            builder.cursor(),
            result,
            index_type,
            single_byte_pages,
        ))
    }

    /// Loads the size, in bytes, of the memory `index` specified.
    ///
    /// Returns the `ir::Value`, typed as a pointer-width integer, that is the
    /// size in bytes.
    fn memory_size_in_bytes(&mut self, pos: &mut FuncCursor<'_>, index: MemoryIndex) -> ir::Value {
        let pointer_type = self.pointer_type();
        let vmctx = self.vmctx(&mut pos.func);
        let is_shared = self.module.memories[index].shared;
        let base = pos.ins().global_value(pointer_type, vmctx);
        match self.module.defined_memory_index(index) {
            Some(def_index) => {
                if is_shared {
                    let offset =
                        i32::try_from(self.offsets.vmctx_vmmemory_pointer(def_index)).unwrap();
                    let vmmemory_ptr =
                        pos.ins()
                            .load(pointer_type, ir::MemFlagsData::trusted(), base, offset);
                    let vmmemory_definition_offset =
                        i64::from(self.offsets.ptr.vmmemory_definition_current_length());
                    let vmmemory_definition_ptr =
                        pos.ins().iadd_imm(vmmemory_ptr, vmmemory_definition_offset);
                    // This atomic access of the
                    // `VMMemoryDefinition::current_length` is direct; no bounds
                    // check is needed. This is possible because shared memory
                    // has a static size (the maximum is always known). Shared
                    // memory is thus built with a static memory plan and no
                    // bounds-checked version of this is implemented.
                    pos.ins().atomic_load(
                        pointer_type,
                        ir::MemFlagsData::trusted(),
                        vmmemory_definition_ptr,
                    )
                } else {
                    let owned_index = self.module.owned_memory_index(def_index);
                    let offset = i32::try_from(
                        self.offsets
                            .vmctx_vmmemory_definition_current_length(owned_index),
                    )
                    .unwrap();
                    pos.ins()
                        .load(pointer_type, ir::MemFlagsData::trusted(), base, offset)
                }
            }
            None => {
                let offset = i32::try_from(self.offsets.vmctx_vmmemory_import_from(index)).unwrap();
                let vmmemory_ptr =
                    pos.ins()
                        .load(pointer_type, ir::MemFlagsData::trusted(), base, offset);
                if is_shared {
                    let vmmemory_definition_offset =
                        i64::from(self.offsets.ptr.vmmemory_definition_current_length());
                    let vmmemory_definition_ptr =
                        pos.ins().iadd_imm(vmmemory_ptr, vmmemory_definition_offset);
                    pos.ins().atomic_load(
                        pointer_type,
                        ir::MemFlagsData::trusted(),
                        vmmemory_definition_ptr,
                    )
                } else {
                    pos.ins().load(
                        pointer_type,
                        ir::MemFlagsData::trusted(),
                        vmmemory_ptr,
                        i32::from(self.offsets.ptr.vmmemory_definition_current_length()),
                    )
                }
            }
        }
    }

    pub fn translate_memory_size(
        &mut self,
        mut pos: FuncCursor<'_>,
        index: MemoryIndex,
    ) -> WasmResult<ir::Value> {
        let current_length_in_bytes = self.memory_size_in_bytes(&mut pos, index);

        let page_size_log2 = i64::from(self.module.memories[index].page_size_log2);
        let current_length_in_pages = pos.ins().ushr_imm(current_length_in_bytes, page_size_log2);
        let single_byte_pages = match page_size_log2 {
            16 => false,
            0 => true,
            _ => unreachable!("only page sizes 2**0 and 2**16 are currently valid"),
        };
        Ok(self.convert_pointer_to_index_type(
            pos,
            current_length_in_pages,
            self.memory(index).idx_type,
            single_byte_pages,
        ))
    }

    pub fn translate_memory_copy(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        src_index: MemoryIndex,
        dst_index: MemoryIndex,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        self.translate_entity_copy(builder, dst_index, src_index, dst, src, len)
    }

    /// Perform a raw bulk-memory-like libcall.
    ///
    /// The main purpose of this helper is to handle situations when fuel and
    /// epochs are enabled to break up the copy into a loop of chunks with
    /// preemption checks between them.
    fn raw_bulk_memory_operation(&mut self, builder: &mut FunctionBuilder<'_>, mut op: BulkOp) {
        // Very scientifically chosen. Or, more seriously, this is just an
        // arbitrary number for now. 100k copies of this size locally takes half
        // a second, so seems like a reasonably large chunk size to not hit perf
        // too much by chunking but also enable time slicing.
        const UNINTERRUPTABLE_CHUNK_SIZE: i64 = 128 << 20;

        let mut pos = builder.cursor();
        let vmctx = self.vmctx_val(&mut pos);
        let pointer_type = self.pointer_type();

        // Performs a raw call to the actual libcall, as dictated by the
        // provided `op`. This unconditionally inserts epoch/fuel checks for all
        // calls.
        let raw_call =
            |env: &mut FuncEnvironment<'_>, builder: &mut FunctionBuilder<'_>, op: &_| {
                if env.tunables.epoch_interruption {
                    env.epoch_check(builder);
                }
                match *op {
                    BulkOp::MemoryCopy { dst, src, len } => {
                        if env.tunables.consume_fuel {
                            // Note that fuel is always a 64-bit counter.
                            let fuel_consumed = match env.pointer_type() {
                                ir::types::I32 => builder.ins().uextend(ir::types::I64, len),
                                ir::types::I64 => len,
                                _ => unreachable!(),
                            };
                            env.manual_fuel_check(builder, fuel_consumed);
                        }
                        let memory_copy = env.builtin_functions.memory_copy(&mut builder.func);
                        builder.ins().call(memory_copy, &[vmctx, dst, src, len]);
                    }
                    BulkOp::MemoryFill { dst, val, len } => {
                        if env.tunables.consume_fuel {
                            let fuel_consumed = match env.pointer_type() {
                                ir::types::I32 => builder.ins().uextend(ir::types::I64, len),
                                ir::types::I64 => len,
                                _ => unreachable!(),
                            };
                            env.manual_fuel_check(builder, fuel_consumed);
                        }
                        let memory_fill = env.builtin_functions.memory_fill(&mut builder.func);
                        builder.ins().call(memory_fill, &[vmctx, dst, val, len]);
                    }
                }
            };

        // If epochs and fuel are disabled, then just call the libcall and
        // return. No need for the loops below.
        if !self.tunables.epoch_interruption && !self.tunables.consume_fuel {
            raw_call(self, builder, &op);
            return;
        }

        // If fuel is enabled, first take all the pending fuel and flush it to
        // our internal variable. This is necessary to avoid picking up all
        // pending fuel on each turn of the loop below.
        if self.tunables.consume_fuel {
            self.fuel_increment_var(builder);
        }

        let current_block = builder.current_block().unwrap();
        let chunk_block = builder.create_block();
        let last_chunk_block = builder.create_block();

        builder.ensure_inserted_block();
        builder.insert_block_after(chunk_block, current_block);
        builder.insert_block_after(last_chunk_block, chunk_block);

        let chunk = builder
            .ins()
            .iconst(pointer_type, UNINTERRUPTABLE_CHUNK_SIZE);

        // Helper closure to test if the length in `op` is larger than `chunk`,
        // and if so do a single chunk. Else this goes to the final block with
        // the final operation.
        let has_chunk_branch = |builder: &mut FunctionBuilder<'_>, op: &_| {
            let len = match *op {
                BulkOp::MemoryCopy { len, .. } | BulkOp::MemoryFill { len, .. } => len,
            };
            let has_chunk = builder.ins().icmp(IntCC::UnsignedGreaterThan, len, chunk);
            match *op {
                BulkOp::MemoryCopy { dst, src, len } => {
                    builder.ins().brif(
                        has_chunk,
                        chunk_block,
                        &[dst.into(), src.into(), len.into()],
                        last_chunk_block,
                        &[dst.into(), src.into(), len.into()],
                    );
                }
                BulkOp::MemoryFill { dst, len, .. } => {
                    builder.ins().brif(
                        has_chunk,
                        chunk_block,
                        &[dst.into(), len.into()],
                        last_chunk_block,
                        &[dst.into(), len.into()],
                    );
                }
            }
        };
        has_chunk_branch(builder, &op);

        let append_block_params = |builder: &mut FunctionBuilder<'_>, block, op: &mut _| match op {
            BulkOp::MemoryCopy { dst, src, len } => {
                *dst = builder.append_block_param(block, pointer_type);
                *src = builder.append_block_param(block, pointer_type);
                *len = builder.append_block_param(block, pointer_type);
            }
            BulkOp::MemoryFill { dst, len, .. } => {
                *dst = builder.append_block_param(block, pointer_type);
                *len = builder.append_block_param(block, pointer_type);
            }
        };

        // In the block with per-chunk copies, each operation performs `chunk`
        // length of bytes and then decrements the current length by `chunk`.
        // Afterwards a condition tests if we do another chunk or break out for
        // the final chunk.
        builder.switch_to_block(chunk_block);
        append_block_params(builder, chunk_block, &mut op);
        let op_len = match &mut op {
            BulkOp::MemoryCopy { len, .. } | BulkOp::MemoryFill { len, .. } => len,
        };
        let remaining_len = *op_len;
        *op_len = chunk;
        raw_call(self, builder, &op);
        match &mut op {
            BulkOp::MemoryCopy { dst, src, len } => {
                *dst = builder.ins().iadd(*dst, chunk);
                *src = builder.ins().iadd(*src, chunk);
                *len = builder.ins().isub(remaining_len, chunk);
            }
            BulkOp::MemoryFill { len, dst, .. } => {
                *dst = builder.ins().iadd(*dst, chunk);
                *len = builder.ins().isub(remaining_len, chunk);
            }
        };
        has_chunk_branch(builder, &op);

        // In the final block we know that the length of the operation is less
        // than `chunk`. This could still be sizable, though, so a final
        // fuel/epoch check is inserted.
        builder.switch_to_block(last_chunk_block);
        append_block_params(builder, last_chunk_block, &mut op);
        raw_call(self, builder, &op);

        builder.seal_block(chunk_block);
        builder.seal_block(last_chunk_block);
    }

    pub fn translate_memory_fill(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        memory_index: MemoryIndex,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) {
        let idx_type = self.memory(memory_index).idx_type;

        // Bounds check `dst+len` and convert it to a raw heap address.
        let raw_heap_addr = self.translate_entity_bounds_check(builder, memory_index, dst, len);

        // Fit the `len` value to `pointer_type`. Note that at this point it's
        // guaranteed inbounds so there's no loss in precision.
        let len_ptr =
            self.unchecked_cast_wasm_addr_to_native_addr(&mut builder.cursor(), len, idx_type);

        self.raw_bulk_memory_operation(
            builder,
            BulkOp::MemoryFill {
                dst: raw_heap_addr,
                val,
                len: len_ptr,
            },
        );
    }

    pub fn translate_memory_init(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        memory_index: MemoryIndex,
        seg_index: u32,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let seg_index = DataIndex::from_u32(seg_index);
        self.translate_entity_copy(builder, memory_index, seg_index, dst, src, len)
    }

    pub fn translate_data_drop(&mut self, mut pos: FuncCursor, seg_index: u32) -> WasmResult<()> {
        let seg_index = DataIndex::from_u32(seg_index);

        // Lookup the passive data segment corresponding to this data segment.
        // If this is an active data segment then it already has length 0 so
        // there's nothing to do.
        let passive_index = match self.translation.passive_data_map[seg_index] {
            Some(idx) => idx,
            None => return Ok(()),
        };

        // For passive data segments to implement `data.drop` it's a store of
        // the value 0 to the `VMContext`'s slot for this passive data segment.
        let vmctx = self.vmctx_val(&mut pos);
        let new_length = pos.ins().iconst(I32, 0);
        pos.ins().store(
            ir::MemFlagsData::trusted(),
            new_length,
            vmctx,
            i32::try_from(self.offsets.vmctx_passive_data_length(passive_index)).unwrap(),
        );

        Ok(())
    }

    pub fn translate_table_size(&mut self, pos: FuncCursor, table_index: TableIndex) -> ir::Value {
        let table_data = self.get_or_create_table(pos.func, table_index);
        let index_type = index_type_to_ir_type(self.table(table_index).idx_type);
        table_data.bound.bound(&*self.isa, pos, index_type)
    }

    /// Copies elements from `src_entity` to `dst_entity`.
    ///
    /// This will perform bounds checks for both entities and raise traps if
    /// anything is out of bounds. Afterwards the actual copy is performed. The
    /// `dst` and `src` parameters are starting offsets, and `len` is the length
    /// of the copy. Both `dst` and `src` have types appropriate to index their
    /// respective entities, and `len` has a type that's the smaller of the two
    /// index types.
    fn translate_entity_copy(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        dst_entity: impl Into<CheckedEntity>,
        src_entity: impl Into<CheckedEntity>,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let dst_entity = dst_entity.into();
        let src_entity = src_entity.into();
        let dst_idx_ty = dst_entity.index_type(self);
        let src_idx_ty = src_entity.index_type(self);

        // The length is 32-bit if either is 32-bit, but if they're both 64-bit
        // then it's 64-bit.
        let len_idx_ty = match (src_idx_ty, dst_idx_ty) {
            (IndexType::I32, _) | (_, IndexType::I32) => IndexType::I32,
            (IndexType::I64, IndexType::I64) => IndexType::I64,
        };
        let src_len = if src_idx_ty == len_idx_ty {
            len
        } else {
            assert_eq!(src_idx_ty, IndexType::I64);
            builder.ins().uextend(I64, len)
        };
        let dst_len = if dst_idx_ty == len_idx_ty {
            len
        } else {
            assert_eq!(dst_idx_ty, IndexType::I64);
            builder.ins().uextend(I64, len)
        };

        // Perform a bounds check for the src/dst entities and compute the raw
        // heap addresses at the same time.
        let dst_raw_addr = self.translate_entity_bounds_check(builder, dst_entity, dst, dst_len);
        let src_raw_addr = self.translate_entity_bounds_check(builder, src_entity, src, src_len);

        // Fit the `len` value to `pointer_type`. Note that at this point it's
        // guaranteed inbounds so there's no loss in precision.
        let len_ptr =
            self.unchecked_cast_wasm_addr_to_native_addr(&mut builder.cursor(), len, len_idx_ty);

        match dst_entity {
            // Memories are always a `memcpy`.
            CheckedEntity::Memory(_) => {
                assert!(matches!(
                    src_entity,
                    CheckedEntity::Memory(_) | CheckedEntity::Data(_)
                ));
                self.raw_bulk_memory_operation(
                    builder,
                    BulkOp::MemoryCopy {
                        dst: dst_raw_addr,
                        src: src_raw_addr,
                        len: len_ptr,
                    },
                );
                Ok(())
            }

            // Tables are sometimes a memcpy, sometimes a per-element loop.
            // Delegate further to figure that out.
            CheckedEntity::Table(dst_table) => {
                let CheckedEntity::Table(src_table) = src_entity else {
                    unreachable!();
                };
                let ty = self.table(dst_table).ref_type;
                let dst_table = self.get_or_create_table(builder.func, dst_table);
                let src_table = self.get_or_create_table(builder.func, src_table);
                assert_eq!(dst_table.element_size, src_table.element_size);
                let one_elem_size = builder
                    .ins()
                    .iconst(self.pointer_type(), i64::from(dst_table.element_size));
                self.emit_raw_array_or_table_copy(
                    builder,
                    dst_entity,
                    src_entity,
                    WasmStorageType::Val(WasmValType::Ref(ty)),
                    dst_raw_addr,
                    src_raw_addr,
                    one_elem_size,
                    len_ptr,
                    src,
                )
            }
            // Note that future refactorings will fill this out soon.
            CheckedEntity::Array => todo!(),

            // Cannot copy into a data segment in wasm.
            CheckedEntity::Data(_) => unreachable!(),
        }
    }

    /// Performs a bounds check and raises a trap if `idx+len` is out-of-bounds
    /// for `len`.
    ///
    /// Returns the raw host-native address of `idx+len`.
    fn translate_entity_bounds_check(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        entity: impl Into<CheckedEntity>,
        idx: ir::Value,
        len: ir::Value,
    ) -> ir::Value {
        let entity = entity.into();
        let pointer_type = self.pointer_type();
        let idx_type = entity.index_type(self);
        let idx_clif_type = index_type_to_ir_type(idx_type);
        assert_eq!(builder.func.dfg.value_type(idx), idx_clif_type);
        assert_eq!(builder.func.dfg.value_type(len), idx_clif_type);

        // Load the entity size, as `pointer_type`.
        let entity_size = match entity {
            CheckedEntity::Memory(i) => self.memory_size_in_bytes(&mut builder.cursor(), i),
            CheckedEntity::Table(i) => {
                let size = self.translate_table_size(builder.cursor(), i);
                self.unchecked_cast_wasm_addr_to_native_addr(&mut builder.cursor(), size, idx_type)
            }
            CheckedEntity::Data(i) => match self.translation.passive_data_map[i] {
                Some(passive_index) => {
                    let vmctx = self.vmctx_val(&mut builder.cursor());
                    let offset =
                        i32::try_from(self.offsets.vmctx_passive_data_length(passive_index))
                            .unwrap();
                    let flags = ir::MemFlagsData::trusted();
                    match pointer_type {
                        I32 => builder.ins().load(I32, flags, vmctx, offset),
                        I64 => builder.ins().uload32(flags, vmctx, offset),
                        _ => unreachable!(),
                    }
                }
                None => builder.ins().iconst(pointer_type, 0),
            },
            // Note that future refactorings will fill this out soon.
            CheckedEntity::Array => todo!(),
        };
        assert_eq!(builder.func.dfg.value_type(entity_size), pointer_type);

        let trap_code = match entity {
            CheckedEntity::Memory(_) => ir::TrapCode::HEAP_OUT_OF_BOUNDS,
            CheckedEntity::Table(_) => TRAP_TABLE_OUT_OF_BOUNDS,
            CheckedEntity::Data(_) => ir::TrapCode::HEAP_OUT_OF_BOUNDS,
            CheckedEntity::Array => TRAP_ARRAY_OUT_OF_BOUNDS,
        };

        // Compute the end index of this operation, casted to the `I64` type.
        //
        // Note that addition can't overflow after extending 32-bits to
        // 64-bits, so no need to check for overflow in the 32-bit index case.
        let end64 = match idx_type {
            IndexType::I32 => {
                let idx64 = builder.ins().uextend(I64, idx);
                let len64 = builder.ins().uextend(I64, len);
                builder.ins().iadd(idx64, len64)
            }
            IndexType::I64 => self.uadd_overflow_trap(builder, idx, len, trap_code),
        };

        // Cast the host-pointer width to a 64-bit bit width.
        let entity_size64 = match pointer_type {
            I32 => builder.ins().uextend(I64, entity_size),
            I64 => entity_size,
            _ => unreachable!(),
        };

        // This is the actual bounds check that verifies that this operation is
        // in-bounds. Once control flow gets past here we know that nothing can
        // overflow and everything is in-bounds.
        let inbounds = builder
            .ins()
            .icmp(IntCC::UnsignedGreaterThan, end64, entity_size64);
        self.trapnz(builder, inbounds, trap_code);

        // Compute the actual raw heap address to return
        let (base, elem_size) = match entity {
            CheckedEntity::Memory(i) => {
                let heap = self.get_or_create_heap(builder.func, i);
                let heap = &self.heaps()[heap];
                (builder.ins().global_value(pointer_type, heap.base), 1)
            }
            CheckedEntity::Table(i) => {
                let table = self.get_or_create_table(builder.func, i);
                (
                    builder.ins().global_value(pointer_type, table.base_gv),
                    table.element_size,
                )
            }
            CheckedEntity::Data(i) => match self.translation.passive_data_map[i] {
                Some(passive_index) => {
                    let vmctx = self.vmctx_val(&mut builder.cursor());
                    let offset =
                        i32::try_from(self.offsets.vmctx_passive_data_base(passive_index)).unwrap();
                    let base = builder.ins().load(
                        self.pointer_type(),
                        ir::MemFlagsData::trusted(),
                        vmctx,
                        offset,
                    );
                    (base, 1)
                }

                // Any address should do for an active data segment, but pick
                // something non-null for now. Note that the length of an active
                // data segment is always 0, so we know that the memcpy, if any,
                // will be 0 elements, so the actual value here doesn't matter.
                None => (builder.ins().iconst(pointer_type, 1), 1),
            },
            // Note that future refactorings will fill this out soon.
            CheckedEntity::Array => todo!(),
        };
        assert_eq!(builder.func.dfg.value_type(base), pointer_type);
        let idx =
            self.unchecked_cast_wasm_addr_to_native_addr(&mut builder.cursor(), idx, idx_type);
        assert_eq!(builder.func.dfg.value_type(idx), pointer_type);
        let byte_offset = builder.ins().imul_imm(idx, i64::from(elem_size));
        builder.ins().iadd(base, byte_offset)
    }

    pub fn translate_table_copy(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        dst_table_index: TableIndex,
        src_table_index: TableIndex,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        self.translate_entity_copy(builder, dst_table_index, src_table_index, dst, src, len)
    }

    /// Emits a copy between two WebAssembly table or array entities.
    ///
    /// This will copy from `src_entity` to `dst_entity` and this assumes that
    /// all bounds checks have already passed. Items will be loaded from
    /// `src_elem_addr` and stored to `dst_elem_addr`. The `elem_ty` is the type
    /// being transferred, `one_elem_size` is the byte size of each element,
    /// `copy_len` is the number of elements being copied, and `src_index` is
    /// the first index within `src_entity` being loaded.
    ///
    /// All values here have type `self.pointer_type()`, except `src_index`
    /// which is typed appropriately to index `src_entity`.
    ///
    /// The main purpose of this function is to deduce if `memcpy` can be used,
    /// and otherwise this will emit an inline copy loop.
    fn emit_raw_array_or_table_copy(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        dst_entity: CheckedEntity,
        src_entity: CheckedEntity,
        elem_ty: WasmStorageType,
        dst_elem_addr: ir::Value,
        src_elem_addr: ir::Value,
        one_elem_size: ir::Value,
        copy_len: ir::Value,
        src_index: ir::Value,
    ) -> WasmResult<()> {
        let pointer_type = self.pointer_type();
        assert_eq!(builder.func.dfg.value_type(dst_elem_addr), pointer_type);
        assert_eq!(builder.func.dfg.value_type(src_elem_addr), pointer_type);
        assert_eq!(builder.func.dfg.value_type(one_elem_size), pointer_type);
        assert_eq!(builder.func.dfg.value_type(copy_len), pointer_type);
        assert_eq!(
            builder.func.dfg.value_type(src_index),
            index_type_to_ir_type(src_entity.index_type(self))
        );

        let type_forbids_memcpy = match elem_ty {
            // Scalar types can always use a memcpy.
            WasmStorageType::I8
            | WasmStorageType::I16
            | WasmStorageType::Val(
                WasmValType::I32
                | WasmValType::I64
                | WasmValType::F32
                | WasmValType::F64
                | WasmValType::V128,
            ) => false,

            WasmStorageType::Val(WasmValType::Ref(ty)) => match ty.heap_type.top() {
                // These types are represented the same in all locations (e.g.
                // tables and the GC heap), so check to see if it's a `VMGcRef`
                // type. If it is then barriers might be needed, meaning memcpy
                // can't be used.
                //
                // FIXME: should add a method to `GcCompiler` to detect when
                // the compiler doesn't actually need barriers, in which case
                // memcpy is fine.
                WasmHeapTopType::Extern
                | WasmHeapTopType::Any
                | WasmHeapTopType::Exn
                | WasmHeapTopType::Cont => ty.heap_type.is_vmgcref_type_and_not_i31(),

                // `funcref` is stored differently in tables and the GC heap, so
                // futher inspection is necessary of where the copy is
                // happening.
                WasmHeapTopType::Func => match src_entity {
                    // Tables of funcrefs might be lazily initialized which
                    // would mean that memcpy isn't suitable. If lazy init is
                    // disabled though then funcrefs are just pointers so a
                    // memcpy can be used.
                    CheckedEntity::Table(_) => self.tunables.table_lazy_init,
                    // The GC heap has integers representing funcrefs, so memcpy
                    // is fine.
                    CheckedEntity::Array => false,
                    // Not possible
                    CheckedEntity::Memory(_) | CheckedEntity::Data(_) => unreachable!(),
                },
            },
        };

        // For memcpy, that's easy, just call the intrinsic with the right
        // parameters.
        if !type_forbids_memcpy {
            let copy_byte_len = builder.ins().imul(one_elem_size, copy_len);
            self.raw_bulk_memory_operation(
                builder,
                BulkOp::MemoryCopy {
                    dst: dst_elem_addr,
                    src: src_elem_addr,
                    len: copy_byte_len,
                },
            );
            return Ok(());
        }

        // For other copies, this is a per-element loop. Use the helper to
        // setup the general structure, and then the per-element closures is
        // used to dispatch `other` further.
        self.translate_per_element_copy(
            builder,
            dst_elem_addr,
            src_elem_addr,
            one_elem_size,
            copy_len,
            src_index,
            &|this, builder, dst, src, src_index| {
                let val = match src_entity {
                    // FIXME: ideally this wouldn't redo the bounds check but
                    // it's easier right now to share the internals of
                    // `translate_table_get` which are a bit tricky with
                    // funcrefs.
                    CheckedEntity::Table(i) => this.translate_table_get(builder, i, src_index)?,
                    CheckedEntity::Array => {
                        gc::read_field_at_addr(this, builder, elem_ty, src, None)?
                    }
                    CheckedEntity::Memory(_) | CheckedEntity::Data { .. } => unreachable!(),
                };
                match dst_entity {
                    CheckedEntity::Table(i) => {
                        this.emit_table_set(builder, i, dst, ir::MemFlagsData::trusted(), val)?;
                    }
                    CheckedEntity::Array => {
                        gc::write_field_at_addr(this, builder, elem_ty, dst, val)?
                    }
                    CheckedEntity::Memory(_) | CheckedEntity::Data { .. } => unreachable!(),
                }

                Ok(())
            },
        )?;

        Ok(())
    }

    /// Performs an inline element-by-element copy from `dst_elem_addr` to
    /// `src_elem_addr`.
    ///
    /// The size of one element  is `one_elem_size` and the number of elements
    /// being copied is `copy_len`. The actual implementation of copying a
    /// single element is the `copy_one` closure which receives the `dst`/`src`
    /// pointers to load/store from, as well as the current index.
    ///
    /// All IR values have type `self.pointer_type()`, except `src_index` which
    /// has an appropriate type to index into the entity copied from.
    pub fn translate_per_element_copy(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        dst_elem_addr: ir::Value,
        src_elem_addr: ir::Value,
        one_elem_size: ir::Value,
        copy_len: ir::Value,
        src_index: ir::Value,
        copy_one: &dyn Fn(
            &mut Self,
            &mut FunctionBuilder<'_>,
            ir::Value,
            ir::Value,
            ir::Value,
        ) -> WasmResult<()>,
    ) -> WasmResult<()> {
        // This is either a forwards copy or a backwards copy depending on the
        // src/dst pointers. The loop here looks like:
        //
        //  current_block:
        //      ...
        //      brif len, nonempty_block, done_block
        //
        //  nonempty_block:
        //      forward = icmp ult dst_elem_addr, src_elem_addr
        //      brif forward,
        //          forward_block(dst_elem_addr, src_elem_addr, src_index),
        //          backwards_block(dst_end_addr, src_end_addr, src_index + len)
        //
        //  forward_block(dst, src, src_index):
        //      *dst = *src
        //      dst += elem_size
        //      src += elem_size
        //      src_index += 1
        //      done = icmp eq src, src_end_addr
        //      brif done, done_block, forward_block(dst, src, src_index)
        //
        //  backwards_block(dst, src, src_index):
        //      dst -= elem_size
        //      src -= elem_size
        //      src_index -= 1
        //      *dst = *src
        //      done = icmp eq src, src_elem_addr
        //      brif done, done_block, backwards_block(dst, src, src_index)
        //
        //  done_block:
        //      ...
        let current_block = builder.current_block().unwrap();
        let nonempty_block = builder.create_block();
        let forward_block = builder.create_block();
        let backwards_block = builder.create_block();
        let done_block = builder.create_block();

        builder.ensure_inserted_block();
        builder.insert_block_after(nonempty_block, current_block);
        builder.insert_block_after(forward_block, nonempty_block);
        builder.insert_block_after(backwards_block, forward_block);
        builder.insert_block_after(done_block, backwards_block);

        // Terminate `current_block` by testing to see if we're copying any
        // elements at all.
        builder
            .ins()
            .brif(copy_len, nonempty_block, &[], done_block, &[]);

        // In the nonempty_block test to see if this is a forward or backwards
        // copy.
        builder.switch_to_block(nonempty_block);
        let dst_first = builder
            .ins()
            .icmp(IntCC::UnsignedLessThan, dst_elem_addr, src_elem_addr);
        let src_index_ty = builder.func.dfg.value_type(src_index);
        let copy_byte_len = builder.ins().imul(copy_len, one_elem_size);
        let dst_end_addr = builder.ins().iadd(dst_elem_addr, copy_byte_len);
        let src_end_addr = builder.ins().iadd(src_elem_addr, copy_byte_len);
        let copy_len_as_src_index_ty = match (self.pointer_type(), src_index_ty) {
            (I32, I32) | (I64, I64) => copy_len,
            (I32, I64) => builder.ins().uextend(I64, copy_len),
            (I64, I32) => builder.ins().ireduce(I32, copy_len),
            _ => unreachable!(),
        };
        let end_index = builder.ins().iadd(src_index, copy_len_as_src_index_ty);
        builder.ins().brif(
            dst_first,
            forward_block,
            &[dst_elem_addr.into(), src_elem_addr.into(), src_index.into()],
            backwards_block,
            &[dst_end_addr.into(), src_end_addr.into(), end_index.into()],
        );

        // Forward copy -- copy one field, then mutate the current pointers, then
        // check to see if we're done.
        builder.switch_to_block(forward_block);
        let dst_cur = builder.append_block_param(forward_block, self.pointer_type());
        let src_cur = builder.append_block_param(forward_block, self.pointer_type());
        let src_index = builder.append_block_param(forward_block, src_index_ty);
        self.translate_loop_header(builder)?;
        copy_one(self, builder, dst_cur, src_cur, src_index)?;
        let dst_next = builder.ins().iadd(dst_cur, one_elem_size);
        let src_next = builder.ins().iadd(src_cur, one_elem_size);
        let src_index_next = builder.ins().iadd_imm(src_index, 1);
        let done = builder.ins().icmp(IntCC::Equal, src_next, src_end_addr);
        builder.ins().brif(
            done,
            done_block,
            &[],
            forward_block,
            &[dst_next.into(), src_next.into(), src_index_next.into()],
        );

        // Backwards copy -- update the pointers, then perform a copy, then check
        // to see if we're done.
        builder.switch_to_block(backwards_block);
        let dst_cur = builder.append_block_param(backwards_block, self.pointer_type());
        let src_cur = builder.append_block_param(backwards_block, self.pointer_type());
        let src_index = builder.append_block_param(backwards_block, src_index_ty);
        self.translate_loop_header(builder)?;
        let dst_cur = builder.ins().isub(dst_cur, one_elem_size);
        let src_cur = builder.ins().isub(src_cur, one_elem_size);
        let one = builder.ins().iconst(src_index_ty, 1);
        let src_index = builder.ins().isub(src_index, one);
        copy_one(self, builder, dst_cur, src_cur, src_index)?;
        let done = builder.ins().icmp(IntCC::Equal, src_cur, src_elem_addr);
        builder.ins().brif(
            done,
            done_block,
            &[],
            backwards_block,
            &[dst_cur.into(), src_cur.into(), src_index.into()],
        );

        builder.switch_to_block(done_block);

        builder.seal_block(nonempty_block);
        builder.seal_block(forward_block);
        builder.seal_block(backwards_block);
        builder.seal_block(done_block);

        Ok(())
    }

    pub fn translate_table_init(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        seg_index: u32,
        table_index: TableIndex,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let mut pos = builder.cursor();
        let table_init = self.builtin_functions.table_init(&mut pos.func);
        let table_index_arg = pos.ins().iconst(I32, i64::from(table_index.as_u32()));
        let seg_index_arg = pos.ins().iconst(I32, i64::from(seg_index));
        let vmctx = self.vmctx_val(&mut pos);
        let index_type = self.table(table_index).idx_type;
        let dst = self.cast_index_to_i64(&mut pos, dst, index_type);
        let src = pos.ins().uextend(I64, src);
        let len = pos.ins().uextend(I64, len);

        pos.ins().call(
            table_init,
            &[vmctx, table_index_arg, seg_index_arg, dst, src, len],
        );

        Ok(())
    }

    pub fn translate_elem_drop(&mut self, mut pos: FuncCursor, elem_index: u32) -> WasmResult<()> {
        let elem_drop = self.builtin_functions.elem_drop(&mut pos.func);
        let elem_index_arg = pos.ins().iconst(I32, elem_index as i64);
        let vmctx = self.vmctx_val(&mut pos);
        pos.ins().call(elem_drop, &[vmctx, elem_index_arg]);
        Ok(())
    }

    pub fn translate_atomic_wait(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        memory_index: MemoryIndex,
        _heap: Heap,
        addr: ir::Value,
        expected: ir::Value,
        timeout: ir::Value,
    ) -> WasmResult<ir::Value> {
        #[cfg(feature = "threads")]
        {
            let mut pos = builder.cursor();
            let addr = self.cast_index_to_i64(&mut pos, addr, self.memory(memory_index).idx_type);
            let implied_ty = pos.func.dfg.value_type(expected);
            let wait_func = self.get_memory_atomic_wait(&mut pos.func, implied_ty);

            let (memory_vmctx, defined_memory_index) =
                self.memory_vmctx_and_defined_index(&mut pos, memory_index);

            let call_inst = pos.ins().call(
                wait_func,
                &[memory_vmctx, defined_memory_index, addr, expected, timeout],
            );
            let ret = pos.func.dfg.inst_results(call_inst)[0];
            Ok(builder.ins().ireduce(ir::types::I32, ret))
        }
        #[cfg(not(feature = "threads"))]
        {
            let _ = (builder, memory_index, addr, expected, timeout);
            Err(wasmtime_environ::WasmError::Unsupported(
                "threads support disabled at compile time".to_string(),
            ))
        }
    }

    pub fn translate_atomic_notify(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        memory_index: MemoryIndex,
        _heap: Heap,
        addr: ir::Value,
        count: ir::Value,
    ) -> WasmResult<ir::Value> {
        #[cfg(feature = "threads")]
        {
            let mut pos = builder.cursor();
            let addr = self.cast_index_to_i64(&mut pos, addr, self.memory(memory_index).idx_type);
            let atomic_notify = self.builtin_functions.memory_atomic_notify(&mut pos.func);

            let (memory_vmctx, defined_memory_index) =
                self.memory_vmctx_and_defined_index(&mut pos, memory_index);
            let call_inst = pos.ins().call(
                atomic_notify,
                &[memory_vmctx, defined_memory_index, addr, count],
            );
            let ret = pos.func.dfg.inst_results(call_inst)[0];
            Ok(builder.ins().ireduce(ir::types::I32, ret))
        }
        #[cfg(not(feature = "threads"))]
        {
            let _ = (builder, memory_index, addr, count);
            Err(wasmtime_environ::WasmError::Unsupported(
                "threads support disabled at compile time".to_string(),
            ))
        }
    }

    pub fn translate_loop_header(&mut self, builder: &mut FunctionBuilder) -> WasmResult<()> {
        // Additionally if enabled check how much fuel we have remaining to see
        // if we've run out by this point.
        if self.tunables.consume_fuel {
            self.fuel_check(builder);
        }

        // If we are performing epoch-based interruption, check to see
        // if the epoch counter has changed.
        if self.tunables.epoch_interruption {
            self.epoch_check(builder);
        }

        Ok(())
    }

    pub fn before_translate_operator(
        &mut self,
        op: &Operator,
        _operand_types: Option<&[WasmValType]>,
        builder: &mut FunctionBuilder,
    ) -> WasmResult<()> {
        if self.tunables.consume_fuel {
            self.fuel_before_op(op, builder, self.is_reachable());
        }
        if self.is_reachable() && self.state_slot.is_some() {
            let builtin = self.builtin_functions.patchable_breakpoint(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let inst = builder.ins().call(builtin, &[vmctx]);
            let tags = self.debug_tags(builder.srcloc());
            builder.func.debug_tags.set(inst, tags);
        }

        Ok(())
    }

    pub fn after_translate_operator(
        &mut self,
        op: &Operator,
        validator: &FuncValidator<impl WasmModuleResources>,
        builder: &mut FunctionBuilder,
    ) -> WasmResult<()> {
        if self.tunables.consume_fuel && self.is_reachable() {
            self.fuel_after_op(op, builder);
        }
        if self.is_reachable() {
            self.update_state_slot_stack(validator, builder)?;
        }
        Ok(())
    }

    pub fn before_unconditionally_trapping_memory_access(&mut self, builder: &mut FunctionBuilder) {
        if self.tunables.consume_fuel {
            self.fuel_increment_var(builder);
            self.fuel_save_from_var(builder);
        }
    }

    pub fn before_translate_function(&mut self, builder: &mut FunctionBuilder) -> WasmResult<()> {
        // If an explicit stack limit is requested, emit one here at the start
        // of the function.
        if let Some(gv) = self.stack_limit_at_function_entry {
            let limit = builder.ins().global_value(self.pointer_type(), gv);
            let sp = builder.ins().get_stack_pointer(self.pointer_type());
            let overflow = builder.ins().icmp(IntCC::UnsignedLessThan, sp, limit);
            self.conditionally_trap(builder, overflow, ir::TrapCode::STACK_OVERFLOW);
        }

        self.update_state_slot_vmctx(builder);

        // Additionally we initialize `fuel_var` if it will get used.
        if self.tunables.consume_fuel {
            self.fuel_function_entry(builder);
        }

        // Initialize `epoch_var` with the current epoch.
        if self.tunables.epoch_interruption {
            self.epoch_function_entry(builder);
        }

        #[cfg(feature = "wmemcheck")]
        if self.compiler.wmemcheck {
            let func_name = self.current_func_name(builder);
            if func_name == Some("malloc") {
                self.check_malloc_start(builder);
            } else if func_name == Some("free") {
                self.check_free_start(builder);
            }
        }

        Ok(())
    }

    pub fn after_translate_function(&mut self, builder: &mut FunctionBuilder) -> WasmResult<()> {
        if self.tunables.consume_fuel && self.is_reachable() {
            self.fuel_function_exit(builder);
        }
        self.finish_debug_metadata(builder);
        Ok(())
    }

    pub fn relaxed_simd_deterministic(&self) -> bool {
        self.tunables.relaxed_simd_deterministic
    }

    pub fn has_native_fma(&self) -> bool {
        self.isa.has_native_fma()
    }

    pub fn is_x86(&self) -> bool {
        self.isa.triple().architecture == target_lexicon::Architecture::X86_64
    }

    pub fn translate_cont_bind(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        contobj: ir::Value,
        args: &[ir::Value],
    ) -> ir::Value {
        stack_switching::instructions::translate_cont_bind(self, builder, contobj, args)
    }

    pub fn translate_cont_new(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        func: ir::Value,
        arg_types: &[WasmValType],
        return_types: &[WasmValType],
    ) -> WasmResult<ir::Value> {
        stack_switching::instructions::translate_cont_new(
            self,
            builder,
            func,
            arg_types,
            return_types,
        )
    }

    pub fn translate_resume(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        type_index: u32,
        contobj: ir::Value,
        resume_args: &[ir::Value],
        resumetable: &[(u32, Option<ir::Block>)],
    ) -> WasmResult<Vec<ir::Value>> {
        stack_switching::instructions::translate_resume(
            self,
            builder,
            type_index,
            contobj,
            resume_args,
            resumetable,
        )
    }

    pub fn translate_suspend(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        tag_index: u32,
        suspend_args: &[ir::Value],
        tag_return_types: &[ir::Type],
    ) -> Vec<ir::Value> {
        stack_switching::instructions::translate_suspend(
            self,
            builder,
            tag_index,
            suspend_args,
            tag_return_types,
        )
    }

    /// Translates switch instructions.
    pub fn translate_switch(
        &mut self,
        builder: &mut FunctionBuilder,
        tag_index: u32,
        contobj: ir::Value,
        switch_args: &[ir::Value],
        return_types: &[ir::Type],
    ) -> WasmResult<Vec<ir::Value>> {
        stack_switching::instructions::translate_switch(
            self,
            builder,
            tag_index,
            contobj,
            switch_args,
            return_types,
        )
    }

    pub fn continuation_arguments(&self, index: TypeIndex) -> &[WasmValType] {
        let idx = self.module.types[index].unwrap_module_type_index();
        self.types[self.types[idx].unwrap_cont().unwrap_module_type_index()]
            .unwrap_func()
            .params()
    }

    pub fn continuation_returns(&self, index: TypeIndex) -> &[WasmValType] {
        let idx = self.module.types[index].unwrap_module_type_index();
        self.types[self.types[idx].unwrap_cont().unwrap_module_type_index()]
            .unwrap_func()
            .results()
    }

    pub fn tag_params(&self, tag_index: TagIndex) -> &[WasmValType] {
        let idx = self.module.tags[tag_index].signature;
        self.types[idx.unwrap_module_type_index()]
            .unwrap_func()
            .params()
    }

    pub fn tag_returns(&self, tag_index: TagIndex) -> &[WasmValType] {
        let idx = self.module.tags[tag_index].signature;
        self.types[idx.unwrap_module_type_index()]
            .unwrap_func()
            .results()
    }

    pub fn use_blendv_for_relaxed_laneselect(&self, ty: Type) -> bool {
        self.isa.has_blendv_lowering(ty)
    }

    pub fn use_x86_pmulhrsw_for_relaxed_q15mul(&self) -> bool {
        self.isa.has_x86_pmulhrsw_lowering()
    }

    pub fn use_x86_pmaddubsw_for_dot(&self) -> bool {
        self.isa.has_x86_pmaddubsw_lowering()
    }

    pub fn handle_before_return(&mut self, retvals: &[ir::Value], builder: &mut FunctionBuilder) {
        #[cfg(feature = "wmemcheck")]
        if self.compiler.wmemcheck {
            let func_name = self.current_func_name(builder);
            if func_name == Some("malloc") {
                self.hook_malloc_exit(builder, retvals);
            } else if func_name == Some("free") {
                self.hook_free_exit(builder);
            }
        }
        #[cfg(not(feature = "wmemcheck"))]
        let _ = (retvals, builder);
    }

    pub fn before_load(
        &mut self,
        builder: &mut FunctionBuilder,
        val_size: u8,
        addr: ir::Value,
        offset: u64,
    ) {
        #[cfg(feature = "wmemcheck")]
        if self.compiler.wmemcheck {
            let check_load = self.builtin_functions.check_load(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let num_bytes = builder.ins().iconst(I32, val_size as i64);
            let offset_val = builder.ins().iconst(I64, offset as i64);
            builder
                .ins()
                .call(check_load, &[vmctx, num_bytes, addr, offset_val]);
        }
        #[cfg(not(feature = "wmemcheck"))]
        let _ = (builder, val_size, addr, offset);
    }

    pub fn before_store(
        &mut self,
        builder: &mut FunctionBuilder,
        val_size: u8,
        addr: ir::Value,
        offset: u64,
    ) {
        #[cfg(feature = "wmemcheck")]
        if self.compiler.wmemcheck {
            let check_store = self.builtin_functions.check_store(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let num_bytes = builder.ins().iconst(I32, val_size as i64);
            let offset_val = builder.ins().iconst(I64, offset as i64);
            builder
                .ins()
                .call(check_store, &[vmctx, num_bytes, addr, offset_val]);
        }
        #[cfg(not(feature = "wmemcheck"))]
        let _ = (builder, val_size, addr, offset);
    }

    pub fn update_global(
        &mut self,
        builder: &mut FunctionBuilder,
        global_index: GlobalIndex,
        value: ir::Value,
    ) {
        #[cfg(feature = "wmemcheck")]
        if self.compiler.wmemcheck {
            if global_index.index() == 0 {
                // We are making the assumption that global 0 is the auxiliary stack pointer.
                let update_stack_pointer =
                    self.builtin_functions.update_stack_pointer(builder.func);
                let vmctx = self.vmctx_val(&mut builder.cursor());
                builder.ins().call(update_stack_pointer, &[vmctx, value]);
            }
        }
        #[cfg(not(feature = "wmemcheck"))]
        let _ = (builder, global_index, value);
    }

    pub fn before_memory_grow(
        &mut self,
        builder: &mut FunctionBuilder,
        num_pages: ir::Value,
        mem_index: MemoryIndex,
    ) {
        #[cfg(feature = "wmemcheck")]
        if self.compiler.wmemcheck && mem_index.as_u32() == 0 {
            let update_mem_size = self.builtin_functions.update_mem_size(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            builder.ins().call(update_mem_size, &[vmctx, num_pages]);
        }
        #[cfg(not(feature = "wmemcheck"))]
        let _ = (builder, num_pages, mem_index);
    }

    /// If the ISA has rounding instructions, let Cranelift use them. But if
    /// not, lower to a libcall here, rather than having Cranelift do it. We
    /// can pass our libcall the vmctx pointer, which we use for stack
    /// overflow checking.
    ///
    /// This helper is generic for all rounding instructions below, both for
    /// scalar and simd types. The `clif_round` argument is the CLIF-level
    /// rounding instruction to use if the ISA has the instruction, and the
    /// `round_builtin` helper is used to determine which element-level
    /// rounding operation builtin is used. Note that this handles the case
    /// when `value` is a vector by doing an element-wise libcall invocation.
    fn isa_round(
        &mut self,
        builder: &mut FunctionBuilder,
        value: ir::Value,
        clif_round: fn(FuncInstBuilder<'_, '_>, ir::Value) -> ir::Value,
        round_builtin: fn(&mut BuiltinFunctions, &mut Function) -> ir::FuncRef,
    ) -> ir::Value {
        if self.isa.has_round() {
            return clif_round(builder.ins(), value);
        }

        let vmctx = self.vmctx_val(&mut builder.cursor());
        let round = round_builtin(&mut self.builtin_functions, builder.func);
        let round_one = |builder: &mut FunctionBuilder, value: ir::Value| {
            let call = builder.ins().call(round, &[vmctx, value]);
            *builder.func.dfg.inst_results(call).first().unwrap()
        };

        let ty = builder.func.dfg.value_type(value);
        if !ty.is_vector() {
            return round_one(builder, value);
        }

        assert_eq!(ty.bits(), 128);
        let zero = builder.func.dfg.constants.insert(V128Imm([0; 16]).into());
        let mut result = builder.ins().vconst(ty, zero);
        for i in 0..u8::try_from(ty.lane_count()).unwrap() {
            let element = builder.ins().extractlane(value, i);
            let element_rounded = round_one(builder, element);
            result = builder.ins().insertlane(result, element_rounded, i);
        }
        result
    }

    pub fn ceil_f32(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.ceil(val),
            BuiltinFunctions::ceil_f32,
        )
    }

    pub fn ceil_f64(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.ceil(val),
            BuiltinFunctions::ceil_f64,
        )
    }

    pub fn ceil_f32x4(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.ceil(val),
            BuiltinFunctions::ceil_f32,
        )
    }

    pub fn ceil_f64x2(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.ceil(val),
            BuiltinFunctions::ceil_f64,
        )
    }

    pub fn floor_f32(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.floor(val),
            BuiltinFunctions::floor_f32,
        )
    }

    pub fn floor_f64(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.floor(val),
            BuiltinFunctions::floor_f64,
        )
    }

    pub fn floor_f32x4(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.floor(val),
            BuiltinFunctions::floor_f32,
        )
    }

    pub fn floor_f64x2(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.floor(val),
            BuiltinFunctions::floor_f64,
        )
    }

    pub fn trunc_f32(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.trunc(val),
            BuiltinFunctions::trunc_f32,
        )
    }

    pub fn trunc_f64(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.trunc(val),
            BuiltinFunctions::trunc_f64,
        )
    }

    pub fn trunc_f32x4(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.trunc(val),
            BuiltinFunctions::trunc_f32,
        )
    }

    pub fn trunc_f64x2(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.trunc(val),
            BuiltinFunctions::trunc_f64,
        )
    }

    pub fn nearest_f32(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.nearest(val),
            BuiltinFunctions::nearest_f32,
        )
    }

    pub fn nearest_f64(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.nearest(val),
            BuiltinFunctions::nearest_f64,
        )
    }

    pub fn nearest_f32x4(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.nearest(val),
            BuiltinFunctions::nearest_f32,
        )
    }

    pub fn nearest_f64x2(&mut self, builder: &mut FunctionBuilder, value: ir::Value) -> ir::Value {
        self.isa_round(
            builder,
            value,
            |ins, val| ins.nearest(val),
            BuiltinFunctions::nearest_f64,
        )
    }

    pub fn swizzle(
        &mut self,
        builder: &mut FunctionBuilder,
        a: ir::Value,
        b: ir::Value,
    ) -> ir::Value {
        // On x86, swizzle would typically be compiled to `pshufb`, except
        // that that's not available on CPUs that lack SSSE3. In that case,
        // fall back to a builtin function.
        if !self.is_x86() || self.isa.has_x86_pshufb_lowering() {
            builder.ins().swizzle(a, b)
        } else {
            let swizzle = self.builtin_functions.i8x16_swizzle(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let call = builder.ins().call(swizzle, &[vmctx, a, b]);
            *builder.func.dfg.inst_results(call).first().unwrap()
        }
    }

    pub fn relaxed_swizzle(
        &mut self,
        builder: &mut FunctionBuilder,
        a: ir::Value,
        b: ir::Value,
    ) -> ir::Value {
        // As above, fall back to a builtin if we lack SSSE3.
        if !self.is_x86() || self.isa.has_x86_pshufb_lowering() {
            if !self.is_x86() || self.relaxed_simd_deterministic() {
                builder.ins().swizzle(a, b)
            } else {
                builder.ins().x86_pshufb(a, b)
            }
        } else {
            let swizzle = self.builtin_functions.i8x16_swizzle(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let call = builder.ins().call(swizzle, &[vmctx, a, b]);
            *builder.func.dfg.inst_results(call).first().unwrap()
        }
    }

    pub fn i8x16_shuffle(
        &mut self,
        builder: &mut FunctionBuilder,
        a: ir::Value,
        b: ir::Value,
        lanes: &[u8; 16],
    ) -> ir::Value {
        // As with swizzle, i8x16.shuffle would also commonly be implemented
        // with pshufb, so if we lack SSSE3, fall back to a builtin.
        if !self.is_x86() || self.isa.has_x86_pshufb_lowering() {
            let lanes = ConstantData::from(&lanes[..]);
            let mask = builder.func.dfg.immediates.push(lanes);
            builder.ins().shuffle(a, b, mask)
        } else {
            let lanes = builder
                .func
                .dfg
                .constants
                .insert(ConstantData::from(&lanes[..]));
            let lanes = builder.ins().vconst(I8X16, lanes);
            let i8x16_shuffle = self.builtin_functions.i8x16_shuffle(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let call = builder.ins().call(i8x16_shuffle, &[vmctx, a, b, lanes]);
            *builder.func.dfg.inst_results(call).first().unwrap()
        }
    }

    pub fn fma_f32x4(
        &mut self,
        builder: &mut FunctionBuilder,
        a: ir::Value,
        b: ir::Value,
        c: ir::Value,
    ) -> ir::Value {
        if self.has_native_fma() {
            builder.ins().fma(a, b, c)
        } else if self.relaxed_simd_deterministic() {
            // Deterministic semantics are "fused multiply and add".
            let fma = self.builtin_functions.fma_f32x4(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let call = builder.ins().call(fma, &[vmctx, a, b, c]);
            *builder.func.dfg.inst_results(call).first().unwrap()
        } else {
            let mul = builder.ins().fmul(a, b);
            builder.ins().fadd(mul, c)
        }
    }

    pub fn fma_f64x2(
        &mut self,
        builder: &mut FunctionBuilder,
        a: ir::Value,
        b: ir::Value,
        c: ir::Value,
    ) -> ir::Value {
        if self.has_native_fma() {
            builder.ins().fma(a, b, c)
        } else if self.relaxed_simd_deterministic() {
            // Deterministic semantics are "fused multiply and add".
            let fma = self.builtin_functions.fma_f64x2(builder.func);
            let vmctx = self.vmctx_val(&mut builder.cursor());
            let call = builder.ins().call(fma, &[vmctx, a, b, c]);
            *builder.func.dfg.inst_results(call).first().unwrap()
        } else {
            let mul = builder.ins().fmul(a, b);
            builder.ins().fadd(mul, c)
        }
    }

    pub fn isa(&self) -> &dyn TargetIsa {
        &*self.isa
    }

    pub fn translate_sdiv(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) -> ir::Value {
        self.guard_signed_divide(builder, lhs, rhs);
        builder.ins().sdiv(lhs, rhs)
    }

    pub fn translate_udiv(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) -> ir::Value {
        self.guard_zero_divisor(builder, rhs);
        builder.ins().udiv(lhs, rhs)
    }

    pub fn translate_srem(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) -> ir::Value {
        self.guard_zero_divisor(builder, rhs);
        builder.ins().srem(lhs, rhs)
    }

    pub fn translate_urem(
        &mut self,
        builder: &mut FunctionBuilder,
        lhs: ir::Value,
        rhs: ir::Value,
    ) -> ir::Value {
        self.guard_zero_divisor(builder, rhs);
        builder.ins().urem(lhs, rhs)
    }

    pub fn translate_fcvt_to_sint(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: ir::Type,
        val: ir::Value,
    ) -> ir::Value {
        // NB: for now avoid translating this entire instruction to CLIF and
        // just do it in a libcall.
        if !self.clif_instruction_traps_enabled() {
            self.guard_fcvt_to_int(builder, ty, val, true);
        }
        builder.ins().fcvt_to_sint(ty, val)
    }

    pub fn translate_fcvt_to_uint(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: ir::Type,
        val: ir::Value,
    ) -> ir::Value {
        if !self.clif_instruction_traps_enabled() {
            self.guard_fcvt_to_int(builder, ty, val, false);
        }
        builder.ins().fcvt_to_uint(ty, val)
    }

    /// Returns whether it's acceptable to rely on traps in CLIF memory-related
    /// instructions (e.g. loads and stores).
    ///
    /// This is enabled if `signals_based_traps` is `true` since signal handlers
    /// are available, but this is additionally forcibly disabled if Pulley is
    /// being targeted since the Pulley runtime doesn't catch segfaults for
    /// itself.
    pub fn clif_memory_traps_enabled(&self) -> bool {
        self.tunables.signals_based_traps && !self.is_pulley()
    }

    /// Returns whether loads from the null address are allowed as signals of
    /// whether to trap or not.
    pub fn load_from_zero_allowed(&self) -> bool {
        // Pulley allows loads-from-zero and otherwise this is only allowed with
        // traps + spectre mitigations.
        self.is_pulley()
            || (self.clif_memory_traps_enabled() && self.heap_access_spectre_mitigation())
    }

    /// Returns whether the current location is reachable.
    pub fn is_reachable(&self) -> bool {
        self.stacks.reachable()
    }
}

// Helper function to convert an `IndexType` to an `ir::Type`.
//
// Implementing From/Into trait for `IndexType` or `ir::Type` would
// introduce an extra dependency between `wasmtime_types` and `cranelift_codegen`.
fn index_type_to_ir_type(index_type: IndexType) -> ir::Type {
    match index_type {
        IndexType::I32 => I32,
        IndexType::I64 => I64,
    }
}

/// Operations to [`FuncEnvironment::raw_bulk_memory_operation`].
enum BulkOp {
    /// A `memory.copy` operation, copying memory from `src` to `dst`.
    ///
    /// All of `dst`, `src`, and `len` must be pre-validated and inbounds. All
    /// must have type `env.pointer_type()`.
    MemoryCopy {
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    },

    /// A `memory.fill` operation, setting all bytes of `dst` to `val`.
    ///
    /// Both of `dst` and `len` must be pre-validated and inbounds. Both must
    /// have type `env.pointer_type()`.
    ///
    /// The `val` field must have type `I32`.
    MemoryFill {
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    },
}

#[derive(Copy, Clone)]
enum CheckedEntity {
    Memory(MemoryIndex),
    Table(TableIndex),
    Data(DataIndex),
    #[cfg_attr(
        not(feature = "gc"),
        expect(dead_code, reason = "not worth the #[cfg]")
    )]
    Array,
}

impl From<MemoryIndex> for CheckedEntity {
    fn from(memory_index: MemoryIndex) -> Self {
        CheckedEntity::Memory(memory_index)
    }
}

impl From<TableIndex> for CheckedEntity {
    fn from(table_index: TableIndex) -> Self {
        CheckedEntity::Table(table_index)
    }
}

impl From<DataIndex> for CheckedEntity {
    fn from(data_index: DataIndex) -> Self {
        CheckedEntity::Data(data_index)
    }
}

impl CheckedEntity {
    fn index_type(&self, env: &FuncEnvironment) -> IndexType {
        match *self {
            CheckedEntity::Memory(i) => env.memory(i).idx_type,
            CheckedEntity::Table(i) => env.table(i).idx_type,
            CheckedEntity::Data(_) => IndexType::I32,
            CheckedEntity::Array => IndexType::I32,
        }
    }
}

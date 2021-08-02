use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir;
use cranelift_codegen::ir::condcodes::*;
use cranelift_codegen::ir::immediates::{Offset32, Uimm64};
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{AbiParam, ArgumentPurpose, Function, InstBuilder, Signature};
use cranelift_codegen::isa::{self, TargetFrontendConfig, TargetIsa};
use cranelift_entity::EntityRef;
use cranelift_frontend::FunctionBuilder;
use cranelift_frontend::Variable;
use cranelift_wasm::{
    self, FuncIndex, FuncTranslationState, GlobalIndex, GlobalVariable, MemoryIndex, TableIndex,
    TargetEnvironment, TypeIndex, WasmError, WasmResult, WasmType,
};
use std::convert::TryFrom;
use std::mem;
use wasmparser::Operator;
use wasmtime_environ::{
    BuiltinFunctionIndex, MemoryPlan, MemoryStyle, Module, TableStyle, Tunables, TypeTables,
    VMOffsets, INTERRUPTED, WASM_PAGE_SIZE,
};

/// Compute an `ir::ExternalName` for a given wasm function index.
pub fn get_func_name(func_index: FuncIndex) -> ir::ExternalName {
    ir::ExternalName::user(0, func_index.as_u32())
}

macro_rules! declare_function_signatures {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident( $( $param:ident ),* ) -> ( $( $result:ident ),* );
        )*
    ) => {
        /// A struct with an `Option<ir::SigRef>` member for every builtin
        /// function, to de-duplicate constructing/getting its signature.
        struct BuiltinFunctionSignatures {
            pointer_type: ir::Type,
            reference_type: ir::Type,
            call_conv: isa::CallConv,
            $(
                $name: Option<ir::SigRef>,
            )*
        }

        impl BuiltinFunctionSignatures {
            fn new(
                pointer_type: ir::Type,
                reference_type: ir::Type,
                call_conv: isa::CallConv,
            ) -> Self {
                Self {
                    pointer_type,
                    reference_type,
                    call_conv,
                    $(
                        $name: None,
                    )*
                }
            }

            fn vmctx(&self) -> AbiParam {
                AbiParam::special(self.pointer_type, ArgumentPurpose::VMContext)
            }

            fn reference(&self) -> AbiParam {
                AbiParam::new(self.reference_type)
            }

            fn pointer(&self) -> AbiParam {
                AbiParam::new(self.pointer_type)
            }

            fn i32(&self) -> AbiParam {
                // Some platform ABIs require i32 values to be zero- or sign-
                // extended to the full register width.  We need to indicate
                // this here by using the appropriate .uext or .sext attribute.
                // The attribute can be added unconditionally; platforms whose
                // ABI does not require such extensions will simply ignore it.
                // Note that currently all i32 arguments or return values used
                // by builtin functions are unsigned, so we always use .uext.
                // If that ever changes, we will have to add a second type
                // marker here.
                AbiParam::new(I32).uext()
            }

            fn i64(&self) -> AbiParam {
                AbiParam::new(I64)
            }

            $(
                fn $name(&mut self, func: &mut Function) -> ir::SigRef {
                    let sig = self.$name.unwrap_or_else(|| {
                        func.import_signature(Signature {
                            params: vec![ $( self.$param() ),* ],
                            returns: vec![ $( self.$result() ),* ],
                            call_conv: self.call_conv,
                        })
                    });
                    self.$name = Some(sig);
                    sig
                }
            )*
        }
    };
}

wasmtime_environ::foreach_builtin_function!(declare_function_signatures);

/// The `FuncEnvironment` implementation for use by the `ModuleEnvironment`.
pub struct FuncEnvironment<'module_environment> {
    isa: &'module_environment (dyn TargetIsa + 'module_environment),
    module: &'module_environment Module,
    types: &'module_environment TypeTables,

    /// The Cranelift global holding the vmctx address.
    vmctx: Option<ir::GlobalValue>,

    /// Caches of signatures for builtin functions.
    builtin_function_signatures: BuiltinFunctionSignatures,

    /// Offsets to struct fields accessed by JIT code.
    pub(crate) offsets: VMOffsets<u8>,

    tunables: &'module_environment Tunables,

    /// A function-local variable which stores the cached value of the amount of
    /// fuel remaining to execute. If used this is modified frequently so it's
    /// stored locally as a variable instead of always referenced from the field
    /// in `*const VMInterrupts`
    fuel_var: cranelift_frontend::Variable,

    /// A function-local variable which caches the value of `*const
    /// VMInterrupts` for this function's vmctx argument. This pointer is stored
    /// in the vmctx itself, but never changes for the lifetime of the function,
    /// so if we load it up front we can continue to use it throughout.
    vminterrupts_ptr: cranelift_frontend::Variable,

    fuel_consumed: i64,
}

impl<'module_environment> FuncEnvironment<'module_environment> {
    pub fn new(
        isa: &'module_environment (dyn TargetIsa + 'module_environment),
        module: &'module_environment Module,
        types: &'module_environment TypeTables,
        tunables: &'module_environment Tunables,
    ) -> Self {
        let builtin_function_signatures = BuiltinFunctionSignatures::new(
            isa.pointer_type(),
            match isa.pointer_type() {
                ir::types::I32 => ir::types::R32,
                ir::types::I64 => ir::types::R64,
                _ => panic!(),
            },
            crate::wasmtime_call_conv(isa),
        );
        Self {
            isa,
            module,
            types,
            vmctx: None,
            builtin_function_signatures,
            offsets: VMOffsets::new(isa.pointer_bytes(), module),
            tunables,
            fuel_var: Variable::new(0),
            vminterrupts_ptr: Variable::new(0),

            // Start with at least one fuel being consumed because even empty
            // functions should consume at least some fuel.
            fuel_consumed: 1,
        }
    }

    fn pointer_type(&self) -> ir::Type {
        self.isa.pointer_type()
    }

    fn vmctx(&mut self, func: &mut Function) -> ir::GlobalValue {
        self.vmctx.unwrap_or_else(|| {
            let vmctx = func.create_global_value(ir::GlobalValueData::VMContext);
            self.vmctx = Some(vmctx);
            vmctx
        })
    }

    fn get_table_copy_func(
        &mut self,
        func: &mut Function,
        dst_table_index: TableIndex,
        src_table_index: TableIndex,
    ) -> (ir::SigRef, usize, usize, BuiltinFunctionIndex) {
        let sig = self.builtin_function_signatures.table_copy(func);
        (
            sig,
            dst_table_index.as_u32() as usize,
            src_table_index.as_u32() as usize,
            BuiltinFunctionIndex::table_copy(),
        )
    }

    fn get_table_init_func(
        &mut self,
        func: &mut Function,
        table_index: TableIndex,
    ) -> (ir::SigRef, usize, BuiltinFunctionIndex) {
        let sig = self.builtin_function_signatures.table_init(func);
        let table_index = table_index.as_u32() as usize;
        (sig, table_index, BuiltinFunctionIndex::table_init())
    }

    fn get_elem_drop_func(&mut self, func: &mut Function) -> (ir::SigRef, BuiltinFunctionIndex) {
        let sig = self.builtin_function_signatures.elem_drop(func);
        (sig, BuiltinFunctionIndex::elem_drop())
    }

    fn get_memory_atomic_wait(
        &mut self,
        func: &mut Function,
        memory_index: MemoryIndex,
        ty: ir::Type,
    ) -> (ir::SigRef, usize, BuiltinFunctionIndex) {
        match ty {
            I32 => (
                self.builtin_function_signatures.memory_atomic_wait32(func),
                memory_index.index(),
                BuiltinFunctionIndex::memory_atomic_wait32(),
            ),
            I64 => (
                self.builtin_function_signatures.memory_atomic_wait64(func),
                memory_index.index(),
                BuiltinFunctionIndex::memory_atomic_wait64(),
            ),
            x => panic!("get_memory_atomic_wait unsupported type: {:?}", x),
        }
    }

    fn get_memory_init_func(&mut self, func: &mut Function) -> (ir::SigRef, BuiltinFunctionIndex) {
        (
            self.builtin_function_signatures.memory_init(func),
            BuiltinFunctionIndex::memory_init(),
        )
    }

    fn get_data_drop_func(&mut self, func: &mut Function) -> (ir::SigRef, BuiltinFunctionIndex) {
        (
            self.builtin_function_signatures.data_drop(func),
            BuiltinFunctionIndex::data_drop(),
        )
    }

    /// Translates load of builtin function and returns a pair of values `vmctx`
    /// and address of the loaded function.
    fn translate_load_builtin_function_address(
        &mut self,
        pos: &mut FuncCursor<'_>,
        callee_func_idx: BuiltinFunctionIndex,
    ) -> (ir::Value, ir::Value) {
        // We use an indirect call so that we don't have to patch the code at runtime.
        let pointer_type = self.pointer_type();
        let vmctx = self.vmctx(&mut pos.func);
        let base = pos.ins().global_value(pointer_type, vmctx);

        let mut mem_flags = ir::MemFlags::trusted();
        mem_flags.set_readonly();

        // Load the callee address.
        let body_offset =
            i32::try_from(self.offsets.vmctx_builtin_function(callee_func_idx)).unwrap();
        let func_addr = pos.ins().load(pointer_type, mem_flags, base, body_offset);

        (base, func_addr)
    }

    /// Generate code to increment or decrement the given `externref`'s
    /// reference count.
    ///
    /// The new reference count is returned.
    fn mutate_extenref_ref_count(
        &mut self,
        builder: &mut FunctionBuilder,
        externref: ir::Value,
        delta: i64,
    ) -> ir::Value {
        debug_assert!(delta == -1 || delta == 1);

        let pointer_type = self.pointer_type();

        // If this changes that's ok, the `atomic_rmw` below just needs to be
        // preceded with an add instruction of `externref` and the offset.
        assert_eq!(self.offsets.vm_extern_data_ref_count(), 0);
        let delta = builder.ins().iconst(pointer_type, delta);
        builder.ins().atomic_rmw(
            pointer_type,
            ir::MemFlags::trusted(),
            ir::AtomicRmwOp::Add,
            externref,
            delta,
        )
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
                readonly: true,
            });
            (global, 0)
        }
    }

    fn declare_vminterrupts_ptr(&mut self, builder: &mut FunctionBuilder<'_>) {
        // We load the `*const VMInterrupts` value stored within vmctx at the
        // head of the function and reuse the same value across the entire
        // function. This is possible since we know that the pointer never
        // changes for the lifetime of the function.
        let pointer_type = self.pointer_type();
        builder.declare_var(self.vminterrupts_ptr, pointer_type);
        let vmctx = self.vmctx(builder.func);
        let base = builder.ins().global_value(pointer_type, vmctx);
        let offset = i32::try_from(self.offsets.vmctx_interrupts()).unwrap();
        let interrupt_ptr = builder
            .ins()
            .load(pointer_type, ir::MemFlags::trusted(), base, offset);
        builder.def_var(self.vminterrupts_ptr, interrupt_ptr);
    }

    fn fuel_function_entry(&mut self, builder: &mut FunctionBuilder<'_>) {
        // On function entry we load the amount of fuel into a function-local
        // `self.fuel_var` to make fuel modifications fast locally. This cache
        // is then periodically flushed to the Store-defined location in
        // `VMInterrupts` later.
        builder.declare_var(self.fuel_var, ir::types::I64);
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

        self.fuel_consumed += match op {
            // Nop and drop generate no code, so don't consume fuel for them.
            Operator::Nop | Operator::Drop => 0,

            // Control flow may create branches, but is generally cheap and
            // free, so don't consume fuel. Note the lack of `if` since some
            // cost is incurred with the conditional check.
            Operator::Block { .. }
            | Operator::Loop { .. }
            | Operator::Unreachable
            | Operator::Return
            | Operator::Else
            | Operator::End => 0,

            // everything else, just call it one operation.
            _ => 1,
        };

        match op {
            // Exiting a function (via a return or unreachable) or otherwise
            // entering a different function (via a call) means that we need to
            // update the fuel consumption in `VMInterrupts` because we're
            // about to move control out of this function itself and the fuel
            // may need to be read.
            //
            // Before this we need to update the fuel counter from our own cost
            // leading up to this function call, and then we can store
            // `self.fuel_var` into `VMInterrupts`.
            Operator::Unreachable
            | Operator::Return
            | Operator::CallIndirect { .. }
            | Operator::Call { .. }
            | Operator::ReturnCall { .. }
            | Operator::ReturnCallIndirect { .. } => {
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

    /// Loads the fuel consumption value from `VMInterrupts` into `self.fuel_var`
    fn fuel_load_into_var(&mut self, builder: &mut FunctionBuilder<'_>) {
        let (addr, offset) = self.fuel_addr_offset(builder);
        let fuel = builder
            .ins()
            .load(ir::types::I64, ir::MemFlags::trusted(), addr, offset);
        builder.def_var(self.fuel_var, fuel);
    }

    /// Stores the fuel consumption value from `self.fuel_var` into
    /// `VMInterrupts`.
    fn fuel_save_from_var(&mut self, builder: &mut FunctionBuilder<'_>) {
        let (addr, offset) = self.fuel_addr_offset(builder);
        let fuel_consumed = builder.use_var(self.fuel_var);
        builder
            .ins()
            .store(ir::MemFlags::trusted(), fuel_consumed, addr, offset);
    }

    /// Returns the `(address, offset)` of the fuel consumption within
    /// `VMInterrupts`, used to perform loads/stores later.
    fn fuel_addr_offset(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
    ) -> (ir::Value, ir::immediates::Offset32) {
        (
            builder.use_var(self.vminterrupts_ptr),
            i32::from(self.offsets.vminterrupts_fuel_consumed()).into(),
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
        let cmp = builder.ins().ifcmp(fuel, zero);
        builder
            .ins()
            .brif(IntCC::SignedGreaterThanOrEqual, cmp, out_of_gas_block, &[]);
        builder.ins().jump(continuation_block, &[]);
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
        let out_of_gas_sig = self.builtin_function_signatures.out_of_gas(builder.func);
        let (vmctx, out_of_gas) = self.translate_load_builtin_function_address(
            &mut builder.cursor(),
            BuiltinFunctionIndex::out_of_gas(),
        );
        builder
            .ins()
            .call_indirect(out_of_gas_sig, out_of_gas, &[vmctx]);
        self.fuel_load_into_var(builder);
        builder.ins().jump(continuation_block, &[]);
        builder.seal_block(continuation_block);

        builder.switch_to_block(continuation_block);
    }
}

impl<'module_environment> TargetEnvironment for FuncEnvironment<'module_environment> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.isa.frontend_config()
    }

    fn reference_type(&self, ty: WasmType) -> ir::Type {
        wasmtime_environ::reference_type(ty, self.pointer_type())
    }
}

impl<'module_environment> cranelift_wasm::FuncEnvironment for FuncEnvironment<'module_environment> {
    fn is_wasm_parameter(&self, _signature: &ir::Signature, index: usize) -> bool {
        // The first two parameters are the vmctx and caller vmctx. The rest are
        // the wasm parameters.
        index >= 2
    }

    fn after_locals(&mut self, num_locals: usize) {
        self.vminterrupts_ptr = Variable::new(num_locals);
        self.fuel_var = Variable::new(num_locals + 1);
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> WasmResult<ir::Table> {
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
                let from_offset = self.offsets.vmctx_vmtable_import_from(index);
                let table = func.create_global_value(ir::GlobalValueData::Load {
                    base: vmctx,
                    offset: Offset32::new(i32::try_from(from_offset).unwrap()),
                    global_type: pointer_type,
                    readonly: true,
                });
                let base_offset = i32::from(self.offsets.vmtable_definition_base());
                let current_elements_offset =
                    i32::from(self.offsets.vmtable_definition_current_elements());
                (table, base_offset, current_elements_offset)
            }
        };

        let base_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(base_offset),
            global_type: pointer_type,
            readonly: false,
        });
        let bound_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(current_elements_offset),
            global_type: self.offsets.type_of_vmtable_definition_current_elements(),
            readonly: false,
        });

        let element_size = u64::from(
            self.reference_type(self.module.table_plans[index].table.wasm_ty)
                .bytes(),
        );

        Ok(func.create_table(ir::TableData {
            base_gv,
            min_size: Uimm64::new(0),
            bound_gv,
            element_size: Uimm64::new(element_size),
            index_type: I32,
        }))
    }

    fn translate_table_grow(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor<'_>,
        table_index: TableIndex,
        _table: ir::Table,
        delta: ir::Value,
        init_value: ir::Value,
    ) -> WasmResult<ir::Value> {
        let (func_idx, func_sig) =
            match self.module.table_plans[table_index].table.wasm_ty {
                WasmType::FuncRef => (
                    BuiltinFunctionIndex::table_grow_funcref(),
                    self.builtin_function_signatures
                        .table_grow_funcref(&mut pos.func),
                ),
                WasmType::ExternRef => (
                    BuiltinFunctionIndex::table_grow_externref(),
                    self.builtin_function_signatures
                        .table_grow_externref(&mut pos.func),
                ),
                _ => return Err(WasmError::Unsupported(
                    "`table.grow` with a table element type that is not `funcref` or `externref`"
                        .into(),
                )),
            };

        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);

        let table_index_arg = pos.ins().iconst(I32, table_index.as_u32() as i64);
        let call_inst = pos.ins().call_indirect(
            func_sig,
            func_addr,
            &[vmctx, table_index_arg, delta, init_value],
        );

        Ok(pos.func.dfg.first_result(call_inst))
    }

    fn translate_table_get(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        table: ir::Table,
        index: ir::Value,
    ) -> WasmResult<ir::Value> {
        let pointer_type = self.pointer_type();

        let plan = &self.module.table_plans[table_index];
        match plan.table.wasm_ty {
            WasmType::FuncRef => match plan.style {
                TableStyle::CallerChecksSignature => {
                    let table_entry_addr = builder.ins().table_addr(pointer_type, table, index, 0);
                    Ok(builder.ins().load(
                        pointer_type,
                        ir::MemFlags::trusted(),
                        table_entry_addr,
                        0,
                    ))
                }
            },
            WasmType::ExternRef => {
                // Our read barrier for `externref` tables is roughly equivalent
                // to the following pseudocode:
                //
                // ```
                // let elem = table[index]
                // if elem is not null:
                //     let (next, end) = VMExternRefActivationsTable bump region
                //     if next != end:
                //         elem.ref_count += 1
                //         *next = elem
                //         next += 1
                //     else:
                //         call activations_table_insert_with_gc(elem)
                // return elem
                // ```
                //
                // This ensures that all `externref`s coming out of tables and
                // onto the stack are safely held alive by the
                // `VMExternRefActivationsTable`.

                let reference_type = self.reference_type(WasmType::ExternRef);

                builder.ensure_inserted_block();
                let continue_block = builder.create_block();
                let non_null_elem_block = builder.create_block();
                let gc_block = builder.create_block();
                let no_gc_block = builder.create_block();
                let current_block = builder.current_block().unwrap();
                builder.insert_block_after(non_null_elem_block, current_block);
                builder.insert_block_after(no_gc_block, non_null_elem_block);
                builder.insert_block_after(gc_block, no_gc_block);
                builder.insert_block_after(continue_block, gc_block);

                // Load the table element.
                let elem_addr = builder.ins().table_addr(pointer_type, table, index, 0);
                let elem =
                    builder
                        .ins()
                        .load(reference_type, ir::MemFlags::trusted(), elem_addr, 0);

                let elem_is_null = builder.ins().is_null(elem);
                builder.ins().brnz(elem_is_null, continue_block, &[]);
                builder.ins().jump(non_null_elem_block, &[]);

                // Load the `VMExternRefActivationsTable::next` bump finger and
                // the `VMExternRefActivationsTable::end` bump boundary.
                builder.switch_to_block(non_null_elem_block);
                let vmctx = self.vmctx(&mut builder.func);
                let vmctx = builder.ins().global_value(pointer_type, vmctx);
                let activations_table = builder.ins().load(
                    pointer_type,
                    ir::MemFlags::trusted(),
                    vmctx,
                    i32::try_from(self.offsets.vmctx_externref_activations_table()).unwrap(),
                );
                let next = builder.ins().load(
                    pointer_type,
                    ir::MemFlags::trusted(),
                    activations_table,
                    i32::try_from(self.offsets.vm_extern_ref_activation_table_next()).unwrap(),
                );
                let end = builder.ins().load(
                    pointer_type,
                    ir::MemFlags::trusted(),
                    activations_table,
                    i32::try_from(self.offsets.vm_extern_ref_activation_table_end()).unwrap(),
                );

                // If `next == end`, then we are at full capacity. Call a
                // builtin to do a GC and insert this reference into the
                // just-swept table for us.
                let at_capacity = builder.ins().icmp(ir::condcodes::IntCC::Equal, next, end);
                builder.ins().brnz(at_capacity, gc_block, &[]);
                builder.ins().jump(no_gc_block, &[]);
                builder.switch_to_block(gc_block);
                let builtin_idx = BuiltinFunctionIndex::activations_table_insert_with_gc();
                let builtin_sig = self
                    .builtin_function_signatures
                    .activations_table_insert_with_gc(builder.func);
                let (vmctx, builtin_addr) = self
                    .translate_load_builtin_function_address(&mut builder.cursor(), builtin_idx);
                builder
                    .ins()
                    .call_indirect(builtin_sig, builtin_addr, &[vmctx, elem]);
                builder.ins().jump(continue_block, &[]);

                // If `next != end`, then:
                //
                // * increment this reference's ref count,
                // * store the reference into the bump table at `*next`,
                // * and finally increment the `next` bump finger.
                builder.switch_to_block(no_gc_block);
                self.mutate_extenref_ref_count(builder, elem, 1);
                builder.ins().store(ir::MemFlags::trusted(), elem, next, 0);

                let new_next = builder
                    .ins()
                    .iadd_imm(next, i64::from(reference_type.bytes()));
                builder.ins().store(
                    ir::MemFlags::trusted(),
                    new_next,
                    activations_table,
                    i32::try_from(self.offsets.vm_extern_ref_activation_table_next()).unwrap(),
                );

                builder.ins().jump(continue_block, &[]);
                builder.switch_to_block(continue_block);

                builder.seal_block(non_null_elem_block);
                builder.seal_block(gc_block);
                builder.seal_block(no_gc_block);
                builder.seal_block(continue_block);

                Ok(elem)
            }
            ty => Err(WasmError::Unsupported(format!(
                "unsupported table type for `table.get` instruction: {:?}",
                ty
            ))),
        }
    }

    fn translate_table_set(
        &mut self,
        builder: &mut FunctionBuilder,
        table_index: TableIndex,
        table: ir::Table,
        value: ir::Value,
        index: ir::Value,
    ) -> WasmResult<()> {
        let pointer_type = self.pointer_type();

        let plan = &self.module.table_plans[table_index];
        match plan.table.wasm_ty {
            WasmType::FuncRef => match plan.style {
                TableStyle::CallerChecksSignature => {
                    let table_entry_addr = builder.ins().table_addr(pointer_type, table, index, 0);
                    builder
                        .ins()
                        .store(ir::MemFlags::trusted(), value, table_entry_addr, 0);
                    Ok(())
                }
            },
            WasmType::ExternRef => {
                // Our write barrier for `externref`s being copied out of the
                // stack and into a table is roughly equivalent to the following
                // pseudocode:
                //
                // ```
                // if value != null:
                //     value.ref_count += 1
                // let current_elem = table[index]
                // table[index] = value
                // if current_elem != null:
                //     current_elem.ref_count -= 1
                //     if current_elem.ref_count == 0:
                //         call drop_externref(current_elem)
                // ```
                //
                // This write barrier is responsible for ensuring that:
                //
                // 1. The value's ref count is incremented now that the
                //    table is holding onto it. This is required for memory safety.
                //
                // 2. The old table element, if any, has its ref count
                //    decremented, and that the wrapped data is dropped if the
                //    ref count reaches zero. This is not required for memory
                //    safety, but is required to avoid leaks. Furthermore, the
                //    destructor might GC or touch this table, so we must only
                //    drop the old table element *after* we've replaced it with
                //    the new `value`!

                builder.ensure_inserted_block();
                let current_block = builder.current_block().unwrap();
                let inc_ref_count_block = builder.create_block();
                builder.insert_block_after(inc_ref_count_block, current_block);
                let check_current_elem_block = builder.create_block();
                builder.insert_block_after(check_current_elem_block, inc_ref_count_block);
                let dec_ref_count_block = builder.create_block();
                builder.insert_block_after(dec_ref_count_block, check_current_elem_block);
                let drop_block = builder.create_block();
                builder.insert_block_after(drop_block, dec_ref_count_block);
                let continue_block = builder.create_block();
                builder.insert_block_after(continue_block, drop_block);

                // Calculate the table address of the current element and do
                // bounds checks. This is the first thing we do, because we
                // don't want to modify any ref counts if this `table.set` is
                // going to trap.
                let table_entry_addr = builder.ins().table_addr(pointer_type, table, index, 0);

                // If value is not null, increment `value`'s ref count.
                //
                // This has to come *before* decrementing the current table
                // element's ref count, because it might reach ref count == zero,
                // causing us to deallocate the current table element. However,
                // if `value` *is* the current table element (and therefore this
                // whole `table.set` is a no-op), then we would incorrectly
                // deallocate `value` and leave it in the table, leading to use
                // after free.
                let value_is_null = builder.ins().is_null(value);
                builder
                    .ins()
                    .brnz(value_is_null, check_current_elem_block, &[]);
                builder.ins().jump(inc_ref_count_block, &[]);
                builder.switch_to_block(inc_ref_count_block);
                self.mutate_extenref_ref_count(builder, value, 1);
                builder.ins().jump(check_current_elem_block, &[]);

                // Grab the current element from the table, and store the new
                // `value` into the table.
                //
                // Note that we load the current element as a pointer, not a
                // reference. This is so that if we call out-of-line to run its
                // destructor, and its destructor triggers GC, this reference is
                // not recorded in the stack map (which would lead to the GC
                // saving a reference to a deallocated object, and then using it
                // after its been freed).
                builder.switch_to_block(check_current_elem_block);
                let current_elem =
                    builder
                        .ins()
                        .load(pointer_type, ir::MemFlags::trusted(), table_entry_addr, 0);
                builder
                    .ins()
                    .store(ir::MemFlags::trusted(), value, table_entry_addr, 0);

                // If the current element is non-null, decrement its reference
                // count. And if its reference count has reached zero, then make
                // an out-of-line call to deallocate it.
                let current_elem_is_null =
                    builder
                        .ins()
                        .icmp_imm(ir::condcodes::IntCC::Equal, current_elem, 0);
                builder
                    .ins()
                    .brz(current_elem_is_null, dec_ref_count_block, &[]);
                builder.ins().jump(continue_block, &[]);

                builder.switch_to_block(dec_ref_count_block);
                let prev_ref_count = self.mutate_extenref_ref_count(builder, current_elem, -1);
                let one = builder.ins().iconst(pointer_type, 1);
                builder
                    .ins()
                    .br_icmp(IntCC::Equal, one, prev_ref_count, drop_block, &[]);
                builder.ins().jump(continue_block, &[]);

                // Call the `drop_externref` builtin to (you guessed it) drop
                // the `externref`.
                builder.switch_to_block(drop_block);
                let builtin_idx = BuiltinFunctionIndex::drop_externref();
                let builtin_sig = self
                    .builtin_function_signatures
                    .drop_externref(builder.func);
                let (_vmctx, builtin_addr) = self
                    .translate_load_builtin_function_address(&mut builder.cursor(), builtin_idx);
                builder
                    .ins()
                    .call_indirect(builtin_sig, builtin_addr, &[current_elem]);
                builder.ins().jump(continue_block, &[]);

                builder.switch_to_block(continue_block);

                builder.seal_block(inc_ref_count_block);
                builder.seal_block(check_current_elem_block);
                builder.seal_block(dec_ref_count_block);
                builder.seal_block(drop_block);
                builder.seal_block(continue_block);

                Ok(())
            }
            ty => Err(WasmError::Unsupported(format!(
                "unsupported table type for `table.set` instruction: {:?}",
                ty
            ))),
        }
    }

    fn translate_table_fill(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor<'_>,
        table_index: TableIndex,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let (builtin_idx, builtin_sig) =
            match self.module.table_plans[table_index].table.wasm_ty {
                WasmType::FuncRef => (
                    BuiltinFunctionIndex::table_fill_funcref(),
                    self.builtin_function_signatures
                        .table_fill_funcref(&mut pos.func),
                ),
                WasmType::ExternRef => (
                    BuiltinFunctionIndex::table_fill_externref(),
                    self.builtin_function_signatures
                        .table_fill_externref(&mut pos.func),
                ),
                _ => return Err(WasmError::Unsupported(
                    "`table.fill` with a table element type that is not `funcref` or `externref`"
                        .into(),
                )),
            };

        let (vmctx, builtin_addr) =
            self.translate_load_builtin_function_address(&mut pos, builtin_idx);

        let table_index_arg = pos.ins().iconst(I32, table_index.as_u32() as i64);
        pos.ins().call_indirect(
            builtin_sig,
            builtin_addr,
            &[vmctx, table_index_arg, dst, val, len],
        );

        Ok(())
    }

    fn translate_ref_null(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor,
        ty: WasmType,
    ) -> WasmResult<ir::Value> {
        Ok(match ty {
            WasmType::FuncRef => pos.ins().iconst(self.pointer_type(), 0),
            WasmType::ExternRef => pos.ins().null(self.reference_type(ty)),
            _ => {
                return Err(WasmError::Unsupported(
                    "`ref.null T` that is not a `funcref` or an `externref`".into(),
                ));
            }
        })
    }

    fn translate_ref_is_null(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor,
        value: ir::Value,
    ) -> WasmResult<ir::Value> {
        let bool_is_null = match pos.func.dfg.value_type(value) {
            // `externref`
            ty if ty.is_ref() => pos.ins().is_null(value),
            // `funcref`
            ty if ty == self.pointer_type() => {
                pos.ins()
                    .icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, value, 0)
            }
            _ => unreachable!(),
        };

        Ok(pos.ins().bint(ir::types::I32, bool_is_null))
    }

    fn translate_ref_func(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor<'_>,
        func_index: FuncIndex,
    ) -> WasmResult<ir::Value> {
        let vmctx = self.vmctx(&mut pos.func);
        let vmctx = pos.ins().global_value(self.pointer_type(), vmctx);
        let offset = self.offsets.vmctx_anyfunc(func_index);
        Ok(pos.ins().iadd_imm(vmctx, i64::from(offset)))
    }

    fn translate_custom_global_get(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor<'_>,
        index: cranelift_wasm::GlobalIndex,
    ) -> WasmResult<ir::Value> {
        debug_assert_eq!(
            self.module.globals[index].wasm_ty,
            WasmType::ExternRef,
            "We only use GlobalVariable::Custom for externref"
        );

        let builtin_index = BuiltinFunctionIndex::externref_global_get();
        let builtin_sig = self
            .builtin_function_signatures
            .externref_global_get(&mut pos.func);

        let (vmctx, builtin_addr) =
            self.translate_load_builtin_function_address(&mut pos, builtin_index);

        let global_index_arg = pos.ins().iconst(I32, index.as_u32() as i64);
        let call_inst =
            pos.ins()
                .call_indirect(builtin_sig, builtin_addr, &[vmctx, global_index_arg]);

        Ok(pos.func.dfg.first_result(call_inst))
    }

    fn translate_custom_global_set(
        &mut self,
        mut pos: cranelift_codegen::cursor::FuncCursor<'_>,
        index: cranelift_wasm::GlobalIndex,
        value: ir::Value,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.module.globals[index].wasm_ty,
            WasmType::ExternRef,
            "We only use GlobalVariable::Custom for externref"
        );

        let builtin_index = BuiltinFunctionIndex::externref_global_set();
        let builtin_sig = self
            .builtin_function_signatures
            .externref_global_set(&mut pos.func);

        let (vmctx, builtin_addr) =
            self.translate_load_builtin_function_address(&mut pos, builtin_index);

        let global_index_arg = pos.ins().iconst(I32, index.as_u32() as i64);
        pos.ins()
            .call_indirect(builtin_sig, builtin_addr, &[vmctx, global_index_arg, value]);

        Ok(())
    }

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> WasmResult<ir::Heap> {
        let pointer_type = self.pointer_type();

        let (ptr, base_offset, current_length_offset) = {
            let vmctx = self.vmctx(func);
            if let Some(def_index) = self.module.defined_memory_index(index) {
                let base_offset =
                    i32::try_from(self.offsets.vmctx_vmmemory_definition_base(def_index)).unwrap();
                let current_length_offset = i32::try_from(
                    self.offsets
                        .vmctx_vmmemory_definition_current_length(def_index),
                )
                .unwrap();
                (vmctx, base_offset, current_length_offset)
            } else {
                let from_offset = self.offsets.vmctx_vmmemory_import_from(index);
                let memory = func.create_global_value(ir::GlobalValueData::Load {
                    base: vmctx,
                    offset: Offset32::new(i32::try_from(from_offset).unwrap()),
                    global_type: pointer_type,
                    readonly: true,
                });
                let base_offset = i32::from(self.offsets.vmmemory_definition_base());
                let current_length_offset =
                    i32::from(self.offsets.vmmemory_definition_current_length());
                (memory, base_offset, current_length_offset)
            }
        };

        // If we have a declared maximum, we can make this a "static" heap, which is
        // allocated up front and never moved.
        let (offset_guard_size, heap_style, readonly_base) = match self.module.memory_plans[index] {
            MemoryPlan {
                style: MemoryStyle::Dynamic,
                offset_guard_size,
                pre_guard_size: _,
                memory: _,
            } => {
                let heap_bound = func.create_global_value(ir::GlobalValueData::Load {
                    base: ptr,
                    offset: Offset32::new(current_length_offset),
                    global_type: pointer_type,
                    readonly: false,
                });
                (
                    Uimm64::new(offset_guard_size),
                    ir::HeapStyle::Dynamic {
                        bound_gv: heap_bound,
                    },
                    false,
                )
            }
            MemoryPlan {
                style: MemoryStyle::Static { bound },
                offset_guard_size,
                pre_guard_size: _,
                memory: _,
            } => (
                Uimm64::new(offset_guard_size),
                ir::HeapStyle::Static {
                    bound: Uimm64::new(u64::from(bound) * u64::from(WASM_PAGE_SIZE)),
                },
                true,
            ),
        };

        let heap_base = func.create_global_value(ir::GlobalValueData::Load {
            base: ptr,
            offset: Offset32::new(base_offset),
            global_type: pointer_type,
            readonly: readonly_base,
        });
        Ok(func.create_heap(ir::HeapData {
            base: heap_base,
            min_size: 0.into(),
            offset_guard_size,
            style: heap_style,
            index_type: I32,
        }))
    }

    fn make_global(
        &mut self,
        func: &mut ir::Function,
        index: GlobalIndex,
    ) -> WasmResult<GlobalVariable> {
        // Although `ExternRef`s live at the same memory location as any other
        // type of global at the same index would, getting or setting them
        // requires ref counting barriers. Therefore, we need to use
        // `GlobalVariable::Custom`, as that is the only kind of
        // `GlobalVariable` for which `cranelift-wasm` supports custom access
        // translation.
        if self.module.globals[index].wasm_ty == WasmType::ExternRef {
            return Ok(GlobalVariable::Custom);
        }

        let (gv, offset) = self.get_global_location(func, index);
        Ok(GlobalVariable::Memory {
            gv,
            offset: offset.into(),
            ty: self.module.globals[index].ty,
        })
    }

    fn make_indirect_sig(
        &mut self,
        func: &mut ir::Function,
        index: TypeIndex,
    ) -> WasmResult<ir::SigRef> {
        let index = self.module.types[index].unwrap_function();
        let sig = crate::indirect_signature(self.isa, self.types, index);
        Ok(func.import_signature(sig))
    }

    fn make_direct_func(
        &mut self,
        func: &mut ir::Function,
        index: FuncIndex,
    ) -> WasmResult<ir::FuncRef> {
        let sig = crate::func_signature(self.isa, self.module, self.types, index);
        let signature = func.import_signature(sig);
        let name = get_func_name(index);
        Ok(func.import_function(ir::ExtFuncData {
            name,
            signature,
            // We currently allocate all code segments independently, so nothing
            // is colocated.
            colocated: false,
        }))
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor<'_>,
        table_index: TableIndex,
        table: ir::Table,
        ty_index: TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        let pointer_type = self.pointer_type();

        let table_entry_addr = pos.ins().table_addr(pointer_type, table, callee, 0);

        // Dereference the table entry to get the pointer to the
        // `VMCallerCheckedAnyfunc`.
        let anyfunc_ptr =
            pos.ins()
                .load(pointer_type, ir::MemFlags::trusted(), table_entry_addr, 0);

        // Check for whether the table element is null, and trap if so.
        pos.ins()
            .trapz(anyfunc_ptr, ir::TrapCode::IndirectCallToNull);

        // Dereference anyfunc pointer to get the function address.
        let mem_flags = ir::MemFlags::trusted();
        let func_addr = pos.ins().load(
            pointer_type,
            mem_flags,
            anyfunc_ptr,
            i32::from(self.offsets.vmcaller_checked_anyfunc_func_ptr()),
        );

        // If necessary, check the signature.
        match self.module.table_plans[table_index].style {
            TableStyle::CallerChecksSignature => {
                let sig_id_size = self.offsets.size_of_vmshared_signature_index();
                let sig_id_type = Type::int(u16::from(sig_id_size) * 8).unwrap();
                let vmctx = self.vmctx(pos.func);
                let base = pos.ins().global_value(pointer_type, vmctx);
                let offset =
                    i32::try_from(self.offsets.vmctx_vmshared_signature_id(ty_index)).unwrap();

                // Load the caller ID.
                let mut mem_flags = ir::MemFlags::trusted();
                mem_flags.set_readonly();
                let caller_sig_id = pos.ins().load(sig_id_type, mem_flags, base, offset);

                // Load the callee ID.
                let mem_flags = ir::MemFlags::trusted();
                let callee_sig_id = pos.ins().load(
                    sig_id_type,
                    mem_flags,
                    anyfunc_ptr,
                    i32::from(self.offsets.vmcaller_checked_anyfunc_type_index()),
                );

                // Check that they match.
                let cmp = pos.ins().icmp(IntCC::Equal, callee_sig_id, caller_sig_id);
                pos.ins().trapz(cmp, ir::TrapCode::BadSignature);
            }
        }

        let mut real_call_args = Vec::with_capacity(call_args.len() + 2);
        let caller_vmctx = pos.func.special_param(ArgumentPurpose::VMContext).unwrap();

        // First append the callee vmctx address.
        let vmctx = pos.ins().load(
            pointer_type,
            mem_flags,
            anyfunc_ptr,
            i32::from(self.offsets.vmcaller_checked_anyfunc_vmctx()),
        );
        real_call_args.push(vmctx);
        real_call_args.push(caller_vmctx);

        // Then append the regular call arguments.
        real_call_args.extend_from_slice(call_args);

        Ok(pos.ins().call_indirect(sig_ref, func_addr, &real_call_args))
    }

    fn translate_call(
        &mut self,
        mut pos: FuncCursor<'_>,
        callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        let mut real_call_args = Vec::with_capacity(call_args.len() + 2);
        let caller_vmctx = pos.func.special_param(ArgumentPurpose::VMContext).unwrap();

        // Handle direct calls to locally-defined functions.
        if !self.module.is_imported_function(callee_index) {
            // First append the callee vmctx address, which is the same as the caller vmctx in
            // this case.
            real_call_args.push(caller_vmctx);

            // Then append the caller vmctx address.
            real_call_args.push(caller_vmctx);

            // Then append the regular call arguments.
            real_call_args.extend_from_slice(call_args);

            return Ok(pos.ins().call(callee, &real_call_args));
        }

        // Handle direct calls to imported functions. We use an indirect call
        // so that we don't have to patch the code at runtime.
        let pointer_type = self.pointer_type();
        let sig_ref = pos.func.dfg.ext_funcs[callee].signature;
        let vmctx = self.vmctx(&mut pos.func);
        let base = pos.ins().global_value(pointer_type, vmctx);

        let mem_flags = ir::MemFlags::trusted();

        // Load the callee address.
        let body_offset =
            i32::try_from(self.offsets.vmctx_vmfunction_import_body(callee_index)).unwrap();
        let func_addr = pos.ins().load(pointer_type, mem_flags, base, body_offset);

        // First append the callee vmctx address.
        let vmctx_offset =
            i32::try_from(self.offsets.vmctx_vmfunction_import_vmctx(callee_index)).unwrap();
        let vmctx = pos.ins().load(pointer_type, mem_flags, base, vmctx_offset);
        real_call_args.push(vmctx);
        real_call_args.push(caller_vmctx);

        // Then append the regular call arguments.
        real_call_args.extend_from_slice(call_args);

        Ok(pos.ins().call_indirect(sig_ref, func_addr, &real_call_args))
    }

    fn translate_memory_grow(
        &mut self,
        mut pos: FuncCursor<'_>,
        index: MemoryIndex,
        _heap: ir::Heap,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        let func_sig = self
            .builtin_function_signatures
            .memory32_grow(&mut pos.func);
        let index_arg = index.index();

        let memory_index = pos.ins().iconst(I32, index_arg as i64);
        let (vmctx, func_addr) = self.translate_load_builtin_function_address(
            &mut pos,
            BuiltinFunctionIndex::memory32_grow(),
        );
        let call_inst = pos
            .ins()
            .call_indirect(func_sig, func_addr, &[vmctx, val, memory_index]);
        Ok(*pos.func.dfg.inst_results(call_inst).first().unwrap())
    }

    fn translate_memory_size(
        &mut self,
        mut pos: FuncCursor<'_>,
        index: MemoryIndex,
        _heap: ir::Heap,
    ) -> WasmResult<ir::Value> {
        let pointer_type = self.pointer_type();
        let vmctx = self.vmctx(&mut pos.func);
        let base = pos.ins().global_value(pointer_type, vmctx);
        let current_length_in_bytes = match self.module.defined_memory_index(index) {
            Some(def_index) => {
                let offset = i32::try_from(
                    self.offsets
                        .vmctx_vmmemory_definition_current_length(def_index),
                )
                .unwrap();
                pos.ins()
                    .load(pointer_type, ir::MemFlags::trusted(), base, offset)
            }
            None => {
                let offset = i32::try_from(self.offsets.vmctx_vmmemory_import_from(index)).unwrap();
                let vmmemory_ptr =
                    pos.ins()
                        .load(pointer_type, ir::MemFlags::trusted(), base, offset);
                pos.ins().load(
                    pointer_type,
                    ir::MemFlags::trusted(),
                    vmmemory_ptr,
                    i32::from(self.offsets.vmmemory_definition_current_length()),
                )
            }
        };
        let current_length_in_pages = pos
            .ins()
            .udiv_imm(current_length_in_bytes, i64::from(WASM_PAGE_SIZE));
        if pointer_type == I32 {
            Ok(current_length_in_pages)
        } else {
            assert_eq!(pointer_type, I64);
            Ok(pos.ins().ireduce(I32, current_length_in_pages))
        }
    }

    fn translate_memory_copy(
        &mut self,
        mut pos: FuncCursor,
        src_index: MemoryIndex,
        _src_heap: ir::Heap,
        dst_index: MemoryIndex,
        _dst_heap: ir::Heap,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let src_index = pos.ins().iconst(I32, i64::from(src_index.as_u32()));
        let dst_index = pos.ins().iconst(I32, i64::from(dst_index.as_u32()));

        let (vmctx, func_addr) = self
            .translate_load_builtin_function_address(&mut pos, BuiltinFunctionIndex::memory_copy());

        let func_sig = self.builtin_function_signatures.memory_copy(&mut pos.func);
        pos.ins().call_indirect(
            func_sig,
            func_addr,
            &[vmctx, dst_index, dst, src_index, src, len],
        );

        Ok(())
    }

    fn translate_memory_fill(
        &mut self,
        mut pos: FuncCursor,
        memory_index: MemoryIndex,
        _heap: ir::Heap,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let func_sig = self.builtin_function_signatures.memory_fill(&mut pos.func);
        let memory_index = memory_index.index();

        let memory_index_arg = pos.ins().iconst(I32, memory_index as i64);

        let (vmctx, func_addr) = self
            .translate_load_builtin_function_address(&mut pos, BuiltinFunctionIndex::memory_fill());

        pos.ins().call_indirect(
            func_sig,
            func_addr,
            &[vmctx, memory_index_arg, dst, val, len],
        );

        Ok(())
    }

    fn translate_memory_init(
        &mut self,
        mut pos: FuncCursor,
        memory_index: MemoryIndex,
        _heap: ir::Heap,
        seg_index: u32,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let (func_sig, func_idx) = self.get_memory_init_func(&mut pos.func);

        let memory_index_arg = pos.ins().iconst(I32, memory_index.index() as i64);
        let seg_index_arg = pos.ins().iconst(I32, seg_index as i64);

        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);

        pos.ins().call_indirect(
            func_sig,
            func_addr,
            &[vmctx, memory_index_arg, seg_index_arg, dst, src, len],
        );

        Ok(())
    }

    fn translate_data_drop(&mut self, mut pos: FuncCursor, seg_index: u32) -> WasmResult<()> {
        let (func_sig, func_idx) = self.get_data_drop_func(&mut pos.func);
        let seg_index_arg = pos.ins().iconst(I32, seg_index as i64);
        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);
        pos.ins()
            .call_indirect(func_sig, func_addr, &[vmctx, seg_index_arg]);
        Ok(())
    }

    fn translate_table_size(
        &mut self,
        mut pos: FuncCursor,
        _table_index: TableIndex,
        table: ir::Table,
    ) -> WasmResult<ir::Value> {
        let size_gv = pos.func.tables[table].bound_gv;
        Ok(pos.ins().global_value(ir::types::I32, size_gv))
    }

    fn translate_table_copy(
        &mut self,
        mut pos: FuncCursor,
        dst_table_index: TableIndex,
        _dst_table: ir::Table,
        src_table_index: TableIndex,
        _src_table: ir::Table,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let (func_sig, dst_table_index_arg, src_table_index_arg, func_idx) =
            self.get_table_copy_func(&mut pos.func, dst_table_index, src_table_index);

        let dst_table_index_arg = pos.ins().iconst(I32, dst_table_index_arg as i64);
        let src_table_index_arg = pos.ins().iconst(I32, src_table_index_arg as i64);

        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);

        pos.ins().call_indirect(
            func_sig,
            func_addr,
            &[
                vmctx,
                dst_table_index_arg,
                src_table_index_arg,
                dst,
                src,
                len,
            ],
        );

        Ok(())
    }

    fn translate_table_init(
        &mut self,
        mut pos: FuncCursor,
        seg_index: u32,
        table_index: TableIndex,
        _table: ir::Table,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> WasmResult<()> {
        let (func_sig, table_index_arg, func_idx) =
            self.get_table_init_func(&mut pos.func, table_index);

        let table_index_arg = pos.ins().iconst(I32, table_index_arg as i64);
        let seg_index_arg = pos.ins().iconst(I32, seg_index as i64);

        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);

        pos.ins().call_indirect(
            func_sig,
            func_addr,
            &[vmctx, table_index_arg, seg_index_arg, dst, src, len],
        );

        Ok(())
    }

    fn translate_elem_drop(&mut self, mut pos: FuncCursor, elem_index: u32) -> WasmResult<()> {
        let (func_sig, func_idx) = self.get_elem_drop_func(&mut pos.func);

        let elem_index_arg = pos.ins().iconst(I32, elem_index as i64);

        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);

        pos.ins()
            .call_indirect(func_sig, func_addr, &[vmctx, elem_index_arg]);

        Ok(())
    }

    fn translate_atomic_wait(
        &mut self,
        mut pos: FuncCursor,
        memory_index: MemoryIndex,
        _heap: ir::Heap,
        addr: ir::Value,
        expected: ir::Value,
        timeout: ir::Value,
    ) -> WasmResult<ir::Value> {
        let implied_ty = pos.func.dfg.value_type(expected);
        let (func_sig, memory_index, func_idx) =
            self.get_memory_atomic_wait(&mut pos.func, memory_index, implied_ty);

        let memory_index_arg = pos.ins().iconst(I32, memory_index as i64);

        let (vmctx, func_addr) = self.translate_load_builtin_function_address(&mut pos, func_idx);

        let call_inst = pos.ins().call_indirect(
            func_sig,
            func_addr,
            &[vmctx, memory_index_arg, addr, expected, timeout],
        );

        Ok(*pos.func.dfg.inst_results(call_inst).first().unwrap())
    }

    fn translate_atomic_notify(
        &mut self,
        mut pos: FuncCursor,
        memory_index: MemoryIndex,
        _heap: ir::Heap,
        addr: ir::Value,
        count: ir::Value,
    ) -> WasmResult<ir::Value> {
        let func_sig = self
            .builtin_function_signatures
            .memory_atomic_notify(&mut pos.func);

        let memory_index_arg = pos.ins().iconst(I32, memory_index.index() as i64);

        let (vmctx, func_addr) = self.translate_load_builtin_function_address(
            &mut pos,
            BuiltinFunctionIndex::memory_atomic_notify(),
        );

        let call_inst =
            pos.ins()
                .call_indirect(func_sig, func_addr, &[vmctx, memory_index_arg, addr, count]);

        Ok(*pos.func.dfg.inst_results(call_inst).first().unwrap())
    }

    fn translate_loop_header(&mut self, builder: &mut FunctionBuilder) -> WasmResult<()> {
        // If enabled check the interrupt flag to prevent long or infinite
        // loops.
        //
        // For more information about this see comments in
        // `crates/environ/src/cranelift.rs`
        if self.tunables.interruptable {
            let pointer_type = self.pointer_type();
            let interrupt_ptr = builder.use_var(self.vminterrupts_ptr);
            let interrupt = builder.ins().load(
                pointer_type,
                ir::MemFlags::trusted(),
                interrupt_ptr,
                i32::from(self.offsets.vminterrupts_stack_limit()),
            );
            // Note that the cast to `isize` happens first to allow sign-extension,
            // if necessary, to `i64`.
            let interrupted_sentinel = builder
                .ins()
                .iconst(pointer_type, INTERRUPTED as isize as i64);
            let cmp = builder
                .ins()
                .icmp(IntCC::Equal, interrupt, interrupted_sentinel);
            builder.ins().trapnz(cmp, ir::TrapCode::Interrupt);
        }

        // Additionally if enabled check how much fuel we have remaining to see
        // if we've run out by this point.
        if self.tunables.consume_fuel {
            self.fuel_check(builder);
        }

        Ok(())
    }

    fn before_translate_operator(
        &mut self,
        op: &Operator,
        builder: &mut FunctionBuilder,
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if self.tunables.consume_fuel {
            self.fuel_before_op(op, builder, state.reachable());
        }
        Ok(())
    }

    fn after_translate_operator(
        &mut self,
        op: &Operator,
        builder: &mut FunctionBuilder,
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if self.tunables.consume_fuel && state.reachable() {
            self.fuel_after_op(op, builder);
        }
        Ok(())
    }

    fn before_translate_function(
        &mut self,
        builder: &mut FunctionBuilder,
        _state: &FuncTranslationState,
    ) -> WasmResult<()> {
        // If the `vminterrupts_ptr` variable will get used then we initialize
        // it here.
        if self.tunables.consume_fuel || self.tunables.interruptable {
            self.declare_vminterrupts_ptr(builder);
        }
        // Additionally we initialize `fuel_var` if it will get used.
        if self.tunables.consume_fuel {
            self.fuel_function_entry(builder);
        }
        Ok(())
    }

    fn after_translate_function(
        &mut self,
        builder: &mut FunctionBuilder,
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if self.tunables.consume_fuel && state.reachable() {
            self.fuel_function_exit(builder);
        }
        Ok(())
    }
}

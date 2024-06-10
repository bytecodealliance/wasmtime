//! `cranelift_wasm` environments for translating Wasm to CLIF.
//!
//! Mostly wrappers around the dummy environments, but also supports
//! pre-configured heaps.

use std::collections::{BTreeMap, HashSet};

use super::config::TestConfig;
use cranelift::prelude::EntityRef;
use cranelift_codegen::{
    ir,
    isa::{TargetFrontendConfig, TargetIsa},
};
use cranelift_wasm::{
    DummyEnvironment, FuncEnvironment, FuncIndex, ModuleEnvironment, TargetEnvironment,
    TypeConvert, TypeIndex, WasmHeapType,
};

pub struct ModuleEnv {
    pub inner: DummyEnvironment,
    pub config: TestConfig,
    pub heap_access_spectre_mitigation: bool,
    pub proof_carrying_code: bool,
}

impl ModuleEnv {
    pub fn new(target_isa: &dyn TargetIsa, config: TestConfig) -> Self {
        let inner = DummyEnvironment::new(target_isa.frontend_config());
        Self {
            inner,
            config,
            heap_access_spectre_mitigation: target_isa
                .flags()
                .enable_heap_access_spectre_mitigation(),
            proof_carrying_code: target_isa.flags().enable_pcc(),
        }
    }
}

impl<'data> ModuleEnvironment<'data> for ModuleEnv {
    fn define_function_body(
        &mut self,
        mut validator: wasmparser::FuncValidator<wasmparser::ValidatorResources>,
        body: wasmparser::FunctionBody<'data>,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .func_bytecode_sizes
            .push(body.get_binary_reader().bytes_remaining());

        let func = {
            let mut func_environ = FuncEnv::new(
                &self.inner.info,
                self.inner.expected_reachability.clone(),
                self.config.clone(),
                self.heap_access_spectre_mitigation,
                self.proof_carrying_code,
            );
            let func_index = FuncIndex::new(
                self.inner.get_num_func_imports() + self.inner.info.function_bodies.len(),
            );

            let sig = func_environ
                .inner
                .vmctx_sig(self.inner.get_func_type(func_index));
            let mut func = ir::Function::with_name_signature(
                ir::UserFuncName::user(0, func_index.as_u32()),
                sig,
            );

            self.inner
                .trans
                .translate_body(&mut validator, body, &mut func, &mut func_environ)?;
            func
        };

        self.inner.info.function_bodies.push(func);

        Ok(())
    }

    fn wasm_features(&self) -> wasmparser::WasmFeatures {
        self.inner.wasm_features()
            | wasmparser::WasmFeatures::MEMORY64
            | wasmparser::WasmFeatures::MULTI_MEMORY
            | wasmparser::WasmFeatures::RELAXED_SIMD
    }

    // ================================================================
    // ====== Everything below here is delegated to `self.inner` ======
    // ================================================================

    fn declare_type_func(
        &mut self,
        wasm_func_type: cranelift_wasm::WasmFuncType,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_type_func(wasm_func_type)
    }

    fn declare_func_import(
        &mut self,
        index: cranelift_wasm::TypeIndex,
        module: &'data str,
        field: &'data str,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_func_import(index, module, field)
    }

    fn declare_table_import(
        &mut self,
        table: cranelift_wasm::Table,
        module: &'data str,
        field: &'data str,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_table_import(table, module, field)
    }

    fn declare_memory_import(
        &mut self,
        memory: cranelift_wasm::Memory,
        module: &'data str,
        field: &'data str,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_memory_import(memory, module, field)
    }

    fn declare_global_import(
        &mut self,
        global: cranelift_wasm::Global,
        module: &'data str,
        field: &'data str,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_global_import(global, module, field)
    }

    fn declare_func_type(
        &mut self,
        index: cranelift_wasm::TypeIndex,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_func_type(index)
    }

    fn declare_table(&mut self, table: cranelift_wasm::Table) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_table(table)
    }

    fn declare_memory(&mut self, memory: cranelift_wasm::Memory) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_memory(memory)
    }

    fn declare_global(
        &mut self,
        global: cranelift_wasm::Global,
        init: cranelift_wasm::GlobalInit,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_global(global, init)
    }

    fn declare_func_export(
        &mut self,
        func_index: cranelift_wasm::FuncIndex,
        name: &'data str,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_func_export(func_index, name)
    }

    fn declare_table_export(
        &mut self,
        table_index: cranelift_wasm::TableIndex,
        name: &'data str,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_table_export(table_index, name)
    }

    fn declare_memory_export(
        &mut self,
        memory_index: cranelift_wasm::MemoryIndex,
        name: &'data str,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_memory_export(memory_index, name)
    }

    fn declare_global_export(
        &mut self,
        global_index: cranelift_wasm::GlobalIndex,
        name: &'data str,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_global_export(global_index, name)
    }

    fn declare_start_func(
        &mut self,
        index: cranelift_wasm::FuncIndex,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_start_func(index)
    }

    fn declare_table_elements(
        &mut self,
        table_index: cranelift_wasm::TableIndex,
        base: Option<cranelift_wasm::GlobalIndex>,
        offset: u32,
        elements: Box<[cranelift_wasm::FuncIndex]>,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .declare_table_elements(table_index, base, offset, elements)
    }

    fn declare_passive_element(
        &mut self,
        index: cranelift_wasm::ElemIndex,
        elements: Box<[cranelift_wasm::FuncIndex]>,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_passive_element(index, elements)
    }

    fn declare_passive_data(
        &mut self,
        data_index: cranelift_wasm::DataIndex,
        data: &'data [u8],
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.declare_passive_data(data_index, data)
    }

    fn declare_data_initialization(
        &mut self,
        memory_index: cranelift_wasm::MemoryIndex,
        base: Option<cranelift_wasm::GlobalIndex>,
        offset: u64,
        data: &'data [u8],
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .declare_data_initialization(memory_index, base, offset, data)
    }
}

impl TypeConvert for ModuleEnv {
    fn lookup_heap_type(&self, _index: wasmparser::UnpackedIndex) -> WasmHeapType {
        todo!()
    }
}

pub struct FuncEnv<'a> {
    pub inner: cranelift_wasm::DummyFuncEnvironment<'a>,
    pub config: TestConfig,
    pub name_to_ir_global: BTreeMap<String, ir::GlobalValue>,
    pub next_heap: usize,
    pub heap_access_spectre_mitigation: bool,
    pub proof_carrying_code: bool,
}

impl<'a> FuncEnv<'a> {
    pub fn new(
        mod_info: &'a cranelift_wasm::DummyModuleInfo,
        expected_reachability: Option<cranelift_wasm::ExpectedReachability>,
        config: TestConfig,
        heap_access_spectre_mitigation: bool,
        proof_carrying_code: bool,
    ) -> Self {
        let inner = cranelift_wasm::DummyFuncEnvironment::new(mod_info, expected_reachability);
        Self {
            inner,
            config,
            name_to_ir_global: Default::default(),
            next_heap: 0,
            heap_access_spectre_mitigation,
            proof_carrying_code,
        }
    }
}

impl TypeConvert for FuncEnv<'_> {
    fn lookup_heap_type(&self, _index: wasmparser::UnpackedIndex) -> WasmHeapType {
        todo!()
    }
}

impl<'a> TargetEnvironment for FuncEnv<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.inner.target_config()
    }

    fn heap_access_spectre_mitigation(&self) -> bool {
        self.heap_access_spectre_mitigation
    }

    fn proof_carrying_code(&self) -> bool {
        self.proof_carrying_code
    }
}

impl<'a> FuncEnvironment for FuncEnv<'a> {
    fn make_heap(
        &mut self,
        func: &mut ir::Function,
        index: cranelift_wasm::MemoryIndex,
    ) -> cranelift_wasm::WasmResult<cranelift_wasm::Heap> {
        if self.next_heap < self.config.heaps.len() {
            let heap = &self.config.heaps[self.next_heap];
            self.next_heap += 1;

            // Create all of the globals our test heap depends on in topological
            // order.
            let mut worklist: Vec<&str> = heap
                .dependencies()
                .filter(|g| !self.name_to_ir_global.contains_key(*g))
                .collect();
            let mut in_worklist: HashSet<&str> = worklist.iter().copied().collect();
            'worklist_fixpoint: while let Some(global_name) = worklist.pop() {
                let was_in_set = in_worklist.remove(global_name);
                debug_assert!(was_in_set);

                let global = &self.config.globals[global_name];

                // Check that all of this global's dependencies have already
                // been created. If not, then enqueue them to be created
                // first and re-enqueue this global.
                for g in global.dependencies() {
                    if !self.name_to_ir_global.contains_key(g) {
                        if in_worklist.contains(&g) {
                            return Err(cranelift_wasm::WasmError::User(format!(
                                "dependency cycle between global '{global_name}' and global '{g}'"
                            )));
                        }

                        worklist.push(global_name);
                        let is_new_entry = in_worklist.insert(global_name);
                        debug_assert!(is_new_entry);

                        worklist.push(g);
                        let is_new_entry = in_worklist.insert(g);
                        debug_assert!(is_new_entry);

                        continue 'worklist_fixpoint;
                    }
                }

                // All of this globals dependencies have already been
                // created, we can create it now!
                let data = global.to_ir(&self.name_to_ir_global);
                let g = func.create_global_value(data);
                self.name_to_ir_global.insert(global_name.to_string(), g);
            }

            Ok(self.inner.heaps.push(heap.to_ir(&self.name_to_ir_global)))
        } else {
            self.inner.make_heap(func, index)
        }
    }

    // ================================================================
    // ====== Everything below here is delegated to `self.inner` ======
    // ================================================================

    fn make_global(
        &mut self,
        func: &mut ir::Function,
        index: cranelift_wasm::GlobalIndex,
    ) -> cranelift_wasm::WasmResult<cranelift_wasm::GlobalVariable> {
        self.inner.make_global(func, index)
    }

    fn make_table(
        &mut self,
        func: &mut ir::Function,
        index: cranelift_wasm::TableIndex,
    ) -> cranelift_wasm::WasmResult<ir::Table> {
        self.inner.make_table(func, index)
    }

    fn make_indirect_sig(
        &mut self,
        func: &mut ir::Function,
        index: cranelift_wasm::TypeIndex,
    ) -> cranelift_wasm::WasmResult<ir::SigRef> {
        self.inner.make_indirect_sig(func, index)
    }

    fn make_direct_func(
        &mut self,
        func: &mut ir::Function,
        index: FuncIndex,
    ) -> cranelift_wasm::WasmResult<ir::FuncRef> {
        self.inner.make_direct_func(func, index)
    }

    fn translate_call_indirect(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder,
        table_index: cranelift_wasm::TableIndex,
        table: ir::Table,
        sig_index: cranelift_wasm::TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> cranelift_wasm::WasmResult<ir::Inst> {
        self.inner.translate_call_indirect(
            builder,
            table_index,
            table,
            sig_index,
            sig_ref,
            callee,
            call_args,
        )
    }

    fn translate_return_call_indirect(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder,
        table_index: cranelift_wasm::TableIndex,
        table: ir::Table,
        sig_index: TypeIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.translate_return_call_indirect(
            builder,
            table_index,
            table,
            sig_index,
            sig_ref,
            callee,
            call_args,
        )
    }

    fn translate_return_call_ref(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .translate_return_call_ref(builder, sig_ref, callee, call_args)
    }

    fn translate_memory_grow(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        index: cranelift_wasm::MemoryIndex,
        heap: cranelift_wasm::Heap,
        val: ir::Value,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner.translate_memory_grow(pos, index, heap, val)
    }

    fn translate_memory_size(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        index: cranelift_wasm::MemoryIndex,
        heap: cranelift_wasm::Heap,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner.translate_memory_size(pos, index, heap)
    }

    fn translate_memory_copy(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        src_index: cranelift_wasm::MemoryIndex,
        src_heap: cranelift_wasm::Heap,
        dst_index: cranelift_wasm::MemoryIndex,
        dst_heap: cranelift_wasm::Heap,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .translate_memory_copy(pos, src_index, src_heap, dst_index, dst_heap, dst, src, len)
    }

    fn translate_memory_fill(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        index: cranelift_wasm::MemoryIndex,
        heap: cranelift_wasm::Heap,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .translate_memory_fill(pos, index, heap, dst, val, len)
    }

    fn translate_memory_init(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        index: cranelift_wasm::MemoryIndex,
        heap: cranelift_wasm::Heap,
        seg_index: u32,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .translate_memory_init(pos, index, heap, seg_index, dst, src, len)
    }

    fn translate_data_drop(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        seg_index: u32,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.translate_data_drop(pos, seg_index)
    }

    fn translate_table_size(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        index: cranelift_wasm::TableIndex,
        table: ir::Table,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner.translate_table_size(pos, index, table)
    }

    fn translate_table_grow(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        table_index: cranelift_wasm::TableIndex,
        table: ir::Table,
        delta: ir::Value,
        init_value: ir::Value,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner
            .translate_table_grow(pos, table_index, table, delta, init_value)
    }

    fn translate_table_get(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder,
        table_index: cranelift_wasm::TableIndex,
        table: ir::Table,
        index: ir::Value,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner
            .translate_table_get(builder, table_index, table, index)
    }

    fn translate_table_set(
        &mut self,
        builder: &mut cranelift_frontend::FunctionBuilder,
        table_index: cranelift_wasm::TableIndex,
        table: ir::Table,
        value: ir::Value,
        index: ir::Value,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .translate_table_set(builder, table_index, table, value, index)
    }

    fn translate_table_copy(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        dst_table_index: cranelift_wasm::TableIndex,
        dst_table: ir::Table,
        src_table_index: cranelift_wasm::TableIndex,
        src_table: ir::Table,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.translate_table_copy(
            pos,
            dst_table_index,
            dst_table,
            src_table_index,
            src_table,
            dst,
            src,
            len,
        )
    }

    fn translate_table_fill(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        table_index: cranelift_wasm::TableIndex,
        dst: ir::Value,
        val: ir::Value,
        len: ir::Value,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .translate_table_fill(pos, table_index, dst, val, len)
    }

    fn translate_table_init(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        seg_index: u32,
        table_index: cranelift_wasm::TableIndex,
        table: ir::Table,
        dst: ir::Value,
        src: ir::Value,
        len: ir::Value,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .translate_table_init(pos, seg_index, table_index, table, dst, src, len)
    }

    fn translate_elem_drop(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        seg_index: u32,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner.translate_elem_drop(pos, seg_index)
    }

    fn translate_ref_func(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        func_index: FuncIndex,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner.translate_ref_func(pos, func_index)
    }

    fn translate_custom_global_get(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        global_index: cranelift_wasm::GlobalIndex,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner.translate_custom_global_get(pos, global_index)
    }

    fn translate_custom_global_set(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        global_index: cranelift_wasm::GlobalIndex,
        val: ir::Value,
    ) -> cranelift_wasm::WasmResult<()> {
        self.inner
            .translate_custom_global_set(pos, global_index, val)
    }

    fn translate_atomic_wait(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        index: cranelift_wasm::MemoryIndex,
        heap: cranelift_wasm::Heap,
        addr: ir::Value,
        expected: ir::Value,
        timeout: ir::Value,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner
            .translate_atomic_wait(pos, index, heap, addr, expected, timeout)
    }

    fn translate_atomic_notify(
        &mut self,
        pos: cranelift_codegen::cursor::FuncCursor,
        index: cranelift_wasm::MemoryIndex,
        heap: cranelift_wasm::Heap,
        addr: ir::Value,
        count: ir::Value,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        self.inner
            .translate_atomic_notify(pos, index, heap, addr, count)
    }

    fn heaps(
        &self,
    ) -> &cranelift_codegen::entity::PrimaryMap<cranelift_wasm::Heap, cranelift_wasm::HeapData>
    {
        self.inner.heaps()
    }

    fn relaxed_simd_deterministic(&self) -> bool {
        self.config.relaxed_simd_deterministic
    }

    fn is_x86(&self) -> bool {
        self.config.target.contains("x86_64")
    }

    fn use_x86_pmaddubsw_for_dot(&self) -> bool {
        self.config.target.contains("x86_64")
    }

    fn translate_call_ref(
        &mut self,
        _builder: &mut cranelift_frontend::FunctionBuilder<'_>,
        _ty: ir::SigRef,
        _func: ir::Value,
        _args: &[ir::Value],
    ) -> cranelift_wasm::WasmResult<ir::Inst> {
        unimplemented!()
    }

    fn translate_ref_i31(
        &mut self,
        _pos: cranelift_codegen::cursor::FuncCursor,
        _val: ir::Value,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        unimplemented!()
    }

    fn translate_i31_get_s(
        &mut self,
        _pos: cranelift_codegen::cursor::FuncCursor,
        _i31ref: ir::Value,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        unimplemented!()
    }

    fn translate_i31_get_u(
        &mut self,
        _pos: cranelift_codegen::cursor::FuncCursor,
        _i31ref: ir::Value,
    ) -> cranelift_wasm::WasmResult<ir::Value> {
        unimplemented!()
    }
}

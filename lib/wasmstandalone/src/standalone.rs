use cton_wasm::{Local, FunctionIndex, GlobalIndex, TableIndex, MemoryIndex, RawByte,
                MemoryAddress, Global, GlobalInit, Table, Memory, WasmRuntime};
use cton_frontend::FunctionBuilder;
use cretonne::ir::{MemFlags, Value, InstBuilder, SigRef, FuncRef, ExtFuncData, FunctionName,
                   Signature, ArgumentType, CallConv};
use cretonne::ir::types::*;
use cretonne::ir::condcodes::IntCC;
use cretonne::ir::immediates::Offset32;
use std::mem::transmute;
use std::ptr::copy_nonoverlapping;
use std::ptr::write;

#[derive(Clone, Debug)]
enum TableElement {
    Trap(),
    Function(FunctionIndex),
}

struct GlobalInfo {
    global: Global,
    offset: usize,
}

struct GlobalsData {
    data: Vec<RawByte>,
    info: Vec<GlobalInfo>,
}

struct TableData {
    data: Vec<MemoryAddress>,
    elements: Vec<TableElement>,
    info: Table,
}

struct MemoryData {
    data: Vec<RawByte>,
    info: Memory,
}

const PAGE_SIZE: usize = 65536;

/// Object containing the standalone runtime information. To be passed after creation as argument
/// to `cton_wasm::translatemodule`.
pub struct StandaloneRuntime {
    globals: GlobalsData,
    tables: Vec<TableData>,
    memories: Vec<MemoryData>,
    instantiated: bool,
    has_current_memory: Option<FuncRef>,
    has_grow_memory: Option<FuncRef>,
}

impl StandaloneRuntime {
    /// Allocates the runtime data structures.
    pub fn new() -> StandaloneRuntime {
        StandaloneRuntime {
            globals: GlobalsData {
                data: Vec::new(),
                info: Vec::new(),
            },
            tables: Vec::new(),
            memories: Vec::new(),
            instantiated: false,
            has_current_memory: None,
            has_grow_memory: None,
        }
    }
}

/// This trait is useful for
/// `cton_wasm::translatemodule` because it
/// tells how to translate runtime-dependent wasm instructions. These functions should not be
/// called by the user.
impl WasmRuntime for StandaloneRuntime {
    fn translate_get_global(
        &self,
        builder: &mut FunctionBuilder<Local>,
        global_index: GlobalIndex,
    ) -> Value {
        debug_assert!(self.instantiated);
        let ty = self.globals.info[global_index].global.ty;
        let offset = self.globals.info[global_index].offset;
        let memflags = MemFlags::new();
        let memoffset = Offset32::new(offset as i32);
        let addr: i64 = unsafe { transmute(self.globals.data.as_ptr()) };
        let addr_val = builder.ins().iconst(I64, addr);
        builder.ins().load(ty, memflags, addr_val, memoffset)
    }
    fn translate_set_global(
        &self,
        builder: &mut FunctionBuilder<Local>,
        global_index: GlobalIndex,
        val: Value,
    ) {
        let offset = self.globals.info[global_index].offset;
        let memflags = MemFlags::new();
        let memoffset = Offset32::new(offset as i32);
        let addr: i64 = unsafe { transmute(self.globals.data.as_ptr()) };
        let addr_val = builder.ins().iconst(I64, addr);
        builder.ins().store(memflags, val, addr_val, memoffset);
    }
    fn translate_memory_base_address(
        &self,
        builder: &mut FunctionBuilder<Local>,
        memory_index: MemoryIndex,
    ) -> Value {
        let addr: i64 = unsafe { transmute(self.memories[memory_index].data.as_ptr()) };
        builder.ins().iconst(I64, addr)
    }
    fn translate_grow_memory(
        &mut self,
        builder: &mut FunctionBuilder<Local>,
        pages: Value,
    ) -> Value {
        debug_assert!(self.instantiated);
        let grow_mem_func = match self.has_grow_memory {
            Some(grow_mem_func) => grow_mem_func,
            None => {
                let sig_ref = builder.import_signature(Signature {
                    call_conv: CallConv::Native,
                    argument_bytes: None,
                    argument_types: vec![ArgumentType::new(I32)],
                    return_types: vec![ArgumentType::new(I32)],
                });
                builder.import_function(ExtFuncData {
                    name: FunctionName::new("grow_memory"),
                    signature: sig_ref,
                })
            }
        };
        self.has_grow_memory = Some(grow_mem_func);
        let call_inst = builder.ins().call(grow_mem_func, &[pages]);
        *builder.inst_results(call_inst).first().unwrap()
    }
    fn translate_current_memory(&mut self, builder: &mut FunctionBuilder<Local>) -> Value {
        debug_assert!(self.instantiated);
        let cur_mem_func = match self.has_current_memory {
            Some(cur_mem_func) => cur_mem_func,
            None => {
                let sig_ref = builder.import_signature(Signature {
                    call_conv: CallConv::Native,
                    argument_bytes: None,
                    argument_types: Vec::new(),
                    return_types: vec![ArgumentType::new(I32)],
                });
                builder.import_function(ExtFuncData {
                    name: FunctionName::new("current_memory"),
                    signature: sig_ref,
                })
            }
        };
        self.has_current_memory = Some(cur_mem_func);
        let call_inst = builder.ins().call(cur_mem_func, &[]);
        *builder.inst_results(call_inst).first().unwrap()
    }
    fn translate_call_indirect<'a>(
        &self,
        builder: &'a mut FunctionBuilder<Local>,
        sig_ref: SigRef,
        index_val: Value,
        call_args: &[Value],
    ) -> &'a [Value] {
        let trap_ebb = builder.create_ebb();
        let continue_ebb = builder.create_ebb();
        let size_val = builder.ins().iconst(I32, self.tables[0].info.size as i64);
        let zero_val = builder.ins().iconst(I32, 0);
        builder.ins().br_icmp(
            IntCC::UnsignedLessThan,
            index_val,
            zero_val,
            trap_ebb,
            &[],
        );
        builder.ins().br_icmp(
            IntCC::UnsignedGreaterThanOrEqual,
            index_val,
            size_val,
            trap_ebb,
            &[],
        );
        builder.seal_block(trap_ebb);
        let offset_val = builder.ins().imul_imm(index_val, 4);
        let base_table_addr: i64 = unsafe { transmute(self.tables[0].data.as_ptr()) };
        let table_addr_val = builder.ins().iconst(I32, base_table_addr);
        let table_entry_addr_val = builder.ins().iadd(table_addr_val, offset_val);
        let memflags = MemFlags::new();
        let memoffset = Offset32::new(0);
        let table_entry_val = builder.ins().load(
            I32,
            memflags,
            table_entry_addr_val,
            memoffset,
        );
        let call_inst = builder.ins().call_indirect(
            sig_ref,
            table_entry_val,
            call_args,
        );
        builder.ins().jump(continue_ebb, &[]);
        builder.seal_block(continue_ebb);
        builder.switch_to_block(trap_ebb, &[]);
        builder.ins().trap();
        builder.switch_to_block(continue_ebb, &[]);
        builder.inst_results(call_inst)
    }

    fn begin_translation(&mut self) {
        debug_assert!(!self.instantiated);
        self.instantiated = true;
        // At instantiation, we allocate memory for the globals, the memories and the tables
        // First the globals
        let mut globals_data_size = 0;
        for globalinfo in &mut self.globals.info {
            globalinfo.offset = globals_data_size;
            globals_data_size += globalinfo.global.ty.bytes() as usize;
        }
        self.globals.data.resize(globals_data_size, 0);
        for globalinfo in &self.globals.info {
            match globalinfo.global.initializer {
                GlobalInit::I32Const(val) => unsafe {
                    write(
                        self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        ) as *mut i32,
                        val,
                    )
                },
                GlobalInit::I64Const(val) => unsafe {
                    write(
                        self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        ) as *mut i64,
                        val,
                    )
                },
                GlobalInit::F32Const(val) => unsafe {
                    write(
                        self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        ) as *mut f32,
                        transmute(val),
                    )
                },
                GlobalInit::F64Const(val) => unsafe {
                    write(
                        self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        ) as *mut f64,
                        transmute(val),
                    )
                },
                GlobalInit::Import() => {
                    // We don't initialize, this is inter-module linking
                    // TODO: support inter-module imports
                }
                GlobalInit::GlobalRef(index) => {
                    let ref_offset = self.globals.info[index].offset;
                    let size = globalinfo.global.ty.bytes();
                    unsafe {
                        let dst = self.globals.data.as_mut_ptr().offset(
                            globalinfo.offset as isize,
                        );
                        let src = self.globals.data.as_ptr().offset(ref_offset as isize);
                        copy_nonoverlapping(src, dst, size as usize)
                    }
                }
            }
        }
    }
    fn next_function(&mut self) {
        self.has_current_memory = None;
        self.has_grow_memory = None;
    }
    fn declare_global(&mut self, global: Global) {
        debug_assert!(!self.instantiated);
        self.globals.info.push(GlobalInfo {
            global: global,
            offset: 0,
        });
    }
    fn declare_table(&mut self, table: Table) {
        debug_assert!(!self.instantiated);
        let mut elements_vec = Vec::with_capacity(table.size);
        elements_vec.resize(table.size, TableElement::Trap());
        let mut addresses_vec = Vec::with_capacity(table.size);
        addresses_vec.resize(table.size, 0);
        self.tables.push(TableData {
            info: table,
            data: addresses_vec,
            elements: elements_vec,
        });
    }
    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        offset: usize,
        elements: &[FunctionIndex],
    ) {
        debug_assert!(!self.instantiated);
        for (i, elt) in elements.iter().enumerate() {
            self.tables[table_index].elements[offset + i] = TableElement::Function(*elt);
        }
    }
    fn declare_memory(&mut self, memory: Memory) {
        debug_assert!(!self.instantiated);
        let mut memory_vec = Vec::with_capacity(memory.pages_count * PAGE_SIZE);
        memory_vec.resize(memory.pages_count * PAGE_SIZE, 0);
        self.memories.push(MemoryData {
            info: memory,
            data: memory_vec,
        });
    }
    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        offset: usize,
        data: &[u8],
    ) -> Result<(), String> {
        if offset + data.len() > self.memories[memory_index].info.pages_count * PAGE_SIZE {
            return Err(String::from("initialization data out of bounds"));
        }
        self.memories[memory_index].data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }
}

/// Convenience functions for the user to be called after execution for debug purposes.
impl StandaloneRuntime {
    /// Returns a slice of the contents of allocated linear memory.
    pub fn inspect_memory(&self, memory_index: usize, address: usize, len: usize) -> &[u8] {
        &self.memories
            .get(memory_index)
            .expect(format!("no memory for index {}", memory_index).as_str())
            .data
            [address..address + len]
    }
    /// Shows the value of a global variable.
    pub fn inspect_global(&self, global_index: usize) -> &[u8] {
        let (offset, len) = (
            self.globals.info[global_index].offset,
            self.globals.info[global_index].global.ty.bytes() as usize,
        );
        &self.globals.data[offset..offset + len]
    }
}

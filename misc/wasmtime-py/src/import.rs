//! Support for a calling of an imported function.

extern crate alloc;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyTuple};

use crate::code_memory::CodeMemory;
use crate::function::Function;
use crate::memory::Memory;
use crate::value::{read_value_from, write_value_to};
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{InstBuilder, StackSlotData, StackSlotKind};
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir, isa};
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_wasm::{DefinedFuncIndex, FuncIndex};
use target_lexicon::HOST;
use wasmtime_environ::{Export, Module};
use wasmtime_runtime::{Imports, InstanceHandle, VMContext, VMFunctionBody};

use alloc::rc::Rc;
use core::cell::RefCell;
use core::cmp;
use std::collections::{HashMap, HashSet};

struct BoundPyFunction {
    name: String,
    obj: PyObject,
}

struct ImportObjState {
    calls: Vec<BoundPyFunction>,
    #[allow(dead_code)]
    code_memory: CodeMemory,
}

unsafe extern "C" fn stub_fn(vmctx: *mut VMContext, call_id: u32, values_vec: *mut i64) {
    let gil = Python::acquire_gil();
    let py = gil.python();

    let mut instance = InstanceHandle::from_vmctx(vmctx);
    let (_name, obj) = {
        let state = instance
            .host_state()
            .downcast_mut::<ImportObjState>()
            .expect("state");
        let name = state.calls[call_id as usize].name.to_owned();
        let obj = state.calls[call_id as usize].obj.clone_ref(py);
        (name, obj)
    };
    let module = instance.module_ref();
    let signature = &module.signatures[module.functions[FuncIndex::new(call_id as usize)]];

    let mut args = Vec::new();
    for i in 1..signature.params.len() {
        args.push(read_value_from(
            py,
            values_vec.offset(i as isize - 1),
            signature.params[i].value_type,
        ))
    }
    let result = obj.call(py, PyTuple::new(py, args), None).expect("result");
    for i in 0..signature.returns.len() {
        let val = if result.is_none() {
            0.into_py(py) // FIXME default ???
        } else {
            if i > 0 {
                panic!("multiple returns unsupported");
            }
            result.clone_ref(py)
        };
        write_value_to(
            py,
            values_vec.offset(i as isize),
            signature.returns[i].value_type,
            val,
        );
    }
}

/// Create a trampoline for invoking a python function.
fn make_trampoline(
    isa: &dyn isa::TargetIsa,
    code_memory: &mut CodeMemory,
    fn_builder_ctx: &mut FunctionBuilderContext,
    call_id: u32,
    signature: &ir::Signature,
) -> *const VMFunctionBody {
    // Mostly reverse copy of the similar method from wasmtime's
    // wasmtime-jit/src/compiler.rs.
    let pointer_type = isa.pointer_type();
    let mut stub_sig = ir::Signature::new(isa.frontend_config().default_call_conv);

    // Add the `vmctx` parameter.
    stub_sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));

    // Add the `call_id` parameter.
    stub_sig.params.push(ir::AbiParam::new(types::I32));

    // Add the `values_vec` parameter.
    stub_sig.params.push(ir::AbiParam::new(pointer_type));

    let values_vec_len = 8 * cmp::max(signature.params.len() - 1, signature.returns.len()) as u32;

    let mut context = Context::new();
    context.func =
        ir::Function::with_name_signature(ir::ExternalName::user(0, 0), signature.clone());

    let ss = context.func.create_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        values_vec_len,
    ));
    let value_size = 8;

    {
        let mut builder = FunctionBuilder::new(&mut context.func, fn_builder_ctx);
        let block0 = builder.create_ebb();

        builder.append_ebb_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let values_vec_ptr_val = builder.ins().stack_addr(pointer_type, ss, 0);
        let mflags = ir::MemFlags::trusted();
        for i in 1..signature.params.len() {
            if i == 0 {
                continue;
            }

            let val = builder.func.dfg.ebb_params(block0)[i];
            builder.ins().store(
                mflags,
                val,
                values_vec_ptr_val,
                ((i - 1) * value_size) as i32,
            );
        }

        let vmctx_ptr_val = builder.func.dfg.ebb_params(block0)[0];
        let call_id_val = builder.ins().iconst(types::I32, call_id as i64);

        let callee_args = vec![vmctx_ptr_val, call_id_val, values_vec_ptr_val];

        let new_sig = builder.import_signature(stub_sig.clone());

        let callee_value = builder
            .ins()
            .iconst(pointer_type, stub_fn as *const VMFunctionBody as i64);
        builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let mflags = ir::MemFlags::trusted();
        let mut results = Vec::new();
        for (i, r) in signature.returns.iter().enumerate() {
            let load = builder.ins().load(
                r.value_type,
                mflags,
                values_vec_ptr_val,
                (i * value_size) as i32,
            );
            results.push(load);
        }
        builder.ins().return_(&results);
        builder.finalize()
    }

    let mut code_buf: Vec<u8> = Vec::new();
    let mut reloc_sink = RelocSink {};
    let mut trap_sink = binemit::NullTrapSink {};
    let mut stackmap_sink = binemit::NullStackmapSink {};
    context
        .compile_and_emit(
            isa,
            &mut code_buf,
            &mut reloc_sink,
            &mut trap_sink,
            &mut stackmap_sink,
        )
        .expect("compile_and_emit");

    code_memory
        .allocate_copy_of_byte_slice(&code_buf)
        .expect("allocate_copy_of_byte_slice")
        .as_ptr()
}

fn parse_annotation_type(s: &str) -> ir::Type {
    match s {
        "I32" | "i32" => types::I32,
        "I64" | "i64" => types::I64,
        "F32" | "f32" => types::F32,
        "F64" | "f64" => types::F64,
        _ => panic!("unknown type in annotations"),
    }
}

fn get_signature_from_py_annotation(
    annot: &PyDict,
    pointer_type: ir::Type,
    call_conv: isa::CallConv,
) -> PyResult<ir::Signature> {
    let mut params = Vec::new();
    params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));
    let mut returns = None;
    for (name, value) in annot.iter() {
        let ty = parse_annotation_type(&value.to_string());
        match name.to_string().as_str() {
            "return" => returns = Some(ty),
            _ => params.push(ir::AbiParam::new(ty)),
        }
    }
    Ok(ir::Signature {
        params,
        returns: match returns {
            Some(r) => vec![ir::AbiParam::new(r)],
            None => vec![],
        },
        call_conv,
    })
}

pub fn into_instance_from_obj(
    py: Python,
    global_exports: Rc<RefCell<HashMap<String, Option<wasmtime_runtime::Export>>>>,
    obj: &PyAny,
) -> PyResult<InstanceHandle> {
    let isa = {
        let isa_builder =
            cranelift_native::builder().expect("host machine is not a supported target");
        let flag_builder = cranelift_codegen::settings::builder();
        isa_builder.finish(cranelift_codegen::settings::Flags::new(flag_builder))
    };

    let mut fn_builder_ctx = FunctionBuilderContext::new();
    let mut module = Module::new();
    let mut finished_functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody> =
        PrimaryMap::new();
    let mut code_memory = CodeMemory::new();

    let pointer_type = types::Type::triple_pointer_type(&HOST);
    let call_conv = isa::CallConv::triple_default(&HOST);

    let obj = obj.cast_as::<PyDict>()?;
    let mut bound_functions = Vec::new();
    let mut dependencies = HashSet::new();
    let mut memories = PrimaryMap::new();
    for (name, item) in obj.iter() {
        if item.is_callable() {
            let sig = if item.get_type().is_subclass::<Function>()? {
                // TODO faster calls?
                let wasm_fn = item.cast_as::<Function>()?;
                dependencies.insert(wasm_fn.instance.clone());
                wasm_fn.get_signature()
            } else if item.hasattr("__annotations__")? {
                let annot = item.getattr("__annotations__")?.cast_as::<PyDict>()?;
                get_signature_from_py_annotation(&annot, pointer_type, call_conv)?
            } else {
                // TODO support calls without annotations?
                continue;
            };

            let sig_id = module.signatures.push(sig.clone());
            let func_id = module.functions.push(sig_id);
            module
                .exports
                .insert(name.to_string(), Export::Function(func_id));
            let trampoline = make_trampoline(
                isa.as_ref(),
                &mut code_memory,
                &mut fn_builder_ctx,
                func_id.index() as u32,
                &sig,
            );
            finished_functions.push(trampoline);

            bound_functions.push(BoundPyFunction {
                name: name.to_string(),
                obj: item.into_py(py),
            });
        } else if item.get_type().is_subclass::<Memory>()? {
            let wasm_mem = item.cast_as::<Memory>()?;
            dependencies.insert(wasm_mem.instance.clone());
            let plan = wasm_mem.get_plan();
            let mem_id = module.memory_plans.push(plan);
            let _mem_id_2 = memories.push(wasm_mem.into_import());
            assert_eq!(mem_id, _mem_id_2);
            let _mem_id_3 = module
                .imported_memories
                .push((String::from(""), String::from("")));
            assert_eq!(mem_id, _mem_id_3);
            module
                .exports
                .insert(name.to_string(), Export::Memory(mem_id));
        }
    }

    let imports = Imports::new(
        dependencies,
        PrimaryMap::new(),
        PrimaryMap::new(),
        memories,
        PrimaryMap::new(),
    );
    let data_initializers = Vec::new();
    let signatures = PrimaryMap::new();

    code_memory.publish();

    let import_obj_state = ImportObjState {
        calls: bound_functions,
        code_memory,
    };

    Ok(InstanceHandle::new(
        Rc::new(module),
        global_exports,
        finished_functions.into_boxed_slice(),
        imports,
        &data_initializers,
        signatures.into_boxed_slice(),
        None,
        Box::new(import_obj_state),
    )
    .expect("instance"))
}

/// We don't expect trampoline compilation to produce any relocations, so
/// this `RelocSink` just asserts that it doesn't recieve any.
struct RelocSink {}

impl binemit::RelocSink for RelocSink {
    fn reloc_ebb(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _ebb_offset: binemit::CodeOffset,
    ) {
        panic!("trampoline compilation should not produce ebb relocs");
    }
    fn reloc_external(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _name: &ir::ExternalName,
        _addend: binemit::Addend,
    ) {
        panic!("trampoline compilation should not produce external symbol relocs");
    }
    fn reloc_constant(
        &mut self,
        _code_offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _constant_offset: ir::ConstantOffset,
    ) {
        panic!("trampoline compilation should not produce constant relocs");
    }
    fn reloc_jt(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _jt: ir::JumpTable,
    ) {
        panic!("trampoline compilation should not produce jump table relocs");
    }
}
